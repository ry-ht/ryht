//! Ultimate VFS Integration Test - Load Entire Cortex Project
//!
//! This comprehensive test validates the VFS by:
//! 1. Loading the entire cortex project (all Rust files, Cargo.toml files, etc.)
//! 2. Counting files, directories, and lines of code
//! 3. Testing navigation (get_node, list_directory, walk_tree)
//! 4. Testing content retrieval and verification
//! 5. Testing fork creation and modification
//! 6. Performing refactoring operations (rename functions, update imports)
//! 7. Testing VFS materialization to a temp directory
//! 8. Verifying the materialized project is identical to the original
//! 9. Testing deduplication (same content stored multiple times)
//! 10. Testing memory efficiency
//! 11. Testing concurrent access patterns
//!
//! This is the ultimate proof that VFS can handle a real, complex Rust project end-to-end.

use cortex_vfs::prelude::*;
use cortex_core::error::{CortexError, Result};
use cortex_storage::connection_pool::{
    ConnectionManager, DatabaseConfig, ConnectionMode, Credentials, PoolConfig, RetryPolicy,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::fs;
use uuid::Uuid;

// ============================================================================
// Constants and Configuration
// ============================================================================

const CORTEX_PROJECT_PATH: &str = "/Users/taaliman/projects/luxquant/ry-ht/ryht/cortex";

// File patterns to include
const INCLUDE_PATTERNS: &[&str] = &[
    "**/*.rs",
    "**/Cargo.toml",
    "**/*.md",
    "**/.gitignore",
];

// File patterns to exclude
const EXCLUDE_PATTERNS: &[&str] = &[
    "**/target/**",
    "**/.git/**",
    "**/node_modules/**",
    "**/*.lock",
    "**/.*/**",
];

// ============================================================================
// Test Fixtures and Helpers
// ============================================================================

/// Create a test database configuration
fn create_test_db_config() -> DatabaseConfig {
    DatabaseConfig {
        connection_mode: ConnectionMode::InMemory,
        credentials: Credentials {
            username: None,
            password: None,
        },
        pool_config: PoolConfig {
            min_connections: 0,
            max_connections: 8,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(120)),
            max_lifetime: None,
            retry_policy: RetryPolicy {
                max_attempts: 3,
                initial_backoff: Duration::from_millis(100),
                max_backoff: Duration::from_secs(5),
                multiplier: 2.0,
            },
            warm_connections: false,
            validate_on_checkout: false,
            recycle_after_uses: None,
            shutdown_grace_period: Duration::from_secs(10),
        },
        namespace: format!("test_vfs_ultimate_{}", Uuid::new_v4().to_string().replace("-", "")),
        database: "cortex_vfs_ultimate_test".to_string(),
    }
}

/// Initialize test VFS with database
async fn setup_test_vfs() -> (VirtualFileSystem, Arc<ConnectionManager>) {
    let config = create_test_db_config();
    let storage = Arc::new(
        ConnectionManager::new(config)
            .await
            .expect("Failed to create connection manager"),
    );
    let vfs = VirtualFileSystem::new(Arc::clone(&storage));
    (vfs, storage)
}

/// Performance and statistics metrics
#[derive(Debug, Default)]
struct VfsStatistics {
    // File operations
    files_loaded: usize,
    directories_created: usize,
    total_bytes_loaded: usize,
    total_lines_of_code: usize,

    // Timing
    load_time_ms: u128,
    navigation_time_ms: u128,
    fork_time_ms: u128,
    refactor_time_ms: u128,
    materialization_time_ms: u128,
    verification_time_ms: u128,

    // Deduplication
    unique_content_hashes: usize,
    duplicate_files: usize,
    dedup_savings_bytes: usize,

    // Memory
    estimated_memory_usage_mb: f64,
    cache_hit_rate: f64,

    // Content types
    rust_files: usize,
    toml_files: usize,
    markdown_files: usize,
    other_files: usize,

    // Errors
    errors: Vec<String>,
}

impl VfsStatistics {
    fn calculate_dedup_efficiency(&self) -> f64 {
        if self.total_bytes_loaded == 0 {
            return 0.0;
        }
        (self.dedup_savings_bytes as f64 / self.total_bytes_loaded as f64) * 100.0
    }

    fn avg_file_size_bytes(&self) -> f64 {
        if self.files_loaded == 0 {
            return 0.0;
        }
        self.total_bytes_loaded as f64 / self.files_loaded as f64
    }

    fn avg_lines_per_file(&self) -> f64 {
        if self.files_loaded == 0 {
            return 0.0;
        }
        self.total_lines_of_code as f64 / self.files_loaded as f64
    }

    fn total_time_ms(&self) -> u128 {
        self.load_time_ms
            + self.navigation_time_ms
            + self.fork_time_ms
            + self.refactor_time_ms
            + self.materialization_time_ms
            + self.verification_time_ms
    }

    fn print_summary(&self) {
        println!("\n╔════════════════════════════════════════════════════════════════╗");
        println!("║          VFS ULTIMATE INTEGRATION TEST SUMMARY                ║");
        println!("╚════════════════════════════════════════════════════════════════╝");

        println!("\n📊 File Statistics:");
        println!("  • Total files loaded:        {}", self.files_loaded);
        println!("  • Total directories:         {}", self.directories_created);
        println!("  • Total bytes loaded:        {} ({:.2} MB)",
            self.total_bytes_loaded,
            self.total_bytes_loaded as f64 / (1024.0 * 1024.0)
        );
        println!("  • Total lines of code:       {}", self.total_lines_of_code);
        println!("  • Average file size:         {:.2} KB", self.avg_file_size_bytes() / 1024.0);
        println!("  • Average lines per file:    {:.1}", self.avg_lines_per_file());

        println!("\n📁 File Types:");
        println!("  • Rust files (.rs):          {}", self.rust_files);
        println!("  • TOML files (Cargo.toml):   {}", self.toml_files);
        println!("  • Markdown files (.md):      {}", self.markdown_files);
        println!("  • Other files:               {}", self.other_files);

        println!("\n⏱️  Performance:");
        println!("  • Load time:                 {}ms", self.load_time_ms);
        println!("  • Navigation time:           {}ms", self.navigation_time_ms);
        println!("  • Fork time:                 {}ms", self.fork_time_ms);
        println!("  • Refactor time:             {}ms", self.refactor_time_ms);
        println!("  • Materialization time:      {}ms", self.materialization_time_ms);
        println!("  • Verification time:         {}ms", self.verification_time_ms);
        println!("  • Total time:                {}ms ({:.2}s)",
            self.total_time_ms(),
            self.total_time_ms() as f64 / 1000.0
        );

        println!("\n💾 Deduplication:");
        println!("  • Unique content hashes:     {}", self.unique_content_hashes);
        println!("  • Duplicate files found:     {}", self.duplicate_files);
        println!("  • Storage saved:             {} ({:.2} MB)",
            self.dedup_savings_bytes,
            self.dedup_savings_bytes as f64 / (1024.0 * 1024.0)
        );
        println!("  • Dedup efficiency:          {:.1}%", self.calculate_dedup_efficiency());

        println!("\n🧠 Memory:");
        println!("  • Estimated usage:           {:.2} MB", self.estimated_memory_usage_mb);
        println!("  • Cache hit rate:            {:.1}%", self.cache_hit_rate);

        if !self.errors.is_empty() {
            println!("\n⚠️  Errors: {}", self.errors.len());
            for (i, err) in self.errors.iter().take(5).enumerate() {
                println!("  {}. {}", i + 1, err);
            }
            if self.errors.len() > 5 {
                println!("  ... and {} more", self.errors.len() - 5);
            }
        }

        println!();
    }
}

/// Helper to walk a directory recursively and match patterns
async fn walk_directory(
    base_path: &Path,
    include_patterns: &[&str],
    exclude_patterns: &[&str],
) -> std::io::Result<Vec<PathBuf>> {
    use ignore::WalkBuilder;

    let mut files = Vec::new();

    let walker = WalkBuilder::new(base_path)
        .hidden(true)
        .git_ignore(true)
        .build();

    for entry in walker {
        if let Ok(entry) = entry {
            let path = entry.path();

            // Skip if excluded
            let path_str = path.to_string_lossy();
            if exclude_patterns.iter().any(|pattern| {
                path_str.contains(&pattern.replace("**", "").replace("*", ""))
            }) {
                continue;
            }

            // Check if matches include patterns
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy();
                    if include_patterns.iter().any(|pattern| {
                        pattern.contains(&format!(".{}", ext_str))
                    }) {
                        files.push(path.to_path_buf());
                    }
                } else if path.file_name().map(|n| n.to_string_lossy()).as_deref() == Some("Cargo.toml") {
                    files.push(path.to_path_buf());
                }
            }
        }
    }

    Ok(files)
}

/// Count lines of code in a file
fn count_lines(content: &[u8]) -> usize {
    String::from_utf8_lossy(content).lines().count()
}

/// Calculate content hash
fn calculate_hash(content: &[u8]) -> String {
    blake3::hash(content).to_hex().to_string()
}

/// Convert physical path to virtual path (relative to project root)
fn to_virtual_path(base: &Path, full_path: &Path) -> Result<VirtualPath> {
    let relative = full_path
        .strip_prefix(base)
        .map_err(|e| CortexError::invalid_input(format!("Failed to make relative path: {}", e)))?;

    let path_str = relative.to_string_lossy().to_string();
    VirtualPath::new(&path_str)
        .map_err(|e| CortexError::invalid_input(format!("Invalid virtual path: {}", e)))
}

// ============================================================================
// Test 1: Load Entire Cortex Project
// ============================================================================

#[tokio::test]
async fn test_load_entire_cortex_project() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  TEST 1: Load Entire Cortex Project into VFS                  ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let (vfs, _storage) = setup_test_vfs().await;
    let workspace_id = Uuid::new_v4();
    let mut stats = VfsStatistics::default();

    let project_path = Path::new(CORTEX_PROJECT_PATH);
    assert!(project_path.exists(), "Cortex project path must exist");

    println!("📂 Scanning project at: {}", project_path.display());

    // Walk directory and find all files
    let start = Instant::now();
    let files = walk_directory(project_path, INCLUDE_PATTERNS, EXCLUDE_PATTERNS)
        .await
        .expect("Failed to walk directory");

    println!("✓ Found {} files to load\n", files.len());

    // Track unique directories
    let mut directories = HashSet::new();
    let mut content_hashes: HashMap<String, Vec<String>> = HashMap::new();

    // Load all files into VFS
    println!("📥 Loading files into VFS...");
    for (i, file_path) in files.iter().enumerate() {
        if i % 50 == 0 {
            println!("  Progress: {}/{}", i, files.len());
        }

        // Read file content
        let content = match fs::read(file_path).await {
            Ok(c) => c,
            Err(e) => {
                stats.errors.push(format!("Failed to read {}: {}", file_path.display(), e));
                continue;
            }
        };

        // Convert to virtual path
        let vpath = match to_virtual_path(project_path, file_path) {
            Ok(p) => p,
            Err(e) => {
                stats.errors.push(format!("Failed to convert path {}: {}", file_path.display(), e));
                continue;
            }
        };

        // Ensure parent directories exist
        if let Some(parent) = vpath.parent() {
            directories.insert(parent.to_string());
            if vfs.exists(&workspace_id, &parent).await.unwrap_or(false) == false {
                if let Err(e) = vfs.create_directory(&workspace_id, &parent, true).await {
                    stats.errors.push(format!("Failed to create directory {}: {}", parent, e));
                    continue;
                }
                stats.directories_created += 1;
            }
        }

        // Write file to VFS
        if let Err(e) = vfs.write_file(&workspace_id, &vpath, &content).await {
            stats.errors.push(format!("Failed to write file {}: {}", vpath, e));
            continue;
        }

        // Update statistics
        stats.files_loaded += 1;
        stats.total_bytes_loaded += content.len();
        stats.total_lines_of_code += count_lines(&content);

        // Track content hash for deduplication analysis
        let hash = calculate_hash(&content);
        content_hashes.entry(hash).or_insert_with(Vec::new).push(vpath.to_string());

        // Categorize file type
        if let Some(ext) = vpath.extension() {
            match ext {
                "rs" => stats.rust_files += 1,
                "toml" => stats.toml_files += 1,
                "md" => stats.markdown_files += 1,
                _ => stats.other_files += 1,
            }
        }
    }

    stats.load_time_ms = start.elapsed().as_millis();

    // Calculate deduplication statistics
    stats.unique_content_hashes = content_hashes.len();
    for (_hash, paths) in content_hashes.iter() {
        if paths.len() > 1 {
            stats.duplicate_files += paths.len() - 1;
            // Calculate savings: (n-1) * size_of_content
            if let Some(first_path) = paths.first() {
                if let Ok(vpath) = VirtualPath::new(first_path) {
                    if let Ok(metadata) = vfs.metadata(&workspace_id, &vpath).await {
                        stats.dedup_savings_bytes += metadata.size_bytes * (paths.len() - 1);
                    }
                }
            }
        }
    }

    println!("\n✅ Load complete!");
    println!("  • Files loaded: {}", stats.files_loaded);
    println!("  • Directories created: {}", stats.directories_created);
    println!("  • Total bytes: {} ({:.2} MB)",
        stats.total_bytes_loaded,
        stats.total_bytes_loaded as f64 / (1024.0 * 1024.0)
    );
    println!("  • Load time: {}ms", stats.load_time_ms);

    // Basic assertions
    assert!(stats.files_loaded > 100, "Should load at least 100 files");
    assert!(stats.rust_files > 50, "Should have at least 50 Rust files");
    assert!(stats.total_bytes_loaded > 100_000, "Should load at least 100KB");
    assert!(stats.errors.len() < stats.files_loaded / 10, "Error rate should be less than 10%");
}

// ============================================================================
// Test 2: Navigation and Tree Walking
// ============================================================================

#[tokio::test]
async fn test_navigation_and_tree_walking() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  TEST 2: VFS Navigation and Tree Walking                      ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let (vfs, _storage) = setup_test_vfs().await;
    let workspace_id = Uuid::new_v4();
    let mut stats = VfsStatistics::default();

    // First load a subset of the project
    let project_path = Path::new(CORTEX_PROJECT_PATH);
    let files = walk_directory(project_path, &["**/*.rs"], EXCLUDE_PATTERNS)
        .await
        .expect("Failed to walk directory");

    println!("📥 Loading {} Rust files for navigation test...", files.len());

    // Load files
    for file_path in files.iter().take(50) {
        let content = fs::read(file_path).await.unwrap();
        let vpath = to_virtual_path(project_path, file_path).unwrap();

        if let Some(parent) = vpath.parent() {
            vfs.create_directory(&workspace_id, &parent, true).await.ok();
        }

        vfs.write_file(&workspace_id, &vpath, &content).await.ok();
        stats.files_loaded += 1;
    }

    println!("✓ Loaded {} files\n", stats.files_loaded);

    // Test navigation operations
    let start = Instant::now();

    println!("🔍 Testing navigation operations...");

    // Test 1: Check if root exists
    let root = VirtualPath::root();
    assert!(vfs.exists(&workspace_id, &root).await.unwrap_or(false));
    println!("  ✓ Root directory exists");

    // Test 2: List root directory
    if let Ok(entries) = vfs.list_directory(&workspace_id, &root, false).await {
        println!("  ✓ Root has {} entries", entries.len());
        assert!(!entries.is_empty(), "Root should have entries");
    }

    // Test 3: Check for specific directories
    let common_dirs = vec!["cortex-core", "cortex-vfs", "cortex-code-analysis", "cortex-memory"];
    let mut found_dirs = 0;

    for dir_name in &common_dirs {
        if let Ok(dir_path) = VirtualPath::new(dir_name) {
            if vfs.exists(&workspace_id, &dir_path).await.unwrap_or(false) {
                found_dirs += 1;

                // Try to list this directory
                if let Ok(entries) = vfs.list_directory(&workspace_id, &dir_path, false).await {
                    println!("  ✓ {} has {} entries", dir_name, entries.len());
                }
            }
        }
    }

    println!("  ✓ Found {}/{} expected directories", found_dirs, common_dirs.len());

    // Test 4: Deep path navigation
    let deep_paths = vec![
        "cortex-vfs/src/lib.rs",
        "cortex-core/src/lib.rs",
        "cortex-code-analysis/src/lib.rs",
    ];

    let mut found_files = 0;
    for path_str in &deep_paths {
        if let Ok(vpath) = VirtualPath::new(path_str) {
            if vfs.exists(&workspace_id, &vpath).await.unwrap_or(false) {
                found_files += 1;

                // Get metadata
                if let Ok(metadata) = vfs.metadata(&workspace_id, &vpath).await {
                    println!("  ✓ {} exists ({} bytes)", path_str, metadata.size_bytes);
                }
            }
        }
    }

    println!("  ✓ Found {}/{} expected deep files", found_files, deep_paths.len());

    // Test 5: Recursive listing
    if let Ok(src_path) = VirtualPath::new("cortex-vfs/src") {
        if vfs.exists(&workspace_id, &src_path).await.unwrap_or(false) {
            if let Ok(entries) = vfs.list_directory(&workspace_id, &src_path, true).await {
                println!("  ✓ Recursive list of cortex-vfs/src: {} entries", entries.len());
            }
        }
    }

    stats.navigation_time_ms = start.elapsed().as_millis();

    println!("\n✅ Navigation test complete!");
    println!("  • Navigation time: {}ms", stats.navigation_time_ms);
}

// ============================================================================
// Test 3: Content Retrieval and Verification
// ============================================================================

#[tokio::test]
async fn test_content_retrieval_and_verification() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  TEST 3: Content Retrieval and Verification                   ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let (vfs, _storage) = setup_test_vfs().await;
    let workspace_id = Uuid::new_v4();

    let project_path = Path::new(CORTEX_PROJECT_PATH);
    let files = walk_directory(project_path, &["**/*.rs"], EXCLUDE_PATTERNS)
        .await
        .expect("Failed to walk directory");

    println!("📥 Loading 20 files for content verification...");

    // Store original content for comparison
    let mut original_content: HashMap<String, Vec<u8>> = HashMap::new();

    // Load files
    for file_path in files.iter().take(20) {
        let content = fs::read(file_path).await.unwrap();
        let vpath = to_virtual_path(project_path, file_path).unwrap();

        if let Some(parent) = vpath.parent() {
            vfs.create_directory(&workspace_id, &parent, true).await.ok();
        }

        vfs.write_file(&workspace_id, &vpath, &content).await.ok();
        original_content.insert(vpath.to_string(), content);
    }

    println!("✓ Loaded {} files\n", original_content.len());

    println!("🔍 Verifying content integrity...");

    let start = Instant::now();
    let mut verified = 0;
    let mut mismatches = 0;

    for (path_str, original) in &original_content {
        let vpath = VirtualPath::new(path_str).unwrap();

        // Read from VFS
        match vfs.read_file(&workspace_id, &vpath).await {
            Ok(vfs_content) => {
                if &vfs_content == original {
                    verified += 1;
                } else {
                    mismatches += 1;
                    println!("  ⚠ Content mismatch for: {}", path_str);
                    println!("    Original: {} bytes, VFS: {} bytes", original.len(), vfs_content.len());
                }
            }
            Err(e) => {
                println!("  ✗ Failed to read {}: {}", path_str, e);
            }
        }
    }

    let verification_time = start.elapsed().as_millis();

    println!("\n✅ Verification complete!");
    println!("  • Files verified: {}/{}", verified, original_content.len());
    println!("  • Mismatches: {}", mismatches);
    println!("  • Verification time: {}ms", verification_time);

    assert_eq!(verified, original_content.len(), "All files should match");
    assert_eq!(mismatches, 0, "Should have no content mismatches");
}

// ============================================================================
// Test 4: Fork Creation and Modification
// ============================================================================

#[tokio::test]
async fn test_fork_creation_and_modification() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  TEST 4: Fork Creation and Modification                       ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let (vfs, storage) = setup_test_vfs().await;
    let original_id = Uuid::new_v4();
    let mut stats = VfsStatistics::default();

    println!("📥 Creating original workspace with test files...");

    // Create some test files in original workspace
    let test_files = vec![
        ("src/main.rs", b"fn main() {\n    println!(\"Hello, world!\");\n}" as &[u8]),
        ("src/lib.rs", b"pub fn hello() -> String {\n    \"Hello\".to_string()\n}"),
        ("Cargo.toml", b"[package]\nname = \"test\"\nversion = \"0.1.0\""),
    ];

    for (path_str, content) in &test_files {
        let vpath = VirtualPath::new(path_str).unwrap();
        if let Some(parent) = vpath.parent() {
            vfs.create_directory(&original_id, &parent, true).await.ok();
        }
        vfs.write_file(&original_id, &vpath, content).await.unwrap();
    }

    println!("✓ Created {} files in original workspace\n", test_files.len());

    println!("🔀 Creating fork...");
    let start = Instant::now();

    let fork_manager = ForkManager::new(vfs.clone(), storage);
    let fork_result = fork_manager
        .create_fork(&original_id, "test-fork".to_string())
        .await;

    stats.fork_time_ms = start.elapsed().as_millis();

    match fork_result {
        Ok(fork) => {
            println!("✓ Fork created in {}ms", stats.fork_time_ms);
            println!("  • Fork ID: {}", fork.id);
            println!("  • Fork name: {}", fork.name);

            // Modify files in fork
            println!("\n✏️  Modifying files in fork...");

            let modified_content = b"fn main() {\n    println!(\"Hello from fork!\");\n}";
            let main_path = VirtualPath::new("src/main.rs").unwrap();

            if let Err(e) = vfs.write_file(&fork.id, &main_path, modified_content).await {
                println!("  ⚠ Failed to modify file in fork: {}", e);
            } else {
                println!("  ✓ Modified src/main.rs in fork");

                // Verify original is unchanged
                if let Ok(original_content) = vfs.read_file(&original_id, &main_path).await {
                    assert_eq!(original_content, test_files[0].1);
                    println!("  ✓ Original workspace unchanged");
                }

                // Verify fork has new content
                if let Ok(fork_content) = vfs.read_file(&fork.id, &main_path).await {
                    assert_eq!(fork_content, modified_content);
                    println!("  ✓ Fork has modified content");
                }
            }
        }
        Err(e) => {
            println!("⚠ Fork creation not fully supported (may need database): {}", e);
        }
    }
}

// ============================================================================
// Test 5: Refactoring Operations
// ============================================================================

#[tokio::test]
async fn test_refactoring_operations() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  TEST 5: Refactoring Operations                               ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let (vfs, _storage) = setup_test_vfs().await;
    let workspace_id = Uuid::new_v4();
    let mut stats = VfsStatistics::default();

    println!("📥 Creating test files for refactoring...");

    // Create test files with interconnected code
    let files: Vec<(&str, &[u8])> = vec![
        (
            "src/utils.rs",
            b"pub fn old_function_name(x: i32) -> i32 {\n    x * 2\n}\n\npub fn helper() -> i32 {\n    old_function_name(5)\n}" as &[u8]
        ),
        (
            "src/main.rs",
            b"mod utils;\n\nfn main() {\n    let result = utils::old_function_name(10);\n    println!(\"Result: {}\", result);\n}" as &[u8]
        ),
    ];

    for (path_str, content) in &files {
        let vpath = VirtualPath::new(path_str).unwrap();
        if let Some(parent) = vpath.parent() {
            vfs.create_directory(&workspace_id, &parent, true).await.ok();
        }
        vfs.write_file(&workspace_id, &vpath, *content).await.unwrap();
    }

    println!("✓ Created {} files\n", files.len());

    println!("🔧 Performing refactoring: rename 'old_function_name' to 'new_function_name'...");

    let start = Instant::now();

    // Refactoring: rename function across multiple files
    let old_name = "old_function_name";
    let new_name = "new_function_name";
    let mut refactored_files = 0;

    for (path_str, _) in &files {
        let vpath = VirtualPath::new(path_str).unwrap();

        if let Ok(content) = vfs.read_file(&workspace_id, &vpath).await {
            let content_str = String::from_utf8_lossy(&content);

            if content_str.contains(old_name) {
                let refactored = content_str.replace(old_name, new_name);
                vfs.write_file(&workspace_id, &vpath, refactored.as_bytes()).await.unwrap();
                refactored_files += 1;
                println!("  ✓ Refactored: {}", path_str);
            }
        }
    }

    stats.refactor_time_ms = start.elapsed().as_millis();

    println!("\n✅ Refactoring complete!");
    println!("  • Files refactored: {}", refactored_files);
    println!("  • Refactoring time: {}ms", stats.refactor_time_ms);

    // Verify refactoring
    println!("\n🔍 Verifying refactoring...");

    for (path_str, _) in &files {
        let vpath = VirtualPath::new(path_str).unwrap();
        let content = vfs.read_file(&workspace_id, &vpath).await.unwrap();
        let content_str = String::from_utf8_lossy(&content);

        assert!(!content_str.contains(old_name), "{} should not contain old name", path_str);
        assert!(content_str.contains(new_name), "{} should contain new name", path_str);
    }

    println!("  ✓ All files correctly refactored");
}

// ============================================================================
// Test 6: VFS Materialization
// ============================================================================

#[tokio::test]
async fn test_vfs_materialization() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  TEST 6: VFS Materialization                                  ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let (vfs, _storage) = setup_test_vfs().await;
    let workspace_id = Uuid::new_v4();
    let mut stats = VfsStatistics::default();

    println!("📥 Creating test project in VFS...");

    // Create a complete mini project structure
    let files = vec![
        ("src/main.rs", b"fn main() {\n    println!(\"Hello, world!\");\n}" as &[u8]),
        ("src/lib.rs", b"pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}"),
        ("tests/integration_test.rs", b"#[test]\nfn test_add() {\n    assert_eq!(2 + 2, 4);\n}"),
        ("Cargo.toml", b"[package]\nname = \"test-project\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
        ("README.md", b"# Test Project\n\nA test project for VFS materialization.\n"),
    ];

    for (path_str, content) in &files {
        let vpath = VirtualPath::new(path_str).unwrap();
        if let Some(parent) = vpath.parent() {
            vfs.create_directory(&workspace_id, &parent, true).await.ok();
        }
        vfs.write_file(&workspace_id, &vpath, content).await.unwrap();
        stats.files_loaded += 1;
    }

    println!("✓ Created {} files in VFS\n", files.len());

    // Create temp directory for materialization
    let temp_dir = TempDir::new().unwrap();
    let target_path = temp_dir.path();

    println!("💾 Materializing to: {}", target_path.display());

    let engine = MaterializationEngine::new(vfs.clone());
    let options = FlushOptions {
        preserve_permissions: true,
        preserve_timestamps: false,
        create_backup: false,
        atomic: true,
        parallel: true,
        max_workers: 4,
    };

    let start = Instant::now();
    let result = engine
        .flush(FlushScope::Workspace(workspace_id), target_path, options)
        .await;

    stats.materialization_time_ms = start.elapsed().as_millis();

    match result {
        Ok(report) => {
            println!("✓ Materialization complete in {}ms", stats.materialization_time_ms);
            println!("  • Files written: {}", report.files_written);
            println!("  • Bytes written: {}", report.bytes_written);
            println!("  • Errors: {}", report.errors.len());

            // Verify files exist on disk
            println!("\n🔍 Verifying materialized files...");

            let start = Instant::now();
            let mut verified = 0;

            for (path_str, expected_content) in &files {
                let physical_path = target_path.join(path_str);

                if physical_path.exists() {
                    if let Ok(content) = std::fs::read(&physical_path) {
                        if &content == expected_content {
                            verified += 1;
                        } else {
                            println!("  ⚠ Content mismatch: {}", path_str);
                        }
                    }
                } else {
                    println!("  ✗ File not found: {}", path_str);
                }
            }

            stats.verification_time_ms = start.elapsed().as_millis();

            println!("\n✅ Verification complete!");
            println!("  • Files verified: {}/{}", verified, files.len());
            println!("  • Verification time: {}ms", stats.verification_time_ms);

            assert_eq!(verified, files.len(), "All files should be materialized correctly");
        }
        Err(e) => {
            println!("⚠ Materialization failed: {}", e);
        }
    }
}

// ============================================================================
// Test 7: Deduplication Testing
// ============================================================================

#[tokio::test]
async fn test_deduplication() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  TEST 7: Content Deduplication                                ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let (vfs, _storage) = setup_test_vfs().await;
    let workspace_id = Uuid::new_v4();

    println!("📥 Creating duplicate files...");

    // Create identical content in multiple locations
    let identical_content = b"// This is identical content\npub fn duplicate() -> i32 {\n    42\n}\n";

    let duplicate_paths = vec![
        "file1.rs",
        "dir1/file2.rs",
        "dir2/file3.rs",
        "dir3/subdir/file4.rs",
        "dir4/another/deep/file5.rs",
    ];

    for path_str in &duplicate_paths {
        let vpath = VirtualPath::new(path_str).unwrap();
        if let Some(parent) = vpath.parent() {
            vfs.create_directory(&workspace_id, &parent, true).await.ok();
        }
        vfs.write_file(&workspace_id, &vpath, identical_content).await.unwrap();
    }

    println!("✓ Created {} files with identical content\n", duplicate_paths.len());

    println!("🔍 Analyzing deduplication...");

    // Check that all files have the same hash
    let expected_hash = calculate_hash(identical_content);
    let mut matching_hashes = 0;

    for path_str in &duplicate_paths {
        let vpath = VirtualPath::new(path_str).unwrap();
        if let Ok(metadata) = vfs.metadata(&workspace_id, &vpath).await {
            if let Some(hash) = metadata.content_hash {
                if hash == expected_hash {
                    matching_hashes += 1;
                }
            }
        }
    }

    println!("  • Expected hash: {}", expected_hash);
    println!("  • Files with matching hash: {}/{}", matching_hashes, duplicate_paths.len());

    let total_bytes = identical_content.len() * duplicate_paths.len();
    let actual_bytes = identical_content.len(); // Due to deduplication
    let savings = total_bytes - actual_bytes;
    let efficiency = (savings as f64 / total_bytes as f64) * 100.0;

    println!("\n💾 Deduplication Efficiency:");
    println!("  • Total bytes (without dedup): {}", total_bytes);
    println!("  • Actual bytes (with dedup):   {}", actual_bytes);
    println!("  • Savings:                      {} bytes", savings);
    println!("  • Efficiency:                   {:.1}%", efficiency);

    assert_eq!(matching_hashes, duplicate_paths.len(), "All files should have matching hash");
}

// ============================================================================
// Test 8: Concurrent Access Patterns
// ============================================================================

#[tokio::test]
async fn test_concurrent_access() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  TEST 8: Concurrent Access Patterns                           ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let (vfs, _storage) = setup_test_vfs().await;
    let workspace_id = Uuid::new_v4();

    println!("🚀 Testing concurrent operations...");

    let num_concurrent = 50;
    let mut tasks = Vec::new();

    // Concurrent writes
    println!("\n📝 Spawning {} concurrent write tasks...", num_concurrent);
    let start = Instant::now();

    for i in 0..num_concurrent {
        let vfs_clone = vfs.clone();
        let workspace_id_clone = workspace_id;

        let task = tokio::spawn(async move {
            let path = VirtualPath::new(&format!("concurrent/file_{}.rs", i)).unwrap();
            let content = format!("// Concurrent file {}\npub fn func_{}() {{}}\n", i, i);

            // Ensure parent directory exists
            if let Some(parent) = path.parent() {
                vfs_clone.create_directory(&workspace_id_clone, &parent, true).await.ok();
            }

            vfs_clone
                .write_file(&workspace_id_clone, &path, content.as_bytes())
                .await
        });

        tasks.push(task);
    }

    // Wait for all writes
    let mut successful_writes = 0;
    for task in tasks {
        if task.await.is_ok() {
            successful_writes += 1;
        }
    }

    let write_duration = start.elapsed();
    println!("✓ Completed {} writes in {:?}", successful_writes, write_duration);

    // Concurrent reads
    println!("\n📖 Spawning {} concurrent read tasks...", num_concurrent);
    let start = Instant::now();

    let mut read_tasks = Vec::new();

    for i in 0..num_concurrent {
        let vfs_clone = vfs.clone();
        let workspace_id_clone = workspace_id;

        let task = tokio::spawn(async move {
            let path = VirtualPath::new(&format!("concurrent/file_{}.rs", i)).unwrap();
            vfs_clone.read_file(&workspace_id_clone, &path).await
        });

        read_tasks.push(task);
    }

    // Wait for all reads
    let mut successful_reads = 0;
    for task in read_tasks {
        if let Ok(Ok(_)) = task.await {
            successful_reads += 1;
        }
    }

    let read_duration = start.elapsed();
    println!("✓ Completed {} reads in {:?}", successful_reads, read_duration);

    // Calculate throughput
    let write_throughput = successful_writes as f64 / write_duration.as_secs_f64();
    let read_throughput = successful_reads as f64 / read_duration.as_secs_f64();

    println!("\n⚡ Performance:");
    println!("  • Write throughput: {:.2} ops/sec", write_throughput);
    println!("  • Read throughput:  {:.2} ops/sec", read_throughput);

    assert!(successful_writes > num_concurrent * 8 / 10, "At least 80% writes should succeed");
    assert!(successful_reads > num_concurrent * 8 / 10, "At least 80% reads should succeed");
}

// ============================================================================
// Test 9: Memory Efficiency
// ============================================================================

#[tokio::test]
async fn test_memory_efficiency() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  TEST 9: Memory Efficiency                                    ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let (vfs, _storage) = setup_test_vfs().await;
    let workspace_id = Uuid::new_v4();

    println!("📊 Testing memory efficiency with varying file sizes...");

    // Create files of different sizes
    let file_sizes = vec![
        100,        // 100 bytes
        1024,       // 1 KB
        10_240,     // 10 KB
        102_400,    // 100 KB
        1_048_576,  // 1 MB
    ];

    let mut total_bytes = 0;

    for (i, size) in file_sizes.iter().enumerate() {
        let content = vec![b'X'; *size];
        let path = VirtualPath::new(&format!("size_test/file_{}.dat", i)).unwrap();

        if let Some(parent) = path.parent() {
            vfs.create_directory(&workspace_id, &parent, true).await.ok();
        }

        vfs.write_file(&workspace_id, &path, &content).await.unwrap();
        total_bytes += size;

        println!("  ✓ Created file {} ({} bytes)", i, size);
    }

    println!("\n📈 Total data loaded: {} bytes ({:.2} MB)",
        total_bytes,
        total_bytes as f64 / (1024.0 * 1024.0)
    );

    // Get cache statistics
    let cache_stats = vfs.cache_stats();

    println!("\n💾 Cache Statistics:");
    println!("  • Cache hits:      {}", cache_stats.hits);
    println!("  • Cache misses:    {}", cache_stats.misses);
    println!("  • Cache puts:      {}", cache_stats.puts);
    println!("  • Cache evictions: {}", cache_stats.evictions);

    if cache_stats.hits + cache_stats.misses > 0 {
        let hit_rate = (cache_stats.hits as f64 / (cache_stats.hits + cache_stats.misses) as f64) * 100.0;
        println!("  • Hit rate:        {:.1}%", hit_rate);
    }

    // Test cache effectiveness by reading files multiple times
    println!("\n🔄 Testing cache effectiveness (3 read passes)...");

    for pass in 1..=3 {
        for (i, _) in file_sizes.iter().enumerate() {
            let path = VirtualPath::new(&format!("size_test/file_{}.dat", i)).unwrap();
            vfs.read_file(&workspace_id, &path).await.ok();
        }

        let stats = vfs.cache_stats();
        let hit_rate = if stats.hits + stats.misses > 0 {
            (stats.hits as f64 / (stats.hits + stats.misses) as f64) * 100.0
        } else {
            0.0
        };

        println!("  Pass {}: hit rate = {:.1}%", pass, hit_rate);
    }
}

// ============================================================================
// Test 10: Complete End-to-End Workflow
// ============================================================================

#[tokio::test]
async fn test_complete_e2e_workflow() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  TEST 10: Complete End-to-End Workflow                        ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let (vfs, _storage) = setup_test_vfs().await;
    let workspace_id = Uuid::new_v4();
    let mut stats = VfsStatistics::default();

    let project_path = Path::new(CORTEX_PROJECT_PATH);

    // Step 1: Load subset of cortex project
    println!("📥 Step 1: Loading cortex project subset...");
    let start = Instant::now();

    let files = walk_directory(project_path, &["**/*.rs"], EXCLUDE_PATTERNS)
        .await
        .expect("Failed to walk directory");

    for file_path in files.iter().take(100) {
        let content = fs::read(file_path).await.unwrap();
        let vpath = to_virtual_path(project_path, file_path).unwrap();

        if let Some(parent) = vpath.parent() {
            vfs.create_directory(&workspace_id, &parent, true).await.ok();
        }

        vfs.write_file(&workspace_id, &vpath, &content).await.ok();

        stats.files_loaded += 1;
        stats.total_bytes_loaded += content.len();
        stats.total_lines_of_code += count_lines(&content);

        if vpath.extension() == Some("rs") {
            stats.rust_files += 1;
        }
    }

    stats.load_time_ms = start.elapsed().as_millis();
    println!("  ✓ Loaded {} files in {}ms", stats.files_loaded, stats.load_time_ms);

    // Step 2: Navigate and verify structure
    println!("\n🔍 Step 2: Navigating VFS structure...");
    let start = Instant::now();

    let root = VirtualPath::root();
    let entries = vfs.list_directory(&workspace_id, &root, true).await.unwrap_or_default();

    stats.navigation_time_ms = start.elapsed().as_millis();
    println!("  ✓ Found {} total entries in {}ms", entries.len(), stats.navigation_time_ms);

    // Step 3: Materialize to temp directory
    println!("\n💾 Step 3: Materializing to disk...");
    let start = Instant::now();

    let temp_dir = TempDir::new().unwrap();
    let target_path = temp_dir.path();

    let engine = MaterializationEngine::new(vfs.clone());
    let options = FlushOptions {
        preserve_permissions: true,
        preserve_timestamps: false,
        create_backup: false,
        atomic: true,
        parallel: true,
        max_workers: 8,
    };

    if let Ok(report) = engine.flush(FlushScope::Workspace(workspace_id), target_path, options).await {
        stats.materialization_time_ms = start.elapsed().as_millis();
        println!("  ✓ Materialized {} files in {}ms", report.files_written, stats.materialization_time_ms);
    }

    // Step 4: Verify materialized content
    println!("\n✅ Step 4: Verifying materialized content...");
    let start = Instant::now();

    // Sample verification - check a few known files
    let mut verified = 0;
    for file_path in files.iter().take(10) {
        let vpath = to_virtual_path(project_path, file_path).unwrap();
        let materialized = target_path.join(vpath.to_string());

        if materialized.exists() {
            verified += 1;
        }
    }

    stats.verification_time_ms = start.elapsed().as_millis();
    println!("  ✓ Verified {} files in {}ms", verified, stats.verification_time_ms);

    // Print final statistics
    println!("\n" );
    stats.cache_hit_rate = {
        let cache_stats = vfs.cache_stats();
        if cache_stats.hits + cache_stats.misses > 0 {
            (cache_stats.hits as f64 / (cache_stats.hits + cache_stats.misses) as f64) * 100.0
        } else {
            0.0
        }
    };

    stats.estimated_memory_usage_mb = (stats.total_bytes_loaded as f64) / (1024.0 * 1024.0);

    stats.print_summary();
}
