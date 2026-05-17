//! Password hashing + invite-token helpers for the OSS / self-hosted auth path.
//!
//! Passwords are hashed with Argon2id using the algorithm defaults (good enough
//! for a self-hosted control plane; verification cost is ~50-100ms on commodity
//! hardware). Hashing only happens on login, accept-invite, and bootstrap — never
//! in the request hot path.
//!
//! Invite tokens are 32 random bytes encoded with URL-safe base64. The plaintext
//! is returned to the inviting admin exactly once; the database stores only its
//! SHA-256 hash so an inadvertent DB dump cannot be replayed against this endpoint.

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::errors::CustomError;

const INVITE_TOKEN_BYTES: usize = 32;
const MIN_PASSWORD_LEN: usize = 8;
const MAX_PASSWORD_LEN: usize = 256;

pub fn hash_password(plaintext: &str) -> Result<String, CustomError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(plaintext.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| CustomError::InternalError(format!("password hashing failed: {e}")))
}

pub fn verify_password(plaintext: &str, encoded_hash: &str) -> Result<bool, CustomError> {
    let parsed = PasswordHash::new(encoded_hash)
        .map_err(|e| CustomError::InternalError(format!("stored password hash invalid: {e}")))?;
    Ok(Argon2::default()
        .verify_password(plaintext.as_bytes(), &parsed)
        .is_ok())
}

pub fn validate_password_strength(plaintext: &str) -> Result<(), CustomError> {
    if plaintext.len() < MIN_PASSWORD_LEN {
        return Err(CustomError::ValidationError(format!(
            "password must be at least {MIN_PASSWORD_LEN} characters"
        )));
    }
    if plaintext.len() > MAX_PASSWORD_LEN {
        return Err(CustomError::ValidationError(format!(
            "password must be at most {MAX_PASSWORD_LEN} characters"
        )));
    }
    Ok(())
}

pub struct GeneratedInviteToken {
    pub plaintext: String,
    pub hash: String,
}

pub fn generate_invite_token() -> GeneratedInviteToken {
    let mut buf = [0u8; INVITE_TOKEN_BYTES];
    rand::rng().fill_bytes(&mut buf);
    let plaintext = URL_SAFE_NO_PAD.encode(buf);
    let hash = hash_invite_token(&plaintext);
    GeneratedInviteToken { plaintext, hash }
}

pub fn hash_invite_token(plaintext: &str) -> String {
    let digest = Sha256::digest(plaintext.as_bytes());
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest.iter() {
        use std::fmt::Write;
        let _ = write!(hex, "{:02x}", byte);
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify_roundtrips() {
        let h = hash_password("correct horse battery staple").expect("hash");
        assert!(verify_password("correct horse battery staple", &h).expect("verify ok"));
        assert!(!verify_password("wrong", &h).expect("verify ok"));
    }

    #[test]
    fn hashes_have_distinct_salts() {
        let a = hash_password("same").expect("hash");
        let b = hash_password("same").expect("hash");
        assert_ne!(a, b);
    }

    #[test]
    fn invite_token_hash_is_deterministic_sha256_hex() {
        let h1 = hash_invite_token("abc");
        let h2 = hash_invite_token("abc");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
        assert!(h1
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn generated_invite_tokens_are_unique() {
        let a = generate_invite_token();
        let b = generate_invite_token();
        assert_ne!(a.plaintext, b.plaintext);
        assert_ne!(a.hash, b.hash);
        assert_eq!(a.hash, hash_invite_token(&a.plaintext));
    }

    #[test]
    fn password_strength_rejects_short() {
        assert!(validate_password_strength("short").is_err());
        assert!(validate_password_strength("longer-than-eight").is_ok());
    }
}
