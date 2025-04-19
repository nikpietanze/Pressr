//! pressr-core - Core library for the pressr load testing tool
//!
//! This crate provides the core functionality for the pressr load testing tool,
//! including data handling, request execution, and result processing.

mod error;
mod data;
mod runner;
mod result;

// Re-export public API
pub use error::{Error, Result};
pub use data::{RequestData};
pub use runner::{Runner, Config};
pub use result::{RequestResult, LoadTestResults}; 