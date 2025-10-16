//! # FFI interface for w3b2-solana-signer
//!
//! This module provides a C-compatible API for generating, loading, managing,
//! and using Solana keypairs from non-Rust environments.
//!
//! ## Thread Safety
//! - The global key table (`KEY_TABLE`) is backed by `DashMap` and **is safe to access from multiple threads**.
//! - Each FFI call is atomic with respect to key storage. You can safely call `load_key`, `sign_with_handle`, etc. concurrently.
//!
//! ## Memory and Lifetime
//! - All buffers returned from Rust must be freed using [`free_buffer`].
//! - Key handles remain valid until explicitly unloaded using [`unload_key`] or [`clear_all_keys`].
//! - Forgetting to unload or clear keys will keep them in memory until process exit.
//!
//! ## Security
//! - Private key bytes are locked in memory via `mlock()` when possible (prevents swap leaks).
//! - On drop or unload, memory is securely zeroized and optionally `munlock()`ed.
//! - If `mlock()` fails, the system continues but prints a warning via [`get_last_error`].
//!
//! ## Error Handling
//! - Most FFI functions report failure by returning `NULL` or `0` and storing the error string internally.
//! - Use [`get_last_error`] to retrieve a null-terminated error string for the last failure in the current thread.

use dashmap::DashMap;
use lazy_static::lazy_static;
use libc::{mlock, munlock};
use solana_sdk::{hash::Hash, message::Message, signature::Keypair, transaction::Transaction};
use std::cell::RefCell;
use std::os::raw::c_char;
use std::ptr;
use std::slice;
use std::sync::atomic::{AtomicU64, Ordering};
use zeroize::Zeroize;

type Handle = u64;

/// Internal key entry, storing secret key bytes and lock status.
struct KeyEntry {
    secret: Vec<u8>,
    locked: bool,
}

impl Drop for KeyEntry {
    fn drop(&mut self) {
        if self.locked {
            unsafe {
                let p = self.secret.as_ptr() as *const libc::c_void;
                let len = self.secret.len();
                let _ = munlock(p, len);
            }
        }
        self.secret.zeroize();
    }
}

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

lazy_static! {
    static ref KEY_TABLE: DashMap<Handle, KeyEntry> = DashMap::new();
}

thread_local! {
    static LAST_ERROR: RefCell<Option<std::ffi::CString>> = const { RefCell::new(None) };
}

fn set_last_error(s: impl Into<String>) {
    let error_str = s.into();
    if let Ok(c_string) = std::ffi::CString::new(error_str) {
        LAST_ERROR.with(|cell| cell.borrow_mut().replace(c_string));
    }
}

/// Returns the last error string for the current thread.
///
/// # Safety
/// - Returns a pointer to an internal static C string.
/// - The pointer remains valid until the next FFI call on the same thread.
///
/// # Returns
/// - `NULL` if no error has occurred yet.
/// - Non-null pointer to a null-terminated UTF-8 string otherwise.
#[no_mangle]
pub extern "C" fn get_last_error() -> *const c_char {
    LAST_ERROR.with(|cell| cell.borrow().as_ref().map_or(ptr::null(), |s| s.as_ptr()))
}

fn try_mlock(buf: &mut [u8]) -> bool {
    unsafe {
        let p = buf.as_ptr() as *const libc::c_void;
        let len = buf.len();
        mlock(p, len) == 0
    }
}

/// Generates a new Solana keypair and returns it as a byte buffer.
///
/// # Safety
/// - `out_len` must be a valid, non-null pointer to a `usize`.
/// - The caller must later call [`free_buffer`] to free the returned buffer.
///
/// # Returns
/// - Pointer to a 64-byte keypair (`[secret||public]`).
/// - `NULL` if arguments are invalid.
///
/// # Example
/// ```c
/// size_t len = 0;
/// uint8_t* key = generate_keypair(&len);
/// // use key ...
/// free_buffer(key, len);
/// ```
#[no_mangle]
pub unsafe extern "C" fn generate_keypair(out_len: *mut usize) -> *mut u8 {
    if out_len.is_null() {
        set_last_error("null out_len argument");
        return ptr::null_mut();
    }

    let kp = Keypair::new();
    let key_bytes = kp.to_bytes();
    let encoded = key_bytes.to_vec();

    let len = encoded.len();
    let boxed = encoded.into_boxed_slice();
    let ptr_out = Box::into_raw(boxed) as *mut u8;

    *out_len = len;
    ptr_out
}

/// Loads a keypair from a byte slice and registers it in the key table.
///
/// # Safety
/// - `key_ptr` must be valid and point to `key_len` bytes.
/// - The key buffer must remain valid for the duration of this call only.
/// - Returns `0` on failure.
///
/// # Notes
/// - Keys are stored securely in process memory.
/// - `mlock()` is attempted; if it fails, a warning is stored.
///
/// # Returns
/// - A unique handle (`Handle`) representing the loaded key.
/// - `0` if the input is invalid.
#[no_mangle]
pub unsafe extern "C" fn load_key(key_ptr: *const u8, key_len: usize) -> Handle {
    if key_ptr.is_null() || key_len == 0 {
        set_last_error("null key or zero length");
        return 0;
    }

    let slice = slice::from_raw_parts(key_ptr, key_len);
    let mut secret = slice.to_vec();

    let locked = try_mlock(&mut secret);
    if !locked {
        set_last_error("mlock failed (process may allow swapping)");
    }

    let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
    KEY_TABLE.insert(handle, KeyEntry { secret, locked });
    handle
}

/// Unloads and securely deletes a key associated with a given handle.
///
/// # Notes
/// - If the handle does not exist, an error is stored via [`get_last_error`].
/// - After unloading, the handle becomes invalid.
#[no_mangle]
pub extern "C" fn unload_key(handle: Handle) {
    if handle == 0 {
        return;
    }
    if KEY_TABLE.remove(&handle).is_none() {
        set_last_error("invalid handle");
    }
}

/// Retrieves the public key from a loaded keypair.
///
/// # Safety
/// - `out_len` must be valid and non-null.
/// - The returned pointer must be freed with [`free_buffer`].
///
/// # Returns
/// - A pointer to 32-byte public key data.
/// - `NULL` if handle is invalid or key malformed.
#[no_mangle]
pub unsafe extern "C" fn get_public_key(handle: Handle, out_len: *mut usize) -> *mut u8 {
    if handle == 0 || out_len.is_null() {
        set_last_error("null or invalid argument");
        return ptr::null_mut();
    }

    let pubkey_bytes = match KEY_TABLE.get(&handle) {
        Some(entry) => {
            if entry.secret.len() < 64 {
                set_last_error("invalid key length");
                return ptr::null_mut();
            }
            entry.secret[32..64].to_vec()
        }
        None => {
            set_last_error("invalid handle");
            return ptr::null_mut();
        }
    };

    let len = pubkey_bytes.len();
    let boxed = pubkey_bytes.into_boxed_slice();
    let ptr_out = Box::into_raw(boxed) as *mut u8;

    *out_len = len;
    ptr_out
}

/// Frees a buffer previously returned by any FFI function.
///
/// # Safety
/// - `ptr` must point to memory allocated by Rust FFI (`Box<[u8]>`).
/// - `len` must match the original allocation size.
///
/// # Notes
/// - Securely zeroes the buffer before freeing.
/// - Safe to call multiple times (no-op if null).
#[no_mangle]
pub unsafe extern "C" fn free_buffer(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    let slice = slice::from_raw_parts_mut(ptr, len);
    slice.zeroize();
    let _ = Box::from_raw(slice as *mut [u8]);
}

/// Signs a Solana transaction message using an inline keypair.
///
/// # Safety
/// - All pointer arguments must be valid and properly aligned.
/// - `msg_ptr` must contain a serialized `Message`.
/// - `blockhash_ptr` must be exactly 32 bytes.
/// - `out_len` must not be null.
///
/// # Returns
/// - Pointer to serialized signed `Transaction` object.
/// - `NULL` if parsing or signing fails.
///
/// # Example
/// ```c
/// uint8_t* tx = sign_with_key_once(msg, msg_len, blockhash, keypair, keypair_len, &out_len);
/// free_buffer(tx, out_len);
/// ```
#[no_mangle]
pub unsafe extern "C" fn sign_with_key_once(
    msg_ptr: *const u8,
    msg_len: usize,
    blockhash_ptr: *const u8,
    keypair_ptr: *const u8,
    keypair_len: usize,
    out_len: *mut usize,
) -> *mut u8 {
    if msg_ptr.is_null()
        || msg_len == 0
        || blockhash_ptr.is_null()
        || keypair_ptr.is_null()
        || keypair_len == 0
        || out_len.is_null()
    {
        set_last_error("null or invalid argument");
        return ptr::null_mut();
    }

    let msg_bytes = slice::from_raw_parts(msg_ptr, msg_len);
    let blockhash_bytes = slice::from_raw_parts(blockhash_ptr, 32);
    let key_bytes = slice::from_raw_parts(keypair_ptr, keypair_len);

    let message = match bincode::serde::decode_from_slice::<Message, _>(
        msg_bytes,
        bincode::config::standard(),
    ) {
        Ok((m, _)) => m,
        Err(e) => {
            set_last_error(format!("Message decode failed: {}", e));
            return ptr::null_mut();
        }
    };

    let kp = match Keypair::try_from(key_bytes) {
        Ok(k) => k,
        Err(e) => {
            set_last_error(format!("Keypair parse failed: {}", e));
            return ptr::null_mut();
        }
    };

    let recent_blockhash = Hash::new_from_array(blockhash_bytes.try_into().unwrap());
    let mut tx = Transaction::new_unsigned(message);
    if let Err(e) = tx.try_sign(&[&kp], recent_blockhash) {
        set_last_error(format!("Transaction sign failed: {}", e));
        return ptr::null_mut();
    }

    let encoded = match bincode::serde::encode_to_vec(&tx, bincode::config::standard()) {
        Ok(v) => v,
        Err(e) => {
            set_last_error(format!("Transaction serialize failed: {}", e));
            return ptr::null_mut();
        }
    };

    let len = encoded.len();
    let boxed = encoded.into_boxed_slice();
    let ptr_out = Box::into_raw(boxed) as *mut u8;

    *out_len = len;
    ptr_out
}

/// Signs a message using a preloaded key handle.
///
/// # Safety
/// - `handle` must be valid and created by [`load_key`] or [`generate_keypair`].
/// - Pointers must be valid and properly aligned.
/// - `out_len` must not be null.
///
/// # Returns
/// - Pointer to serialized signed transaction.
/// - `NULL` on failure.
///
/// # Notes
/// - Internally calls [`sign_with_key_once`].
#[no_mangle]
pub unsafe extern "C" fn sign_with_handle(
    handle: Handle,
    msg_ptr: *const u8,
    msg_len: usize,
    blockhash_ptr: *const u8,
    out_len: *mut usize,
) -> *mut u8 {
    if handle == 0
        || msg_ptr.is_null()
        || msg_len == 0
        || blockhash_ptr.is_null()
        || out_len.is_null()
    {
        set_last_error("null or invalid argument");
        return ptr::null_mut();
    }

    let secret = match KEY_TABLE.get(&handle) {
        Some(entry) => entry.secret.clone(),
        None => {
            set_last_error("invalid handle");
            return ptr::null_mut();
        }
    };

    sign_with_key_once(
        msg_ptr,
        msg_len,
        blockhash_ptr,
        secret.as_ptr(),
        secret.len(),
        out_len,
    )
}

/// Clears all loaded keys from memory.
///
/// # Notes
/// - Immediately zeroizes all stored secrets.
/// - Should be called before program exit for security reasons.
#[no_mangle]
pub extern "C" fn clear_all_keys() {
    KEY_TABLE.clear();
    KEY_TABLE.shrink_to_fit();
}
