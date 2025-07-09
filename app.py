import os
import discord
import pyotp
import json
import asyncio
from dotenv import load_dotenv
from datetime import datetime, timedelta
from discord.ext import tasks

load_dotenv()
DISCORD_TOKEN = os.getenv('DISCORD_TOKEN')

# formating     ----
COL_WITH_PJ = 13
COL_WITH_EMAIL = 33
COL_WITH_PASS = 18
COL_WITH_PIN = 7
# end formating ----

class AccountManager(discord.Client):
    channels = {}
    target_msgs = {}

    async def on_ready(self):
        print(f'Logged on as {self.user}!')

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
        print(f"Start {delay:.2f} seconds to align loop")
        await asyncio.sleep(delay)
        self.update_accounts.start()

    @tasks.loop(seconds=30.0)
    async def update_accounts(self):
        try:
            await self.report_accounts()
        except Exception as e:
            print(f"[Error!] Critical error, can't send accounts, try again in 30s --> {e}")

    async def report_accounts(self):
        # base content
        header = f"| {'PJ':^{COL_WITH_PJ}}| {'Email':^{COL_WITH_EMAIL}}| {'Password':^{COL_WITH_PASS}}| {'OTP':^{COL_WITH_PIN}}| {'Pin':^{COL_WITH_PIN}}| {'Kafra':^{COL_WITH_PIN}}|"
        sep_line = f"+-{'-' * COL_WITH_PJ}+-{'-' * COL_WITH_EMAIL}+-{'-' * COL_WITH_PASS}+-{'-' * COL_WITH_PIN}+-{'-' * COL_WITH_PIN}+-{'-' * COL_WITH_PIN}+"
        header = f"\n{sep_line}\n{header}\n{sep_line}"

        # Reload account info
        with open('accounts.json') as f:
            accounts = json.load(f)

        # foreach discord account sections
        for d_info in accounts['discord']:

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

            acc_details = await self.get_accounts(d_info['accounts'])
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

    async def get_accounts(self, accounts):

        acc_dump = []

        for acc in accounts:
            otp_code = ''
            if 'OTP' in acc:
                totp = pyotp.TOTP(acc['OTP'])
                otp_code = totp.now()
            
            acc_info  = f"| {acc['slug']:<{COL_WITH_PJ}}| "
            acc_info += f"{acc['email']:<{COL_WITH_EMAIL}}| "
            acc_info += f"{acc['password']:<{COL_WITH_PASS}}| "
            acc_info += f"{otp_code:^{COL_WITH_PIN}}| "
            acc_info += f"{acc['pin']:^{COL_WITH_PIN}}| "
            acc_info += f"{acc['kafra']:^{COL_WITH_PIN}}|"

            acc_dump.append(acc_info)

        return acc_dump

# Discord bot Intents (permissions):
intents = discord.Intents.default()
#intents.message_content = True

# Run BOT
client = AccountManager(intents=intents)
client.run(DISCORD_TOKEN)