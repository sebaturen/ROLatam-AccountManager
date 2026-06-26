import asyncio
import websockets
import json

class WebSocketManager:

    def __init__(self, client_manager, host='0.0.0.0', port=8765):
        self.client_manager = client_manager
        self.host = host
        self.port = port
        self.server = None

    async def handle_client(self, websocket):
        client_id = None
        perma_key = None
        try:
            print(f"[WebSocket] New connection from {websocket.remote_address}")
            
            # first msg should be login~
            auth_msg = await websocket.recv()
            auth_data = json.loads(auth_msg)
            auth_type = auth_data.get('type')

            if auth_type == 'auth_temp':
                temp_key = auth_data.get('temp_key')
                client_id = self.client_manager.validate_temp_key(temp_key)

                if client_id:
                    perma_key = self.client_manager.generate_permanent_key(client_id)
                    response = {
                        'type': 'auth_success',
                        'permanent_key': perma_key,
                        'client_id': client_id
                    }
                    await websocket.send(json.dumps(response))
                    self.client_manager.register_connection(perma_key, websocket, client_id)
                    print(f"[WebSocket] Permakey generated for client {client_id}")
                else:
                    response = {
                        'type': 'auth_error',
                        'message': 'Invalid temporal key'
                    }
                    await websocket.send(json.dumps(response))
                    print(f"[WebSocket] Invalid temp key")
                    return
            elif auth_type == 'auth_permanent':
                perma_key = auth_data.get('permanent_key')
                print(f"[WebSocket] Reconnecting client {perma_key}")
                client_id = auth_data.get('client_id')
                
                if self.client_manager.validate_permanent_key(perma_key, client_id):
                    response = {
                        'type': 'auth_success',
                        'permanent_key': perma_key,
                        'client_id': client_id
                    }
                    await websocket.send(json.dumps(response))
                    self.client_manager.register_connection(perma_key, websocket, client_id)
                    print(f"[WebSocket] Client {client_id} was reconnected")
                else:
                    response = {
                        'type': 'auth_error',
                        'message': 'Invalid permanent key for client id'
                    }
                    await websocket.send(json.dumps(response))
                    return
            else:
                print(f"[WebSocket] No type for client {client_id}")
                response = {
                    'type': 'auth_error',
                    'message': 'Unknown authentication type'
                }
                await websocket.send(json.dumps(response))
                return
            
            async for message in websocket:
                try:
                    data = json.loads(message)
                    print(f"[WebSocket] Process client request: {client_id} | {data}")
                    await self.handle_message(data, websocket, client_id)
                except json.JSONDecodeError:
                    print(f"[WebSocket] Invalid JSON from client {client_id}")
                except Exception as e:
                    print(f"[WebScoket] Error handling message from client {client_id}: {e}")

        except websockets.exceptions.ConnectionClosed:
            print(f"[WebSocket] Connection close by client {client_id or 'unknown'}")
        except Exception as e:
            print(f"[WebSocket] Error with client {client_id or 'unkown'}: {e}")
        finally:
            if perma_key:
                _ = 1
                # unregister connection?

    async def handle_message(self, data, websocket, client_id):
        msg_type = data.get('type')
        if msg_type == 'reporting':
            await self.client_manager.report_progress(client_id, data)
        print(f"[WebSocket] {msg_type} received from client {client_id}")


    async def start(self):
        self.server = await websockets.serve(
            self.handle_client,
            self.host,
            self.port
        )

        print(f"[WebSocket] Server running on ws://{self.host}:{self.port}")
        await asyncio.Future()