//! Windmill Local Mode
//!
//! This crate provides a minimal local mode for Windmill using libSQL (SQLite/Turso)
//! instead of PostgreSQL. The goal is to support preview execution end-to-end
//! with a lightweight, embedded database.
//!
//! ## Scope
//! - Script preview execution
//! - Flow preview execution
//! - In-memory or file-based SQLite storage
//! - Remote Turso database support for multi-writer scenarios
//!
//! ## Non-goals (for this experiment)
//! - Full feature parity with PostgreSQL mode
//! - Multi-worker support (single embedded worker)
//! - Persistence of scripts/flows (only jobs)

pub mod db;
pub mod schema;
pub mod jobs;
pub mod queue;
pub mod executor;
pub mod worker;
pub mod server;
pub mod error;

pub use db::LocalDb;
pub use error::LocalError;
pub use worker::Worker;
pub use server::LocalServer;
