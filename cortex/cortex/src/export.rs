//! Export functionality for various data formats.
//!
//! Supports exporting Cortex data to multiple formats:
//! - JSON
//! - CSV
//! - YAML
//! - Markdown

use anyhow::{Context, Result};
use cortex_storage::ConnectionManager;
use cortex_vfs::VirtualFileSystem;
use serde::Serialize;
use serde_json::json;
use std::path::Path;
use std::sync::Arc;

/// Export format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Csv,
    Yaml,
    Markdown,
}

impl ExportFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "csv" => Some(Self::Csv),
            "yaml" | "yml" => Some(Self::Yaml),
            "md" | "markdown" => Some(Self::Markdown),
            _ => None,
        }
    }

    pub fn extension(&self) -> &str {
        match self {
            Self::Json => "json",
            Self::Csv => "csv",
            Self::Yaml => "yaml",
            Self::Markdown => "md",
        }
    }
}

/// Export data to a file
pub fn export_to_file<T: Serialize>(
    data: &T,
    path: &Path,
    format: ExportFormat,
) -> Result<()> {
    let content = match format {
        ExportFormat::Json => export_json(data)?,
        ExportFormat::Csv => export_csv(data)?,
        ExportFormat::Yaml => export_yaml(data)?,
        ExportFormat::Markdown => export_markdown(data)?,
    };

    std::fs::write(path, content).context("Failed to write export file")?;

    Ok(())
}

/// Export to JSON format
pub fn export_json<T: Serialize>(data: &T) -> Result<String> {
    serde_json::to_string_pretty(data).context("Failed to serialize to JSON")
}

/// Export to CSV format
pub fn export_csv<T: Serialize>(data: &T) -> Result<String> {
    // For CSV export, we need the data to be a sequence
    // This is a simplified version - real implementation would handle complex structures
    let json = serde_json::to_value(data)?;

    if let serde_json::Value::Array(items) = json {
        if items.is_empty() {
            return Ok(String::new());
        }

        let mut csv = String::new();

        // Extract headers from first item
        if let Some(first) = items.first() {
            if let serde_json::Value::Object(map) = first {
                let headers: Vec<String> = map.keys().cloned().collect();
                csv.push_str(&headers.join(","));
                csv.push('\n');

                // Write rows
                for item in &items {
                    if let serde_json::Value::Object(map) = item {
                        let row: Vec<String> = headers
                            .iter()
                            .map(|key| {
                                map.get(key)
                                    .and_then(|v| match v {
                                        serde_json::Value::String(s) => Some(escape_csv(s)),
                                        serde_json::Value::Number(n) => Some(n.to_string()),
                                        serde_json::Value::Bool(b) => Some(b.to_string()),
                                        serde_json::Value::Null => Some(String::new()),
                                        _ => Some(format!("{}", v)),
                                    })
                                    .unwrap_or_default()
                            })
                            .collect();
                        csv.push_str(&row.join(","));
                        csv.push('\n');
                    }
                }
            }
        }

        Ok(csv)
    } else {
        anyhow::bail!("CSV export requires an array of objects")
    }
}

/// Escape CSV field
fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Export to YAML format
pub fn export_yaml<T: Serialize>(data: &T) -> Result<String> {
    serde_yaml::to_string(data).context("Failed to serialize to YAML")
}

/// Export to Markdown format
pub fn export_markdown<T: Serialize>(data: &T) -> Result<String> {
    let json = serde_json::to_value(data)?;

    let mut markdown = String::new();
    markdown.push_str("# Cortex Export\n\n");

    match json {
        serde_json::Value::Array(items) => {
            if items.is_empty() {
                markdown.push_str("*No items to export*\n");
                return Ok(markdown);
            }

            // Create a table if items are objects
            if let Some(first) = items.first() {
                if let serde_json::Value::Object(map) = first {
                    let headers: Vec<String> = map.keys().cloned().collect();

                    // Table header
                    markdown.push_str("| ");
                    markdown.push_str(&headers.join(" | "));
                    markdown.push_str(" |\n");

                    // Separator
                    markdown.push_str("| ");
                    markdown.push_str(&vec!["---"; headers.len()].join(" | "));
                    markdown.push_str(" |\n");

                    // Rows
                    for item in &items {
                        if let serde_json::Value::Object(map) = item {
                            markdown.push_str("| ");
                            let row: Vec<String> = headers
                                .iter()
                                .map(|key| {
                                    map.get(key)
                                        .and_then(|v| match v {
                                            serde_json::Value::String(s) => {
                                                Some(s.replace('|', "\\|"))
                                            }
                                            serde_json::Value::Number(n) => Some(n.to_string()),
                                            serde_json::Value::Bool(b) => Some(b.to_string()),
                                            serde_json::Value::Null => Some(String::from("*null*")),
                                            _ => Some(format!("{}", v)),
                                        })
                                        .unwrap_or_default()
                                })
                                .collect();
                            markdown.push_str(&row.join(" | "));
                            markdown.push_str(" |\n");
                        }
                    }
                }
            }
        }
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                markdown.push_str(&format!("## {}\n\n", key));
                markdown.push_str(&format!("```\n{}\n```\n\n", value));
            }
        }
        _ => {
            markdown.push_str(&format!("```\n{}\n```\n", json));
        }
    }

    Ok(markdown)
}

/// Export workspace data
pub async fn export_workspace(
    storage: Arc<ConnectionManager>,
    workspace_name: &str,
    output_path: &Path,
    format: ExportFormat,
) -> Result<()> {
    use crate::output;

    output::info(format!("Exporting workspace '{}'...", workspace_name));

    // Get workspace from storage
    let conn = storage.acquire().await
        .context("Failed to acquire database connection")?;

    // Query workspace by name
    let query = "SELECT * FROM workspace WHERE name = $name LIMIT 1";
    let workspace_name_owned = workspace_name.to_string();
    let mut result = conn.connection()
        .query(query)
        .bind(("name", workspace_name_owned))
        .await
        .context("Failed to query workspace")?;

    let workspace: Option<cortex_vfs::Workspace> = result.take(0)
        .context("Failed to deserialize workspace")?;

    let workspace = workspace
        .ok_or_else(|| anyhow::anyhow!("Workspace '{}' not found", workspace_name))?;

    // Count files in workspace
    let file_count_query = format!("SELECT count() AS count FROM file WHERE workspace_id = '{}'", workspace.id);
    let mut file_result = conn.connection()
        .query(&file_count_query)
        .await
        .context("Failed to count files")?;

    let file_count: Option<i64> = file_result.take("count")
        .unwrap_or(Some(0));

    // Get file list from workspace
    let files_query = format!("SELECT * FROM file WHERE workspace_id = '{}'", workspace.id);
    let mut files_result = conn.connection()
        .query(&files_query)
        .await
        .context("Failed to query files")?;

    let files: Vec<serde_json::Value> = files_result.take(0)
        .unwrap_or_default();

    // Collect file information
    let file_items: Vec<_> = files.iter().map(|file| {
        json!({
            "path": file.get("path").and_then(|p| p.as_str()).unwrap_or(""),
            "size": file.get("size").and_then(|s| s.as_i64()).unwrap_or(0),
            "modified_at": file.get("updated_at").and_then(|u| u.as_str()).unwrap_or(""),
        })
    }).collect();

    let data = json!({
        "workspace": {
            "id": workspace.id.to_string(),
            "name": workspace.name,
            "type": format!("{:?}", workspace.workspace_type),
            "source": format!("{:?}", workspace.source_type),
            "created_at": workspace.created_at.to_rfc3339(),
            "updated_at": workspace.updated_at.to_rfc3339(),
        },
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "files_count": file_count.unwrap_or(0),
        "items": file_items,
    });

    export_to_file(&data, output_path, format)?;

    output::success(format!(
        "Exported workspace '{}' ({} files) to {}",
        workspace_name,
        files.len(),
        output_path.display()
    ));

    Ok(())
}

/// Export memory episodes
pub async fn export_episodes(
    storage: Arc<ConnectionManager>,
    workspace_name: Option<String>,
    output_path: &Path,
    format: ExportFormat,
    limit: Option<usize>,
) -> Result<()> {
    use crate::output;

    output::info("Exporting memory episodes...");

    let conn = storage.acquire().await
        .context("Failed to acquire database connection")?;

    // Build query with optional workspace filter
    let query = if let Some(ref workspace) = workspace_name {
        format!(
            "SELECT * FROM episode WHERE workspace = '{}' ORDER BY created_at DESC {}",
            workspace,
            limit.map(|l| format!("LIMIT {}", l)).unwrap_or_default()
        )
    } else {
        format!(
            "SELECT * FROM episode ORDER BY created_at DESC {}",
            limit.map(|l| format!("LIMIT {}", l)).unwrap_or_default()
        )
    };

    let mut result = conn.connection()
        .query(&query)
        .await
        .context("Failed to query episodes")?;

    let episodes: Vec<serde_json::Value> = result.take(0)
        .context("Failed to deserialize episodes")?;

    export_to_file(&episodes, output_path, format)?;

    output::success(format!(
        "Exported {} episode(s) to {}",
        episodes.len(),
        output_path.display()
    ));

    Ok(())
}

/// Export search results
pub async fn export_search_results(
    query: &str,
    results: &serde_json::Value,
    output_path: &Path,
    format: ExportFormat,
) -> Result<()> {
    use crate::output;

    let data = serde_json::json!({
        "query": query,
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "results": results,
    });

    export_to_file(&data, output_path, format)?;

    output::success(format!("Exported search results to {}", output_path.display()));

    Ok(())
}

/// Export statistics
pub async fn export_stats(
    storage: Arc<ConnectionManager>,
    output_path: &Path,
    format: ExportFormat,
) -> Result<()> {
    use crate::output;

    output::info("Exporting system statistics...");

    let conn = storage.acquire().await
        .context("Failed to acquire database connection")?;

    // Get workspace count
    let mut result = conn.connection()
        .query("SELECT count() AS count FROM workspace GROUP ALL")
        .await
        .context("Failed to count workspaces")?;
    let workspace_count: Option<i64> = result.take("count")
        .unwrap_or(Some(0));

    // Get file count and total size
    let mut result = conn.connection()
        .query("SELECT count() AS count, math::sum(size) AS total_size FROM file GROUP ALL")
        .await
        .context("Failed to count files")?;
    let file_count: Option<i64> = result.take("count")
        .unwrap_or(Some(0));
    let total_size: Option<i64> = result.take("total_size")
        .unwrap_or(Some(0));

    // Get episode count
    let mut result = conn.connection()
        .query("SELECT count() AS count FROM episode GROUP ALL")
        .await
        .context("Failed to count episodes")?;
    let episode_count: Option<i64> = result.take("count")
        .unwrap_or(Some(0));

    // Get code unit count
    let mut result = conn.connection()
        .query("SELECT count() AS count FROM code_unit GROUP ALL")
        .await
        .context("Failed to count code units")?;
    let code_unit_count: Option<i64> = result.take("count")
        .unwrap_or(Some(0));

    let stats = json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "workspaces": workspace_count.unwrap_or(0),
        "files": file_count.unwrap_or(0),
        "total_size_bytes": total_size.unwrap_or(0),
        "memory": {
            "episodes": episode_count.unwrap_or(0),
            "semantic_nodes": code_unit_count.unwrap_or(0),
        },
    });

    export_to_file(&stats, output_path, format)?;

    output::success(format!("Exported statistics to {}", output_path.display()));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_export_format_from_extension() {
        assert_eq!(ExportFormat::from_extension("json"), Some(ExportFormat::Json));
        assert_eq!(ExportFormat::from_extension("csv"), Some(ExportFormat::Csv));
        assert_eq!(ExportFormat::from_extension("yaml"), Some(ExportFormat::Yaml));
        assert_eq!(ExportFormat::from_extension("yml"), Some(ExportFormat::Yaml));
        assert_eq!(ExportFormat::from_extension("md"), Some(ExportFormat::Markdown));
        assert_eq!(ExportFormat::from_extension("unknown"), None);
    }

    #[test]
    fn test_export_json() {
        let data = json!({
            "name": "test",
            "value": 42
        });

        let result = export_json(&data).unwrap();
        assert!(result.contains("\"name\""));
        assert!(result.contains("\"test\""));
        assert!(result.contains("42"));
    }

    #[test]
    fn test_export_csv() {
        let data = json!([
            {"name": "Alice", "age": 30},
            {"name": "Bob", "age": 25}
        ]);

        let result = export_csv(&data).unwrap();
        assert!(result.contains("name,age") || result.contains("age,name"));
        assert!(result.contains("Alice"));
        assert!(result.contains("Bob"));
    }

    #[test]
    fn test_escape_csv() {
        assert_eq!(escape_csv("simple"), "simple");
        assert_eq!(escape_csv("with,comma"), "\"with,comma\"");
        assert_eq!(escape_csv("with\"quote"), "\"with\"\"quote\"");
    }

    #[test]
    fn test_export_yaml() {
        let data = json!({
            "name": "test",
            "value": 42
        });

        let result = export_yaml(&data).unwrap();
        assert!(result.contains("name:"));
        assert!(result.contains("test"));
    }

    #[test]
    fn test_export_markdown() {
        let data = json!([
            {"name": "Alice", "age": 30},
            {"name": "Bob", "age": 25}
        ]);

        let result = export_markdown(&data).unwrap();
        assert!(result.contains("# Cortex Export"));
        assert!(result.contains("|"));
        assert!(result.contains("Alice"));
        assert!(result.contains("Bob"));
    }

    #[test]
    fn test_export_csv_empty() {
        let data = json!([]);
        let result = export_csv(&data).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_export_csv_non_array() {
        let data = json!({"name": "test"});
        let result = export_csv(&data);
        assert!(result.is_err());
    }
}
