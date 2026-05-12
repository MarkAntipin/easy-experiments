//! Integration test binary for the password-auth surface:
//! `POST /admin/v1/auth/login` and `POST /admin/v1/auth/accept-invite`.
//!
//! Run with:   cargo test --test auth

mod common;

#[path = "auth/mod.rs"]
mod auth;
