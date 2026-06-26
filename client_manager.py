import os
import uuid
import json
import pyotp
import discord
from datetime import datetime
from typing import Dict

class ClientManager:
    
    DATA_FILE = 'clients_data.json'

    def __init__(self):
        self.clients: Dict[str, dict] = {}
        self.connected_clients: Dict[str, dict] = {}
        self.load_data()

    def load_data(self):
        if os.path.exists(ClientManager.DATA_FILE):
            try:
                with open(ClientManager.DATA_FILE, 'r') as f:
                    self.clients = json.load(f)
                print(f"[ClientManager] Clients loaded {len(self.clients)}")
            except Exception as e:
                print(f"[ClientManager] Error loading data: {e}")
        else:
            with open(ClientManager.DATA_FILE, 'w') as f:
                json.dump(self.clients, f, indent=2)

    def save_data(self):
        try:
            with open(ClientManager.DATA_FILE, 'w', encoding='utf-8') as f:
                json.dump(self.clients, f, indent=2)
            print(f"[ClientManager] Data saved")
        except Exception as e:
            print(f"[ClientManager] Error saving data: {e}")

    def generate_temp_key(self, id, name):
        client = self.clients.get(str(id))
        if client and client['status'] == 'active':
            return "Account already registered"

        temp_key = f"TEMP_{uuid.uuid4().hex[:16].upper()}"
        client_id = str(id)

        self.clients[client_id] = {
            'temp_key': temp_key,
            'permanent_key': None,
            'name': name,
            'created_at': datetime.now().isoformat(),
            'last_connection': None,
            'status': 'pending'
        }

        self.save_data()
        print(f"[ClientManager] Generate temp key for '{name}': {temp_key}")
        return temp_key
    
    def validate_temp_key(self, temp_key):
        for client_id, data in self.clients.items():
            if data.get('temp_key') == temp_key and data.get('status') == 'pending':
                return client_id
        return None
    
    def generate_permanent_key(self, client_id):
        if client_id not in self.clients:
            raise ValueError(f"Client {client_id} not found")
        
        permanent_key = f"PERM_{uuid.uuid4().hex.upper()}"
        self.clients[client_id]['permanent_key'] = permanent_key
        self.clients[client_id]['status'] = 'active'
        self.clients[client_id]['last_connection'] = datetime.now().isoformat()
        
        self.save_data()
        print(f"[ClientManager] Generated permanent key for client {client_id}")
        return permanent_key

    def register_connection(self, permanent_key: str, websocket, client_id: str):
        self.connected_clients[client_id] = {
            'websocket': websocket,
            'client_id': client_id,
            'perma_key': permanent_key,
            'connected_at': datetime.now().isoformat()
        }
        print(f"[ClientManager] Client {client_id} connected")

    def unregister_connection(self, client_id: str):
        if client_id in self.connected_clients:
            del self.connected_clients[client_id]
            print(f"[ClientManager] Client {client_id} disconnected")

    def has_logged_in(self, user_id):
        if str(user_id) in self.clients:
            return True
        return False
    
    def has_active_connection(self, user_id):
        if str(user_id) in self.connected_clients:
            return True
        return False
    
    async def send_login_command(self, interaction: discord.Interaction, account_number):
        user_id = str(interaction.user.id)
        if self.has_logged_in(user_id) and self.has_active_connection(user_id):
            acc_info = self.get_account_by_number(account_number)
            client = self.connected_clients.get(user_id)
            client['interaction'] = interaction
            websocket = client['websocket']

            totp = pyotp.TOTP(acc_info['OTP'])
            login_data = {
                'type': 'login',
                'email': acc_info['email'],
                'password': acc_info['password'],
                'pin': acc_info['pin'],
                'otp': totp.now()
            }
            await websocket.send(json.dumps(login_data))
            print(f"[ClientManager] Login command sent to user {user_id} for account {acc_info['email']}")
            return True
        return False
    
    def validate_permanent_key(self, perma_key, client_id):
        client = self.clients.get(client_id)
        if not client:
            return False
        
        if client.get('permanent_key') == perma_key:
            client['last_connection'] = datetime.now().isoformat()
            self.save_data()
            return True
        
        return False
    
    def get_account_by_number(self, number):
        with open('accounts.json', 'r', encoding='utf-8') as f:
            accounts_data = json.load(f)
        all_accounts = []
        for ds in accounts_data['discord']:
            all_accounts.extend(ds['accounts'])
        if 1 <= number <= len(all_accounts):
            return all_accounts[number-1]
        
        print(f"[ClientManager] Invalid account number {number}")
        return None

    async def report_progress(self, client_id, data):
        if str(client_id) in self.connected_clients:
            client_conn = self.connected_clients[str(client_id)]
            if 'interaction' in client_conn:
                interacion : discord.Interaction = client_conn['interaction']
                await interacion.edit_original_response(content=data['message'])