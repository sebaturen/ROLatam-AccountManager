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
ACCOUNT_CHANNEL = os.getenv('ACCOUNT_CHANNEL')
MESSAGE_ID = os.getenv('MESSAGE_ID')

class AccountManager(discord.Client):
    target_msg = None
    with open('accounts.json') as f:
        accounts = json.load(f)['accounts']

    async def on_ready(self):
        print(f'Logged on as {self.user}!')

        channel = await self.fetch_channel(ACCOUNT_CHANNEL)
        if not MESSAGE_ID:
            await channel.send("COPY MSG ID")
        else:
            self.target_msg = await channel.fetch_message(MESSAGE_ID)
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
        if self.target_msg:
            now = datetime.now().strftime("%H:%M:%S")
            acc_msg = await self.make_account_msg()
            await self.target_msg.edit(content=f"{acc_msg}")

    async def make_account_msg(self):
        # formating     ----
        pj_col_with = 13
        email_col_with = 33
        pass_col_with = 18
        pin_pass_col_with = 6
        # end formating ----

        acc_dump = []
        acc_dump.append(f"{'PJ':<{pj_col_with}}| {'Email':<{email_col_with}}| {'Password':<{pass_col_with}}| {'Pin':<{pin_pass_col_with}}| {'Kafra':<{pin_pass_col_with}}| {'OTP':<{pin_pass_col_with}}")
        acc_dump.append("-" * len(acc_dump[0]))

        for acc in self.accounts:
            otp_code = ''
            if 'OTP' in acc:
                totp = pyotp.TOTP(acc['OTP'])
                otp_code = totp.now()
            
            acc_info  = f"{acc['slug']:<{pj_col_with}}| "
            acc_info += f"{acc['email']:<{email_col_with}}| "
            acc_info += f"{acc['password']:<{pass_col_with}}| "
            acc_info += f"{acc['pin']:<{pin_pass_col_with}}| "
            acc_info += f"{acc['kafra']:<{pin_pass_col_with}}| "
            acc_info += f"{otp_code:<{pin_pass_col_with}}"

            acc_dump.append(acc_info)

        acc_joined = "\n".join(acc_dump)
        return f"```{acc_joined}```"

# Discord bot Intents (permissions):
intents = discord.Intents.default()
#intents.message_content = True

# Run BOT
client = AccountManager(intents=intents)
client.run(DISCORD_TOKEN)