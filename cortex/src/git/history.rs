use anyhow::{Context, Result, anyhow};
use git2::{Repository, DiffOptions, BlameOptions};
use serde::{Deserialize, Serialize};
use std::path::Path;
use chrono::{DateTime, Utc};

/// Information about a single commit affecting a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub sha: String,
    pub author: String,
    pub author_email: String,
    pub date: DateTime<Utc>,
    pub message: String,
    pub changes: String,
    pub insertions: usize,
    pub deletions: usize,
}

/// Information about a line from git blame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameInfo {
    pub line: usize,
    pub author: String,
    pub author_email: String,
    pub sha: String,
    pub date: DateTime<Utc>,
    pub content: String,
}

/// Git history wrapper using git2
pub struct GitHistory {
    repo: Repository,
}

impl GitHistory {
    /// Create a new GitHistory instance for a repository
    ///
    /// # Arguments
    /// * `repo_path` - Path to the git repository (can be any path inside the repo)
    pub fn new(repo_path: &Path) -> Result<Self> {
        let repo = Repository::discover(repo_path)
            .context("Failed to discover git repository")?;

        Ok(Self { repo })
    }

    /// Get the evolution of a file through git history
    ///
    /// # Arguments
    /// * `file_path` - Path to the file (relative to repository root)
    /// * `max_commits` - Maximum number of commits to retrieve
    pub fn get_file_evolution(&self, file_path: &Path, max_commits: usize) -> Result<Vec<CommitInfo>> {
        let mut revwalk = self.repo.revwalk()
            .context("Failed to create revwalk")?;

        revwalk.push_head()
            .context("Failed to push HEAD")?;

        let mut commits = Vec::new();
        let mut count = 0;

        // Get the relative path from repository root
        let workdir = self.repo.workdir()
            .ok_or_else(|| anyhow!("Repository has no working directory"))?;

        let relative_path = if file_path.is_absolute() {
            file_path.strip_prefix(workdir)
                .context("File path is not within repository")?
        } else {
            file_path
        };

        for oid in revwalk {
            if count >= max_commits {
                break;
            }

            let oid = oid.context("Failed to get commit OID")?;
            let commit = self.repo.find_commit(oid)
                .context("Failed to find commit")?;

            // Check if this commit touched the file
            if self.commit_touches_file(&commit, relative_path)? {
                let commit_info = self.extract_commit_info(&commit, relative_path)?;
                commits.push(commit_info);
                count += 1;
            }
        }

        Ok(commits)
    }

    /// Get blame information for a file
    ///
    /// # Arguments
    /// * `file_path` - Path to the file (relative to repository root)
    /// * `start_line` - Optional starting line (1-indexed)
    /// * `end_line` - Optional ending line (1-indexed)
    pub fn get_blame(
        &self,
        file_path: &Path,
        start_line: Option<usize>,
        end_line: Option<usize>,
    ) -> Result<Vec<BlameInfo>> {
        // Get the relative path from repository root
        let workdir = self.repo.workdir()
            .ok_or_else(|| anyhow!("Repository has no working directory"))?;

        let relative_path = if file_path.is_absolute() {
            file_path.strip_prefix(workdir)
                .context("File path is not within repository")?
        } else {
            file_path
        };

        let mut blame_opts = BlameOptions::new();

        if let Some(start) = start_line {
            if let Some(end) = end_line {
                blame_opts.min_line(start);
                blame_opts.max_line(end);
            }
        }

        let blame = self.repo.blame_file(relative_path, Some(&mut blame_opts))
            .context("Failed to get git blame")?;

        // Read file content to get line text
        let file_full_path = workdir.join(relative_path);
        let content = std::fs::read_to_string(&file_full_path)
            .context("Failed to read file content")?;

        let lines: Vec<&str> = content.lines().collect();
        let mut blame_infos = Vec::new();

        for (idx, line_text) in lines.iter().enumerate() {
            let line_num = idx + 1; // 1-indexed

            // Check if we should include this line
            if let Some(start) = start_line {
                if line_num < start {
                    continue;
                }
            }
            if let Some(end) = end_line {
                if line_num > end {
                    break;
                }
            }

            if let Some(hunk) = blame.get_line(line_num) {
                let commit_id = hunk.final_commit_id();
                let commit = self.repo.find_commit(commit_id)
                    .context("Failed to find commit for blame")?;

                let author = commit.author();
                let timestamp = commit.time().seconds();

                // Convert to DateTime<Utc>
                let date = DateTime::from_timestamp(timestamp, 0)
                    .unwrap_or_else(Utc::now);

                blame_infos.push(BlameInfo {
                    line: line_num,
                    author: author.name().unwrap_or("Unknown").to_string(),
                    author_email: author.email().unwrap_or("").to_string(),
                    sha: format!("{}", commit_id),
                    date,
                    content: line_text.to_string(),
                });
            }
        }

        Ok(blame_infos)
    }

    /// Check if a commit touches a specific file
    fn commit_touches_file(&self, commit: &git2::Commit, file_path: &Path) -> Result<bool> {
        // Get parent commit (if any)
        let parent = if commit.parent_count() > 0 {
            Some(commit.parent(0)?)
        } else {
            None
        };

        let commit_tree = commit.tree()?;

        if let Some(parent) = parent {
            let parent_tree = parent.tree()?;

            let mut diff_opts = DiffOptions::new();
            diff_opts.pathspec(file_path);

            let diff = self.repo.diff_tree_to_tree(
                Some(&parent_tree),
                Some(&commit_tree),
                Some(&mut diff_opts),
            )?;

            Ok(diff.deltas().count() > 0)
        } else {
            // First commit - check if file exists
            Ok(commit_tree.get_path(file_path).is_ok())
        }
    }

    /// Extract commit information with diff stats
    fn extract_commit_info(&self, commit: &git2::Commit, file_path: &Path) -> Result<CommitInfo> {
        let author = commit.author();
        let timestamp = commit.time().seconds();
        let date = DateTime::from_timestamp(timestamp, 0)
            .unwrap_or_else(Utc::now);

        let sha = format!("{}", commit.id());
        let author_name = author.name().unwrap_or("Unknown").to_string();
        let author_email = author.email().unwrap_or("").to_string();
        let message = commit.message().unwrap_or("").trim().to_string();

        // Calculate diff stats
        let (insertions, deletions) = self.get_diff_stats(commit, file_path)?;

        let changes = if insertions > 0 || deletions > 0 {
            format!("+{} -{}", insertions, deletions)
        } else {
            "No changes".to_string()
        };

        Ok(CommitInfo {
            sha,
            author: author_name,
            author_email,
            date,
            message,
            changes,
            insertions,
            deletions,
        })
    }

    /// Get diff statistics for a file in a commit
    fn get_diff_stats(&self, commit: &git2::Commit, file_path: &Path) -> Result<(usize, usize)> {
        let parent = if commit.parent_count() > 0 {
            Some(commit.parent(0)?)
        } else {
            None
        };

        let commit_tree = commit.tree()?;

        let mut diff_opts = DiffOptions::new();
        diff_opts.pathspec(file_path);

        let diff = if let Some(parent) = parent {
            let parent_tree = parent.tree()?;
            self.repo.diff_tree_to_tree(
                Some(&parent_tree),
                Some(&commit_tree),
                Some(&mut diff_opts),
            )?
        } else {
            // First commit - diff against empty tree
            self.repo.diff_tree_to_tree(
                None,
                Some(&commit_tree),
                Some(&mut diff_opts),
            )?
        };

        let stats = diff.stats()?;
        Ok((stats.insertions(), stats.deletions()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::TempDir;

    /// Helper to create a test git repository
    fn create_test_repo() -> Result<(TempDir, PathBuf)> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().to_path_buf();

        // Initialize git repo
        Repository::init(&repo_path)?;

        // Configure git
        let output = Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&repo_path)
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to configure git user.name"));
        }

        let output = Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&repo_path)
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to configure git user.email"));
        }

        Ok((temp_dir, repo_path))
    }

    /// Helper to commit a file
    fn commit_file(repo_path: &Path, filename: &str, content: &str, message: &str) -> Result<()> {
        let file_path = repo_path.join(filename);
        fs::write(&file_path, content)?;

        let output = Command::new("git")
            .args(["add", filename])
            .current_dir(repo_path)
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to add file"));
        }

        let output = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(repo_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to commit: {}", stderr));
        }

        Ok(())
    }

    #[test]
    fn test_git_history_new() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;

        let git_history = GitHistory::new(&repo_path)?;
        assert!(git_history.repo.path().exists());

        Ok(())
    }

    #[test]
    fn test_git_history_new_non_repo() {
        let temp_dir = TempDir::new().unwrap();
        let result = GitHistory::new(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_get_file_evolution() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;

        // Create initial commit
        commit_file(&repo_path, "test.txt", "Line 1\n", "Initial commit")?;

        // Modify file
        commit_file(&repo_path, "test.txt", "Line 1\nLine 2\n", "Add line 2")?;

        // Modify again
        commit_file(&repo_path, "test.txt", "Line 1\nLine 2\nLine 3\n", "Add line 3")?;

        let git_history = GitHistory::new(&repo_path)?;
        let evolution = git_history.get_file_evolution(Path::new("test.txt"), 10)?;

        assert_eq!(evolution.len(), 3);

        // Check newest commit first
        assert_eq!(evolution[0].message, "Add line 3");
        assert_eq!(evolution[1].message, "Add line 2");
        assert_eq!(evolution[2].message, "Initial commit");

        // Check author info
        assert_eq!(evolution[0].author, "Test User");
        assert_eq!(evolution[0].author_email, "test@example.com");

        Ok(())
    }

    #[test]
    fn test_get_file_evolution_max_commits() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;

        // Create multiple commits
        commit_file(&repo_path, "test.txt", "v1\n", "Commit 1")?;
        commit_file(&repo_path, "test.txt", "v2\n", "Commit 2")?;
        commit_file(&repo_path, "test.txt", "v3\n", "Commit 3")?;
        commit_file(&repo_path, "test.txt", "v4\n", "Commit 4")?;

        let git_history = GitHistory::new(&repo_path)?;
        let evolution = git_history.get_file_evolution(Path::new("test.txt"), 2)?;

        assert_eq!(evolution.len(), 2);
        assert_eq!(evolution[0].message, "Commit 4");
        assert_eq!(evolution[1].message, "Commit 3");

        Ok(())
    }

    #[test]
    fn test_get_file_evolution_nonexistent_file() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;

        commit_file(&repo_path, "other.txt", "content\n", "Initial commit")?;

        let git_history = GitHistory::new(&repo_path)?;
        let evolution = git_history.get_file_evolution(Path::new("nonexistent.txt"), 10)?;

        assert_eq!(evolution.len(), 0);

        Ok(())
    }

    #[test]
    fn test_get_blame() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;

        // Create file with multiple lines
        commit_file(&repo_path, "test.txt", "Line 1\nLine 2\nLine 3\n", "Initial commit")?;

        // Modify line 2
        commit_file(&repo_path, "test.txt", "Line 1\nModified Line 2\nLine 3\n", "Modify line 2")?;

        let git_history = GitHistory::new(&repo_path)?;
        let blame_info = git_history.get_blame(Path::new("test.txt"), None, None)?;

        assert_eq!(blame_info.len(), 3);

        // Check line numbers
        assert_eq!(blame_info[0].line, 1);
        assert_eq!(blame_info[1].line, 2);
        assert_eq!(blame_info[2].line, 3);

        // Check content
        assert_eq!(blame_info[0].content, "Line 1");
        assert_eq!(blame_info[1].content, "Modified Line 2");
        assert_eq!(blame_info[2].content, "Line 3");

        // Check author
        assert_eq!(blame_info[0].author, "Test User");
        assert_eq!(blame_info[1].author, "Test User");

        Ok(())
    }

    #[test]
    fn test_get_blame_with_line_range() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;

        commit_file(&repo_path, "test.txt", "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\n", "Initial commit")?;

        let git_history = GitHistory::new(&repo_path)?;
        let blame_info = git_history.get_blame(Path::new("test.txt"), Some(2), Some(4))?;

        assert_eq!(blame_info.len(), 3);
        assert_eq!(blame_info[0].line, 2);
        assert_eq!(blame_info[1].line, 3);
        assert_eq!(blame_info[2].line, 4);

        Ok(())
    }

    #[test]
    fn test_get_blame_nonexistent_file() {
        let (_temp_dir, repo_path) = create_test_repo().unwrap();
        commit_file(&repo_path, "other.txt", "content\n", "Initial commit").unwrap();

        let git_history = GitHistory::new(&repo_path).unwrap();
        let result = git_history.get_blame(Path::new("nonexistent.txt"), None, None);

        assert!(result.is_err());
    }

    #[test]
    fn test_diff_stats() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;

        // Create initial file with 3 lines
        commit_file(&repo_path, "test.txt", "Line 1\nLine 2\nLine 3\n", "Initial commit")?;

        // Add 2 lines
        commit_file(&repo_path, "test.txt", "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\n", "Add 2 lines")?;

        let git_history = GitHistory::new(&repo_path)?;
        let evolution = git_history.get_file_evolution(Path::new("test.txt"), 1)?;

        assert_eq!(evolution.len(), 1);
        assert_eq!(evolution[0].insertions, 2);
        assert_eq!(evolution[0].deletions, 0);
        assert_eq!(evolution[0].changes, "+2 -0");

        Ok(())
    }
}
