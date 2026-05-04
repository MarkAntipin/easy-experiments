use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::errors::CustomError;
use crate::models::{ApiKeyRow, ExperimentsDB};
use crate::repository::{db_create_api_key, db_list_api_keys, db_revoke_api_key, NewApiKey};

pub const API_KEY_PREFIX: &str = "eek-";
const SECRET_BYTES: usize = 32;
const PREFIX_DISPLAY_CHARS: usize = 12;

pub struct GeneratedApiKey {
    pub plaintext: String,
    pub hash: String,
    pub prefix: String,
}

//  TODO: review, but looks ok
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

pub struct CreatedApiKey {
    pub api_key_id: String,
    pub name: String,
    pub plaintext: String,
    pub prefix: String,
    pub created_at: i64,
}

pub async fn create(
    db: &ExperimentsDB,
    company_id: &str,
    name: String,
) -> Result<CreatedApiKey, CustomError> {
    let generated = generate_api_key();
    let (api_key_id, created_at) = db_create_api_key(
        db,
        NewApiKey {
            company_id,
            name: &name,
            key_hash: &generated.hash,
            key_prefix: &generated.prefix,
        },
    )
    .await?;

    Ok(CreatedApiKey {
        api_key_id,
        name,
        plaintext: generated.plaintext,
        prefix: generated.prefix,
        created_at,
    })
}

pub async fn list(
    db: &ExperimentsDB,
    company_id: &str,
) -> Result<Vec<ApiKeyRow>, CustomError> {
    db_list_api_keys(db, company_id).await
}

pub async fn revoke(
    db: &ExperimentsDB,
    api_key_id: &str,
    company_id: &str,
) -> Result<(), CustomError> {
    if db_revoke_api_key(db, api_key_id, company_id).await? {
        Ok(())
    } else {
        Err(CustomError::NotFoundError(format!(
            "API key with id '{}' not found",
            api_key_id
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_api_key_is_deterministic_sha256_hex() {
        let got = hash_api_key("eek-abc");
        assert_eq!(got.len(), 64);
        assert!(got.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_eq!(hash_api_key("eek-abc"), got);
        assert_ne!(hash_api_key("eek-abc"), hash_api_key("eek-abd"));
        assert_eq!(hash_api_key("").len(), 64);
        // Cross-check a known digest so a future format change (e.g., hex casing) is caught.
        assert_eq!(
            got,
            "33c2083987e4ce0d429f085d1004ccc6fbf4ecc2139e076dbea48a1c1fe2ce49"
        );
    }

    #[test]
    fn display_prefix_takes_first_twelve_chars() {
        assert_eq!(display_prefix("eek-abcdefghij"), "eek-abcdefgh");
        // shorter than limit returns full string
        assert_eq!(display_prefix("eek-xy"), "eek-xy");
        assert_eq!(display_prefix(""), "");
        // multibyte chars are counted as chars, not bytes
        let s = "日本語テスト-abcdef";
        let prefix = display_prefix(s);
        assert_eq!(prefix.chars().count(), 12);
    }

    #[test]
    fn is_plausible_api_key_requires_prefix_and_body() {
        assert!(is_plausible_api_key("eek-abc"));
        // missing prefix
        assert!(!is_plausible_api_key("abc"));
        // prefix with empty body
        assert!(!is_plausible_api_key(API_KEY_PREFIX));
        // empty
        assert!(!is_plausible_api_key(""));
        // wrong prefix
        assert!(!is_plausible_api_key("xek-abc"));
    }

    #[test]
    fn generate_api_key_produces_consistent_fields() {
        let k = generate_api_key();
        assert!(k.plaintext.starts_with(API_KEY_PREFIX));
        assert!(is_plausible_api_key(&k.plaintext));
        assert_eq!(k.hash, hash_api_key(&k.plaintext));
        assert_eq!(k.prefix, display_prefix(&k.plaintext));
        // randomness — two calls should not collide
        let k2 = generate_api_key();
        assert_ne!(k.plaintext, k2.plaintext);
        assert_ne!(k.hash, k2.hash);
    }
}
