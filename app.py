import os
import asyncio
from dotenv import load_dotenv
from websocket_manager import WebSocketManager
from client_manager import ClientManager
from discord_manager import DiscordManager

load_dotenv()
DISCORD_TOKEN = os.getenv('DISCORD_TOKEN')

async def main():
    clients = ClientManager()

    # Star websocket
    websocket = WebSocketManager(clients)
    asyncio.create_task(websocket.start())

    # Discord BOT
    discord = DiscordManager(clients)
    try:
        await discord.start(DISCORD_TOKEN)
    except KeyboardInterrupt:
        discord.close()


if __name__ == "__main__":
    asyncio.run(main())