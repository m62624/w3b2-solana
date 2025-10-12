import os
import random
import asyncio
import time
import threading
import streamlit as st
from http.server import HTTPServer, SimpleHTTPRequestHandler

import anchorpy
from solders.keypair import Keypair
from solders.pubkey import Pubkey
from anchorpy import Program, Provider, Wallet
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Confirmed
import grpc
import json

# Generate these files using:
# python -m grpc_tools.protoc -I../proto --python_out=. --pyi_out=. --grpc_python_out=. ../proto/gateway.proto ../proto/types.proto ../proto/protocols.proto
try:
    import gateway_pb2, gateway_pb2_grpc, types_pb2, protocols_pb2
except ImportError:
    st.error("gRPC files not found. Please generate them using the command in the source code.")
    st.stop()

# --- Constants ---
# These are now read from environment variables at the bottom of the file.
GATEWAY_HOST = os.environ.get("GATEWAY_HOST", "localhost")
GATEWAY_PORT = int(os.environ.get("GATEWAY_PORT", "50051"))
SOLANA_RPC_URL = os.environ.get("SOLANA_RPC_URL", "http://localhost:8899")
PROGRAM_ID_STR = os.environ.get("PROGRAM_ID", "HykRMCadVCe49q4GVrXKTwLG3fqCEgd5W5qQqN3AFAEY") # Fallback for local dev
DEVNET_RPC_URL = "https://api.devnet.solana.com"

class GatewayClient:
    def __init__(self, host, port, program_id_str):
        channel_address = f'{host}:{port}'
        print(f"Connecting to gRPC gateway at {channel_address}...")
        self.channel = grpc.insecure_channel(channel_address)
        self.stub = gateway_pb2_grpc.BridgeGatewayServiceStub(self.channel)
        self.program_id = Pubkey.from_string(program_id_str)
        self._wait_for_channel_ready()

    def _wait_for_channel_ready(self):
        print("Waiting for gRPC channel to be ready...")
        grpc.channel_ready_future(self.channel).result(timeout=30)
        print("gRPC channel is ready.")

    def listen_as_user(self, user_pda: Pubkey):
        request = gateway_pb2.ListenRequest(pda=str(user_pda))
        return self.stub.ListenAsUser(request)

class Bot:
    def __init__(self, name: str, keypair: Keypair, pda: Pubkey, app: 'SocialApp'):
        self.name = name
        self.keypair = keypair
        self.pda = pda
        self.app = app
        self.message_count = 0
        self.program = self.app.get_program_for_keypair(keypair)

    async def send_message(self, recipient_name: str, message: str):
        print(f"[{self.name}] -> [{recipient_name}]: {message}")
        payload = f"MSG:{message}".encode('utf-8')
        
        # We need to create a dummy oracle signature for free commands
        oracle_kp = Keypair()
        command_id = 1
        price = 0
        timestamp = int(time.time())
        msg_to_sign = command_id.to_bytes(2, 'little') + price.to_bytes(8, 'little') + timestamp.to_bytes(8, 'little', signed=True)
        oracle_sig = oracle_kp.sign(msg_to_sign)

        await self.program.methods.user_dispatch_command(
            command_id, price, timestamp, payload
        ).accounts({
            "authority": self.keypair.pubkey(), 
            "user_profile": self.pda, 
            "admin_profile": self.app.admin_pda,
            "instructions": anchorpy.SYSVAR_INSTRUCTIONS_PUBKEY,
        }).pre_instructions([
            anchorpy.Program.build_ed25519_instruction(oracle_kp.pubkey(), bytes(oracle_sig), msg_to_sign)
        ]).rpc()
        self.message_count += 1

    async def simulate_file_transfer(self, recipient_keypair: Keypair, recipient_name: str):
        session_id = random.randint(0, 2**64 - 1)
        print(f"[{self.name}] -> [{recipient_name}]: Initiating simulated file transfer (Session: {session_id})")
        http_port = 8000 + random.randint(0, 100)
        handler = SimpleHTTPRequestHandler
        httpd = HTTPServer(("", http_port), handler)
        def run_server():
            with open("dummy_file.zip", "w") as f: f.write("dummy")
            httpd.serve_forever()
        server_thread = threading.Thread(target=run_server, daemon=True)
        server_thread.start()

        encrypted_key = b"encrypted_key_for_" + recipient_name.encode('utf-8')
        destination = protocols_pb2.Destination(url=f"http://localhost:{http_port}/dummy_file.zip")
        command_config = protocols_pb2.CommandConfig(session_id=session_id, encrypted_session_key=encrypted_key, destination=destination, meta=b"file_transfer")
        payload = command_config.SerializeToString()

        oracle_kp = Keypair()
        command_id = 2
        price = 0
        timestamp = int(time.time())
        msg_to_sign = command_id.to_bytes(2, 'little') + price.to_bytes(8, 'little') + timestamp.to_bytes(8, 'little', signed=True)
        oracle_sig = oracle_kp.sign(msg_to_sign)

        await self.program.methods.user_dispatch_command(
            command_id, price, timestamp, payload
        ).accounts({
            "authority": self.keypair.pubkey(), 
            "user_profile": self.pda, 
            "admin_profile": self.app.admin_pda,
            "instructions": anchorpy.SYSVAR_INSTRUCTIONS_PUBKEY,
        }).pre_instructions([
            anchorpy.Program.build_ed25519_instruction(oracle_kp.pubkey(), bytes(oracle_sig), msg_to_sign)
        ]).rpc()
        self.message_count += 1

        print(f"[{recipient_name}]: 'Downloading' from http://localhost:{http_port}/dummy_file.zip")
        httpd.shutdown()
        print(f"[{self.name}] HTTP server stopped.")

        await self.program.methods.log_action(session_id, 200).accounts({
            "authority": self.keypair.pubkey(),
            "user_profile": self.pda, 
            "admin_profile": self.app.admin_pda
        }).rpc()
        
        recipient_pda, _ = Pubkey.find_program_address([b"user", bytes(recipient_keypair.pubkey()), bytes(self.app.admin_pda)], self.app.program_id)
        recipient_program = self.app.get_program_for_keypair(recipient_keypair)
        await recipient_program.methods.log_action(session_id, 200).accounts({
            "authority": recipient_keypair.pubkey(),
            "user_profile": recipient_pda, 
            "admin_profile": self.app.admin_pda
        }).rpc()

    async def use_paid_feature(self, recipient_name: str):
        price = 1_000_000
        print(f"[{self.name}] -> [{recipient_name}]: Using paid feature 'Send Sticker' for {price} lamports.")
        payload = b"STICKER:smiley_face"

        oracle_kp = self.app.admin # In a real app, this would be a separate oracle key
        command_id = 3
        timestamp = int(time.time())
        msg_to_sign = command_id.to_bytes(2, 'little') + price.to_bytes(8, 'little') + timestamp.to_bytes(8, 'little', signed=True)
        oracle_sig = oracle_kp.sign(msg_to_sign)

        await self.program.methods.user_dispatch_command(
            command_id, price, timestamp, payload
        ).accounts({
            "authority": self.keypair.pubkey(), 
            "user_profile": self.pda, 
            "admin_profile": self.app.admin_pda,
            "instructions": anchorpy.SYSVAR_INSTRUCTIONS_PUBKEY,
        }).pre_instructions([
            anchorpy.Program.build_ed25519_instruction(oracle_kp.pubkey(), bytes(oracle_sig), msg_to_sign)
        ]).rpc()
        self.message_count += 1

class SocialApp:
    def __init__(self):
        # gRPC client is now only for listening to events
        self.gateway_client = GatewayClient(GATEWAY_HOST, GATEWAY_PORT, PROGRAM_ID_STR)
        
        # Setup anchorpy
        self.program_id = Pubkey.from_string(PROGRAM_ID_STR)
        self.client = AsyncClient(SOLANA_RPC_URL, commitment=Confirmed)
        
        # Use a well-known path for the IDL inside the container
        idl_path = "/app/artifacts/w3b2_solana_program.json"
        if not os.path.exists(idl_path):
            st.error(f"IDL file not found at {idl_path}. Make sure artifacts are mounted correctly.")
            st.stop()
        with open(idl_path) as f:
            idl = anchorpy.Idl.from_json(f.read())
        
        self.admin, self.alice, self.bob, self.human_user = Keypair(), Keypair(), Keypair(), Keypair()
        self.admin_program = Program(idl, self.program_id, Provider(self.client, Wallet(self.admin)))

        self.admin_pda, _ = Pubkey.find_program_address([b"admin", bytes(self.admin.pubkey())], self.program_id)
        self.alice_pda, _ = Pubkey.find_program_address([b"user", bytes(self.alice.pubkey()), bytes(self.admin_pda)], self.program_id)
        self.bob_pda, _ = Pubkey.find_program_address([b"user", bytes(self.bob.pubkey()), bytes(self.admin_pda)], self.program_id)
        self.human_user_pda, _ = Pubkey.find_program_address([b"user", bytes(self.human_user.pubkey()), bytes(self.admin_pda)], self.program_id)

        self.bot_alice = Bot("Alice", self.alice, self.alice_pda, self)
        self.bot_bob = Bot("Bob", self.bob, self.bob_pda, self)

        self.stop_event = threading.Event()
        self.lock = threading.Lock()

    def get_program_for_keypair(self, keypair: Keypair) -> Program:
        return Program(self.admin_program.idl, self.program_id, Provider(self.client, Wallet(keypair)))

    async def airdrop(self, pubkey: Pubkey, lamports: int):
        try:
            resp = await self.client.request_airdrop(pubkey, lamports)
            await self.client.confirm_transaction(resp.value, commitment=Confirmed)
            print(f"Airdropped {lamports} to {pubkey} on {SOLANA_RPC_URL}.")
        except Exception as e:
            print(f"Airdrop failed: {e}")

    async def setup_onchain_state(self):
        print("\n--- Starting On-Chain Setup ---")
        print("1. Airdropping SOL...")
        await self.airdrop(self.admin.pubkey(), 1_000_000_000)
        await self.airdrop(self.alice.pubkey(), 1_000_000_000)
        await self.airdrop(self.bob.pubkey(), 1_000_000_000)
        await self.airdrop(self.human_user.pubkey(), 1_000_000_000)

        print("\n2. Registering Admin Profile...")
        sig = await self.admin_program.methods.admin_register_profile(
            self.admin.pubkey() # communication_pubkey
        ).accounts({
            "authority": self.admin.pubkey(), 
            "admin_profile": self.admin_pda, 
            "system_program": anchorpy.SYSTEM_PROGRAM_ID
        }).rpc()
        print(f"  -> Signature: {sig}")

        print("\n3. Creating User Profiles...")
        alice_program = self.get_program_for_keypair(self.alice)
        sig = await alice_program.methods.user_create_profile(
            self.admin_pda, self.alice.pubkey()
        ).accounts({
            "authority": self.alice.pubkey(), "admin_profile": self.admin_pda, "user_profile": self.alice_pda, "system_program": anchorpy.SYSTEM_PROGRAM_ID
        }).rpc()
        print(f"  -> Alice's Profile. Signature: {sig}")

        bob_program = self.get_program_for_keypair(self.bob)
        sig = await bob_program.methods.user_create_profile(
            self.admin_pda, self.bob.pubkey()
        ).accounts({
            "authority": self.bob.pubkey(), "admin_profile": self.admin_pda, "user_profile": self.bob_pda, "system_program": anchorpy.SYSTEM_PROGRAM_ID
        }).rpc()
        print(f"  -> Bob's Profile. Signature: {sig}")

        print("\n4. Depositing funds for bots (0.1 SOL each)...")
        deposit_amount = 100_000_000
        sig = await alice_program.methods.user_deposit(deposit_amount).accounts({
            "authority": self.alice.pubkey(), "user_profile": self.alice_pda, "admin_profile": self.admin_pda, "system_program": anchorpy.SYSTEM_PROGRAM_ID
        }).rpc()
        print(f"  -> Deposited to Alice. Signature: {sig}")

        sig = await bob_program.methods.user_deposit(deposit_amount).accounts({
            "authority": self.bob.pubkey(), "user_profile": self.bob_pda, "admin_profile": self.admin_pda, "system_program": anchorpy.SYSTEM_PROGRAM_ID
        }).rpc()
        print(f"  -> Deposited to Bob. Signature: {sig}")
        print("\n--- On-Chain Setup Complete ---")

    def run_bot_chat(self):
        turn = 0
        while not self.stop_event.is_set():
            sender, receiver = (self.bot_alice, self.bot_bob) if turn % 2 == 0 else (self.bot_bob, self.bot_alice)

            if st.session_state.bot_status.get(sender.name) == "Banned":
                print(f"[{sender.name}] is banned, skipping turn.")
                turn +=1
                time.sleep(5)
                continue

            async def run_action():
                if sender.message_count > 0 and sender.message_count % 8 == 0:
                    await sender.use_paid_feature(receiver.name)
                elif sender.message_count > 0 and sender.message_count % 5 == 0:
                    await sender.simulate_file_transfer(receiver.keypair, receiver.name)
                else:
                    await sender.send_message(receiver.name, f"Hello! This is message #{sender.message_count + 1}")
            asyncio.run(run_action())
            turn += 1
            time.sleep(5)

    def listen_for_events(self, bot_name: str, bot_pda: Pubkey):
        while not self.stop_event.is_set():
            try:
                stream = self.gateway_client.listen_as_user(bot_pda)
                print(f"[{bot_name} Listener] Started listening for events on {bot_pda}")
                for item in stream:
                    if self.stop_event.is_set(): break
                    event_type = item.event.WhichOneof('event')
                    with self.lock:
                        if event_type == "user_command_dispatched":
                            event = item.event.user_command_dispatched
                            sender_pda_str = Pubkey.from_string(event.sender_user_pda)
                            sender_name = "You"
                            if sender_pda_str == self.alice_pda: sender_name = "Alice"
                            elif sender_pda_str == self.bob_pda: sender_name = "Bob"

                            payload = event.payload
                            if payload.startswith(b"MSG:"):
                                message = payload.decode('utf-8').split(":", 1)[1]
                                st.session_state.chat_log.append(f"[{sender_name}]: {message}")
                            elif event.command_id == 2:
                                st.session_state.chat_log.append(f"[{sender_name}] sent a file transfer request.")
                            elif event.command_id == 3:
                                st.session_state.chat_log.append(f"[{sender_name}] sent a paid sticker!")
                        elif event_type == "user_banned":
                            st.session_state.bot_status[bot_name] = "Banned"
                            st.session_state.chat_log.append(f"[ADMIN]: {bot_name} has been banned.")
                    st.experimental_rerun()
            except grpc.RpcError as e:
                print(f"[{bot_name} Listener] gRPC Error, reconnecting in 5s: {e.details()}")
                time.sleep(5)

    def start_all_threads(self):
        threading.Thread(target=self.run_bot_chat, daemon=True).start()
        threading.Thread(target=self.listen_for_events, args=("Alice", self.alice_pda), daemon=True).start()
        threading.Thread(target=self.listen_for_events, args=("Bob", self.bob_pda), daemon=True).start()

    def run_ui(self):
        st.title("Hybrid Social Network")
        col1, col2 = st.columns(2)
        with col1:
            st.metric(label="Alice Status", value=st.session_state.bot_status.get("Alice", "Unknown"))
            if st.button("Ban Alice"):
                st.session_state.chat_log.append("[ADMIN]: Banning Alice...")
                asyncio.run(self.admin_program.methods.admin_ban_user().accounts({
                    "authority": self.admin.pubkey(), "admin_profile": self.admin_pda, "user_profile": self.alice_pda
                }).rpc())
        with col2:
            st.metric(label="Bob Status", value=st.session_state.bot_status.get("Bob", "Unknown"))
            if st.button("Ban Bob"):
                st.session_state.chat_log.append("[ADMIN]: Banning Bob...")
                asyncio.run(self.admin_program.methods.admin_ban_user().accounts({
                    "authority": self.admin.pubkey(), "admin_profile": self.admin_pda, "user_profile": self.bob_pda
                }).rpc())

        st.divider()
        chat_history = "\n".join(st.session_state.chat_log)
        st.text_area("Chat Log", value=chat_history, height=400, key="chat_window", disabled=True)

        with st.form(key="message_form", clear_on_submit=True):
            recipient = st.selectbox("Recipient", ["Alice", "Bob"])
            message_text = st.text_input("Your message:")
            if st.form_submit_button(label='Send Message') and message_text:
                recipient_map = {"Alice": self.bot_alice, "Bob": self.bot_bob}

                st.session_state.chat_log.append(f"[You] -> [{recipient}]: {message_text}")
                payload = f"MSG:{message_text}".encode('utf-8')

                human_program = self.get_program_for_keypair(self.human_user)
                
                # Create a dummy oracle signature for this free command
                oracle_kp = Keypair()
                command_id = 1
                price = 0
                timestamp = int(time.time())
                msg_to_sign = command_id.to_bytes(2, 'little') + price.to_bytes(8, 'little') + timestamp.to_bytes(8, 'little', signed=True)
                oracle_sig = oracle_kp.sign(msg_to_sign)

                asyncio.run(human_program.methods.user_dispatch_command(
                    command_id, price, timestamp, payload
                ).accounts({
                    "authority": self.human_user.pubkey(), "user_profile": self.human_user_pda, "admin_profile": self.admin_pda, "instructions": anchorpy.SYSVAR_INSTRUCTIONS_PUBKEY
                }).pre_instructions([
                    anchorpy.Program.build_ed25519_instruction(oracle_kp.pubkey(), bytes(oracle_sig), msg_to_sign)
                ]).rpc())
                st.experimental_rerun()

def main():
    """
    Main function to run the Streamlit application.
    Handles the initialization of the app state on the first run.
    """
    if 'app' not in st.session_state:
        print("--- Initializing Social App State ---")
        app = SocialApp()
        st.session_state.app = app

        try:
            # This block runs only once on app startup
            asyncio.run(app.setup_onchain_state())
            # Create the human user profile after airdrop
            human_program = app.get_program_for_keypair(app.human_user)
            asyncio.run(human_program.methods.user_create_profile(app.admin_pda, app.human_user.pubkey()).accounts({"authority": app.human_user.pubkey(), "admin_profile": app.admin_pda, "user_profile": app.human_user_pda, "system_program": anchorpy.SYSTEM_PROGRAM_ID}).rpc())
            print("Human user profile created.")
            st.session_state.chat_log = ["Welcome to the Hybrid Social Network!"]
            st.session_state.bot_status = {"Alice": "Online", "Bob": "Online"}
            app.start_all_threads()
            print("--- All background threads started. UI is ready. ---")
        except Exception as e:
            print(f"!!! ON-CHAIN SETUP FAILED: {e}")
            st.session_state.chat_log = [f"ERROR: On-chain setup failed: {e}"]
            st.session_state.bot_status = {"Alice": "Error", "Bob": "Error"}

    st.session_state.app.run_ui()

if __name__ == "__main__":
    main()