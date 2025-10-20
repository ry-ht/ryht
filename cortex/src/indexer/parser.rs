use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Project type detected in monorepo
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectType {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Mixed,
}

/// A project within a monorepo
#[derive(Debug, Clone)]
pub struct Project {
    pub name: String,
    pub path: PathBuf,
    pub project_type: ProjectType,
    pub marker_files: Vec<PathBuf>,
    pub dependencies: Vec<String>,
}

/// Monorepo parser for detecting and managing multiple projects
pub struct MonorepoParser {
    /// Project markers to detect
    markers: HashMap<&'static str, ProjectType>,
}

impl MonorepoParser {
    pub fn new() -> Self {
        let mut markers = HashMap::new();
        markers.insert("Cargo.toml", ProjectType::Rust);
        markers.insert("package.json", ProjectType::TypeScript);
        markers.insert("tsconfig.json", ProjectType::TypeScript);
        markers.insert("setup.py", ProjectType::Python);
        markers.insert("pyproject.toml", ProjectType::Python);
        markers.insert("go.mod", ProjectType::Go);

        Self { markers }
    }

    /// Detect all projects in a directory
    pub async fn detect_projects(&self, root: &Path) -> Result<Vec<Project>> {
        let mut projects = Vec::new();
        let mut visited = std::collections::HashSet::new();

        self.scan_directory(root, &mut projects, &mut visited)
            .await?;

        // Remove duplicate projects (keep the one with most marker files)
        self.deduplicate_projects(&mut projects);

        Ok(projects)
    }

    /// Recursively scan directory for project markers
    fn scan_directory<'a>(
        &'a self,
        dir: &'a Path,
        projects: &'a mut Vec<Project>,
        visited: &'a mut std::collections::HashSet<PathBuf>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
        // Avoid infinite loops
        let canonical = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
        if visited.contains(&canonical) {
            return Ok(());
        }
        visited.insert(canonical.clone());

        // Check for project markers in this directory
        let mut marker_files = Vec::new();
        let mut project_types = Vec::new();

        let mut entries = fs::read_dir(dir).await?;
        let mut subdirs = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Check if this is a project marker
            if let Some(project_type) = self.markers.get(file_name) {
                marker_files.push(path.clone());
                project_types.push(project_type.clone());
            }

            // Collect subdirectories (but skip common ignore patterns)
            if path.is_dir() {
                let should_skip = matches!(
                    file_name,
                    "node_modules"
                        | "target"
                        | "dist"
                        | "build"
                        | ".git"
                        | "__pycache__"
                        | "vendor"
                );

                if !should_skip {
                    subdirs.push(path);
                }
            }
        }

        // If we found markers, this is a project
        if !marker_files.is_empty() {
            let project_type = Self::determine_project_type(&project_types);
            let project_name = Self::extract_project_name(dir, &marker_files).await?;
            let dependencies = Self::extract_dependencies(dir, &marker_files).await?;

            projects.push(Project {
                name: project_name,
                path: dir.to_path_buf(),
                project_type,
                marker_files,
                dependencies,
            });

            // Don't scan subdirectories if we found a project
            // (to avoid detecting nested projects as separate)
            return Ok(());
        }

        // Scan subdirectories
        for subdir in subdirs {
            self.scan_directory(&subdir, projects, visited).await?;
        }

        Ok(())
        })
    }

    /// Determine the primary project type
    fn determine_project_type(types: &[ProjectType]) -> ProjectType {
        if types.is_empty() {
            return ProjectType::Mixed;
        }

        if types.len() == 1 {
            return types[0].clone();
        }

        // If multiple types, check for patterns
        let has_rust = types.contains(&ProjectType::Rust);
        let has_ts = types.contains(&ProjectType::TypeScript);

        if has_rust && has_ts {
            ProjectType::Mixed
        } else if has_rust {
            ProjectType::Rust
        } else if has_ts {
            ProjectType::TypeScript
        } else {
            types[0].clone()
        }
    }

    /// Extract project name from marker files
    async fn extract_project_name(
        dir: &Path,
        marker_files: &[PathBuf],
    ) -> Result<String> {
        // Try to extract name from package.json
        for marker in marker_files {
            if marker.file_name().and_then(|n| n.to_str()) == Some("package.json") {
                if let Ok(content) = fs::read_to_string(marker).await {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(name) = json.get("name").and_then(|n| n.as_str()) {
                            return Ok(name.to_string());
                        }
                    }
                }
            }
        }

        // Try to extract name from Cargo.toml
        for marker in marker_files {
            if marker.file_name().and_then(|n| n.to_str()) == Some("Cargo.toml") {
                if let Ok(content) = fs::read_to_string(marker).await {
                    if let Ok(toml) = toml::from_str::<toml::Value>(&content) {
                        if let Some(name) = toml
                            .get("package")
                            .and_then(|p| p.get("name"))
                            .and_then(|n| n.as_str())
                        {
                            return Ok(name.to_string());
                        }
                    }
                }
            }
        }

        // Fall back to directory name
        Ok(dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string())
    }

    /// Extract dependencies from project files
    async fn extract_dependencies(
        _dir: &Path,
        marker_files: &[PathBuf],
    ) -> Result<Vec<String>> {
        let mut dependencies = Vec::new();

        // Extract from package.json
        for marker in marker_files {
            if marker.file_name().and_then(|n| n.to_str()) == Some("package.json") {
                if let Ok(content) = fs::read_to_string(marker).await {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
                            for (name, _) in deps {
                                dependencies.push(name.clone());
                            }
                        }
                    }
                }
            }
        }

        // Extract from Cargo.toml
        for marker in marker_files {
            if marker.file_name().and_then(|n| n.to_str()) == Some("Cargo.toml") {
                if let Ok(content) = fs::read_to_string(marker).await {
                    if let Ok(toml) = toml::from_str::<toml::Value>(&content) {
                        if let Some(deps) = toml.get("dependencies").and_then(|d| d.as_table()) {
                            for (name, _) in deps {
                                dependencies.push(name.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(dependencies)
    }

    /// Remove duplicate projects
    fn deduplicate_projects(&self, projects: &mut Vec<Project>) {
        let mut seen: HashMap<PathBuf, usize> = HashMap::new();

        let mut to_remove = Vec::new();
        for (idx, project) in projects.iter().enumerate() {
            if let Some(&existing_idx) = seen.get(&project.path) {
                // Keep the one with more marker files
                if project.marker_files.len() > projects[existing_idx].marker_files.len() {
                    to_remove.push(existing_idx);
                    seen.insert(project.path.clone(), idx);
                } else {
                    to_remove.push(idx);
                }
            } else {
                seen.insert(project.path.clone(), idx);
            }
        }

        // Remove duplicates (in reverse order to maintain indices)
        to_remove.sort_unstable();
        to_remove.reverse();
        for idx in to_remove {
            projects.remove(idx);
        }
    }

    /// Build cross-project dependency graph
    pub fn build_dependency_graph(
        &self,
        projects: &[Project],
    ) -> HashMap<String, Vec<String>> {
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        // Create a set of project names for fast lookup
        let project_names: std::collections::HashSet<String> =
            projects.iter().map(|p| p.name.clone()).collect();

        // Build graph
        for project in projects {
            let mut project_deps = Vec::new();

            for dep in &project.dependencies {
                // Check if this dependency is another project in the monorepo
                if project_names.contains(dep) {
                    project_deps.push(dep.clone());
                }
            }

            graph.insert(project.name.clone(), project_deps);
        }

        graph
    }
}

impl Default for MonorepoParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_detect_single_rust_project() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("test-project");
        fs::create_dir(&project_dir).await.unwrap();

        // Create Cargo.toml
        fs::write(
            project_dir.join("Cargo.toml"),
            r#"
            [package]
            name = "test-project"
            version = "0.1.0"
            "#,
        )
        .await
        .unwrap();

        let parser = MonorepoParser::new();
        let projects = parser.detect_projects(temp_dir.path()).await.unwrap();

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "test-project");
        assert_eq!(projects[0].project_type, ProjectType::Rust);
    }

    #[tokio::test]
    async fn test_detect_monorepo() {
        let temp_dir = TempDir::new().unwrap();

        // Create project1 (Rust)
        let project1 = temp_dir.path().join("packages/project1");
        fs::create_dir_all(&project1).await.unwrap();
        fs::write(
            project1.join("Cargo.toml"),
            r#"
            [package]
            name = "project1"
            version = "0.1.0"
            "#,
        )
        .await
        .unwrap();

        // Create project2 (TypeScript)
        let project2 = temp_dir.path().join("packages/project2");
        fs::create_dir_all(&project2).await.unwrap();
        fs::write(
            project2.join("package.json"),
            r#"
            {
                "name": "project2",
                "version": "1.0.0"
            }
            "#,
        )
        .await
        .unwrap();

        let parser = MonorepoParser::new();
        let projects = parser.detect_projects(temp_dir.path()).await.unwrap();

        assert_eq!(projects.len(), 2);
        assert!(projects.iter().any(|p| p.name == "project1"));
        assert!(projects.iter().any(|p| p.name == "project2"));
    }

    #[tokio::test]
    async fn test_dependency_graph() {
        let temp_dir = TempDir::new().unwrap();

        // Create project1 that depends on project2
        let project1 = temp_dir.path().join("project1");
        fs::create_dir(&project1).await.unwrap();
        fs::write(
            project1.join("package.json"),
            r#"
            {
                "name": "project1",
                "dependencies": {
                    "project2": "1.0.0"
                }
            }
            "#,
        )
        .await
        .unwrap();

        // Create project2
        let project2 = temp_dir.path().join("project2");
        fs::create_dir(&project2).await.unwrap();
        fs::write(
            project2.join("package.json"),
            r#"
            {
                "name": "project2"
            }
            "#,
        )
        .await
        .unwrap();

        let parser = MonorepoParser::new();
        let projects = parser.detect_projects(temp_dir.path()).await.unwrap();
        let graph = parser.build_dependency_graph(&projects);

        assert_eq!(graph.get("project1").unwrap(), &vec!["project2"]);
    }
}
