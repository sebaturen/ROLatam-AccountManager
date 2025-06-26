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

# formating     ----
COL_WITH_PJ = 13
COL_WITH_EMAIL = 33
COL_WITH_PASS = 18
COL_WITH_PIN = 7
# end formating ----

class AccountManager(discord.Client):
    channel = None
    target_msg = []
    accounts = None

    async def on_ready(self):
        print(f'Logged on as {self.user}!')

        self.channel = await self.fetch_channel(ACCOUNT_CHANNEL)
        if not MESSAGE_ID:
            await channel.send("COPY MSG ID")
        else:
            msg = await self.channel.fetch_message(MESSAGE_ID)
            self.target_msg.append(msg)
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

            with open('accounts.json') as f:
                self.accounts = json.load(f)['accounts']

            now = datetime.now().strftime("%H:%M:%S")
            acc_details = await self.get_accounts()


            sep_line = f"+-{'-' * COL_WITH_PJ}+-{'-' * COL_WITH_EMAIL}+-{'-' * COL_WITH_PASS}+-{'-' * COL_WITH_PIN}+-{'-' * COL_WITH_PIN}+-{'-' * COL_WITH_PIN}+"
            header = f"\n{sep_line}\n{acc_details.pop(0)}\n{sep_line}"
            acc_msg = header
            i = 0

            while len(acc_details) > 0:
                acc_msg += f"\n{acc_details.pop(0)}"
                acc_msg += f"\n{sep_line}"

                if len(acc_details) == 0 or ( (len(acc_msg) + len(acc_details[0]) + len(sep_line)) >= 1900):
                    if len(self.target_msg) <= i:
                        await self.next_mssg(self.target_msg[i-1])
                    final_msg = f"```{acc_msg}```"
                    if len(acc_details) == 0:
                        final_msg += f"\nLastUpdate: {now}"
                    await self.target_msg[i].edit(content=final_msg)
                    i += 1
                    acc_msg = header
    
    async def next_mssg(self, pre_msg):
        async for msg in self.channel.history(after=pre_msg, limit=1, oldest_first=True):
            self.target_msg.append(msg)
            return
        
        new_msg = await self.channel.send("adding...")
        self.target_msg.append(new_msg)

    async def get_accounts(self):

        acc_dump = []
        acc_dump.append(f"| {'PJ':^{COL_WITH_PJ}}| {'Email':^{COL_WITH_EMAIL}}| {'Password':^{COL_WITH_PASS}}| {'OTP':^{COL_WITH_PIN}}| {'Pin':^{COL_WITH_PIN}}| {'Kafra':^{COL_WITH_PIN}}|")

        for acc in self.accounts:
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