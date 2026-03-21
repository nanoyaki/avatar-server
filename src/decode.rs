use hmac::{
    Hmac, Mac,
    digest::{self, KeyInit},
};

pub fn is_valid(shared_secret: &str, value: &str, secret_hex: &str) -> bool {
    let mut mac = <Hmac<sha2::Sha256> as KeyInit>::new_from_slice(shared_secret.as_bytes())
        .expect("hmac secret");
    digest::Update::update(&mut mac, value.as_bytes());

    let Ok(sig_bytes) = hex::decode(secret_hex.trim()) else {
        return false;
    };

    mac.verify_slice(&sig_bytes).is_ok()
}
