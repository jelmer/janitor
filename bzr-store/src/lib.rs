//! BZR Store service for the Janitor project.
//!
//! This service provides HTTP-accessible Bazaar repositories with administrative and public interfaces.
//! It uses PyO3 to integrate with the Python Breezy library for Bazaar protocol support.

#![deny(missing_docs)]

pub mod config;
pub mod database;
pub mod error;
pub mod repository;
pub mod web;

pub use config::Config;
pub use error::{BzrError, Result};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
