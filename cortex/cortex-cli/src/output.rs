//! Output formatting utilities for the Cortex CLI.
//!
//! This module provides utilities for beautiful, user-friendly terminal output including:
//! - Colored output with consistent styling
//! - Table formatting for lists
//! - Progress bars for long operations
//! - JSON output for scripting
//! - User prompts and confirmations

use anyhow::{Context, Result};
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};
use console::style;
use dialoguer::{Confirm, Input, Select};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use std::fmt::Display;
use std::time::Duration;

/// Output format for CLI commands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable formatted output
    Human,
    /// JSON output for scripting
    Json,
    /// Plain text without colors
    Plain,
}

impl OutputFormat {
    pub fn from_flag(json: bool, plain: bool) -> Self {
        if json {
            Self::Json
        } else if plain {
            Self::Plain
        } else {
            Self::Human
        }
    }
}

/// Print a success message
pub fn success(msg: impl Display) {
    println!("{} {}", style("✓").green().bold(), msg);
}

/// Print an error message
pub fn error(msg: impl Display) {
    eprintln!("{} {}", style("✗").red().bold(), msg);
}

/// Print a warning message
pub fn warning(msg: impl Display) {
    println!("{} {}", style("⚠").yellow().bold(), msg);
}

/// Print an info message
pub fn info(msg: impl Display) {
    println!("{} {}", style("ℹ").blue().bold(), msg);
}

/// Print a section header
pub fn header(msg: impl Display) {
    println!("\n{}", style(msg).bold().underlined());
}

/// Print a key-value pair
pub fn kv(key: impl Display, value: impl Display) {
    println!("  {}: {}", style(key).cyan(), value);
}

/// Create a spinner for long-running operations
pub fn spinner(msg: impl Into<String>) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.blue} {msg}")
            .unwrap(),
    );
    pb.set_message(msg.into());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

/// Create a progress bar for known progress
pub fn progress_bar(len: u64, msg: impl Into<String>) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message(msg.into());
    pb
}

/// Prompt user for confirmation
pub fn confirm(msg: impl Into<String>) -> Result<bool> {
    Confirm::new()
        .with_prompt(msg)
        .default(false)
        .interact()
        .context("Failed to get user confirmation")
}

/// Prompt user for input
pub fn prompt<T: Clone + std::str::FromStr + Display>(msg: impl Into<String>) -> Result<T>
where
    <T as std::str::FromStr>::Err: Display,
{
    Input::new()
        .with_prompt(msg)
        .interact_text()
        .context("Failed to get user input")
}

/// Prompt user for input with default
pub fn prompt_default<T: Clone + std::str::FromStr + Display>(
    msg: impl Into<String>,
    default: T,
) -> Result<T>
where
    <T as std::str::FromStr>::Err: Display,
{
    Input::new()
        .with_prompt(msg)
        .default(default)
        .interact_text()
        .context("Failed to get user input")
}

/// Prompt user to select from a list
pub fn select(msg: impl Into<String>, items: &[impl ToString + Display]) -> Result<usize> {
    Select::new()
        .with_prompt(msg)
        .items(items)
        .interact()
        .context("Failed to get user selection")
}

/// Create a formatted table
pub struct TableBuilder {
    table: Table,
}

impl TableBuilder {
    pub fn new() -> Self {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic);
        Self { table }
    }

    pub fn header<I, S>(mut self, headers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String> + Display,
    {
        let row: Vec<Cell> = headers
            .into_iter()
            .map(|h| Cell::new(h).fg(Color::Cyan))
            .collect();
        self.table.set_header(row);
        self
    }

    pub fn row<I, S>(mut self, cells: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String> + Display,
    {
        let row: Vec<Cell> = cells.into_iter().map(|c| Cell::new(c)).collect();
        self.table.add_row(row);
        self
    }

    pub fn build(self) -> Table {
        self.table
    }

    pub fn print(self) {
        println!("{}", self.table);
    }
}

impl Default for TableBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Format bytes in human-readable form
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}

/// Format duration in human-readable form
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// Format timestamp in human-readable form
pub fn format_timestamp(ts: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(ts);

    if duration.num_seconds() < 60 {
        "just now".to_string()
    } else if duration.num_minutes() < 60 {
        format!("{} minutes ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{} hours ago", duration.num_hours())
    } else if duration.num_days() < 7 {
        format!("{} days ago", duration.num_days())
    } else {
        ts.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    }
}

/// Output data in the specified format
pub fn output<T: Serialize>(data: &T, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(data)?;
            println!("{}", json);
        }
        OutputFormat::Human | OutputFormat::Plain => {
            // Default to JSON for structured data in plain mode
            let json = serde_json::to_string_pretty(data)?;
            println!("{}", json);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1536 * 1024 * 1024), "1.50 GB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m");
    }

    #[test]
    fn test_output_format() {
        assert_eq!(OutputFormat::from_flag(true, false), OutputFormat::Json);
        assert_eq!(OutputFormat::from_flag(false, true), OutputFormat::Plain);
        assert_eq!(OutputFormat::from_flag(false, false), OutputFormat::Human);
        assert_eq!(OutputFormat::from_flag(true, true), OutputFormat::Json); // JSON takes precedence
    }

    #[test]
    fn test_table_builder() {
        let table = TableBuilder::new()
            .header(vec!["Name", "Age", "City"])
            .row(vec!["Alice", "30", "NYC"])
            .row(vec!["Bob", "25", "SF"])
            .build();

        assert_eq!(table.row_count(), 2);
    }
}
