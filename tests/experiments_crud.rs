//! Integration tests for the `/admin/v1/experiments` CRUD handlers.
//!
//! Organised by handler under `tests/experiments/`. All tests run against a
//! real (in-memory) SQLite database and hit the real actix-web stack via
//! `reqwest`. See `tests/common/mod.rs` for the shared harness.
//!
//! Run with:   cargo test --test experiments_crud

mod common;
mod experiments;
