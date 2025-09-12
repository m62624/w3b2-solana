/// Well-known command ids.
pub const CMD_PUBLISH_PUBKEY: u64 = 1; // payload = raw [u8;32] (service or client pubkey)
pub const CMD_REQUEST_CONNECTION: u64 = 2; // payload = Borsh(CommandConfig) with encrypted_session_key
// (add other numeric ids as needed)
