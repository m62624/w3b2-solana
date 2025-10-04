mod instructions;

use instructions::*;
use solana_sdk::signature::Signer;
use w3b2_program::events::OffChainActionLogged;

#[test]
fn test_log_action_by_user_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();
    let (_, admin_pda, user_authority, user_pda) = setup_profiles(&mut svm);

    let session_id = 12345;
    let action_code = 200;

    // === 2. Act ===
    println!("User logging an action...");
    let logs = log::log_action(
        &mut svm,
        &user_authority, // The user is the signer
        user_pda,
        admin_pda,
        session_id,
        action_code,
    );
    println!("Action logged by user.");

    // === 3. Assert ===
    let events = parse_events::<OffChainActionLogged>(&logs);
    let event = events.last().expect("No events were emitted!");

    assert_eq!(event.actor, user_authority.pubkey());
    assert_eq!(event.target, admin_pda);
    assert_eq!(event.session_id, session_id);
    assert_eq!(event.action_code, action_code);

    println!("✅ Log Action by User Test Passed!");
    println!("   -> Actor: {} (User)", event.actor);
    println!("   -> Target: {} (Admin PDA)", event.target);
}

#[test]
fn test_log_action_by_admin_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();
    let (admin_authority, admin_pda, _user_authority, user_pda) = setup_profiles(&mut svm);

    let session_id = 54321;
    let action_code = 404; // e.g., HTTP Not Found

    // === 2. Act ===
    println!("Admin logging an action...");
    let logs = log::log_action(
        &mut svm,
        &admin_authority, // The admin is the signer
        user_pda,
        admin_pda,
        session_id,
        action_code,
    );
    println!("Action logged by admin.");

    // === 3. Assert ===
    let events = parse_events::<OffChainActionLogged>(&logs);
    let event = events.last().expect("No events were emitted!");

    assert_eq!(event.actor, admin_authority.pubkey());
    assert_eq!(event.target, user_pda);
    assert_eq!(event.session_id, session_id);
    assert_eq!(event.action_code, action_code);

    println!("✅ Log Action by Admin Test Passed!");
    println!("   -> Actor: {} (Admin)", event.actor);
    println!("   -> Target: {} (User PDA)", event.target);
}
