//! Concurrent Processing Module
//!
//! This module provides both synchronous and asynchronous concurrent file processing.

pub mod sync_runner;

#[cfg(feature = "async")]
pub mod async_runner;

// Re-export sync types
pub use sync_runner::{ConcurrentRunner, FilesData};

// Re-export async types
#[cfg(feature = "async")]
pub use async_runner::{AsyncRunner, AsyncFilesData, AsyncProgress};
