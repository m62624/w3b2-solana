use rand::Rng;
use std::time::Duration;
use tokio::time;

struct User {
    name: &'static str,
}

struct Oracle;

impl Oracle {
    fn sign_purchase(&self, user: &User, item: &str) -> String {
        println!("[ORACLE] User '{}' requested to purchase '{}'.", user.name, item);
        println!("[ORACLE] Verifying payment and business logic...");
        // In a real scenario, this would involve complex logic.
        // Here, we just simulate a successful signing.
        let signature = format!("signed-by-oracle-for-{}-purchase-of-{}", user.name, item);
        println!("[ORACLE] Logic successful. Returning signature: {}", signature);
        signature
    }
}

async fn run_chat_simulation() {
    let alice = User { name: "Alice" };
    let bob = User { name: "Bob" };
    let oracle = Oracle;
    let users = [&alice, &bob];
    let mut rng = rand::thread_rng();

    let messages = [
        "Hey, how are you?",
        "Did you see the latest update?",
        "I'm working on the new feature.",
        "Let's sync up tomorrow.",
        "That's a great idea!",
    ];

    let premium_items = [
        "Power-Up",
        "Extra Life",
        "Special Skin",
    ];

    println!("[SIMULATION] Starting chat simulation between Alice and Bob.");
    let mut interval = time::interval(Duration::from_secs(5));

    loop {
        interval.tick().await;
        let sender_index = rng.gen_range(0..users.len());
        let receiver_index = 1 - sender_index;
        let sender = users[sender_index];
        let receiver = users[receiver_index];

        // Decide whether to send a message or make a purchase
        if rng.gen_bool(0.25) { // 25% chance to make a purchase
            let item_index = rng.gen_range(0..premium_items.len());
            let item_to_buy = premium_items[item_index];

            println!("\n--- Purchase Event ---");
            println!("[{}] I want to buy the '{}'!", sender.name, item_to_buy);

            // 1. User requests signature from the Oracle
            let oracle_signature = oracle.sign_purchase(sender, item_to_buy);

            // 2. User "submits" this signature to the main system (e.g., on-chain)
            println!("[{}] Purchase successful! I used the oracle signature to get my '{}'.", sender.name, item_to_buy);
            println!("--- End Purchase Event ---\n");

        } else { // 75% chance to send a message
            let message_index = rng.gen_range(0..messages.len());
            let message = messages[message_index];
            println!("[{}] -> [{}]: {}", sender.name, receiver.name, message);
        }
    }
}

#[tokio::main]
async fn main() {
    run_chat_simulation().await;
}