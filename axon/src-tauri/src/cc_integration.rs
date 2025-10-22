//! Integration layer between axon and cc-sdk
//! Provides Tauri-specific wrappers and conversions

use cc_sdk::{ClaudeClient, Result as CcResult, Error as CcError};
use anyhow::Result;

/// Convert cc-sdk Error to anyhow::Error
pub fn convert_error(err: CcError) -> anyhow::Error {
    anyhow::anyhow!("{}", err)
}

/// Trait extension for Result conversion
pub trait ResultExt<T> {
    fn to_anyhow(self) -> Result<T>;
}

impl<T> ResultExt<T> for CcResult<T> {
    fn to_anyhow(self) -> Result<T> {
        self.map_err(convert_error)
    }
}
