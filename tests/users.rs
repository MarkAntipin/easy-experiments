//! Integration test binary for `/admin/v1/users` handlers.
//!
//! Run with:   cargo test --test users

mod common;

#[path = "users/mod.rs"]
mod users;
