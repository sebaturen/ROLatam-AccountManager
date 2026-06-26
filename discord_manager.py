
import discord
import pyotp
import json
import asyncio
from datetime import datetime, timedelta
from discord.ext import tasks
from discord import app_commands

# formating     ----
COL_WITH_PJ = 17
COL_WITH_EMAIL = 33
COL_WITH_PASS = 15
COL_WITH_PIN = 6
COL_WITH_N = 3
# end formating ----

class DiscordManager(discord.Client):
    channels = {}
    target_msgs = {}

    def __init__(self, client_manager, *args, **kwargs):
        intents = discord.Intents.default()
        super().__init__(intents=intents, *args, **kwargs)
        self.tree = discord.app_commands.CommandTree(self)
        self.client_manager = client_manager

    async def setup_hook(self):
        self.register_commands()
        await self.tree.sync()
        print(f'[DiscordManager] Slash commands synced')

    async def on_ready(self):
        print(f'[DiscordManager] Logged on as {self.user}!')

        await self.start_aligned_loop()

    async def start_aligned_loop(self):
        now = datetime.now()
        
        if now.second < 1:
            target = now.replace(second=1, microsecond=0)
        elif now.second < 31:
            target = now.replace(second=31, microsecond=0)
        else:
            target = (now + timedelta(minutes=1)).replace(second=1, microsecond=0)

        delay = (target - now).total_seconds()
        print(f"[DiscordManager] Start {delay:.2f} seconds to align loop")
        await asyncio.sleep(delay)
        self.update_accounts.start()

    @tasks.loop(seconds=30.0)
    async def update_accounts(self):
        try:
            await self.report_accounts()
            print(f"[DiscordManager][Info] Process completed")
        except Exception as e:
            print(f"[DiscordManager][Error!] Critical error, can't send accounts, try again in 30s --> {e}")

    async def report_accounts(self):
        # base content
        header = f"| {'N':^{COL_WITH_N}}| {'PJ':^{COL_WITH_PJ}}| {'Email':^{COL_WITH_EMAIL}}| {'Password':^{COL_WITH_PASS}}| {'OTP':^{COL_WITH_PIN}}| {'Pin':^{COL_WITH_PIN}}| {'Kafra':^{COL_WITH_PIN}}|"
        sep_line = f"+-{'-' * COL_WITH_N}+-{'-' * COL_WITH_PJ}+-{'-' * COL_WITH_EMAIL}+-{'-' * COL_WITH_PASS}+-{'-' * COL_WITH_PIN}+-{'-' * COL_WITH_PIN}+-{'-' * COL_WITH_PIN}+"
        header = f"\n{sep_line}\n{header}\n{sep_line}"

        # Reload account info
        with open('accounts.json') as f:
            accounts = json.load(f)

        # foreach discord account sections
        acc_total = 0
        for d_info in accounts['discord']:
            print(f"[DiscordManager][Info] Preparing account {d_info['channel_id']}")

            if d_info['channel_id'] not in self.channels:
                self.channels[d_info['channel_id']] = await self.fetch_channel(d_info['channel_id'])
            channel = self.channels[d_info['channel_id']]

            if not d_info['message_id']:
                await channel.send("COPY MSG ID")
                return
            else:
                self.target_msgs[d_info['message_id']] = []
                self.target_msgs[d_info['message_id']].append(await channel.fetch_message(d_info['message_id']))
            target_msg = self.target_msgs[d_info['message_id']]

            acc_details = await self.get_accounts(d_info['accounts'], acc_total)
            now = datetime.now().strftime("%H:%M:%S")
            
            # printing msg
            acc_msg = header
            i = 0
            while len(acc_details) > 0:
                next_line = acc_details.pop(0)
                acc_msg += f"\n{next_line}\n{sep_line}"

                next_item = acc_details[0] if acc_details else ""
                will_exceed_limit = len(acc_msg) + len(next_item) + len(sep_line) >= 1900

                if not acc_details or will_exceed_limit:
                    if len(target_msg) <= i:
                        next_msg = await self.next_mssg(target_msg[i - 1], channel)
                        target_msg.append(next_msg)
                    
                    final_msg = f"```{acc_msg}```"
                    if not acc_details:
                        final_msg += f"\nLastUpdate: {now}"

                    await target_msg[i].edit(content=final_msg)
                    i += 1
                    acc_msg = header
    
    async def next_mssg(self, pre_msg, channel):
        async for msg in channel.history(after=pre_msg, limit=1, oldest_first=True):
            return msg
        
        return await channel.send("adding...")

    async def get_accounts(self, accounts, acc_total):

        acc_dump = []

        for acc in accounts:
            acc_total += 1
            otp_code = ''
            if 'OTP' in acc:
                totp = pyotp.TOTP(acc['OTP'])
                otp_code = totp.now()
            
            acc_info  = f"| {acc_total:02} | "
            acc_info += f"{acc['slug']:<{COL_WITH_PJ}}| "
            acc_info += f"{acc['email']:<{COL_WITH_EMAIL}}| "
            acc_info += f"{acc['password']:<{COL_WITH_PASS}}| "
            acc_info += f"{otp_code:^{COL_WITH_PIN}}| "
            acc_info += f"{acc['pin']:^{COL_WITH_PIN}}| "
            acc_info += f"{acc['kafra']:^{COL_WITH_PIN}}|"

            acc_dump.append(acc_info)

        return acc_dump

    def register_commands(self):
        @self.tree.command(name="auth", description="Generate login key")
        async def auth(interaction: discord.Interaction):
            temp_key = self.client_manager.generate_temp_key(interaction.user.id, interaction.user.name)
            await interaction.response.send_message(f"Auth key: {temp_key}", ephemeral=True)
            asyncio.create_task(self.delete_interaction(interaction))

        @self.tree.command(name="login", description="Login account")
        @app_commands.describe(number="Number of account to login")
        async def login(interaction: discord.Interaction, number: int):
            auth_stats = self.client_manager.has_logged_in(interaction.user.id)
            if auth_stats:
                active_connection = self.client_manager.has_active_connection(interaction.user.id)
                if active_connection:
                    asyncio.create_task(self.client_manager.send_login_command(interaction, number))
                    await interaction.response.send_message(f"Starting login, open a Ragnarok client...", ephemeral=True)
                else:
                    await interaction.response.send_message(f"Client is not connected", ephemeral=True)
            else:
                await interaction.response.send_message(f"Account not loggin", ephemeral=True)
            asyncio.create_task(self.delete_interaction(interaction))

    async def delete_interaction(self, interaction):
        await asyncio.sleep(120.0)
        try:
            await interaction.delete_original_response()
        except Exception as e:
            print(f"[DiscordManager] No se pudo borrar el mensaje: {e}")