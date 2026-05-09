//! Integration test binary for `/admin/v1/experiments` handlers.
//!
//! Run with:   cargo test --test experiments

mod common;

#[path = "experiments/mod.rs"]
mod experiments;
