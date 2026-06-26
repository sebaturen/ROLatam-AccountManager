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
```
3. Invite the Bot to Your Server
Use the OAuth2 URL Generator in the developer portal to generate an invite link.
Make sure the bot has the following permissions:
- Send Messages
- Manage Messages
- Read Message History

4. Add Accounts
The bot reads account data from an `accounts.json` file. Use the following structure:
```
{
    "discord": [
        {
            "channel_id": "<discord channel id>",
            "message_id": "<first msg id, remove line on first run, then add with your ID>",
            "accounts": [
                {
                    "slug": "<pj name>",
                    "email": "<login email>",
                    "password": "<login password>",
                    "pin": "<login pin>",
                    "kafra": "<kafra pin>",
                    "OTP": "<OTP code>"
                },
                { ... more accounts }...
            ]
        },
        {
            "channel_id": "11111",
            "message_id": "22222",
            "accounts": [
                {
                "slug": "example2",
                "email": "example2@mail.com",
                "password": "password123",
                "pin": "0000",
                "kafra": "0000",
                "OTP": "112233"
                }
            ]
        }
    ]
}
```

5. Run the Bot
```
python app.py
```

6. Add the Message ID
Once the bot sends its first message, right-click the message and select “Copy Message ID”.
Update your `accounts.json`
```
    "message_id": "<your msg ID>",
```

#### Disclaimer
Client app is auto develop by Copilot / Claude IA :eyes:

## Client (Windows Auto-Login)

The client automates Ragnarok Online login using external USB HID hardware (Raspberry Pi Pico) and OpenCV template matching.

### Requirements
- Windows 10/11
- Raspberry Pi Pico with CircuitPython HID firmware (see `client/firmware/`)
- OpenCV 4.10 DLL

### Building from Source
1. Install Rust toolchain
2. Set OpenCV environment variables:
```cmd
set OPENCV_INCLUDE_PATHS=C:\path\to\opencv\build\include
set OPENCV_LINK_PATHS=C:\path\to\opencv\build\x64\vc16\lib
set LIBCLANG_PATH=C:\path\to\LLVM\bin
```
3. Build release:
```cmd
cd client
.\build.bat --release
```
4. Copy `opencv_world4100.dll` to executable directory

### Distribution Package
```
ROLatamClient\
├── rolatam_client.exe
├── opencv_world4100.dll
└── resources\
    ├── login_page.png
    ├── otp_page.png
    ├── server_select.png
    ├── pin_select.png
    ├── ok_pin.png
    └── 0.bmp - 9.bmp
```

### Usage
1. Install client and connect Raspberry Pi Pico HID device
2. Run `rolatam_client.exe` (creates `config.json` if missing)
3. Get authentication key from Discord bot using `/auth`
4. Edit `config.json`:
```json
{
  "server_url": "ws://your-server:3000",
  "temp_key": "your-auth-key-from-discord"
}
```
5. Open Ragnarok Online to login screen
6. Use Discord command: `/login <account_number>`
7. Client automatically completes login sequence

### How It Works
- Client connects to server via WebSocket
- Server sends login request with credentials
- Client validates HID device connection
- Client performs template matching to find UI elements
- Client sends input via USB HID device
- Entire login process takes ~10-12 seconds