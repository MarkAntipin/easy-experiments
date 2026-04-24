use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use sha2::{Digest, Sha256};

pub const API_KEY_PREFIX: &str = "eek_";
const SECRET_BYTES: usize = 32;
const PREFIX_DISPLAY_CHARS: usize = 12;

pub struct GeneratedApiKey {
    pub plaintext: String,
    pub hash: String,
    pub prefix: String,
}

pub fn generate_api_key() -> GeneratedApiKey {
    let mut buf = [0u8; SECRET_BYTES];
    rand::rng().fill_bytes(&mut buf);
    let plaintext = format!("{}{}", API_KEY_PREFIX, URL_SAFE_NO_PAD.encode(buf));
    let hash = hash_api_key(&plaintext);
    let prefix = display_prefix(&plaintext);
    GeneratedApiKey { plaintext, hash, prefix }
}

pub fn hash_api_key(plaintext: &str) -> String {
    let digest = Sha256::digest(plaintext.as_bytes());
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest.iter() {
        use std::fmt::Write;
        let _ = write!(hex, "{:02x}", byte);
    }
    hex
}

pub fn display_prefix(plaintext: &str) -> String {
    plaintext.chars().take(PREFIX_DISPLAY_CHARS).collect()
}

pub fn is_plausible_api_key(candidate: &str) -> bool {
    candidate.starts_with(API_KEY_PREFIX) && candidate.len() > API_KEY_PREFIX.len()
}
