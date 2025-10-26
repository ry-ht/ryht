//! E2E Test Phase 1: Project Ingestion and VFS Verification
//!
//! This test validates the entire Cortex system by loading itself into VFS.
//! It serves as the ultimate integration test, verifying:
//! - VFS operations (read, write, metadata)
//! - Parser functionality (extract code units from real code)
//! - Semantic memory storage (store and query code units)
//! - External project loading (import from physical filesystem)
//! - Content verification (ensure accuracy and completeness)
//!
//! Test Strategy:
//! 1. Load entire Cortex project (all 9 crates, ~300 .rs files)
//! 2. Verify VFS correctness (all files accessible, content matches)
//! 3. Parse and ingest all Rust code (extract thousands of code units)
//! 4. Verify semantic memory storage (query and validate)
//! 5. Test specific critical files in detail
//! 6. Performance benchmarking (ingestion speed, parse rate)
//!
//! This is a PRODUCTION-GRADE test that validates the entire system.

use cortex_core::{
    error::{CortexError, Result},
    types::CodeUnitType,
};
use cortex_ingestion::prelude::*;
use cortex_memory::{SemanticMemorySystem, prelude::MemoryQuery};
use cortex_code_analysis::CodeParser;
use cortex_storage::connection_pool::{
    ConnectionManager, ConnectionMode, Credentials, DatabaseConfig, PoolConfig,
};
use cortex_vfs::{VirtualFileSystem, VirtualPath, FileIngestionPipeline};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, warn};

// ============================================================================
// Test Configuration
// ============================================================================

const CORTEX_PROJECT_PATH: &str = "/Users/taaliman/projects/luxquant/ry-ht/ryht/cortex";

const EXPECTED_CRATES: &[&str] = &[
    "cortex-core",
    "cortex-storage",
    "cortex-vfs",
    "cortex-memory",
    "cortex-semantic",
    "cortex-ingestion",
    "cortex-code-analysis",
    "cortex-mcp",
    "cortex",
];

// Conservative estimates based on current codebase
const MIN_EXPECTED_RS_FILES: usize = 80;
const MIN_EXPECTED_TOTAL_FILES: usize = 100;
const MIN_EXPECTED_FUNCTIONS: usize = 300;
const MIN_EXPECTED_STRUCTS: usize = 100;
const MIN_EXPECTED_ENUMS: usize = 30;
const MIN_EXPECTED_TRAITS: usize = 20;
const MIN_EXPECTED_TOTAL_UNITS: usize = 1000;

// Performance targets
const MAX_INGESTION_TIME_SECS: u64 = 60;
const MIN_FILES_PER_SECOND: f64 = 2.0;

// ============================================================================
// Test Metrics Tracker
// ============================================================================

#[derive(Debug, Default)]
struct TestMetrics {
    // File statistics
    total_files_loaded: usize,
    rust_files_loaded: usize,
    total_bytes_processed: u64,

    // Code unit statistics
    functions_extracted: usize,
    structs_extracted: usize,
    enums_extracted: usize,
    traits_extracted: usize,
    methods_extracted: usize,
    impls_extracted: usize,
    total_units_extracted: usize,

    // Directory structure
    directories_created: usize,
    crates_discovered: HashMap<String, CrateStats>,

    // Performance metrics
    load_duration_ms: u64,
    parse_duration_ms: u64,
    storage_duration_ms: u64,
    total_duration_ms: u64,

    // Error tracking
    parse_errors: Vec<String>,
    storage_errors: Vec<String>,
    verification_errors: Vec<String>,
}

#[derive(Debug, Default, Clone)]
struct CrateStats {
    name: String,
    file_count: usize,
    unit_count: usize,
    line_count: usize,
}

impl TestMetrics {
    fn new() -> Self {
        Self::default()
    }

    fn files_per_second(&self) -> f64 {
        if self.total_duration_ms == 0 {
            return 0.0;
        }
        (self.rust_files_loaded as f64) / (self.total_duration_ms as f64 / 1000.0)
    }

    fn units_per_second(&self) -> f64 {
        if self.total_duration_ms == 0 {
            return 0.0;
        }
        (self.total_units_extracted as f64) / (self.total_duration_ms as f64 / 1000.0)
    }

    fn mb_per_second(&self) -> f64 {
        if self.total_duration_ms == 0 {
            return 0.0;
        }
        let mb = self.total_bytes_processed as f64 / 1_048_576.0;
        mb / (self.total_duration_ms as f64 / 1000.0)
    }

    fn print_report(&self) {
        println!("\n{}", "=".repeat(80));
        println!("           E2E PHASE 1: PROJECT INGESTION - TEST REPORT");
        println!("{}", "=".repeat(80));

        println!("\n📁 FILE STATISTICS");
        println!("   Total Files Loaded:     {}", self.total_files_loaded);
        println!("   Rust Files Loaded:      {}", self.rust_files_loaded);
        println!("   Directories Created:    {}", self.directories_created);
        println!("   Total Bytes Processed:  {} MB", self.total_bytes_processed / 1_048_576);

        println!("\n📊 CODE UNIT EXTRACTION");
        println!("   Functions:              {}", self.functions_extracted);
        println!("   Methods:                {}", self.methods_extracted);
        println!("   Structs:                {}", self.structs_extracted);
        println!("   Enums:                  {}", self.enums_extracted);
        println!("   Traits:                 {}", self.traits_extracted);
        println!("   Impl Blocks:            {}", self.impls_extracted);
        println!("   ────────────────────────────────");
        println!("   TOTAL UNITS:            {}", self.total_units_extracted);

        println!("\n📦 CRATE BREAKDOWN");
        let mut crates: Vec<_> = self.crates_discovered.values().collect();
        crates.sort_by(|a, b| b.unit_count.cmp(&a.unit_count));
        for crate_stats in crates {
            println!(
                "   {:20} - {} files, {} units",
                crate_stats.name, crate_stats.file_count, crate_stats.unit_count
            );
        }

        println!("\n⏱️  PERFORMANCE METRICS");
        println!("   Load Time:              {}ms", self.load_duration_ms);
        println!("   Parse Time:             {}ms", self.parse_duration_ms);
        println!("   Storage Time:           {}ms", self.storage_duration_ms);
        println!("   Total Time:             {}ms ({:.2}s)",
            self.total_duration_ms,
            self.total_duration_ms as f64 / 1000.0
        );
        println!("   Files/Second:           {:.2}", self.files_per_second());
        println!("   Units/Second:           {:.2}", self.units_per_second());
        println!("   MB/Second:              {:.2}", self.mb_per_second());

        if !self.parse_errors.is_empty() {
            println!("\n⚠️  PARSE ERRORS ({})", self.parse_errors.len());
            for (i, err) in self.parse_errors.iter().take(5).enumerate() {
                println!("   {}. {}", i + 1, err);
            }
            if self.parse_errors.len() > 5 {
                println!("   ... and {} more", self.parse_errors.len() - 5);
            }
        }

        if !self.storage_errors.is_empty() {
            println!("\n⚠️  STORAGE ERRORS ({})", self.storage_errors.len());
            for (i, err) in self.storage_errors.iter().take(5).enumerate() {
                println!("   {}. {}", i + 1, err);
            }
            if self.storage_errors.len() > 5 {
                println!("   ... and {} more", self.storage_errors.len() - 5);
            }
        }

        if !self.verification_errors.is_empty() {
            println!("\n❌ VERIFICATION ERRORS ({})", self.verification_errors.len());
            for (i, err) in self.verification_errors.iter().enumerate() {
                println!("   {}. {}", i + 1, err);
            }
        }

        println!("\n{}", "=".repeat(80));

        // Determine pass/fail
        let passed = self.verification_errors.is_empty()
            && self.rust_files_loaded >= MIN_EXPECTED_RS_FILES
            && self.total_units_extracted >= MIN_EXPECTED_TOTAL_UNITS;

        if passed {
            println!("✅ TEST PASSED - All verification checks succeeded!");
        } else {
            println!("❌ TEST FAILED - See errors above");
        }
        println!("{}\n", "=".repeat(80));
    }
}

// ============================================================================
// Test Infrastructure
// ============================================================================

struct TestContext {
    workspace_id: uuid::Uuid,
    vfs: Arc<VirtualFileSystem>,
    parser: Arc<tokio::sync::Mutex<CodeParser>>,
    semantic_memory: Arc<SemanticMemorySystem>,
    ingestion_pipeline: Arc<FileIngestionPipeline>,
    metrics: TestMetrics,
}

impl TestContext {
    async fn new() -> Result<Self> {
        // Create in-memory database for speed (no server required!)
        let db_config = DatabaseConfig {
            connection_mode: ConnectionMode::InMemory,
            credentials: Credentials::default(),
            pool_config: PoolConfig::default(),
            namespace: "cortex_e2e_phase1".to_string(),
            database: format!("test_{}", uuid::Uuid::new_v4().simple()),
        };

        let connection_manager = Arc::new(ConnectionManager::new(db_config).await?);

        // Initialize components
        let vfs = Arc::new(VirtualFileSystem::new(connection_manager.clone()));
        let parser = Arc::new(tokio::sync::Mutex::new(CodeParser::new()?));
        let semantic_memory = Arc::new(SemanticMemorySystem::new(connection_manager));

        let ingestion_pipeline = Arc::new(FileIngestionPipeline::new(
            parser.clone(),
            vfs.clone(),
            semantic_memory.clone(),
        ));

        let workspace_id = uuid::Uuid::new_v4();

        info!("Test context created with workspace_id: {}", workspace_id);

        Ok(Self {
            workspace_id,
            vfs,
            parser,
            semantic_memory,
            ingestion_pipeline,
            metrics: TestMetrics::new(),
        })
    }
}

// ============================================================================
// Phase 1: Load Entire Cortex Project into VFS
// ============================================================================

async fn phase1_load_project(ctx: &mut TestContext) -> Result<()> {
    info!("=== PHASE 1: Load Entire Cortex Project into VFS ===");
    let phase_start = Instant::now();

    let cortex_path = Path::new(CORTEX_PROJECT_PATH);
    if !cortex_path.exists() {
        return Err(CortexError::invalid_input(format!(
            "Cortex project path does not exist: {}",
            CORTEX_PROJECT_PATH
        )));
    }

    info!("Loading project from: {}", CORTEX_PROJECT_PATH);

    // Use ProjectLoader to import the entire project
    let loader = ProjectLoader::new();
    let options = ProjectImportOptions {
        respect_gitignore: true,
        follow_links: false,
        process_code: true,
        generate_embeddings: false,
        ..Default::default()
    };

    let (imported_files, import_report) = loader
        .import_project(cortex_path, options)
        .await?;

    info!("Import report: {} files imported, {} skipped, {} errors",
        import_report.files_imported,
        import_report.files_skipped,
        import_report.errors
    );

    // Now write all imported files to VFS
    for imported_file in &imported_files {
        // Read the physical file
        let physical_path = cortex_path.join(&imported_file.relative_path);
        let content = tokio::fs::read(&physical_path).await.map_err(|e| {
            CortexError::InvalidInput(format!("Failed to read {}: {}", physical_path.display(), e))
        })?;

        // Create VirtualPath
        let vpath = VirtualPath::new(&imported_file.relative_path)
            .map_err(|e| CortexError::invalid_input(format!("Invalid path {}: {}", imported_file.relative_path, e)))?;

        // Ensure parent directory exists
        if let Some(parent) = vpath.parent() {
            let _ = ctx.vfs.create_directory(&ctx.workspace_id, &parent, true).await;
            ctx.metrics.directories_created += 1;
        }

        // Write to VFS
        ctx.vfs.write_file(&ctx.workspace_id, &vpath, &content).await?;

        ctx.metrics.total_files_loaded += 1;
        ctx.metrics.total_bytes_processed += content.len() as u64;

        if imported_file.relative_path.ends_with(".rs") {
            ctx.metrics.rust_files_loaded += 1;

            // Track crate statistics
            if let Some(crate_name) = extract_crate_name(&imported_file.relative_path) {
                let crate_stats = ctx.metrics.crates_discovered
                    .entry(crate_name.to_string())
                    .or_insert_with(|| CrateStats {
                        name: crate_name.to_string(),
                        ..Default::default()
                    });
                crate_stats.file_count += 1;
            }
        }

        if ctx.metrics.total_files_loaded % 50 == 0 {
            debug!("Loaded {} files...", ctx.metrics.total_files_loaded);
        }
    }

    ctx.metrics.load_duration_ms = phase_start.elapsed().as_millis() as u64;

    info!(
        "✅ Phase 1 complete: Loaded {} files ({} Rust files) in {}ms",
        ctx.metrics.total_files_loaded,
        ctx.metrics.rust_files_loaded,
        ctx.metrics.load_duration_ms
    );

    Ok(())
}

// ============================================================================
// Phase 2: Verify VFS Correctness
// ============================================================================

async fn phase2_verify_vfs(ctx: &mut TestContext) -> Result<()> {
    info!("=== PHASE 2: Verify VFS Correctness ===");
    let phase_start = Instant::now();

    let cortex_path = Path::new(CORTEX_PROJECT_PATH);

    // Sample verification: Check a few critical files
    let critical_files = vec![
        "cortex-core/src/types.rs",
        "cortex-vfs/src/virtual_filesystem.rs",
        "cortex-code-analysis/src/rust_parser.rs",
        "cortex-memory/src/lib.rs",
        "Cargo.toml",
    ];

    for file_path in &critical_files {
        let vpath = VirtualPath::new(file_path)
            .map_err(|e| CortexError::invalid_input(format!("Invalid path {}: {}", file_path, e)))?;

        // Read from VFS
        let vfs_content = match ctx.vfs.read_file(&ctx.workspace_id, &vpath).await {
            Ok(content) => content,
            Err(e) => {
                ctx.metrics.verification_errors.push(
                    format!("Failed to read {} from VFS: {}", file_path, e)
                );
                continue;
            }
        };

        // Read from physical filesystem
        let physical_path = cortex_path.join(file_path);
        let physical_content = match tokio::fs::read(&physical_path).await {
            Ok(content) => content,
            Err(e) => {
                warn!("Could not read physical file {} (may not exist): {}", file_path, e);
                continue;
            }
        };

        // Compare byte-for-byte
        if vfs_content != physical_content {
            ctx.metrics.verification_errors.push(
                format!(
                    "Content mismatch for {}: VFS has {} bytes, physical has {} bytes",
                    file_path,
                    vfs_content.len(),
                    physical_content.len()
                )
            );
        } else {
            info!("✅ Content verified: {} ({} bytes)", file_path, vfs_content.len());
        }
    }

    // Verify directory structure
    for crate_name in EXPECTED_CRATES {
        let crate_path = VirtualPath::new(crate_name)
            .map_err(|e| CortexError::invalid_input(format!("Invalid crate path {}: {}", crate_name, e)))?;

        // Check if directory exists by trying to list it
        match ctx.vfs.list_directory(&ctx.workspace_id, &crate_path, false).await {
            Ok(_) => {
                debug!("✅ Crate directory exists: {}", crate_name);
            }
            Err(_) => {
                ctx.metrics.verification_errors.push(
                    format!("Missing crate directory: {}", crate_name)
                );
            }
        }
    }

    let duration = phase_start.elapsed().as_millis();
    info!("✅ Phase 2 complete: VFS verification in {}ms", duration);

    Ok(())
}

// ============================================================================
// Phase 3: Parse and Ingest All Rust Code
// ============================================================================

async fn phase3_parse_and_ingest(ctx: &mut TestContext) -> Result<()> {
    info!("=== PHASE 3: Parse and Ingest All Rust Code ===");
    let phase_start = Instant::now();

    // Get all files from VFS
    let root = VirtualPath::root();
    let all_nodes = ctx.vfs.list_directory(&ctx.workspace_id, &root, true).await?;

    // Filter to .rs files only
    let rust_files: Vec<_> = all_nodes
        .into_iter()
        .filter(|node| {
            node.is_file() && node.path.to_string().ends_with(".rs")
        })
        .collect();

    info!("Found {} Rust files to process", rust_files.len());

    let mut files_processed = 0;

    for vnode in &rust_files {
        // Ingest file through pipeline
        match ctx.ingestion_pipeline.ingest_file(&ctx.workspace_id, &vnode.path).await {
            Ok(result) => {
                // Track extracted units by type
                // Note: The ingestion pipeline doesn't provide detailed breakdown,
                // so we'll query semantic memory after ingestion
                ctx.metrics.total_units_extracted += result.units_stored;

                // Track crate statistics
                if let Some(crate_name) = extract_crate_name(&result.file_path) {
                    if let Some(crate_stats) = ctx.metrics.crates_discovered.get_mut(crate_name) {
                        crate_stats.unit_count += result.units_stored;
                    }
                }

                if !result.errors.is_empty() {
                    for error in result.errors {
                        ctx.metrics.parse_errors.push(
                            format!("{}: {}", result.file_path, error)
                        );
                    }
                }
            }
            Err(e) => {
                warn!("Failed to ingest {}: {}", vnode.path, e);
                ctx.metrics.parse_errors.push(
                    format!("{}: {}", vnode.path, e)
                );
            }
        }

        files_processed += 1;
        if files_processed % 20 == 0 {
            debug!("Processed {}/{} files...", files_processed, rust_files.len());
        }
    }

    ctx.metrics.parse_duration_ms = phase_start.elapsed().as_millis() as u64;

    info!(
        "✅ Phase 3 complete: Parsed and ingested {} files, extracted {} code units in {}ms",
        files_processed,
        ctx.metrics.total_units_extracted,
        ctx.metrics.parse_duration_ms
    );

    Ok(())
}

// ============================================================================
// Phase 4: Query and Verify Semantic Memory
// ============================================================================

async fn phase4_verify_semantic_memory(ctx: &mut TestContext) -> Result<()> {
    info!("=== PHASE 4: Query and Verify Semantic Memory ===");
    let phase_start = Instant::now();

    // Get statistics from semantic memory
    let stats = ctx.semantic_memory.get_statistics().await?;
    info!("Semantic memory statistics: {:?}", stats);

    // Use search_units to get all units
    // Empty query with empty embedding should return all units
    let query = MemoryQuery::new(String::new());
    let all_units = ctx.semantic_memory.search_units(&query, &vec![]).await?;
    let all_unit_items: Vec<_> = all_units.into_iter().map(|r| r.item).collect();

    info!("Total units in semantic memory: {}", all_unit_items.len());

    // Count by type
    for unit in &all_unit_items {
        match unit.unit_type {
            CodeUnitType::Function | CodeUnitType::AsyncFunction => {
                ctx.metrics.functions_extracted += 1;
            }
            CodeUnitType::Method => {
                ctx.metrics.methods_extracted += 1;
            }
            CodeUnitType::Struct => {
                ctx.metrics.structs_extracted += 1;
            }
            CodeUnitType::Enum => {
                ctx.metrics.enums_extracted += 1;
            }
            CodeUnitType::Trait => {
                ctx.metrics.traits_extracted += 1;
            }
            CodeUnitType::ImplBlock => {
                ctx.metrics.impls_extracted += 1;
            }
            _ => {}
        }
    }

    // Verify we can query by file path
    let test_file = "cortex-core/src/types.rs";
    let units_in_file = ctx.semantic_memory.get_units_in_file(test_file).await?;
    info!("Units in {}: {}", test_file, units_in_file.len());

    if units_in_file.is_empty() {
        ctx.metrics.verification_errors.push(
            format!("Expected code units in {}, found none", test_file)
        );
    }

    // Verify we can retrieve individual units
    if let Some(unit) = all_unit_items.first() {
        match ctx.semantic_memory.get_unit(unit.id).await? {
            Some(_) => {
                debug!("✅ Successfully retrieved unit by ID");
            }
            None => {
                ctx.metrics.verification_errors.push(
                    "Failed to retrieve unit by ID".to_string()
                );
            }
        }
    }

    ctx.metrics.storage_duration_ms = phase_start.elapsed().as_millis() as u64;

    info!(
        "✅ Phase 4 complete: Verified semantic memory in {}ms",
        ctx.metrics.storage_duration_ms
    );

    Ok(())
}

// ============================================================================
// Phase 5: Verify Specific Critical Files
// ============================================================================

async fn phase5_verify_critical_files(ctx: &mut TestContext) -> Result<()> {
    info!("=== PHASE 5: Verify Critical Files in Detail ===");
    let phase_start = Instant::now();

    // Test: cortex-core/src/types.rs should have CodeUnit, CodeUnitType, etc.
    verify_file_has_types(
        ctx,
        "cortex-core/src/types.rs",
        &["CodeUnit", "CodeUnitType", "Language", "Visibility"],
    ).await?;

    // Test: cortex-vfs/src/virtual_filesystem.rs should have VirtualFileSystem
    verify_file_has_types(
        ctx,
        "cortex-vfs/src/virtual_filesystem.rs",
        &["VirtualFileSystem"],
    ).await?;

    // Test: cortex-code-analysis/src/rust_parser.rs should have RustParser
    verify_file_has_types(
        ctx,
        "cortex-code-analysis/src/rust_parser.rs",
        &["RustParser"],
    ).await?;

    let duration = phase_start.elapsed().as_millis();
    info!("✅ Phase 5 complete: Verified critical files in {}ms", duration);

    Ok(())
}

async fn verify_file_has_types(
    ctx: &mut TestContext,
    file_path: &str,
    expected_types: &[&str],
) -> Result<()> {
    let units = ctx.semantic_memory.get_units_in_file(file_path).await?;

    for expected_type in expected_types {
        let found = units.iter().any(|u| u.name == *expected_type);
        if found {
            debug!("✅ Found {} in {}", expected_type, file_path);
        } else {
            ctx.metrics.verification_errors.push(
                format!("Expected to find {} in {}, but not found", expected_type, file_path)
            );
        }
    }

    Ok(())
}

// ============================================================================
// Phase 6: Performance Verification
// ============================================================================

fn phase6_verify_performance(ctx: &mut TestContext) {
    info!("=== PHASE 6: Performance Verification ===");

    // Check ingestion time
    let total_secs = ctx.metrics.total_duration_ms / 1000;
    if total_secs > MAX_INGESTION_TIME_SECS {
        ctx.metrics.verification_errors.push(
            format!(
                "Ingestion took {}s, expected < {}s",
                total_secs,
                MAX_INGESTION_TIME_SECS
            )
        );
    } else {
        info!("✅ Ingestion time: {}s (target: <{}s)", total_secs, MAX_INGESTION_TIME_SECS);
    }

    // Check file processing rate
    let files_per_sec = ctx.metrics.files_per_second();
    if files_per_sec < MIN_FILES_PER_SECOND {
        ctx.metrics.verification_errors.push(
            format!(
                "Processing rate {:.2} files/sec, expected >= {:.2}",
                files_per_sec,
                MIN_FILES_PER_SECOND
            )
        );
    } else {
        info!("✅ Processing rate: {:.2} files/sec", files_per_sec);
    }

    info!("✅ Phase 6 complete: Performance verification");
}

// ============================================================================
// Phase 7: Final Validation
// ============================================================================

fn phase7_final_validation(ctx: &mut TestContext) {
    info!("=== PHASE 7: Final Validation ===");

    // Verify minimum file counts
    if ctx.metrics.rust_files_loaded < MIN_EXPECTED_RS_FILES {
        ctx.metrics.verification_errors.push(
            format!(
                "Expected at least {} Rust files, found {}",
                MIN_EXPECTED_RS_FILES,
                ctx.metrics.rust_files_loaded
            )
        );
    }

    if ctx.metrics.total_files_loaded < MIN_EXPECTED_TOTAL_FILES {
        ctx.metrics.verification_errors.push(
            format!(
                "Expected at least {} total files, found {}",
                MIN_EXPECTED_TOTAL_FILES,
                ctx.metrics.total_files_loaded
            )
        );
    }

    // Verify minimum code unit counts
    if ctx.metrics.total_units_extracted < MIN_EXPECTED_TOTAL_UNITS {
        ctx.metrics.verification_errors.push(
            format!(
                "Expected at least {} total code units, found {}",
                MIN_EXPECTED_TOTAL_UNITS,
                ctx.metrics.total_units_extracted
            )
        );
    }

    if ctx.metrics.functions_extracted < MIN_EXPECTED_FUNCTIONS {
        ctx.metrics.verification_errors.push(
            format!(
                "Expected at least {} functions, found {}",
                MIN_EXPECTED_FUNCTIONS,
                ctx.metrics.functions_extracted
            )
        );
    }

    if ctx.metrics.structs_extracted < MIN_EXPECTED_STRUCTS {
        ctx.metrics.verification_errors.push(
            format!(
                "Expected at least {} structs, found {}",
                MIN_EXPECTED_STRUCTS,
                ctx.metrics.structs_extracted
            )
        );
    }

    if ctx.metrics.enums_extracted < MIN_EXPECTED_ENUMS {
        ctx.metrics.verification_errors.push(
            format!(
                "Expected at least {} enums, found {}",
                MIN_EXPECTED_ENUMS,
                ctx.metrics.enums_extracted
            )
        );
    }

    if ctx.metrics.traits_extracted < MIN_EXPECTED_TRAITS {
        ctx.metrics.verification_errors.push(
            format!(
                "Expected at least {} traits, found {}",
                MIN_EXPECTED_TRAITS,
                ctx.metrics.traits_extracted
            )
        );
    }

    // Verify all expected crates were found
    for expected_crate in EXPECTED_CRATES {
        if !ctx.metrics.crates_discovered.contains_key(*expected_crate) {
            ctx.metrics.verification_errors.push(
                format!("Expected crate not found: {}", expected_crate)
            );
        }
    }

    info!("✅ Phase 7 complete: Final validation");
}

// ============================================================================
// Helper Functions
// ============================================================================

fn extract_crate_name(path: &str) -> Option<&str> {
    // Extract crate name from path like "cortex-core/src/lib.rs"
    if let Some(first_component) = path.split('/').next() {
        if first_component.starts_with("cortex-") {
            return Some(first_component);
        }
    }
    None
}

// ============================================================================
// Main Test
// ============================================================================

#[tokio::test]
#[ignore = "Requires manual SurrealDB setup - see docs/testing.md"]
async fn test_e2e_phase1_cortex_self_ingestion() -> Result<()> {
    // NOTE: This test requires a running SurrealDB instance.
    // The InMemory mode is not yet fully functional due to SurrealDB Any engine limitations.
    //
    // To run this test:
    // 1. Install SurrealDB: https://surrealdb.com/install
    // 2. Start server: surreal start --bind 0.0.0.0:8000 memory
    // 3. Update DatabaseConfig to use: ConnectionMode::Local { endpoint: "ws://localhost:8000" }
    // 4. Run: cargo test test_e2e_phase1_cortex_self_ingestion -- --ignored
    //
    // Initialize tracing (optional, commented out to avoid dependency issues)
    // tracing_subscriber::fmt::init();

    info!("\n{}", "=".repeat(80));
    info!("    E2E PHASE 1: CORTEX SELF-INGESTION TEST");
    info!("{}", "=".repeat(80));

    let test_start = Instant::now();

    // Initialize test context
    let mut ctx = TestContext::new().await?;

    // Execute all phases
    phase1_load_project(&mut ctx).await?;
    phase2_verify_vfs(&mut ctx).await?;
    phase3_parse_and_ingest(&mut ctx).await?;
    phase4_verify_semantic_memory(&mut ctx).await?;
    phase5_verify_critical_files(&mut ctx).await?;

    ctx.metrics.total_duration_ms = test_start.elapsed().as_millis() as u64;

    phase6_verify_performance(&mut ctx);
    phase7_final_validation(&mut ctx);

    // Print comprehensive report
    ctx.metrics.print_report();

    // Assert no verification errors
    assert!(
        ctx.metrics.verification_errors.is_empty(),
        "Test failed with {} verification errors:\n{}",
        ctx.metrics.verification_errors.len(),
        ctx.metrics.verification_errors.join("\n")
    );

    // Assert minimum expectations
    assert!(
        ctx.metrics.rust_files_loaded >= MIN_EXPECTED_RS_FILES,
        "Expected at least {} Rust files, found {}",
        MIN_EXPECTED_RS_FILES,
        ctx.metrics.rust_files_loaded
    );

    assert!(
        ctx.metrics.total_units_extracted >= MIN_EXPECTED_TOTAL_UNITS,
        "Expected at least {} code units, found {}",
        MIN_EXPECTED_TOTAL_UNITS,
        ctx.metrics.total_units_extracted
    );

    assert!(
        ctx.metrics.functions_extracted >= MIN_EXPECTED_FUNCTIONS,
        "Expected at least {} functions, found {}",
        MIN_EXPECTED_FUNCTIONS,
        ctx.metrics.functions_extracted
    );

    // Performance assertion
    assert!(
        ctx.metrics.total_duration_ms / 1000 <= MAX_INGESTION_TIME_SECS,
        "Ingestion took {}s, expected <= {}s",
        ctx.metrics.total_duration_ms / 1000,
        MAX_INGESTION_TIME_SECS
    );

    info!("\n✅ ALL TESTS PASSED!");
    info!("{}\n", "=".repeat(80));

    Ok(())
}
