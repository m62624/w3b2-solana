#![allow(deprecated)]
use serial_test::serial;
use solana_sdk::{
    hash::Hash,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use std::{ffi::CStr, ptr};
use w3b2_solana_signer::ffi::{
    clear_all_keys, free_buffer, generate_keypair, get_last_error, get_public_key, load_key,
    sign_with_handle, sign_with_key_once, unload_key,
};

#[test]
fn test_sign_with_key_once_ffi_equivalence() {
    let kp = Keypair::new();
    let to = Pubkey::new_unique();
    let ix = system_instruction::transfer(&kp.pubkey(), &to, 42);
    let msg = Message::new(&[ix], Some(&kp.pubkey()));
    let msg_bytes = bincode::serde::encode_to_vec(&msg, bincode::config::standard()).unwrap();
    let blockhash = Hash::new_unique();

    let mut tx_direct = Transaction::new_unsigned(msg.clone());
    tx_direct.sign(&[&kp], blockhash);

    let mut out_len: usize = 0;
    let ptr = unsafe {
        sign_with_key_once(
            msg_bytes.as_ptr(),
            msg_bytes.len(),
            blockhash.as_ref().as_ptr(),
            kp.to_bytes().as_ptr(),
            64,
            &mut out_len,
        )
    };
    assert!(!ptr.is_null(), "sign_with_key_once returned null");

    let tx_bytes = unsafe { std::slice::from_raw_parts(ptr, out_len) };
    let tx_ffi: Transaction =
        bincode::serde::decode_from_slice(tx_bytes, bincode::config::standard())
            .unwrap()
            .0;
    unsafe { free_buffer(ptr, out_len) };
    assert_eq!(tx_ffi.signatures, tx_direct.signatures);
}

#[test]
#[serial]
fn test_load_unload_key_lifecycle() {
    let kp = Keypair::new();
    let key_bytes = kp.to_bytes();

    let handle = unsafe { load_key(key_bytes.as_ptr(), key_bytes.len()) };
    assert_ne!(handle, 0, "load_key should return a non-zero handle");

    unload_key(handle);
    unload_key(handle);

    let error_msg = unsafe { CStr::from_ptr(get_last_error()).to_str().unwrap() };
    assert_eq!(error_msg, "invalid handle");
    clear_all_keys();
}

#[test]
#[serial]
fn test_sign_with_handle_success() {
    let kp = Keypair::new();
    let key_bytes = kp.to_bytes();
    let handle = unsafe { load_key(key_bytes.as_ptr(), key_bytes.len()) };
    assert_ne!(handle, 0);

    let to = Pubkey::new_unique();
    let ix = system_instruction::transfer(&kp.pubkey(), &to, 100);
    let msg = Message::new(&[ix], Some(&kp.pubkey()));
    let msg_bytes = bincode::serde::encode_to_vec(&msg, bincode::config::standard()).unwrap();
    let blockhash = Hash::new_unique();

    let mut out_len: usize = 0;
    let ptr = unsafe {
        sign_with_handle(
            handle,
            msg_bytes.as_ptr(),
            msg_bytes.len(),
            blockhash.as_ref().as_ptr(),
            &mut out_len,
        )
    };
    assert!(!ptr.is_null(), "sign_with_handle returned null");

    let mut tx_direct = Transaction::new_unsigned(msg.clone());
    tx_direct.sign(&[&kp], blockhash);

    let tx_bytes_ffi = unsafe { std::slice::from_raw_parts(ptr, out_len) };
    let tx_ffi: Transaction =
        bincode::serde::decode_from_slice(tx_bytes_ffi, bincode::config::standard())
            .unwrap()
            .0;

    assert_eq!(tx_ffi.signatures, tx_direct.signatures);

    unsafe {
        free_buffer(ptr, out_len);
        unload_key(handle);
        clear_all_keys();
    }
}

#[test]
#[serial]
fn test_error_handling() {
    let handle = unsafe { load_key(ptr::null(), 10) };
    assert_eq!(handle, 0);
    let error_msg = unsafe { CStr::from_ptr(get_last_error()).to_str().unwrap() };
    assert_eq!(error_msg, "null key or zero length");

    let handle = unsafe { load_key([0u8; 64].as_ptr(), 0) };
    assert_eq!(handle, 0);
    let error_msg = unsafe { CStr::from_ptr(get_last_error()).to_str().unwrap() };
    assert_eq!(error_msg, "null key or zero length");

    unload_key(99999);
    let error_msg = unsafe { CStr::from_ptr(get_last_error()).to_str().unwrap() };
    assert_eq!(error_msg, "invalid handle");

    let mut out_len: usize = 0;
    let ptr =
        unsafe { sign_with_key_once(ptr::null(), 0, ptr::null(), ptr::null(), 0, &mut out_len) };
    assert!(ptr.is_null());
    let error_msg = unsafe { CStr::from_ptr(get_last_error()).to_str().unwrap() };
    assert_eq!(error_msg, "null or invalid argument");

    let ptr = unsafe {
        sign_with_handle(
            99999,
            [0u8; 10].as_ptr(),
            10,
            [0u8; 32].as_ptr(),
            &mut out_len,
        )
    };
    assert!(ptr.is_null());
    let error_msg = unsafe { CStr::from_ptr(get_last_error()).to_str().unwrap() };
    assert_eq!(error_msg, "invalid handle");

    let kp = Keypair::new();
    let msg = Message::new(&[], Some(&kp.pubkey()));
    let msg_bytes = bincode::serde::encode_to_vec(&msg, bincode::config::standard()).unwrap();
    let blockhash = Hash::new_unique();
    let ptr = unsafe {
        sign_with_handle(
            99999,
            msg_bytes.as_ptr(),
            msg_bytes.len(),
            blockhash.as_ref().as_ptr(),
            &mut out_len,
        )
    };
    assert!(ptr.is_null());
    let error_msg = unsafe { CStr::from_ptr(get_last_error()).to_str().unwrap() };
    assert_eq!(error_msg, "invalid handle");
    clear_all_keys();
}

#[test]
#[serial]
fn test_no_leak_on_generate_keypair_success() {
    clear_all_keys();
    let mut out_len: usize = 0;
    let ptr = unsafe { generate_keypair(&mut out_len) };

    assert!(!ptr.is_null());
    assert_eq!(out_len, 64);

    unsafe {
        free_buffer(ptr, out_len);
    }
    clear_all_keys();
}

#[test]
#[serial]
fn test_no_leak_on_sign_failure() {
    let kp = Keypair::new();
    let blockhash = Hash::new_unique();
    let invalid_msg_bytes = [1, 2, 3, 4];

    let mut out_len: usize = 0;
    let ptr = unsafe {
        sign_with_key_once(
            invalid_msg_bytes.as_ptr(),
            invalid_msg_bytes.len(),
            blockhash.as_ref().as_ptr(),
            kp.to_bytes().as_ptr(),
            64,
            &mut out_len,
        )
    };
    assert!(ptr.is_null());
    let error_msg = unsafe { CStr::from_ptr(get_last_error()).to_str().unwrap() };
    assert!(error_msg.contains("Message decode failed"));
    clear_all_keys();
}

#[test]
#[serial]
fn test_free_null_buffer_is_safe() {
    unsafe {
        free_buffer(ptr::null_mut(), 0);
        free_buffer(ptr::null_mut(), 100);
    }
    clear_all_keys();
}

#[test]
#[serial]
fn test_get_public_key_success() {
    clear_all_keys();
    let kp = Keypair::new();
    let key_bytes = kp.to_bytes();
    let expected_pubkey_bytes = kp.pubkey().to_bytes();

    let handle = unsafe { load_key(key_bytes.as_ptr(), key_bytes.len()) };
    assert_ne!(handle, 0);

    let mut out_len: usize = 0;
    let pubkey_ptr = unsafe { get_public_key(handle, &mut out_len) };
    assert!(!pubkey_ptr.is_null());
    assert_eq!(out_len, 32);

    let ffi_pubkey_bytes = unsafe { std::slice::from_raw_parts(pubkey_ptr, out_len) };
    assert_eq!(ffi_pubkey_bytes, expected_pubkey_bytes);

    unsafe {
        free_buffer(pubkey_ptr, out_len);
        unload_key(handle);
    }
    clear_all_keys();
}
