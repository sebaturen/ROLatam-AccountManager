# ROLatam Account Manager
A simple Discord bot to help manage and share Ragnarok Online accounts.
The bot posts and continuously edits a single message to keep the OTP codes up to date.

## Setup Instructions
1. Create Your Discord Bot
Go to the Discord Developer Portal and create a new application. Enable the bot and copy the token. (https://discord.com/developers/applications)

3. Create a `.env` File
Add the following environment variables:
```
DISCORD_TOKEN=<your discord token>
ACCOUNT_CHANNEL=<ID of the channel where the bot should post account data>
```
3. Invite the Bot to Your Server
Use the OAuth2 URL Generator in the developer portal to generate an invite link.
Make sure the bot has the following permissions:
- Send Messages
- Manage Messages
- Read Message History

4. Run the Bot
```
python app.py
```

5. Add the Message ID
Once the bot sends its first message, right-click the message and select “Copy Message ID”.
Update your `.env` file:
```
DISCORD_TOKEN=<your discord token>
ACCOUNT_CHANNEL=<channel ID>
MESSAGE_ID=<message ID from the bot>
```

6. Add Accounts
The bot reads account data from an `accounts.json` file. Use the following structure:
```
{
    "accounts": [
        {
            "slug": "<pj name>",
            "email": "<login email>",
            "password": "<login password>",
            "pin": "<login pin>",
            "kafra": "<kafra pin>",
            "OTP": "<OTP code>"
        },
        {
          "slug": "example2",
          "email": "example2@mail.com",
          "password": "password123",
          "pin": "0000",
          "kafra": "0000",
          "OTP": "112233"
        }
        { ... more accounts }...
    ]
}
```
