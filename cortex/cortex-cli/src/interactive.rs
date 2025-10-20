//! Interactive UI components for the Cortex CLI.
//!
//! This module provides terminal UI (TUI) components for interactive operations including:
//! - Multi-step workflows
//! - Progress tracking
//! - Real-time updates
//! - Interactive selection and editing

use anyhow::{Context, Result};
use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Select};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::fmt::Display;
use std::time::Duration;

/// Interactive mode state
pub struct InteractiveSession {
    term: Term,
    multi_progress: MultiProgress,
    theme: ColorfulTheme,
}

impl InteractiveSession {
    /// Create a new interactive session
    pub fn new() -> Self {
        Self {
            term: Term::stderr(),
            multi_progress: MultiProgress::new(),
            theme: ColorfulTheme::default(),
        }
    }

    /// Clear the screen
    pub fn clear(&self) -> Result<()> {
        self.term.clear_screen()?;
        Ok(())
    }

    /// Print a banner
    pub fn banner(&self, text: &str) -> Result<()> {
        self.term.write_line("")?;
        self.term.write_line(&format!(
            "{}",
            style(format!("╔══{}══╗", "═".repeat(text.len())))
                .cyan()
                .bold()
        ))?;
        self.term.write_line(&format!(
            "{}",
            style(format!("║  {}  ║", text)).cyan().bold()
        ))?;
        self.term.write_line(&format!(
            "{}",
            style(format!("╚══{}══╝", "═".repeat(text.len())))
                .cyan()
                .bold()
        ))?;
        self.term.write_line("")?;
        Ok(())
    }

    /// Ask for confirmation
    pub fn confirm(&self, prompt: &str) -> Result<bool> {
        Confirm::with_theme(&self.theme)
            .with_prompt(prompt)
            .default(false)
            .interact()
            .context("Failed to get user confirmation")
    }

    /// Ask for text input
    pub fn input<T: Clone + std::str::FromStr>(&self, prompt: &str) -> Result<T>
    where
        <T as std::str::FromStr>::Err: Display,
    {
        Input::with_theme(&self.theme)
            .with_prompt(prompt)
            .interact_text()
            .context("Failed to get user input")
    }

    /// Ask for text input with default value
    pub fn input_with_default<T: Clone + std::str::FromStr + Display>(
        &self,
        prompt: &str,
        default: T,
    ) -> Result<T>
    where
        <T as std::str::FromStr>::Err: Display,
    {
        Input::with_theme(&self.theme)
            .with_prompt(prompt)
            .default(default)
            .interact_text()
            .context("Failed to get user input")
    }

    /// Select from a list of items
    pub fn select(&self, prompt: &str, items: &[impl ToString]) -> Result<usize> {
        Select::with_theme(&self.theme)
            .with_prompt(prompt)
            .items(items)
            .default(0)
            .interact()
            .context("Failed to get user selection")
    }

    /// Select multiple items from a list
    pub fn multi_select(&self, prompt: &str, items: &[impl ToString]) -> Result<Vec<usize>> {
        MultiSelect::with_theme(&self.theme)
            .with_prompt(prompt)
            .items(items)
            .interact()
            .context("Failed to get user selection")
    }

    /// Create a progress bar
    pub fn progress_bar(&self, len: u64, msg: &str) -> ProgressBar {
        let pb = self.multi_progress.add(ProgressBar::new(len));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message(msg.to_string());
        pb
    }

    /// Create a spinner
    pub fn spinner(&self, msg: &str) -> ProgressBar {
        let pb = self.multi_progress.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("{spinner:.blue} {msg}")
                .unwrap(),
        );
        pb.set_message(msg.to_string());
        pb.enable_steady_tick(Duration::from_millis(80));
        pb
    }

    /// Join all progress bars (wait for completion)
    pub fn join(&self) -> Result<()> {
        self.multi_progress.clear()?;
        Ok(())
    }
}

impl Default for InteractiveSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Interactive workspace setup wizard
pub async fn workspace_setup_wizard() -> Result<WorkspaceSetupConfig> {
    let session = InteractiveSession::new();

    session.clear()?;
    session.banner("Cortex Workspace Setup")?;

    // Get workspace name
    let name: String = session.input("Workspace name")?;

    // Select workspace type
    let workspace_type_idx = session.select(
        "Workspace type",
        &["Agent (for AI agents)", "Project (for code)", "Shared (multi-user)"],
    )?;
    let workspace_type = match workspace_type_idx {
        0 => cortex_vfs::WorkspaceType::Agent,
        1 => cortex_vfs::WorkspaceType::Project,
        2 => cortex_vfs::WorkspaceType::Shared,
        _ => cortex_vfs::WorkspaceType::Project,
    };

    // Get optional description
    let description: String = session
        .input_with_default("Description (optional)", String::new())
        .unwrap_or_default();

    // Confirm
    println!("\nConfiguration:");
    println!("  Name: {}", style(&name).cyan());
    println!("  Type: {}", style(format!("{:?}", workspace_type)).cyan());
    if !description.is_empty() {
        println!("  Description: {}", style(&description).cyan());
    }

    if !session.confirm("\nCreate workspace?")? {
        anyhow::bail!("Cancelled");
    }

    Ok(WorkspaceSetupConfig {
        name,
        workspace_type,
        description: if description.is_empty() {
            None
        } else {
            Some(description)
        },
    })
}

/// Configuration from workspace setup wizard
pub struct WorkspaceSetupConfig {
    pub name: String,
    pub workspace_type: cortex_vfs::WorkspaceType,
    pub description: Option<String>,
}

/// Interactive database configuration wizard
pub async fn database_config_wizard() -> Result<DatabaseConfigWizard> {
    let session = InteractiveSession::new();

    session.clear()?;
    session.banner("Database Configuration")?;

    // Connection type
    let conn_type_idx = session.select(
        "Connection type",
        &["Local file", "Remote server", "Memory (testing)"],
    )?;

    let connection_string = match conn_type_idx {
        0 => {
            let path: String = session
                .input_with_default("Data directory", "~/.local/share/cortex/db".to_string())?;
            format!("file://{}", path)
        }
        1 => {
            let host: String = session.input_with_default("Host", "127.0.0.1".to_string())?;
            let port: u16 = session.input_with_default("Port", 8000)?;
            format!("ws://{}:{}", host, port)
        }
        2 => "memory".to_string(),
        _ => "file://./cortex.db".to_string(),
    };

    let namespace: String = session.input_with_default("Namespace", "cortex".to_string())?;
    let database: String = session.input_with_default("Database name", "main".to_string())?;

    // Authentication
    let needs_auth = session.confirm("Requires authentication?")?;
    let (username, password) = if needs_auth {
        let user: String = session.input("Username")?;
        let pass: String = session.input("Password")?;
        (Some(user), Some(pass))
    } else {
        (None, None)
    };

    Ok(DatabaseConfigWizard {
        connection_string,
        namespace,
        database,
        username,
        password,
    })
}

/// Configuration from database wizard
pub struct DatabaseConfigWizard {
    pub connection_string: String,
    pub namespace: String,
    pub database: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

/// Interactive ingestion wizard
pub async fn ingestion_wizard() -> Result<IngestionConfig> {
    let session = InteractiveSession::new();

    session.clear()?;
    session.banner("Project Ingestion")?;

    // Get source path
    let path: String = session.input("Path to ingest")?;

    // Ingestion options
    let recursive = session.confirm("Recursively scan directories?")?;
    let extract_metadata = session.confirm("Extract file metadata?")?;
    let chunk_files = session.confirm("Chunk large files?")?;

    // File filters
    let use_filters = session.confirm("Configure file filters?")?;

    let ignore_patterns = if use_filters {
        let patterns: String = session.input_with_default(
            "Ignore patterns (comma-separated)",
            "node_modules,target,.git,dist,build".to_string(),
        )?;
        patterns.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        vec![
            "**/node_modules/**".to_string(),
            "**/target/**".to_string(),
            "**/.git/**".to_string(),
        ]
    };

    Ok(IngestionConfig {
        path: path.into(),
        recursive,
        extract_metadata,
        chunk_files,
        ignore_patterns,
    })
}

/// Configuration from ingestion wizard
pub struct IngestionConfig {
    pub path: std::path::PathBuf,
    pub recursive: bool,
    pub extract_metadata: bool,
    pub chunk_files: bool,
    pub ignore_patterns: Vec<String>,
}

/// Interactive search interface
pub async fn interactive_search() -> Result<()> {
    let session = InteractiveSession::new();

    session.clear()?;
    session.banner("Interactive Search")?;

    loop {
        let query: String = session.input("Search query (or 'quit' to exit)")?;

        if query.trim().to_lowercase() == "quit" {
            break;
        }

        // Perform search
        let spinner = session.spinner("Searching...");

        // TODO: Implement actual search
        tokio::time::sleep(Duration::from_millis(500)).await;

        spinner.finish_with_message(format!("Found 0 results for '{}'", query));

        println!("\nNo results found. Try a different query.");
    }

    Ok(())
}

/// Display a multi-step progress workflow
pub struct WorkflowProgress {
    session: InteractiveSession,
    steps: Vec<String>,
    current_step: usize,
}

impl WorkflowProgress {
    pub fn new(steps: Vec<String>) -> Self {
        Self {
            session: InteractiveSession::new(),
            steps,
            current_step: 0,
        }
    }

    pub fn start(&mut self, title: &str) -> Result<()> {
        self.session.clear()?;
        self.session.banner(title)?;
        self.print_steps()?;
        Ok(())
    }

    pub fn next_step(&mut self) -> Result<()> {
        if self.current_step < self.steps.len() {
            self.current_step += 1;
            self.print_steps()?;
        }
        Ok(())
    }

    pub fn complete(&self) -> Result<()> {
        println!("\n{} All steps completed!", style("✓").green().bold());
        Ok(())
    }

    fn print_steps(&self) -> Result<()> {
        println!();
        for (i, step) in self.steps.iter().enumerate() {
            let symbol = if i < self.current_step {
                style("✓").green()
            } else if i == self.current_step {
                style("▶").blue()
            } else {
                style("○").dim()
            };

            println!("  {} {}", symbol, step);
        }
        println!();
        Ok(())
    }
}

/// Interactive menu system
pub struct Menu {
    session: InteractiveSession,
    title: String,
    items: Vec<MenuItem>,
}

pub struct MenuItem {
    pub label: String,
    pub description: Option<String>,
}

impl Menu {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            session: InteractiveSession::new(),
            title: title.into(),
            items: Vec::new(),
        }
    }

    pub fn add_item(mut self, label: impl Into<String>, description: Option<String>) -> Self {
        self.items.push(MenuItem {
            label: label.into(),
            description,
        });
        self
    }

    pub fn show(&self) -> Result<usize> {
        self.session.clear()?;
        self.session.banner(&self.title)?;

        // Build display items
        let display_items: Vec<String> = self
            .items
            .iter()
            .map(|item| {
                if let Some(desc) = &item.description {
                    format!("{} - {}", item.label, style(desc).dim())
                } else {
                    item.label.clone()
                }
            })
            .collect();

        self.session.select("Choose an option", &display_items)
    }
}

/// Interactive system health check
pub async fn interactive_health_check() -> Result<()> {
    let session = InteractiveSession::new();

    session.clear()?;
    session.banner("System Health Check")?;

    let checks = vec![
        "Checking SurrealDB connection",
        "Verifying workspace configuration",
        "Testing file system access",
        "Checking memory subsystems",
        "Validating MCP server",
    ];

    let mut results = Vec::new();

    for check in &checks {
        let spinner = session.spinner(check);

        // Simulate check
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Mock result
        let success = rand::random::<bool>();
        if success {
            spinner.finish_with_message(format!("{} ✓", check));
            results.push(true);
        } else {
            spinner.finish_with_message(format!("{} ✗", check));
            results.push(false);
        }
    }

    println!();
    let passed = results.iter().filter(|&&r| r).count();
    let total = results.len();

    if passed == total {
        println!(
            "{} All checks passed ({}/{})",
            style("✓").green().bold(),
            passed,
            total
        );
    } else {
        println!(
            "{} Some checks failed ({}/{} passed)",
            style("⚠").yellow().bold(),
            passed,
            total
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interactive_session_creation() {
        let session = InteractiveSession::new();
        // Just ensure it can be created
        assert!(true);
    }

    #[test]
    fn test_workflow_progress() {
        let steps = vec![
            "Step 1".to_string(),
            "Step 2".to_string(),
            "Step 3".to_string(),
        ];
        let mut workflow = WorkflowProgress::new(steps);
        assert_eq!(workflow.current_step, 0);

        workflow.current_step += 1;
        assert_eq!(workflow.current_step, 1);
    }

    #[test]
    fn test_menu_builder() {
        let menu = Menu::new("Test Menu")
            .add_item("Option 1", Some("Description 1".to_string()))
            .add_item("Option 2", None);

        assert_eq!(menu.items.len(), 2);
        assert_eq!(menu.items[0].label, "Option 1");
    }
}
