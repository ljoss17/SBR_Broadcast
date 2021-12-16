use talk::crypto::{KeyChain, Statement};

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
enum Header {
    Age,
    Height,
}

#[derive(Serialize, Deserialize)]
struct Age(u32);

#[derive(Serialize, Deserialize)]
struct Height(u32);

impl Statement for Age {
    type Header = Header;
    const HEADER: Header = Header::Age;
}

impl Statement for Height {
    type Header = Header;
    const HEADER: Header = Header::Height;
}

#[tokio::main]
async fn main() {
    let alice_keychain = KeyChain::random();
    let alice_keycard = alice_keychain.keycard();

    let age2: u32 = 30;
    let age = Age(30);
    let signature = alice_keychain.sign(&age).unwrap();
    let signature2 = alice_keychain.sign(&Age(age2)).unwrap();

    // Send `age` and `signature` to Bob...

    if signature.verify(&alice_keycard, &age).is_ok() {
        println!("Verified!");
    }

    if signature2.verify(&alice_keycard, &Age(age2)).is_ok() {
        println!("Verified2!");
    }
}
