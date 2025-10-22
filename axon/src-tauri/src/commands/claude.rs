use anyhow::{Context, Result};
use cc_sdk::core::SessionId;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tauri::{AppHandle, Emitter, Manager};


/// Represents a project in the ~/.claude/projects directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// The project ID (derived from the directory name)
    pub id: String,
    /// The original project path (decoded from the directory name)
    pub path: String,
    /// List of session IDs (JSONL file names without extension)
    pub sessions: Vec<String>,
    /// Unix timestamp when the project directory was created
    pub created_at: u64,
    /// Unix timestamp of the most recent session (if any)
    pub most_recent_session: Option<u64>,
}

/// Represents a session with its metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// The session ID (UUID)
    pub id: String,
    /// The project ID this session belongs to
    pub project_id: String,
    /// The project path
    pub project_path: String,
    /// Optional todo data associated with this session
    pub todo_data: Option<serde_json::Value>,
    /// Unix timestamp when the session file was created
    pub created_at: u64,
    /// First user message content (if available)
    pub first_message: Option<String>,
    /// Timestamp of the first user message (if available)
    pub message_timestamp: Option<String>,
}

/// Represents a message entry in the JSONL file
#[derive(Debug, Deserialize)]
struct JsonlEntry {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    entry_type: Option<String>,
    message: Option<MessageContent>,
    timestamp: Option<String>,
}

/// Represents the message content
#[derive(Debug, Deserialize)]
struct MessageContent {
    role: Option<String>,
    content: Option<String>,
}

/// Represents the settings from ~/.claude/settings.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeSettings {
    #[serde(flatten)]
    pub data: serde_json::Value,
}

impl Default for ClaudeSettings {
    fn default() -> Self {
        Self {
            data: serde_json::json!({}),
        }
    }
}

/// Represents the Claude Code version status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeVersionStatus {
    /// Whether Claude Code is installed and working
    pub is_installed: bool,
    /// The version string if available
    pub version: Option<String>,
    /// The full output from the command
    pub output: String,
}

/// Represents a CLAUDE.md file found in the project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMdFile {
    /// Relative path from the project root
    pub relative_path: String,
    /// Absolute path to the file
    pub absolute_path: String,
    /// File size in bytes
    pub size: u64,
    /// Last modified timestamp
    pub modified: u64,
}

/// Represents a file or directory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// The name of the file or directory
    pub name: String,
    /// The full path
    pub path: String,
    /// Whether this is a directory
    pub is_directory: bool,
    /// File size in bytes (0 for directories)
    pub size: u64,
    /// File extension (if applicable)
    pub extension: Option<String>,
}

/// Finds the full path to the claude binary
/// This is necessary because macOS apps have a limited PATH environment
fn find_claude_binary(app_handle: &AppHandle) -> Result<String, String> {
    crate::claude_binary::find_claude_binary(app_handle)
}

/// Gets the path to the ~/.claude directory
fn get_claude_dir() -> Result<PathBuf> {
    dirs::home_dir()
        .context("Could not find home directory")?
        .join(".claude")
        .canonicalize()
        .context("Could not find ~/.claude directory")
}

/// Gets the actual project path by reading the cwd from the JSONL entries
fn get_project_path_from_sessions(project_dir: &PathBuf) -> Result<String, String> {
    // Try to read any JSONL file in the directory
    let entries = fs::read_dir(project_dir)
        .map_err(|e| format!("Failed to read project directory: {}", e))?;

    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                // Read the JSONL file and find the first line with a valid cwd
                if let Ok(file) = fs::File::open(&path) {
                    let reader = BufReader::new(file);
                    // Check first few lines instead of just the first line
                    // Some session files may have null cwd in the first line
                    for line in reader.lines().take(10) {
                        if let Ok(line_content) = line {
                            // Parse the JSON and extract cwd
                            if let Ok(json) =
                                serde_json::from_str::<serde_json::Value>(&line_content)
                            {
                                if let Some(cwd) = json.get("cwd").and_then(|v| v.as_str()) {
                                    if !cwd.is_empty() {
                                        return Ok(cwd.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Err("Could not determine project path from session files".to_string())
}

/// Decodes a project directory name back to its original path
/// The directory names in ~/.claude/projects are encoded paths
/// DEPRECATED: Use get_project_path_from_sessions instead when possible
fn decode_project_path(encoded: &str) -> String {
    // This is a fallback - the encoding isn't reversible when paths contain hyphens
    // For example: -Users-mufeedvh-dev-jsonl-viewer could be /Users/mufeedvh/dev/jsonl-viewer
    // or /Users/mufeedvh/dev/jsonl/viewer
    encoded.replace('-', "/")
}

/// Extracts the first valid user message from a JSONL file
fn extract_first_user_message(jsonl_path: &PathBuf) -> (Option<String>, Option<String>) {
    let file = match fs::File::open(jsonl_path) {
        Ok(file) => file,
        Err(_) => return (None, None),
    };

    let reader = BufReader::new(file);

    for line in reader.lines() {
        if let Ok(line) = line {
            if let Ok(entry) = serde_json::from_str::<JsonlEntry>(&line) {
                if let Some(message) = entry.message {
                    if message.role.as_deref() == Some("user") {
                        if let Some(content) = message.content {
                            // Skip if it contains the caveat message
                            if content.contains("Caveat: The messages below were generated by the user while running local commands") {
                                continue;
                            }

                            // Skip if it starts with command tags
                            if content.starts_with("<command-name>")
                                || content.starts_with("<local-command-stdout>")
                            {
                                continue;
                            }

                            // Found a valid user message
                            return (Some(content), entry.timestamp);
                        }
                    }
                }
            }
        }
    }

    (None, None)
}


/// Gets the user's home directory path
#[tauri::command]
pub async fn get_home_directory() -> Result<String, String> {
    dirs::home_dir()
        .and_then(|path| path.to_str().map(|s| s.to_string()))
        .ok_or_else(|| "Could not determine home directory".to_string())
}

/// Lists all projects in the ~/.claude/projects directory
#[tauri::command]
pub async fn list_projects() -> Result<Vec<Project>, String> {
    // Use the adapter to list projects via cc-sdk
    crate::adapters::session::list_projects_adapter()
        .await
        .map_err(|e| {
            log::error!("Failed to list projects via adapter: {}", e);
            format!("Failed to list projects: {}", e)
        })
}

/// Creates a new project for the given directory path
#[tauri::command]
pub async fn create_project(path: String) -> Result<Project, String> {
    // Use the adapter to create a project
    crate::adapters::session::create_project_adapter(path)
        .await
        .map_err(|e| {
            log::error!("Failed to create project via adapter: {}", e);
            format!("Failed to create project: {}", e)
        })
}

/// Gets sessions for a specific project
#[tauri::command]
pub async fn get_project_sessions(project_id: String) -> Result<Vec<Session>, String> {
    // Use the adapter to get project sessions via cc-sdk
    crate::adapters::session::get_project_sessions_adapter(project_id)
        .await
        .map_err(|e| {
            log::error!("Failed to get project sessions via adapter: {}", e);
            format!("Failed to get project sessions: {}", e)
        })
}

/// Reads the Claude settings file
#[tauri::command]
pub async fn get_claude_settings() -> Result<ClaudeSettings, String> {
    log::info!("Reading Claude settings");

    let claude_dir = get_claude_dir().map_err(|e| e.to_string())?;
    let settings_path = claude_dir.join("settings.json");

    if !settings_path.exists() {
        log::warn!("Settings file not found, returning empty settings");
        return Ok(ClaudeSettings {
            data: serde_json::json!({}),
        });
    }

    let content = fs::read_to_string(&settings_path)
        .map_err(|e| format!("Failed to read settings file: {}", e))?;

    let data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse settings JSON: {}", e))?;

    Ok(ClaudeSettings { data })
}

/// Opens a new Claude Code session by executing the claude command
#[tauri::command]
pub async fn open_new_session(app: AppHandle, path: Option<String>) -> Result<String, String> {
    log::info!("Opening new Claude Code session at path: {:?}", path);

    #[cfg(not(debug_assertions))]
    let _claude_path = find_claude_binary(&app)?;

    #[cfg(debug_assertions)]
    let claude_path = find_claude_binary(&app)?;

    // In production, we can't use std::process::Command directly
    // The user should launch Claude Code through other means or use the execute_claude_code command
    #[cfg(not(debug_assertions))]
    {
        log::error!("Cannot spawn processes directly in production builds");
        return Err("Direct process spawning is not available in production builds. Please use Claude Code directly or use the integrated execution commands.".to_string());
    }

    #[cfg(debug_assertions)]
    {
        let mut cmd = std::process::Command::new(claude_path);

        // If a path is provided, use it; otherwise use current directory
        if let Some(project_path) = path {
            cmd.current_dir(&project_path);
        }

        // Execute the command
        match cmd.spawn() {
            Ok(_) => {
                log::info!("Successfully launched Claude Code");
                Ok("Claude Code session started".to_string())
            }
            Err(e) => {
                log::error!("Failed to launch Claude Code: {}", e);
                Err(format!("Failed to launch Claude Code: {}", e))
            }
        }
    }
}

/// Reads the CLAUDE.md system prompt file
#[tauri::command]
pub async fn get_system_prompt() -> Result<String, String> {
    log::info!("Reading CLAUDE.md system prompt");

    let claude_dir = get_claude_dir().map_err(|e| e.to_string())?;
    let claude_md_path = claude_dir.join("CLAUDE.md");

    if !claude_md_path.exists() {
        log::warn!("CLAUDE.md not found");
        return Ok(String::new());
    }

    fs::read_to_string(&claude_md_path).map_err(|e| format!("Failed to read CLAUDE.md: {}", e))
}

/// Checks if Claude Code is installed and gets its version
#[tauri::command]
pub async fn check_claude_version(app: AppHandle) -> Result<ClaudeVersionStatus, String> {
    log::info!("Checking Claude Code version");

    let claude_path = match find_claude_binary(&app) {
        Ok(path) => path,
        Err(e) => {
            return Ok(ClaudeVersionStatus {
                is_installed: false,
                version: None,
                output: e,
            });
        }
    };

    use log::debug;
    debug!("Claude path: {}", claude_path);

    // In production builds, we can't check the version directly
    #[cfg(not(debug_assertions))]
    {
        log::warn!("Cannot check claude version in production build");
        // If we found a path (either stored or in common locations), assume it's installed
        if claude_path != "claude" && PathBuf::from(&claude_path).exists() {
            return Ok(ClaudeVersionStatus {
                is_installed: true,
                version: None,
                output: "Claude binary found at: ".to_string() + &claude_path,
            });
        } else {
            return Ok(ClaudeVersionStatus {
                is_installed: false,
                version: None,
                output: "Cannot verify Claude installation in production build. Please ensure Claude Code is installed.".to_string(),
            });
        }
    }

    #[cfg(debug_assertions)]
    {
        let output = std::process::Command::new(claude_path)
            .arg("--version")
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                // Use regex to directly extract version pattern (e.g., "1.0.41")
                let version_regex =
                    regex::Regex::new(r"(\d+\.\d+\.\d+(?:-[a-zA-Z0-9.-]+)?(?:\+[a-zA-Z0-9.-]+)?)")
                        .ok();

                let version = if let Some(regex) = version_regex {
                    regex
                        .captures(&stdout)
                        .and_then(|captures| captures.get(1))
                        .map(|m| m.as_str().to_string())
                } else {
                    None
                };

                let full_output = if stderr.is_empty() {
                    stdout.clone()
                } else {
                    format!("{}\n{}", stdout, stderr)
                };

                // Check if the output matches the expected format
                // Expected format: "1.0.17 (Claude Code)" or similar
                let is_valid = stdout.contains("(Claude Code)") || stdout.contains("Claude Code");

                Ok(ClaudeVersionStatus {
                    is_installed: is_valid && output.status.success(),
                    version,
                    output: full_output.trim().to_string(),
                })
            }
            Err(e) => {
                log::error!("Failed to run claude command: {}", e);
                Ok(ClaudeVersionStatus {
                    is_installed: false,
                    version: None,
                    output: format!("Command not found: {}", e),
                })
            }
        }
    }
}

/// Saves the CLAUDE.md system prompt file
#[tauri::command]
pub async fn save_system_prompt(content: String) -> Result<String, String> {
    log::info!("Saving CLAUDE.md system prompt");

    let claude_dir = get_claude_dir().map_err(|e| e.to_string())?;
    let claude_md_path = claude_dir.join("CLAUDE.md");

    fs::write(&claude_md_path, content).map_err(|e| format!("Failed to write CLAUDE.md: {}", e))?;

    Ok("System prompt saved successfully".to_string())
}

/// Saves the Claude settings file
#[tauri::command]
pub async fn save_claude_settings(settings: serde_json::Value) -> Result<String, String> {
    log::info!("Saving Claude settings");

    let claude_dir = get_claude_dir().map_err(|e| e.to_string())?;
    let settings_path = claude_dir.join("settings.json");

    // Pretty print the JSON with 2-space indentation
    let json_string = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    fs::write(&settings_path, json_string)
        .map_err(|e| format!("Failed to write settings file: {}", e))?;

    Ok("Settings saved successfully".to_string())
}

/// Recursively finds all CLAUDE.md files in a project directory
#[tauri::command]
pub async fn find_claude_md_files(project_path: String) -> Result<Vec<ClaudeMdFile>, String> {
    log::info!("Finding CLAUDE.md files in project: {}", project_path);

    let path = PathBuf::from(&project_path);
    if !path.exists() {
        return Err(format!("Project path does not exist: {}", project_path));
    }

    let mut claude_files = Vec::new();
    find_claude_md_recursive(&path, &path, &mut claude_files)?;

    // Sort by relative path
    claude_files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    log::info!("Found {} CLAUDE.md files", claude_files.len());
    Ok(claude_files)
}

/// Helper function to recursively find CLAUDE.md files
fn find_claude_md_recursive(
    current_path: &PathBuf,
    project_root: &PathBuf,
    claude_files: &mut Vec<ClaudeMdFile>,
) -> Result<(), String> {
    let entries = fs::read_dir(current_path)
        .map_err(|e| format!("Failed to read directory {:?}: {}", current_path, e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        // Skip hidden files/directories
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') {
                continue;
            }
        }

        if path.is_dir() {
            // Skip common directories that shouldn't be searched
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                if matches!(
                    dir_name,
                    "node_modules" | "target" | ".git" | "dist" | "build" | ".next" | "__pycache__"
                ) {
                    continue;
                }
            }

            find_claude_md_recursive(&path, project_root, claude_files)?;
        } else if path.is_file() {
            // Check if it's a CLAUDE.md file (case insensitive)
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.eq_ignore_ascii_case("CLAUDE.md") {
                    let metadata = fs::metadata(&path)
                        .map_err(|e| format!("Failed to read file metadata: {}", e))?;

                    let relative_path = path
                        .strip_prefix(project_root)
                        .map_err(|e| format!("Failed to get relative path: {}", e))?
                        .to_string_lossy()
                        .to_string();

                    let modified = metadata
                        .modified()
                        .unwrap_or(SystemTime::UNIX_EPOCH)
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();

                    claude_files.push(ClaudeMdFile {
                        relative_path,
                        absolute_path: path.to_string_lossy().to_string(),
                        size: metadata.len(),
                        modified,
                    });
                }
            }
        }
    }

    Ok(())
}

/// Reads a specific CLAUDE.md file by its absolute path
#[tauri::command]
pub async fn read_claude_md_file(file_path: String) -> Result<String, String> {
    log::info!("Reading CLAUDE.md file: {}", file_path);

    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))
}

/// Saves a specific CLAUDE.md file by its absolute path
#[tauri::command]
pub async fn save_claude_md_file(file_path: String, content: String) -> Result<String, String> {
    log::info!("Saving CLAUDE.md file: {}", file_path);

    let path = PathBuf::from(&file_path);

    // Ensure the parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
    }

    fs::write(&path, content).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok("File saved successfully".to_string())
}

/// Loads the JSONL history for a specific session
#[tauri::command]
pub async fn load_session_history(
    session_id: String,
    project_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    // Use the adapter to load session history via cc-sdk
    crate::adapters::session::load_session_history_adapter(session_id, project_id)
        .await
        .map_err(|e| {
            log::error!("Failed to load session history via adapter: {}", e);
            format!("Failed to load session history: {}", e)
        })
}


/// Get all running Claude sessions
#[tauri::command]
pub async fn list_running_claude_sessions(
    registry: tauri::State<'_, crate::process::ProcessRegistryState>,
) -> Result<Vec<crate::process::ProcessInfo>, String> {
    // Get cc_sdk::process::ProcessInfo and convert to crate::process::ProcessInfo
    let sessions = registry.0.get_running_claude_sessions();
    Ok(sessions.into_iter().map(|info| crate::process::ProcessInfo {
        run_id: info.run_id,
        process_type: match info.process_type {
            cc_sdk::process::ProcessType::ClaudeSession { session_id } => {
                crate::process::ProcessType::ClaudeSession { session_id }
            }
            cc_sdk::process::ProcessType::AgentRun { agent_id, agent_name } => {
                crate::process::ProcessType::AgentRun { agent_id, agent_name }
            }
        },
        pid: info.pid,
        started_at: chrono::DateTime::<chrono::Utc>::from(info.started_at),
        project_path: info.project_path,
        task: info.task,
        model: info.model,
    }).collect())
}

/// Get live output from a Claude session
#[tauri::command]
pub async fn get_claude_session_output(
    registry: tauri::State<'_, crate::process::ProcessRegistryState>,
    session_id: String,
) -> Result<String, String> {
    // Find the process by session ID
    let session_id_obj = SessionId::new(&session_id);
    if let Some(handle) = registry.0.get(&session_id_obj) {
        handle.get_output().await.map_err(|e| e.to_string())
    } else {
        Ok(String::new())
    }
}


/// Lists files and directories in a given path
#[tauri::command]
pub async fn list_directory_contents(directory_path: String) -> Result<Vec<FileEntry>, String> {
    log::info!("Listing directory contents: '{}'", directory_path);

    // Check if path is empty
    if directory_path.trim().is_empty() {
        log::error!("Directory path is empty or whitespace");
        return Err("Directory path cannot be empty".to_string());
    }

    let path = PathBuf::from(&directory_path);
    log::debug!("Resolved path: {:?}", path);

    if !path.exists() {
        log::error!("Path does not exist: {:?}", path);
        return Err(format!("Path does not exist: {}", directory_path));
    }

    if !path.is_dir() {
        log::error!("Path is not a directory: {:?}", path);
        return Err(format!("Path is not a directory: {}", directory_path));
    }

    let mut entries = Vec::new();

    let dir_entries =
        fs::read_dir(&path).map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in dir_entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let entry_path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|e| format!("Failed to read metadata: {}", e))?;

        // Skip hidden files/directories unless they are .claude directories
        if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') && name != ".claude" {
                continue;
            }
        }

        let name = entry_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let extension = if metadata.is_file() {
            entry_path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_string())
        } else {
            None
        };

        entries.push(FileEntry {
            name,
            path: entry_path.to_string_lossy().to_string(),
            is_directory: metadata.is_dir(),
            size: metadata.len(),
            extension,
        });
    }

    // Sort: directories first, then files, alphabetically within each group
    entries.sort_by(|a, b| match (a.is_directory, b.is_directory) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(entries)
}

/// Search for files and directories matching a pattern
#[tauri::command]
pub async fn search_files(base_path: String, query: String) -> Result<Vec<FileEntry>, String> {
    log::info!("Searching files in '{}' for: '{}'", base_path, query);

    // Check if path is empty
    if base_path.trim().is_empty() {
        log::error!("Base path is empty or whitespace");
        return Err("Base path cannot be empty".to_string());
    }

    // Check if query is empty
    if query.trim().is_empty() {
        log::warn!("Search query is empty, returning empty results");
        return Ok(Vec::new());
    }

    let path = PathBuf::from(&base_path);
    log::debug!("Resolved search base path: {:?}", path);

    if !path.exists() {
        log::error!("Base path does not exist: {:?}", path);
        return Err(format!("Path does not exist: {}", base_path));
    }

    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    search_files_recursive(&path, &path, &query_lower, &mut results, 0)?;

    // Sort by relevance: exact matches first, then by name
    results.sort_by(|a, b| {
        let a_exact = a.name.to_lowercase() == query_lower;
        let b_exact = b.name.to_lowercase() == query_lower;

        match (a_exact, b_exact) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    // Limit results to prevent overwhelming the UI
    results.truncate(50);

    Ok(results)
}

fn search_files_recursive(
    current_path: &PathBuf,
    base_path: &PathBuf,
    query: &str,
    results: &mut Vec<FileEntry>,
    depth: usize,
) -> Result<(), String> {
    // Limit recursion depth to prevent excessive searching
    if depth > 5 || results.len() >= 50 {
        return Ok(());
    }

    let entries = fs::read_dir(current_path)
        .map_err(|e| format!("Failed to read directory {:?}: {}", current_path, e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let entry_path = entry.path();

        // Skip hidden files/directories
        if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') {
                continue;
            }

            // Check if name matches query
            if name.to_lowercase().contains(query) {
                let metadata = entry
                    .metadata()
                    .map_err(|e| format!("Failed to read metadata: {}", e))?;

                let extension = if metadata.is_file() {
                    entry_path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e.to_string())
                } else {
                    None
                };

                results.push(FileEntry {
                    name: name.to_string(),
                    path: entry_path.to_string_lossy().to_string(),
                    is_directory: metadata.is_dir(),
                    size: metadata.len(),
                    extension,
                });
            }
        }

        // Recurse into directories
        if entry_path.is_dir() {
            // Skip common directories that shouldn't be searched
            if let Some(dir_name) = entry_path.file_name().and_then(|n| n.to_str()) {
                if matches!(
                    dir_name,
                    "node_modules" | "target" | ".git" | "dist" | "build" | ".next" | "__pycache__"
                ) {
                    continue;
                }
            }

            search_files_recursive(&entry_path, base_path, query, results, depth + 1)?;
        }
    }

    Ok(())
}

/// Creates a checkpoint for the current session state
#[tauri::command]
pub async fn create_checkpoint(
    app: tauri::State<'_, crate::checkpoint::state::CheckpointState>,
    session_id: String,
    project_id: String,
    project_path: String,
    message_index: Option<usize>,
    description: Option<String>,
) -> Result<crate::checkpoint::CheckpointResult, String> {
    log::info!(
        "Creating checkpoint for session: {} in project: {}",
        session_id,
        project_id
    );

    let manager = app
        .get_or_create_manager(
            session_id.clone(),
            project_id.clone(),
            PathBuf::from(&project_path),
        )
        .await
        .map_err(|e| format!("Failed to get checkpoint manager: {}", e))?;

    // Always load current session messages from the JSONL file
    let session_path = get_claude_dir()
        .map_err(|e| e.to_string())?
        .join("projects")
        .join(&project_id)
        .join(format!("{}.jsonl", session_id));

    if session_path.exists() {
        let file = fs::File::open(&session_path)
            .map_err(|e| format!("Failed to open session file: {}", e))?;
        let reader = BufReader::new(file);

        let mut line_count = 0;
        for line in reader.lines() {
            if let Some(index) = message_index {
                if line_count > index {
                    break;
                }
            }
            if let Ok(line) = line {
                manager
                    .track_message(line)
                    .await
                    .map_err(|e| format!("Failed to track message: {}", e))?;
            }
            line_count += 1;
        }
    }

    manager
        .create_checkpoint(description, None)
        .await
        .map_err(|e| format!("Failed to create checkpoint: {}", e))
}

/// Restores a session to a specific checkpoint
#[tauri::command]
pub async fn restore_checkpoint(
    app: tauri::State<'_, crate::checkpoint::state::CheckpointState>,
    checkpoint_id: String,
    session_id: String,
    project_id: String,
    project_path: String,
) -> Result<crate::checkpoint::CheckpointResult, String> {
    log::info!(
        "Restoring checkpoint: {} for session: {}",
        checkpoint_id,
        session_id
    );

    let manager = app
        .get_or_create_manager(
            session_id.clone(),
            project_id.clone(),
            PathBuf::from(&project_path),
        )
        .await
        .map_err(|e| format!("Failed to get checkpoint manager: {}", e))?;

    let result = manager
        .restore_checkpoint(&checkpoint_id)
        .await
        .map_err(|e| format!("Failed to restore checkpoint: {}", e))?;

    // Update the session JSONL file with restored messages
    let claude_dir = get_claude_dir().map_err(|e| e.to_string())?;
    let session_path = claude_dir
        .join("projects")
        .join(&result.checkpoint.project_id)
        .join(format!("{}.jsonl", session_id));

    // The manager has already restored the messages internally,
    // but we need to update the actual session file
    let (_, _, messages) = manager
        .storage
        .load_checkpoint(&result.checkpoint.project_id, &session_id, &checkpoint_id)
        .map_err(|e| format!("Failed to load checkpoint data: {}", e))?;

    fs::write(&session_path, messages)
        .map_err(|e| format!("Failed to update session file: {}", e))?;

    Ok(result)
}

/// Lists all checkpoints for a session
#[tauri::command]
pub async fn list_checkpoints(
    app: tauri::State<'_, crate::checkpoint::state::CheckpointState>,
    session_id: String,
    project_id: String,
    project_path: String,
) -> Result<Vec<crate::checkpoint::Checkpoint>, String> {
    log::info!(
        "Listing checkpoints for session: {} in project: {}",
        session_id,
        project_id
    );

    let manager = app
        .get_or_create_manager(session_id, project_id, PathBuf::from(&project_path))
        .await
        .map_err(|e| format!("Failed to get checkpoint manager: {}", e))?;

    Ok(manager.list_checkpoints().await)
}

/// Forks a new timeline branch from a checkpoint
#[tauri::command]
pub async fn fork_from_checkpoint(
    app: tauri::State<'_, crate::checkpoint::state::CheckpointState>,
    checkpoint_id: String,
    session_id: String,
    project_id: String,
    project_path: String,
    new_session_id: String,
    description: Option<String>,
) -> Result<crate::checkpoint::CheckpointResult, String> {
    log::info!(
        "Forking from checkpoint: {} to new session: {}",
        checkpoint_id,
        new_session_id
    );

    let claude_dir = get_claude_dir().map_err(|e| e.to_string())?;

    // First, copy the session file to the new session
    let source_session_path = claude_dir
        .join("projects")
        .join(&project_id)
        .join(format!("{}.jsonl", session_id));
    let new_session_path = claude_dir
        .join("projects")
        .join(&project_id)
        .join(format!("{}.jsonl", new_session_id));

    if source_session_path.exists() {
        fs::copy(&source_session_path, &new_session_path)
            .map_err(|e| format!("Failed to copy session file: {}", e))?;
    }

    // Create manager for the new session
    let manager = app
        .get_or_create_manager(
            new_session_id.clone(),
            project_id,
            PathBuf::from(&project_path),
        )
        .await
        .map_err(|e| format!("Failed to get checkpoint manager: {}", e))?;

    manager
        .fork_from_checkpoint(&checkpoint_id, description)
        .await
        .map_err(|e| format!("Failed to fork checkpoint: {}", e))
}

/// Gets the timeline for a session
#[tauri::command]
pub async fn get_session_timeline(
    app: tauri::State<'_, crate::checkpoint::state::CheckpointState>,
    session_id: String,
    project_id: String,
    project_path: String,
) -> Result<crate::checkpoint::SessionTimeline, String> {
    log::info!(
        "Getting timeline for session: {} in project: {}",
        session_id,
        project_id
    );

    let manager = app
        .get_or_create_manager(session_id, project_id, PathBuf::from(&project_path))
        .await
        .map_err(|e| format!("Failed to get checkpoint manager: {}", e))?;

    Ok(manager.get_timeline().await)
}

/// Updates checkpoint settings for a session
#[tauri::command]
pub async fn update_checkpoint_settings(
    app: tauri::State<'_, crate::checkpoint::state::CheckpointState>,
    session_id: String,
    project_id: String,
    project_path: String,
    auto_checkpoint_enabled: bool,
    checkpoint_strategy: String,
) -> Result<(), String> {
    use crate::checkpoint::CheckpointStrategy;

    log::info!("Updating checkpoint settings for session: {}", session_id);

    let strategy = match checkpoint_strategy.as_str() {
        "manual" => CheckpointStrategy::Manual,
        "per_prompt" => CheckpointStrategy::PerPrompt,
        "per_tool_use" => CheckpointStrategy::PerToolUse,
        "smart" => CheckpointStrategy::Smart,
        _ => {
            return Err(format!(
                "Invalid checkpoint strategy: {}",
                checkpoint_strategy
            ))
        }
    };

    let manager = app
        .get_or_create_manager(session_id, project_id, PathBuf::from(&project_path))
        .await
        .map_err(|e| format!("Failed to get checkpoint manager: {}", e))?;

    manager
        .update_settings(auto_checkpoint_enabled, strategy)
        .await
        .map_err(|e| format!("Failed to update settings: {}", e))
}

/// Gets diff between two checkpoints
#[tauri::command]
pub async fn get_checkpoint_diff(
    from_checkpoint_id: String,
    to_checkpoint_id: String,
    session_id: String,
    project_id: String,
) -> Result<crate::checkpoint::CheckpointDiff, String> {
    use crate::checkpoint::storage::CheckpointStorage;

    log::info!(
        "Getting diff between checkpoints: {} -> {}",
        from_checkpoint_id,
        to_checkpoint_id
    );

    let claude_dir = get_claude_dir().map_err(|e| e.to_string())?;
    let storage = CheckpointStorage::new(claude_dir);

    // Load both checkpoints
    let (from_checkpoint, from_files, _) = storage
        .load_checkpoint(&project_id, &session_id, &from_checkpoint_id)
        .map_err(|e| format!("Failed to load source checkpoint: {}", e))?;
    let (to_checkpoint, to_files, _) = storage
        .load_checkpoint(&project_id, &session_id, &to_checkpoint_id)
        .map_err(|e| format!("Failed to load target checkpoint: {}", e))?;

    // Build file maps
    let mut from_map: std::collections::HashMap<PathBuf, &crate::checkpoint::FileSnapshot> =
        std::collections::HashMap::new();
    for file in &from_files {
        from_map.insert(file.file_path.clone(), file);
    }

    let mut to_map: std::collections::HashMap<PathBuf, &crate::checkpoint::FileSnapshot> =
        std::collections::HashMap::new();
    for file in &to_files {
        to_map.insert(file.file_path.clone(), file);
    }

    // Calculate differences
    let mut modified_files = Vec::new();
    let mut added_files = Vec::new();
    let mut deleted_files = Vec::new();

    // Check for modified and deleted files
    for (path, from_file) in &from_map {
        if let Some(to_file) = to_map.get(path) {
            if from_file.hash != to_file.hash {
                // File was modified
                let additions = to_file.content.lines().count();
                let deletions = from_file.content.lines().count();

                modified_files.push(crate::checkpoint::FileDiff {
                    path: path.clone(),
                    additions,
                    deletions,
                    diff_content: None, // TODO: Generate actual diff
                });
            }
        } else {
            // File was deleted
            deleted_files.push(path.clone());
        }
    }

    // Check for added files
    for (path, _) in &to_map {
        if !from_map.contains_key(path) {
            added_files.push(path.clone());
        }
    }

    // Calculate token delta
    let token_delta = (to_checkpoint.metadata.total_tokens as i64)
        - (from_checkpoint.metadata.total_tokens as i64);

    Ok(crate::checkpoint::CheckpointDiff {
        from_checkpoint_id,
        to_checkpoint_id,
        modified_files,
        added_files,
        deleted_files,
        token_delta,
    })
}

/// Tracks a message for checkpointing
#[tauri::command]
pub async fn track_checkpoint_message(
    app: tauri::State<'_, crate::checkpoint::state::CheckpointState>,
    session_id: String,
    project_id: String,
    project_path: String,
    message: String,
) -> Result<(), String> {
    log::info!("Tracking message for session: {}", session_id);

    let manager = app
        .get_or_create_manager(session_id, project_id, PathBuf::from(project_path))
        .await
        .map_err(|e| format!("Failed to get checkpoint manager: {}", e))?;

    manager
        .track_message(message)
        .await
        .map_err(|e| format!("Failed to track message: {}", e))
}

/// Checks if auto-checkpoint should be triggered
#[tauri::command]
pub async fn check_auto_checkpoint(
    app: tauri::State<'_, crate::checkpoint::state::CheckpointState>,
    session_id: String,
    project_id: String,
    project_path: String,
    message: String,
) -> Result<bool, String> {
    log::info!("Checking auto-checkpoint for session: {}", session_id);

    let manager = app
        .get_or_create_manager(session_id.clone(), project_id, PathBuf::from(project_path))
        .await
        .map_err(|e| format!("Failed to get checkpoint manager: {}", e))?;

    Ok(manager.should_auto_checkpoint(&message).await)
}

/// Triggers cleanup of old checkpoints
#[tauri::command]
pub async fn cleanup_old_checkpoints(
    app: tauri::State<'_, crate::checkpoint::state::CheckpointState>,
    session_id: String,
    project_id: String,
    project_path: String,
    keep_count: usize,
) -> Result<usize, String> {
    log::info!(
        "Cleaning up old checkpoints for session: {}, keeping {}",
        session_id,
        keep_count
    );

    let manager = app
        .get_or_create_manager(
            session_id.clone(),
            project_id.clone(),
            PathBuf::from(project_path),
        )
        .await
        .map_err(|e| format!("Failed to get checkpoint manager: {}", e))?;

    manager
        .storage
        .cleanup_old_checkpoints(&project_id, &session_id, keep_count)
        .map_err(|e| format!("Failed to cleanup checkpoints: {}", e))
}

/// Gets checkpoint settings for a session
#[tauri::command]
pub async fn get_checkpoint_settings(
    app: tauri::State<'_, crate::checkpoint::state::CheckpointState>,
    session_id: String,
    project_id: String,
    project_path: String,
) -> Result<serde_json::Value, String> {
    log::info!("Getting checkpoint settings for session: {}", session_id);

    let manager = app
        .get_or_create_manager(session_id, project_id, PathBuf::from(project_path))
        .await
        .map_err(|e| format!("Failed to get checkpoint manager: {}", e))?;

    let timeline = manager.get_timeline().await;

    Ok(serde_json::json!({
        "auto_checkpoint_enabled": timeline.auto_checkpoint_enabled,
        "checkpoint_strategy": timeline.checkpoint_strategy,
        "total_checkpoints": timeline.total_checkpoints,
        "current_checkpoint_id": timeline.current_checkpoint_id,
    }))
}

/// Clears checkpoint manager for a session (cleanup on session end)
#[tauri::command]
pub async fn clear_checkpoint_manager(
    app: tauri::State<'_, crate::checkpoint::state::CheckpointState>,
    session_id: String,
) -> Result<(), String> {
    log::info!("Clearing checkpoint manager for session: {}", session_id);

    app.remove_manager(&session_id).await;
    Ok(())
}

/// Gets checkpoint state statistics (for debugging/monitoring)
#[tauri::command]
pub async fn get_checkpoint_state_stats(
    app: tauri::State<'_, crate::checkpoint::state::CheckpointState>,
) -> Result<serde_json::Value, String> {
    let active_count = app.active_count().await;
    let active_sessions = app.list_active_sessions().await;

    Ok(serde_json::json!({
        "active_managers": active_count,
        "active_sessions": active_sessions,
    }))
}

/// Gets files modified in the last N minutes for a session
#[tauri::command]
pub async fn get_recently_modified_files(
    app: tauri::State<'_, crate::checkpoint::state::CheckpointState>,
    session_id: String,
    project_id: String,
    project_path: String,
    minutes: i64,
) -> Result<Vec<String>, String> {
    use chrono::{Duration, Utc};

    log::info!(
        "Getting files modified in the last {} minutes for session: {}",
        minutes,
        session_id
    );

    let manager = app
        .get_or_create_manager(session_id, project_id, PathBuf::from(project_path))
        .await
        .map_err(|e| format!("Failed to get checkpoint manager: {}", e))?;

    let since = Utc::now() - Duration::minutes(minutes);
    let modified_files = manager.get_files_modified_since(since).await;

    // Also log the last modification time
    if let Some(last_mod) = manager.get_last_modification_time().await {
        log::info!("Last file modification was at: {}", last_mod);
    }

    Ok(modified_files
        .into_iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect())
}

/// Track session messages from the frontend for checkpointing
#[tauri::command]
pub async fn track_session_messages(
    state: tauri::State<'_, crate::checkpoint::state::CheckpointState>,
    session_id: String,
    project_id: String,
    project_path: String,
    messages: Vec<String>,
) -> Result<(), String> {
    log::info!(
        "Tracking {} messages for session {}",
        messages.len(),
        session_id
    );

    let manager = state
        .get_or_create_manager(
            session_id.clone(),
            project_id.clone(),
            PathBuf::from(&project_path),
        )
        .await
        .map_err(|e| format!("Failed to get checkpoint manager: {}", e))?;

    for message in messages {
        manager
            .track_message(message)
            .await
            .map_err(|e| format!("Failed to track message: {}", e))?;
    }

    Ok(())
}

/// Gets hooks configuration from settings at specified scope
#[tauri::command]
pub async fn get_hooks_config(
    scope: String,
    project_path: Option<String>,
) -> Result<serde_json::Value, String> {
    log::info!(
        "Getting hooks config for scope: {}, project: {:?}",
        scope,
        project_path
    );

    let settings_path = match scope.as_str() {
        "user" => get_claude_dir()
            .map_err(|e| e.to_string())?
            .join("settings.json"),
        "project" => {
            let path = project_path.ok_or("Project path required for project scope")?;
            PathBuf::from(path).join(".claude").join("settings.json")
        }
        "local" => {
            let path = project_path.ok_or("Project path required for local scope")?;
            PathBuf::from(path)
                .join(".claude")
                .join("settings.local.json")
        }
        _ => return Err("Invalid scope".to_string()),
    };

    if !settings_path.exists() {
        log::info!(
            "Settings file does not exist at {:?}, returning empty hooks",
            settings_path
        );
        return Ok(serde_json::json!({}));
    }

    let content = fs::read_to_string(&settings_path)
        .map_err(|e| format!("Failed to read settings: {}", e))?;

    let settings: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse settings: {}", e))?;

    Ok(settings
        .get("hooks")
        .cloned()
        .unwrap_or(serde_json::json!({})))
}

/// Updates hooks configuration in settings at specified scope
#[tauri::command]
pub async fn update_hooks_config(
    scope: String,
    hooks: serde_json::Value,
    project_path: Option<String>,
) -> Result<String, String> {
    log::info!(
        "Updating hooks config for scope: {}, project: {:?}",
        scope,
        project_path
    );

    let settings_path = match scope.as_str() {
        "user" => get_claude_dir()
            .map_err(|e| e.to_string())?
            .join("settings.json"),
        "project" => {
            let path = project_path.ok_or("Project path required for project scope")?;
            let claude_dir = PathBuf::from(path).join(".claude");
            fs::create_dir_all(&claude_dir)
                .map_err(|e| format!("Failed to create .claude directory: {}", e))?;
            claude_dir.join("settings.json")
        }
        "local" => {
            let path = project_path.ok_or("Project path required for local scope")?;
            let claude_dir = PathBuf::from(path).join(".claude");
            fs::create_dir_all(&claude_dir)
                .map_err(|e| format!("Failed to create .claude directory: {}", e))?;
            claude_dir.join("settings.local.json")
        }
        _ => return Err("Invalid scope".to_string()),
    };

    // Read existing settings or create new
    let mut settings = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)
            .map_err(|e| format!("Failed to read settings: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse settings: {}", e))?
    } else {
        serde_json::json!({})
    };

    // Update hooks section
    settings["hooks"] = hooks;

    // Write back with pretty formatting
    let json_string = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    fs::write(&settings_path, json_string)
        .map_err(|e| format!("Failed to write settings: {}", e))?;

    Ok("Hooks configuration updated successfully".to_string())
}

/// Validates a hook command by dry-running it
#[tauri::command]
pub async fn validate_hook_command(command: String) -> Result<serde_json::Value, String> {
    log::info!("Validating hook command syntax");

    // Validate syntax without executing
    let mut cmd = std::process::Command::new("bash");
    cmd.arg("-n") // Syntax check only
        .arg("-c")
        .arg(&command);

    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                Ok(serde_json::json!({
                    "valid": true,
                    "message": "Command syntax is valid"
                }))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Ok(serde_json::json!({
                    "valid": false,
                    "message": format!("Syntax error: {}", stderr)
                }))
            }
        }
        Err(e) => Err(format!("Failed to validate command: {}", e)),
    }
}

// ============================================================================
// ClaudeClient-based implementation using cc-sdk
// ============================================================================

/// Helper function to create a ClaudeClient with the appropriate configuration
async fn create_claude_client(
    app: &AppHandle,
    model: String,
    project_path: String,
    continue_conversation: bool,
    resume_session_id: Option<String>,
) -> Result<Arc<cc_sdk::ClaudeClient<cc_sdk::core::state::Connected>>, String> {
    use cc_sdk::core::ModelId;
    use cc_sdk::permissions::PermissionMode;

    log::info!(
        "Creating ClaudeClient: model={}, project={}, continue={}, resume={:?}",
        model,
        project_path,
        continue_conversation,
        resume_session_id
    );

    // Find claude binary
    let claude_path = find_claude_binary(app)?;

    // Build client
    let mut builder = cc_sdk::ClaudeClient::builder()
        .binary(cc_sdk::core::BinaryPath::new(claude_path))
        .model(ModelId::from(model.as_str()))
        .permission_mode(PermissionMode::BypassPermissions)
        .working_directory(&project_path);

    // Handle continue/resume
    if let Some(session_id) = resume_session_id {
        builder = builder.resume_session(cc_sdk::core::SessionId::from(session_id));
    } else if continue_conversation {
        builder = builder.continue_conversation(true);
    }

    let client = builder
        .configure()
        .connect()
        .await
        .map_err(|e| format!("Failed to connect client: {}", e))?
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    log::info!("ClaudeClient created successfully, session_id: {}", client.session_id());

    Ok(Arc::new(client))
}

/// Helper function to handle streaming from ClaudeClient
async fn handle_claude_stream(
    app: AppHandle,
    client: Arc<cc_sdk::ClaudeClient<cc_sdk::core::state::Connected>>,
    prompt: String,
    model: String,
    project_path: String,
) -> Result<(), String> {
    use futures::StreamExt;

    let session_id = client.session_id().to_string();
    let client_session_id = session_id.clone();
    let app_clone = app.clone();

    log::info!("Starting stream handler for session: {}", session_id);

    // Send prompt
    let mut stream = client
        .send(&prompt)
        .await
        .map_err(|e| format!("Failed to send prompt: {}", e))?;

    // Spawn task to handle streaming
    tokio::spawn(async move {
        log::info!("Stream processing started for session: {}", client_session_id);

        while let Some(message_result) = stream.next().await {
            match message_result {
                Ok(message) => {
                    // Convert message to JSON string
                    let json_str = match serde_json::to_string(&message) {
                        Ok(s) => s,
                        Err(e) => {
                            log::error!("Failed to serialize message: {}", e);
                            continue;
                        }
                    };

                    log::debug!("Claude output ({}): {}", client_session_id, json_str);

                    // Emit to frontend with session isolation
                    let _ = app_clone.emit(&format!("claude-output:{}", client_session_id), &json_str);
                    // Also emit generic for backward compatibility
                    let _ = app_clone.emit("claude-output", &json_str);

                    // Check if this is a result message (conversation complete)
                    if let cc_sdk::messages::Message::Result { .. } = message {
                        log::info!("Received Result message, session complete: {}", client_session_id);

                        // Small delay to ensure all messages are processed
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                        let _ = app_clone.emit(&format!("claude-complete:{}", client_session_id), true);
                        let _ = app_clone.emit("claude-complete", true);
                        break;
                    }
                }
                Err(e) => {
                    log::error!("Stream error for session {}: {}", client_session_id, e);
                    let error_msg = format!("Error: {}", e);
                    let _ = app_clone.emit(&format!("claude-error:{}", client_session_id), &error_msg);
                    let _ = app_clone.emit("claude-error", &error_msg);

                    // Emit complete with error status
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    let _ = app_clone.emit(&format!("claude-complete:{}", client_session_id), false);
                    let _ = app_clone.emit("claude-complete", false);
                    break;
                }
            }
        }

        log::info!("Stream processing completed for session: {}", client_session_id);

        // Note: Client will be disconnected when removed from registry or when the Arc is dropped
    });

    Ok(())
}

/// Execute a new Claude Code session using ClaudeClient
#[tauri::command]
pub async fn execute_claude_code(
    app: AppHandle,
    project_path: String,
    prompt: String,
    model: String,
) -> Result<String, String> {
    log::info!(
        "execute_claude_code: project={}, model={}",
        project_path,
        model
    );

    let client = create_claude_client(&app, model.clone(), project_path.clone(), false, None).await?;
    let session_id = client.session_id().to_string();

    // Store client in registry
    use crate::claude_client_registry::ClaudeClientRegistryState;
    let registry = app.state::<ClaudeClientRegistryState>();
    registry.0.register_client(session_id.clone(), client.clone()).await;

    // Handle streaming
    handle_claude_stream(app, client, prompt, model, project_path).await?;

    Ok(session_id)
}

/// Continue an existing Claude Code conversation using ClaudeClient
#[tauri::command]
pub async fn continue_claude_code(
    app: AppHandle,
    project_path: String,
    prompt: String,
    model: String,
) -> Result<String, String> {
    log::info!(
        "continue_claude_code: project={}, model={}",
        project_path,
        model
    );

    let client = create_claude_client(&app, model.clone(), project_path.clone(), true, None).await?;
    let session_id = client.session_id().to_string();

    // Store client in registry
    use crate::claude_client_registry::ClaudeClientRegistryState;
    let registry = app.state::<ClaudeClientRegistryState>();
    registry.0.register_client(session_id.clone(), client.clone()).await;

    // Handle streaming
    handle_claude_stream(app, client, prompt, model, project_path).await?;

    Ok(session_id)
}

/// Resume an existing Claude Code session by ID using ClaudeClient
#[tauri::command]
pub async fn resume_claude_code(
    app: AppHandle,
    project_path: String,
    session_id: String,
    prompt: String,
    model: String,
) -> Result<String, String> {
    log::info!(
        "resume_claude_code: session={}, project={}, model={}",
        session_id,
        project_path,
        model
    );

    let client = create_claude_client(
        &app,
        model.clone(),
        project_path.clone(),
        false,
        Some(session_id.clone()),
    )
    .await?;

    // Store client in registry
    use crate::claude_client_registry::ClaudeClientRegistryState;
    let registry = app.state::<ClaudeClientRegistryState>();
    registry.0.register_client(session_id.clone(), client.clone()).await;

    // Handle streaming
    handle_claude_stream(app, client, prompt, model, project_path).await?;

    Ok(session_id)
}

/// Cancel a Claude Code execution using ClaudeClient
#[tauri::command]
pub async fn cancel_claude_execution(
    app: AppHandle,
    session_id: String,
) -> Result<(), String> {
    log::info!("cancel_claude_execution: session={}", session_id);

    // Get client from registry
    use crate::claude_client_registry::ClaudeClientRegistryState;
    let registry = app.state::<ClaudeClientRegistryState>();

    if let Some(_client) = registry.0.get_client(&session_id).await {
        log::info!("Found client for session {}, removing from registry...", session_id);

        // Remove from registry - the client will be dropped and disconnected
        registry.0.remove_client(&session_id).await;

        // Emit cancellation events
        let _ = app.emit(&format!("claude-cancelled:{}", session_id), true);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let _ = app.emit(&format!("claude-complete:{}", session_id), false);

        // Also emit generic events
        let _ = app.emit("claude-cancelled", true);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let _ = app.emit("claude-complete", false);

        log::info!("Client disconnected successfully for session: {}", session_id);
    } else {
        log::warn!("No active client found for session: {}", session_id);

        // Still emit cancellation events for UI consistency
        let _ = app.emit(&format!("claude-cancelled:{}", session_id), true);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let _ = app.emit(&format!("claude-complete:{}", session_id), false);
        let _ = app.emit("claude-cancelled", true);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let _ = app.emit("claude-complete", false);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    /// Helper function to create a test session file
    fn create_test_session_file(
        dir: &PathBuf,
        filename: &str,
        content: &str,
    ) -> Result<(), std::io::Error> {
        let file_path = dir.join(filename);
        let mut file = fs::File::create(file_path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    #[test]
    fn test_get_project_path_from_sessions_normal_case() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        // Create a session file with cwd on the first line
        let content = r#"{"type":"system","cwd":"/Users/test/my-project"}"#;
        create_test_session_file(&project_dir, "session1.jsonl", content).unwrap();

        let result = get_project_path_from_sessions(&project_dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/Users/test/my-project");
    }

    #[test]
    fn test_get_project_path_from_sessions_with_hyphen() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        // This is the bug scenario - project path contains hyphens
        let content = r#"{"type":"system","cwd":"/Users/test/data-discovery"}"#;
        create_test_session_file(&project_dir, "session1.jsonl", content).unwrap();

        let result = get_project_path_from_sessions(&project_dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/Users/test/data-discovery");
    }

    #[test]
    fn test_get_project_path_from_sessions_null_cwd_first_line() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        // First line has null cwd, second line has valid path
        let content = format!(
            "{}\n{}",
            r#"{"type":"system","cwd":null}"#,
            r#"{"type":"system","cwd":"/Users/test/valid-path"}"#
        );
        create_test_session_file(&project_dir, "session1.jsonl", &content).unwrap();

        let result = get_project_path_from_sessions(&project_dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/Users/test/valid-path");
    }

    #[test]
    fn test_get_project_path_from_sessions_multiple_lines() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        // Multiple lines with cwd appearing on line 5
        let content = format!(
            "{}\n{}\n{}\n{}\n{}",
            r#"{"type":"other"}"#,
            r#"{"type":"system","cwd":null}"#,
            r#"{"type":"message"}"#,
            r#"{"type":"system"}"#,
            r#"{"type":"system","cwd":"/Users/test/project"}"#
        );
        create_test_session_file(&project_dir, "session1.jsonl", &content).unwrap();

        let result = get_project_path_from_sessions(&project_dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/Users/test/project");
    }

    #[test]
    fn test_get_project_path_from_sessions_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        let result = get_project_path_from_sessions(&project_dir);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Could not determine project path from session files"
        );
    }

    #[test]
    fn test_get_project_path_from_sessions_no_jsonl_files() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        // Create a non-JSONL file
        create_test_session_file(&project_dir, "readme.txt", "Some text").unwrap();

        let result = get_project_path_from_sessions(&project_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_project_path_from_sessions_no_cwd() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        // JSONL file without any cwd field
        let content = format!(
            "{}\n{}\n{}",
            r#"{"type":"system"}"#, r#"{"type":"message"}"#, r#"{"type":"other"}"#
        );
        create_test_session_file(&project_dir, "session1.jsonl", &content).unwrap();

        let result = get_project_path_from_sessions(&project_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_project_path_from_sessions_multiple_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        // Create multiple session files - should return from first valid one
        create_test_session_file(
            &project_dir,
            "session1.jsonl",
            r#"{"type":"system","cwd":"/path1"}"#,
        )
        .unwrap();
        create_test_session_file(
            &project_dir,
            "session2.jsonl",
            r#"{"type":"system","cwd":"/path2"}"#,
        )
        .unwrap();

        let result = get_project_path_from_sessions(&project_dir);
        assert!(result.is_ok());
        // Should get one of the paths (implementation checks first file it finds)
        let path = result.unwrap();
        assert!(path == "/path1" || path == "/path2");
    }
}
