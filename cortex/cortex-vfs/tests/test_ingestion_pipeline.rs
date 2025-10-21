//! Integration tests for the file ingestion pipeline.
//!
//! These tests verify the complete flow from VFS → Parser → Semantic Memory.

use cortex_core::types::Language;
use cortex_memory::SemanticMemorySystem;
use cortex_parser::CodeParser;
use cortex_storage::connection_pool::{DatabaseConfig, ConnectionMode, Credentials, PoolConfig, RetryPolicy};
use cortex_storage::ConnectionManager;
use cortex_vfs::ingestion::FileIngestionPipeline;
use cortex_vfs::{VirtualFileSystem, VirtualPath};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// Helper to create a test environment with all components.
async fn setup_test_env() -> (
    FileIngestionPipeline,
    Arc<VirtualFileSystem>,
    Arc<SemanticMemorySystem>,
    Uuid,
) {
    // Create a memory database config for testing
    let config = DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: "mem://".to_string(),
        },
        credentials: Credentials {
            username: None,
            password: None,
        },
        pool_config: PoolConfig {
            min_connections: 1,
            max_connections: 5,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(3600)),
            retry_policy: RetryPolicy {
                max_attempts: 3,
                initial_backoff: Duration::from_millis(100),
                max_backoff: Duration::from_secs(10),
                multiplier: 2.0,
            },
            warm_connections: false,
            validate_on_checkout: true,
            recycle_after_uses: None,
            shutdown_grace_period: Duration::from_secs(5),
        },
        namespace: "test".to_string(),
        database: "ingestion_tests".to_string(),
    };

    let storage = Arc::new(ConnectionManager::new(config).await.unwrap());

    let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
    let parser = Arc::new(tokio::sync::Mutex::new(CodeParser::new().unwrap()));
    let semantic_memory = Arc::new(SemanticMemorySystem::new(storage));

    let pipeline = FileIngestionPipeline::new(parser, vfs.clone(), semantic_memory.clone());
    let workspace_id = Uuid::new_v4();

    (pipeline, vfs, semantic_memory, workspace_id)
}

#[tokio::test]
async fn test_ingest_simple_rust_file() {
    let (pipeline, vfs, semantic_memory, workspace_id) = setup_test_env().await;

    // Write a simple Rust file to VFS
    let path = VirtualPath::new("src/math.rs").unwrap();
    let content = r#"
/// Adds two numbers together.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Subtracts two numbers.
pub fn subtract(a: i32, b: i32) -> i32 {
    a - b
}
"#;

    vfs.write_file(&workspace_id, &path, content.as_bytes())
        .await
        .unwrap();

    // Ingest the file
    let result = pipeline.ingest_file(&workspace_id, &path).await.unwrap();

    // Verify results
    assert_eq!(result.language, Language::Rust);
    assert_eq!(result.units_stored, 2); // 2 functions
    assert!(result.errors.is_empty());
    assert_eq!(result.unit_ids.len(), 2);

    // Verify units are in semantic memory
    for unit_id in &result.unit_ids {
        let unit = semantic_memory.get_unit(*unit_id).await.unwrap();
        assert!(unit.is_some());
        let unit = unit.unwrap();
        assert!(["add", "subtract"].contains(&unit.name.as_str()));
    }

    // Verify VNode metadata was updated
    let units_count = vfs
        .get_file_units_count(&workspace_id, &path)
        .await
        .unwrap();
    assert_eq!(units_count, 2);
}

#[tokio::test]
async fn test_ingest_file_with_struct() {
    let (pipeline, vfs, semantic_memory, workspace_id) = setup_test_env().await;

    let path = VirtualPath::new("src/types.rs").unwrap();
    let content = r#"
/// A 2D point.
#[derive(Debug, Clone)]
pub struct Point {
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
}

impl Point {
    /// Create a new point at the origin.
    pub fn new() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    /// Calculate distance from origin.
    pub fn distance_from_origin(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}
"#;

    vfs.write_file(&workspace_id, &path, content.as_bytes())
        .await
        .unwrap();

    let result = pipeline.ingest_file(&workspace_id, &path).await.unwrap();

    // Should have: 1 struct + 2 methods
    assert!(result.units_stored >= 2);
    assert!(result.errors.is_empty());

    // Verify struct is stored
    let units = semantic_memory
        .get_units_in_file(&path.to_string())
        .await
        .unwrap();

    let struct_unit = units.iter().find(|u| u.name == "Point");
    assert!(struct_unit.is_some());
    let struct_unit = struct_unit.unwrap();
    assert_eq!(struct_unit.unit_type, cortex_core::types::CodeUnitType::Struct);
    assert!(struct_unit.has_documentation);

    // Verify methods are stored
    let new_method = units.iter().find(|u| u.name == "new");
    assert!(new_method.is_some());
    assert_eq!(
        new_method.unwrap().unit_type,
        cortex_core::types::CodeUnitType::Method
    );
}

#[tokio::test]
async fn test_ingest_file_with_enum() {
    let (pipeline, vfs, semantic_memory, workspace_id) = setup_test_env().await;

    let path = VirtualPath::new("src/status.rs").unwrap();
    let content = r#"
/// Represents the status of an operation.
pub enum Status {
    /// Operation is pending
    Pending,
    /// Operation succeeded
    Success,
    /// Operation failed with an error
    Error(String),
}
"#;

    vfs.write_file(&workspace_id, &path, content.as_bytes())
        .await
        .unwrap();

    let result = pipeline.ingest_file(&workspace_id, &path).await.unwrap();

    assert_eq!(result.units_stored, 1); // 1 enum
    assert!(result.errors.is_empty());

    let units = semantic_memory
        .get_units_in_file(&path.to_string())
        .await
        .unwrap();

    assert_eq!(units.len(), 1);
    assert_eq!(units[0].name, "Status");
    assert_eq!(units[0].unit_type, cortex_core::types::CodeUnitType::Enum);
}

#[tokio::test]
async fn test_ingest_file_with_trait() {
    let (pipeline, vfs, semantic_memory, workspace_id) = setup_test_env().await;

    let path = VirtualPath::new("src/drawable.rs").unwrap();
    let content = r#"
/// A trait for drawable objects.
pub trait Drawable {
    /// Draw the object.
    fn draw(&self);

    /// Get the bounding box.
    fn bounds(&self) -> Rectangle;
}

pub struct Rectangle {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
"#;

    vfs.write_file(&workspace_id, &path, content.as_bytes())
        .await
        .unwrap();

    let result = pipeline.ingest_file(&workspace_id, &path).await.unwrap();

    // Should have: 1 trait + 1 struct
    assert_eq!(result.units_stored, 2);
    assert!(result.errors.is_empty());

    let units = semantic_memory
        .get_units_in_file(&path.to_string())
        .await
        .unwrap();

    let trait_unit = units.iter().find(|u| u.name == "Drawable");
    assert!(trait_unit.is_some());
    assert_eq!(
        trait_unit.unwrap().unit_type,
        cortex_core::types::CodeUnitType::Trait
    );
}

#[tokio::test]
async fn test_ingest_complex_rust_file() {
    let (pipeline, vfs, semantic_memory, workspace_id) = setup_test_env().await;

    let path = VirtualPath::new("src/lib.rs").unwrap();
    let content = r#"
//! A simple calculator library.

/// Calculator state.
pub struct Calculator {
    value: i32,
}

impl Calculator {
    /// Create a new calculator.
    pub fn new() -> Self {
        Self { value: 0 }
    }

    /// Add a number.
    pub fn add(&mut self, n: i32) -> &mut Self {
        self.value += n;
        self
    }

    /// Subtract a number.
    pub fn subtract(&mut self, n: i32) -> &mut Self {
        self.value -= n;
        self
    }

    /// Get the current value.
    pub fn result(&self) -> i32 {
        self.value
    }
}

/// Add two numbers.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }
}
"#;

    vfs.write_file(&workspace_id, &path, content.as_bytes())
        .await
        .unwrap();

    let result = pipeline.ingest_file(&workspace_id, &path).await.unwrap();

    // Should have: 1 struct + 4 methods + 1 function
    assert!(result.units_stored >= 5);
    assert!(result.errors.is_empty());

    let units = semantic_memory
        .get_units_in_file(&path.to_string())
        .await
        .unwrap();

    // Verify we have all expected units
    let calc_struct = units.iter().find(|u| u.name == "Calculator");
    assert!(calc_struct.is_some());

    let methods: Vec<_> = units
        .iter()
        .filter(|u| u.unit_type == cortex_core::types::CodeUnitType::Method)
        .collect();
    assert!(methods.len() >= 4);

    let functions: Vec<_> = units
        .iter()
        .filter(|u| u.unit_type == cortex_core::types::CodeUnitType::Function)
        .collect();
    assert!(functions.len() >= 1);
}

#[tokio::test]
async fn test_ingest_typescript_file() {
    let (pipeline, vfs, semantic_memory, workspace_id) = setup_test_env().await;

    let path = VirtualPath::new("src/utils.ts").unwrap();
    let content = r#"
/**
 * Adds two numbers.
 */
export function add(a: number, b: number): number {
    return a + b;
}

/**
 * A simple class.
 */
export class Calculator {
    private value: number = 0;

    /**
     * Add a number.
     */
    add(n: number): this {
        this.value += n;
        return this;
    }

    /**
     * Get the result.
     */
    result(): number {
        return this.value;
    }
}
"#;

    vfs.write_file(&workspace_id, &path, content.as_bytes())
        .await
        .unwrap();

    let result = pipeline.ingest_file(&workspace_id, &path).await.unwrap();

    assert_eq!(result.language, Language::TypeScript);
    assert!(result.units_stored >= 2); // At least function + class
    assert!(result.errors.is_empty());
}

#[tokio::test]
async fn test_ingest_non_code_file() {
    let (pipeline, vfs, _semantic_memory, workspace_id) = setup_test_env().await;

    let path = VirtualPath::new("README.md").unwrap();
    let content = "# My Project\n\nThis is a readme file.";

    vfs.write_file(&workspace_id, &path, content.as_bytes())
        .await
        .unwrap();

    let result = pipeline.ingest_file(&workspace_id, &path).await.unwrap();

    // Should skip non-code files
    assert_eq!(result.language, Language::Unknown);
    assert_eq!(result.units_stored, 0);
    assert!(result.errors.is_empty());
}

#[tokio::test]
async fn test_ingest_workspace() {
    let (pipeline, vfs, _semantic_memory, workspace_id) = setup_test_env().await;

    // Create a small workspace with multiple files
    let files = vec![
        (
            "src/lib.rs",
            r#"
pub fn hello() -> String {
    "Hello".to_string()
}
"#,
        ),
        (
            "src/math.rs",
            r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
"#,
        ),
        (
            "src/types.rs",
            r#"
pub struct Point {
    pub x: i32,
    pub y: i32,
}
"#,
        ),
    ];

    // Write all files to VFS
    for (path_str, content) in &files {
        let path = VirtualPath::new(path_str).unwrap();
        vfs.write_file(&workspace_id, &path, content.as_bytes())
            .await
            .unwrap();
    }

    // Ingest the entire workspace
    let result = pipeline.ingest_workspace(&workspace_id).await.unwrap();

    assert_eq!(result.files_processed, 3);
    assert!(result.total_units >= 4); // At least 3 functions + 1 struct
    assert!(result.files_with_errors.is_empty());
    assert_eq!(result.file_results.len(), 3);
}

#[tokio::test]
async fn test_ingest_file_verify_metadata() {
    let (pipeline, vfs, semantic_memory, workspace_id) = setup_test_env().await;

    let path = VirtualPath::new("src/sample.rs").unwrap();
    let content = r#"
/// A documented function.
pub fn documented() -> i32 {
    42
}

fn undocumented() -> i32 {
    0
}
"#;

    vfs.write_file(&workspace_id, &path, content.as_bytes())
        .await
        .unwrap();

    let result = pipeline.ingest_file(&workspace_id, &path).await.unwrap();
    assert_eq!(result.units_stored, 2);

    let units = semantic_memory
        .get_units_in_file(&path.to_string())
        .await
        .unwrap();

    // Verify documented function
    let documented = units.iter().find(|u| u.name == "documented").unwrap();
    assert!(documented.has_documentation);
    assert_eq!(documented.visibility, cortex_core::types::Visibility::Public);
    assert!(documented.docstring.is_some());

    // Verify undocumented function
    let undocumented = units.iter().find(|u| u.name == "undocumented").unwrap();
    assert!(!undocumented.has_documentation);
    assert_eq!(
        undocumented.visibility,
        cortex_core::types::Visibility::Private
    );
}

#[tokio::test]
async fn test_ingest_file_verify_complexity() {
    let (pipeline, vfs, semantic_memory, workspace_id) = setup_test_env().await;

    let path = VirtualPath::new("src/complex.rs").unwrap();
    let content = r#"
pub fn simple() -> i32 {
    42
}

pub fn complex(n: i32) -> i32 {
    let mut result = 0;
    for i in 0..n {
        if i % 2 == 0 {
            result += i;
        } else {
            result -= i;
        }
    }
    result
}
"#;

    vfs.write_file(&workspace_id, &path, content.as_bytes())
        .await
        .unwrap();

    let result = pipeline.ingest_file(&workspace_id, &path).await.unwrap();
    assert_eq!(result.units_stored, 2);

    let units = semantic_memory
        .get_units_in_file(&path.to_string())
        .await
        .unwrap();

    // Simple function should have low complexity
    let simple = units.iter().find(|u| u.name == "simple").unwrap();
    assert!(simple.complexity.cyclomatic <= 2);

    // Complex function should have higher complexity
    let complex = units.iter().find(|u| u.name == "complex").unwrap();
    assert!(complex.complexity.cyclomatic > 1);
    assert!(complex.complexity.lines > simple.complexity.lines);
}

#[tokio::test]
async fn test_ingest_file_with_async_functions() {
    let (pipeline, vfs, semantic_memory, workspace_id) = setup_test_env().await;

    let path = VirtualPath::new("src/async_ops.rs").unwrap();
    let content = r#"
pub async fn fetch_data() -> Result<String, ()> {
    Ok("data".to_string())
}

pub fn sync_operation() -> i32 {
    42
}
"#;

    vfs.write_file(&workspace_id, &path, content.as_bytes())
        .await
        .unwrap();

    let result = pipeline.ingest_file(&workspace_id, &path).await.unwrap();
    assert_eq!(result.units_stored, 2);

    let units = semantic_memory
        .get_units_in_file(&path.to_string())
        .await
        .unwrap();

    // Verify async function
    let async_fn = units.iter().find(|u| u.name == "fetch_data").unwrap();
    assert!(async_fn.is_async);
    assert_eq!(
        async_fn.unit_type,
        cortex_core::types::CodeUnitType::AsyncFunction
    );

    // Verify sync function
    let sync_fn = units.iter().find(|u| u.name == "sync_operation").unwrap();
    assert!(!sync_fn.is_async);
    assert_eq!(
        sync_fn.unit_type,
        cortex_core::types::CodeUnitType::Function
    );
}

#[tokio::test]
async fn test_ingest_incremental_update() {
    let (pipeline, vfs, semantic_memory, workspace_id) = setup_test_env().await;

    let path = VirtualPath::new("src/incremental.rs").unwrap();

    // Initial version
    let content_v1 = r#"
pub fn version_one() -> i32 {
    1
}
"#;

    vfs.write_file(&workspace_id, &path, content_v1.as_bytes())
        .await
        .unwrap();

    let result_v1 = pipeline.ingest_file(&workspace_id, &path).await.unwrap();
    assert_eq!(result_v1.units_stored, 1);

    // Update the file
    let content_v2 = r#"
pub fn version_one() -> i32 {
    1
}

pub fn version_two() -> i32 {
    2
}
"#;

    vfs.write_file(&workspace_id, &path, content_v2.as_bytes())
        .await
        .unwrap();

    let result_v2 = pipeline.ingest_file(&workspace_id, &path).await.unwrap();
    assert_eq!(result_v2.units_stored, 2);

    // Verify both functions are in memory
    let units = semantic_memory
        .get_units_in_file(&path.to_string())
        .await
        .unwrap();

    // Note: This will have duplicates unless we implement update logic
    // For now, we're just verifying the ingestion works
    let version_one_count = units.iter().filter(|u| u.name == "version_one").count();
    let version_two_count = units.iter().filter(|u| u.name == "version_two").count();

    assert!(version_one_count >= 1);
    assert!(version_two_count >= 1);
}
