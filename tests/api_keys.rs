//! Integration test binary for `/admin/v1/api-keys` handlers.
//!
//! Run with:   cargo test --test api_keys

mod common;

#[path = "api_keys/mod.rs"]
mod api_keys;
