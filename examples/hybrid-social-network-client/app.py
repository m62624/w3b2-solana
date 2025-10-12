import base64
import os
import random
import time
import threading
import streamlit as st
from http.server import HTTPServer, SimpleHTTPRequestHandler
from solders.keypair import Keypair
from solders.pubkey import Pubkey
from solders.hash import Hash
from solders.transaction import Transaction, Message, Instruction
from solders.system_program import TransferParams, transfer
from solders.compute_budget import set_compute_unit_limit, set_compute_unit_price
from solana.rpc.api import Client
from solana.rpc.commitment import Confirmed
from solana.rpc.types import TxOpts
import grpc
import base58

# --- Generate gRPC Code ---
from generate_grpc_code import generate_code
generate_code()

from generated import gateway_pb2, gateway_pb2_grpc, types_pb2, protocols_pb2

# --- Constants ---
# These are now read from environment variables at the bottom of the file.
GATEWAY_HOST = os.environ.get("GATEWAY_HOST", "localhost")
GATEWAY_PORT = int(os.environ.get("GATEWAY_PORT", 9090))
SOLANA_RPC_URL = os.environ.get("SOLANA_RPC_URL", "http://localhost:8899")
PROGRAM_ID_STR = os.environ.get("PROGRAM_ID", "HykRMCadVCe49q4GVrXKTwLG3fqCEgd5W5qQqN3AFAEY") # Fallback for local dev
DEVNET_RPC_URL = "https://api.devnet.solana.com"

class SolanaClient:
    def __init__(self, rpc_url):
        self.client = Client(rpc_url, commitment=Confirmed)

    def airdrop(self, pubkey: Pubkey, lamports: int):
        try:
            resp = self.client.request_airdrop(pubkey, lamports)
            self.client.confirm_transaction(resp.value, commitment=Confirmed)
            print(f"Airdropped {lamports} to {pubkey} on {self.client.endpoint}.")
        except Exception as e:
            print(f"Airdrop on {self.client.endpoint} failed: {e}. Trying Devnet...")
            devnet_client = Client(DEVNET_RPC_URL, commitment=Confirmed)
            try:
                resp = devnet_client.request_airdrop(pubkey, lamports)
                devnet_client.confirm_transaction(resp.value, commitment=Confirmed)
                print(f"Airdropped {lamports} to {pubkey} on Devnet.")
            except Exception as e_dev:
                print(f"Devnet airdrop also failed: {e_dev}")

    def get_latest_blockhash(self) -> Hash:
        return self.client.get_latest_blockhash().value.blockhash

    def sign_and_send_transaction(self, tx_base64: str, signer: Keypair) -> str:
        tx_bytes = base64.b64decode(tx_base64)
        tx = Transaction.from_bytes(tx_bytes)
        tx.sign([signer], self.get_latest_blockhash())
        raw_tx = tx.serialize()
        tx_sig_resp = self.client.send_raw_transaction(raw_tx, opts=TxOpts(skip_preflight=True))
        self.client.confirm_transaction(tx_sig_resp.value, commitment=Confirmed)
        return str(tx_sig_resp.value)

    @staticmethod
    def get_admin_profile_pda(program_id: Pubkey, authority: Pubkey) -> Pubkey:
        seeds = [b"admin_profile", bytes(authority)]
        pda, _ = Pubkey.find_program_address(seeds, program_id)
        return pda

    @staticmethod
    def get_user_profile_pda(program_id: Pubkey, authority: Pubkey, admin_pda: Pubkey) -> Pubkey:
        seeds = [b"user_profile", bytes(authority), bytes(admin_pda)]
        pda, _ = Pubkey.find_program_address(seeds, program_id)
        return pda

class GatewayClient:
    def __init__(self, host, port, program_id_str):
        self.channel = grpc.insecure_channel(f'{host}:{port}')
        self.stub = gateway_pb2_grpc.BridgeGatewayServiceStub(self.channel)
        self.program_id = Pubkey.from_string(program_id_str)

    def prepare_admin_register_profile(self, admin_keypair: Keypair) -> str:
        request = types_pb2.PrepareAdminRegisterProfileRequest(
            authority_pubkey=str(admin_keypair.pubkey()),
            communication_pubkey=str(admin_keypair.pubkey())
        )
        response = self.stub.PrepareAdminRegisterProfile(request)
        return response.unsigned_tx

    def prepare_user_create_profile(self, user_keypair: Keypair, admin_pda: Pubkey) -> str:
        request = types_pb2.PrepareUserCreateProfileRequest(
            authority_pubkey=str(user_keypair.pubkey()),
            target_admin_pda=str(admin_pda),
            communication_pubkey=str(user_keypair.pubkey())
        )
        response = self.stub.PrepareUserCreateProfile(request)
        return response.unsigned_tx

    def prepare_user_deposit(self, user_keypair: Keypair, admin_pda: Pubkey, amount: int) -> str:
        request = types_pb2.PrepareUserDepositRequest(
            authority_pubkey=str(user_keypair.pubkey()),
            admin_profile_pda=str(admin_pda),
            amount=amount
        )
        response = self.stub.PrepareUserDeposit(request)
        return response.unsigned_tx

    def prepare_user_dispatch_command(self, user_keypair: Keypair, admin_pda: Pubkey, command_id: int, payload: bytes, price: int = 0) -> str:
        oracle_keypair = Keypair()
        timestamp = int(time.time())
        message_to_sign = command_id.to_bytes(2, 'little') + price.to_bytes(8, 'little') + timestamp.to_bytes(8, 'little')
        oracle_signature = oracle_keypair.sign(message_to_sign).to_bytes()
        request = types_pb2.PrepareUserDispatchCommandRequest(
            authority_pubkey=str(user_keypair.pubkey()),
            target_admin_pda=str(admin_pda),
            command_id=command_id,
            price=price,
            timestamp=timestamp,
            payload=payload,
            oracle_pubkey=str(oracle_keypair.pubkey()),
            oracle_signature=oracle_signature
        )
        response = self.stub.PrepareUserDispatchCommand(request)
        return response.unsigned_tx

    def prepare_log_action(self, signer_keypair: Keypair, user_pda: Pubkey, admin_pda: Pubkey, session_id: int, action_code: int) -> str:
        request = types_pb2.PrepareLogActionRequest(
            authority_pubkey=str(signer_keypair.pubkey()),
            user_profile_pda=str(user_pda),
            admin_profile_pda=str(admin_pda),
            session_id=session_id,
            action_code=action_code
        )
        response = self.stub.PrepareLogAction(request)
        return response.unsigned_tx

    def prepare_admin_ban_user(self, admin_keypair: Keypair, user_pda_to_ban: Pubkey) -> str:
        request = types_pb2.PrepareAdminBanUserRequest(
            authority_pubkey=str(admin_keypair.pubkey()),
            target_user_profile_pda=str(user_pda_to_ban)
        )
        response = self.stub.PrepareAdminBanUser(request)
        return response.unsigned_tx

    def listen_as_user(self, user_pda: Pubkey):
        request = types_pb2.ListenRequest(pda=str(user_pda))
        return self.stub.ListenAsUser(request)

class Bot:
    def __init__(self, name: str, keypair: Keypair, pda: Pubkey, app: 'SocialApp'):
        self.name = name
        self.keypair = keypair
        self.pda = pda
        self.app = app
        self.message_count = 0

    def send_message(self, recipient_name: str, message: str):
        print(f"[{self.name}] -> [{recipient_name}]: {message}")
        payload = f"MSG:{message}".encode('utf-8')
        unsigned_tx = self.app.gateway_client.prepare_user_dispatch_command(
            self.keypair, self.app.admin_pda, 1, payload, price=0
        )
        self.app.solana_client.sign_and_send_transaction(unsigned_tx, self.keypair)
        self.message_count += 1

    def simulate_file_transfer(self, recipient_keypair: Keypair, recipient_name: str):
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

        unsigned_tx = self.app.gateway_client.prepare_user_dispatch_command(self.keypair, self.app.admin_pda, 2, payload, price=0)
        self.app.solana_client.sign_and_send_transaction(unsigned_tx, self.keypair)
        self.message_count += 1

        print(f"[{recipient_name}]: 'Downloading' from http://localhost:{http_port}/dummy_file.zip")
        httpd.shutdown()
        print(f"[{self.name}] HTTP server stopped.")

        log_tx = self.app.gateway_client.prepare_log_action(self.keypair, self.pda, self.app.admin_pda, session_id, 200)
        self.app.solana_client.sign_and_send_transaction(log_tx, self.keypair)
        recipient_pda = self.app.solana_client.get_user_profile_pda(self.app.gateway_client.program_id, recipient_keypair.pubkey(), self.app.admin_pda)
        log_tx = self.app.gateway_client.prepare_log_action(recipient_keypair, recipient_pda, self.app.admin_pda, session_id, 200)
        self.app.solana_client.sign_and_send_transaction(log_tx, recipient_keypair)

    def use_paid_feature(self, recipient_name: str):
        price = 1_000_000
        print(f"[{self.name}] -> [{recipient_name}]: Using paid feature 'Send Sticker' for {price} lamports.")
        payload = b"STICKER:smiley_face"
        unsigned_tx = self.app.gateway_client.prepare_user_dispatch_command(self.keypair, self.app.admin_pda, 3, payload, price=price)
        self.app.solana_client.sign_and_send_transaction(unsigned_tx, self.keypair)
        self.message_count += 1

class SocialApp:
    def __init__(self):
        self.solana_client = SolanaClient(SOLANA_RPC_URL)
        self.gateway_client = GatewayClient(GATEWAY_HOST, GATEWAY_PORT, PROGRAM_ID_STR)
        self.admin, self.alice, self.bob, self.human_user = Keypair(), Keypair(), Keypair(), Keypair()

        self.admin_pda = SolanaClient.get_admin_profile_pda(self.gateway_client.program_id, self.admin.pubkey())
        self.alice_pda = SolanaClient.get_user_profile_pda(self.gateway_client.program_id, self.alice.pubkey(), self.admin_pda)
        self.bob_pda = SolanaClient.get_user_profile_pda(self.gateway_client.program_id, self.bob.pubkey(), self.admin_pda)

        self.bot_alice = Bot("Alice", self.alice, self.alice_pda, self)
        self.bot_bob = Bot("Bob", self.bob, self.bob_pda, self)

        self.stop_event = threading.Event()
        self.lock = threading.Lock()

    def setup_onchain_state(self):
        print("\n--- Starting On-Chain Setup ---")
        print("1. Airdropping SOL...")
        self.solana_client.airdrop(self.admin.pubkey(), 1_000_000_000)
        self.solana_client.airdrop(self.alice.pubkey(), 1_000_000_000)
        self.solana_client.airdrop(self.bob.pubkey(), 1_000_000_000)
        self.solana_client.airdrop(self.human_user.pubkey(), 1_000_000_000)

        print("\n2. Registering Admin Profile...")
        unsigned_tx_b64 = self.gateway_client.prepare_admin_register_profile(self.admin)
        sig = self.solana_client.sign_and_send_transaction(unsigned_tx_b64, self.admin)
        print(f"  -> Signature: {sig}")

        print("\n3. Creating User Profiles...")
        unsigned_tx_b64 = self.gateway_client.prepare_user_create_profile(self.alice, self.admin_pda)
        sig = self.solana_client.sign_and_send_transaction(unsigned_tx_b64, self.alice)
        print(f"  -> Alice's Profile. Signature: {sig}")
        unsigned_tx_b64 = self.gateway_client.prepare_user_create_profile(self.bob, self.admin_pda)
        sig = self.solana_client.sign_and_send_transaction(unsigned_tx_b64, self.bob)
        print(f"  -> Bob's Profile. Signature: {sig}")

        print("\n4. Depositing funds for bots (0.1 SOL each)...")
        deposit_amount = 100_000_000
        unsigned_tx_b64 = self.gateway_client.prepare_user_deposit(self.alice, self.admin_pda, deposit_amount)
        sig = self.solana_client.sign_and_send_transaction(unsigned_tx_b64, self.alice)
        print(f"  -> Deposited to Alice. Signature: {sig}")
        unsigned_tx_b64 = self.gateway_client.prepare_user_deposit(self.bob, self.admin_pda, deposit_amount)
        sig = self.solana_client.sign_and_send_transaction(unsigned_tx_b64, self.bob)
        print(f"  -> Deposited to Bob. Signature: {sig}")
        print("\n--- On-Chain Setup Complete ---")

    def run_bot_chat(self):
        turn = 0
        while not self.stop_event.is_set():
            sender = self.bot_alice if turn % 2 == 0 else self.bot_bob
            receiver = self.bot_bob if turn % 2 == 0 else self.bot_alice

            if st.session_state.bot_status.get(sender.name) == "Banned":
                print(f"[{sender.name}] is banned, skipping turn.")
                turn +=1
                time.sleep(5)
                continue

            if sender.message_count > 0 and sender.message_count % 8 == 0:
                sender.use_paid_feature(receiver.name)
            elif sender.message_count > 0 and sender.message_count % 2 == 0:
                sender.simulate_file_transfer(receiver.keypair, receiver.name)
            else:
                sender.send_message(receiver.name, f"Hello! This is message #{sender.message_count + 1}")
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
                tx = self.gateway_client.prepare_admin_ban_user(self.admin, self.alice_pda)
                self.solana_client.sign_and_send_transaction(tx, self.admin)
        with col2:
            st.metric(label="Bob Status", value=st.session_state.bot_status.get("Bob", "Unknown"))
            if st.button("Ban Bob"):
                st.session_state.chat_log.append("[ADMIN]: Banning Bob...")
                tx = self.gateway_client.prepare_admin_ban_user(self.admin, self.bob_pda)
                self.solana_client.sign_and_send_transaction(tx, self.admin)

        st.divider()
        chat_history = "\n".join(st.session_state.chat_log)
        st.text_area("Chat Log", value=chat_history, height=400, key="chat_window", disabled=True)

        with st.form(key="message_form", clear_on_submit=True):
            recipient = st.selectbox("Recipient", ["Alice", "Bob"])
            message_text = st.text_input("Your message:")
            if st.form_submit_button(label='Send Message') and message_text:
                recipient_map = {"Alice": self.bot_alice, "Bob": self.bot_bob}
                target_bot = recipient_map[recipient]

                st.session_state.chat_log.append(f"[You] -> [{recipient}]: {message_text}")
                payload = f"MSG:{message_text}".encode('utf-8')
                tx = self.gateway_client.prepare_user_dispatch_command(self.human_user, self.admin_pda, 1, payload)
                self.solana_client.sign_and_send_transaction(tx, self.human_user)
                st.experimental_rerun()

if __name__ == "__main__":
    if 'app' not in st.session_state:
        print("--- Initializing Social App State ---")
        app = SocialApp()
        st.session_state.app = app

        try:
            app.setup_onchain_state()
        except Exception as e:
            print(f"!!! ON-CHAIN SETUP FAILED: {e}")
            print("!!! This is expected if the gateway/validator are not running.")
            print("!!! The application will continue with simulated identities.")

        st.session_state.chat_log = ["Welcome to the Hybrid Social Network!"]
        st.session_state.bot_status = {"Alice": "Online", "Bob": "Online"}

        app.start_all_threads()
        print("--- All background threads started. UI is ready. ---")

    st.session_state.app.run_ui()