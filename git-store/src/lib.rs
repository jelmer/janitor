//! Git Store crate for the Janitor project.
//!
//! This crate provides functionality for storing and managing Git repositories.

#![deny(missing_docs)]

pub mod config;
pub mod database;
pub mod error;
pub mod git_http;
pub mod repository;
pub mod web;

pub use config::Config;
pub use error::{GitStoreError, Result};