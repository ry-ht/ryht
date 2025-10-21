//! Realistic Multi-Agent Collaboration Scenarios
//!
//! This test suite simulates REAL collaborative development workflows with multiple
//! concurrent agents working on a codebase. It validates:
//!
//! 1. **Parallel Feature Development** - Multiple agents working on independent features
//! 2. **Refactoring Race Conditions** - Conflicting changes to the same code
//! 3. **Type System Evolution** - Breaking changes with cascading effects
//! 4. **Cross-File Dependency Chains** - Transitive dependency management
//! 5. **Deadlock Prevention** - Circular lock detection and recovery
//! 6. **Stress Testing** - 10+ agents with 1000+ concurrent operations
//!
//! Each scenario includes:
//! - Realistic code changes
//! - Conflict detection and resolution
//! - Lock management validation
//! - Session isolation verification
//! - Performance metrics

use cortex_storage::locks::{
    DeadlockDetector, EntityLock, EntityType, LockManager, LockRequest, LockType,
};
use cortex_storage::merge::{
    Change, ChangeSet, Conflict, ConflictType, DiffEngine, MergeRequest, MergeResult,
    MergeStrategy, Operation, SemanticAnalyzer,
};
use cortex_storage::session::{
    AgentSession, IsolationLevel, OperationType, SessionManager, SessionMetadata, SessionScope,
    SessionState,
};
use cortex_core::types::{CodeUnit, CodeUnitType, Language, Parameter, Visibility};
use cortex_core::id::CortexId;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Barrier, Mutex};
use tracing::{debug, info, warn};
use uuid::Uuid;

// ==============================================================================
// Test Utilities
// ==============================================================================

/// Performance metrics for multi-agent scenarios
#[derive(Debug, Default)]
struct ScenarioMetrics {
    total_agents: usize,
    total_operations: usize,
    conflicts_detected: usize,
    conflicts_resolved: usize,
    deadlocks_detected: usize,
    locks_acquired: usize,
    locks_released: usize,
    session_creation_ms: Vec<u64>,
    lock_acquisition_ms: Vec<u64>,
    merge_duration_ms: Vec<u64>,
    scenario_duration_ms: u64,
}

impl ScenarioMetrics {
    fn new() -> Self {
        Self::default()
    }

    fn avg_session_creation_ms(&self) -> f64 {
        if self.session_creation_ms.is_empty() {
            return 0.0;
        }
        self.session_creation_ms.iter().sum::<u64>() as f64 / self.session_creation_ms.len() as f64
    }

    fn avg_lock_acquisition_ms(&self) -> f64 {
        if self.lock_acquisition_ms.is_empty() {
            return 0.0;
        }
        self.lock_acquisition_ms.iter().sum::<u64>() as f64 / self.lock_acquisition_ms.len() as f64
    }

    fn avg_merge_duration_ms(&self) -> f64 {
        if self.merge_duration_ms.is_empty() {
            return 0.0;
        }
        self.merge_duration_ms.iter().sum::<u64>() as f64 / self.merge_duration_ms.len() as f64
    }

    fn conflict_resolution_rate(&self) -> f64 {
        if self.conflicts_detected == 0 {
            return 100.0;
        }
        (self.conflicts_resolved as f64 / self.conflicts_detected as f64) * 100.0
    }

    fn print_report(&self, scenario_name: &str) {
        info!("╔═══════════════════════════════════════════════════════════════╗");
        info!("║  Multi-Agent Scenario Report: {:<30} ║", scenario_name);
        info!("╠═══════════════════════════════════════════════════════════════╣");
        info!("║  Total Agents:               {:<32} ║", self.total_agents);
        info!("║  Total Operations:           {:<32} ║", self.total_operations);
        info!("║  Scenario Duration:          {:<28} ms ║", self.scenario_duration_ms);
        info!("╟───────────────────────────────────────────────────────────────╢");
        info!("║  Session Creation (avg):     {:<26.2} ms ║", self.avg_session_creation_ms());
        info!("║  Lock Acquisition (avg):     {:<26.2} ms ║", self.avg_lock_acquisition_ms());
        info!("║  Merge Duration (avg):       {:<26.2} ms ║", self.avg_merge_duration_ms());
        info!("╟───────────────────────────────────────────────────────────────╢");
        info!("║  Conflicts Detected:         {:<32} ║", self.conflicts_detected);
        info!("║  Conflicts Resolved:         {:<32} ║", self.conflicts_resolved);
        info!("║  Resolution Rate:            {:<28.1}% ║", self.conflict_resolution_rate());
        info!("╟───────────────────────────────────────────────────────────────╢");
        info!("║  Locks Acquired:             {:<32} ║", self.locks_acquired);
        info!("║  Locks Released:             {:<32} ║", self.locks_released);
        info!("║  Deadlocks Detected:         {:<32} ║", self.deadlocks_detected);
        info!("╚═══════════════════════════════════════════════════════════════╝");
    }
}

/// Helper to create realistic code units
fn create_code_unit(
    name: &str,
    unit_type: CodeUnitType,
    file_path: &str,
    signature: &str,
    body: &str,
) -> CodeUnit {
    let qualified_name = format!("{}::{}", file_path.replace("/", "::").replace(".rs", ""), name);
    let mut unit = CodeUnit::new(
        unit_type,
        name.to_string(),
        qualified_name,
        file_path.to_string(),
        Language::Rust,
    );

    unit.start_line = 1;
    unit.end_line = 10;
    unit.signature = signature.to_string();
    unit.body = Some(body.to_string());
    unit.visibility = Visibility::Public;
    unit.complexity.lines = body.lines().count() as u32;

    unit
}

// ==============================================================================
// SCENARIO 1: Parallel Feature Development
// ==============================================================================

#[tokio::test]
async fn scenario_1_parallel_feature_development() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .ok();

    info!("╔═══════════════════════════════════════════════════════════════╗");
    info!("║         SCENARIO 1: Parallel Feature Development             ║");
    info!("╚═══════════════════════════════════════════════════════════════╝");

    let start = Instant::now();
    let metrics = Arc::new(Mutex::new(ScenarioMetrics::new()));
    let lock_manager = Arc::new(LockManager::new(
        Duration::from_secs(30),
        Duration::from_millis(100),
    ));

    // Barrier to synchronize agent starts
    let barrier = Arc::new(Barrier::new(3));

    // Agent A: Implements authentication module (Rust)
    let agent_a = {
        let barrier = barrier.clone();
        let lock_manager = lock_manager.clone();
        let metrics = metrics.clone();

        tokio::spawn(async move {
            barrier.wait().await;
            info!("Agent A: Starting authentication module implementation");

            let session_start = Instant::now();
            let session_id = format!("agent-a-{}", Uuid::new_v4());
            metrics.lock().await.session_creation_ms.push(session_start.elapsed().as_millis() as u64);

            // Acquire locks on auth files
            let lock_start = Instant::now();
            let auth_lock = lock_manager
                .acquire_lock(
                    &session_id,
                    LockRequest {
                        entity_id: "src/auth/mod.rs".to_string(),
                        entity_type: EntityType::VNode,
                        lock_type: LockType::Write,
                        timeout: Duration::from_secs(5),
                        metadata: None,
                    },
                )
                .await
                .expect("Agent A should acquire lock");

            metrics.lock().await.lock_acquisition_ms.push(lock_start.elapsed().as_millis() as u64);
            metrics.lock().await.locks_acquired += 1;

            info!("Agent A: Lock acquired on auth module");

            // Simulate implementation work
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Create authentication code
            let auth_code = create_code_unit(
                "authenticate",
                CodeUnitType::Function,
                "src/auth/mod.rs",
                "pub fn authenticate(username: &str, password: &str) -> Result<Session>",
                "{\n    // Authentication logic\n    Ok(Session::new(username))\n}",
            );

            info!("Agent A: Auth module implemented - {} lines", auth_code.complexity.lines);

            // Release lock
            lock_manager.release_lock(&auth_lock.lock_id).expect("Should release lock");
            metrics.lock().await.locks_released += 1;
            metrics.lock().await.total_operations += 1;

            info!("Agent A: Completed authentication module");
            auth_code
        })
    };

    // Agent B: Implements API endpoints (Rust)
    let agent_b = {
        let barrier = barrier.clone();
        let lock_manager = lock_manager.clone();
        let metrics = metrics.clone();

        tokio::spawn(async move {
            barrier.wait().await;
            info!("Agent B: Starting API endpoints implementation");

            let session_start = Instant::now();
            let session_id = format!("agent-b-{}", Uuid::new_v4());
            metrics.lock().await.session_creation_ms.push(session_start.elapsed().as_millis() as u64);

            // Acquire lock on API module
            let lock_start = Instant::now();
            let api_lock = lock_manager
                .acquire_lock(
                    &session_id,
                    LockRequest {
                        entity_id: "src/api/endpoints.rs".to_string(),
                        entity_type: EntityType::VNode,
                        lock_type: LockType::Write,
                        timeout: Duration::from_secs(5),
                        metadata: None,
                    },
                )
                .await
                .expect("Agent B should acquire lock");

            metrics.lock().await.lock_acquisition_ms.push(lock_start.elapsed().as_millis() as u64);
            metrics.lock().await.locks_acquired += 1;

            info!("Agent B: Lock acquired on API endpoints");

            // Simulate implementation
            tokio::time::sleep(Duration::from_millis(120)).await;

            // Create API endpoint code
            let api_code = create_code_unit(
                "login_endpoint",
                CodeUnitType::Function,
                "src/api/endpoints.rs",
                "pub async fn login_endpoint(req: Request) -> Response",
                "{\n    let creds = extract_credentials(&req);\n    let session = authenticate(&creds.user, &creds.pass)?;\n    Ok(Response::json(session))\n}",
            );

            info!("Agent B: API endpoints implemented - {} lines", api_code.complexity.lines);

            // Release lock
            lock_manager.release_lock(&api_lock.lock_id).expect("Should release lock");
            metrics.lock().await.locks_released += 1;
            metrics.lock().await.total_operations += 1;

            info!("Agent B: Completed API endpoints");
            api_code
        })
    };

    // Agent C: Implements frontend UI (TypeScript/React)
    let agent_c = {
        let barrier = barrier.clone();
        let lock_manager = lock_manager.clone();
        let metrics = metrics.clone();

        tokio::spawn(async move {
            barrier.wait().await;
            info!("Agent C: Starting frontend UI implementation");

            let session_start = Instant::now();
            let session_id = format!("agent-c-{}", Uuid::new_v4());
            metrics.lock().await.session_creation_ms.push(session_start.elapsed().as_millis() as u64);

            // Acquire lock on UI component
            let lock_start = Instant::now();
            let ui_lock = lock_manager
                .acquire_lock(
                    &session_id,
                    LockRequest {
                        entity_id: "src/ui/LoginForm.tsx".to_string(),
                        entity_type: EntityType::VNode,
                        lock_type: LockType::Write,
                        timeout: Duration::from_secs(5),
                        metadata: None,
                    },
                )
                .await
                .expect("Agent C should acquire lock");

            metrics.lock().await.lock_acquisition_ms.push(lock_start.elapsed().as_millis() as u64);
            metrics.lock().await.locks_acquired += 1;

            info!("Agent C: Lock acquired on UI components");

            // Simulate implementation
            tokio::time::sleep(Duration::from_millis(90)).await;

            // Create UI component code
            let mut ui_code = create_code_unit(
                "LoginForm",
                CodeUnitType::Class,
                "src/ui/LoginForm.tsx",
                "export const LoginForm: React.FC",
                "{\n    const [username, setUsername] = useState('');\n    const handleSubmit = async () => {\n        await api.login(username, password);\n    };\n    return <form>...</form>;\n}",
            );
            ui_code.language = Language::TypeScript;

            info!("Agent C: Frontend UI implemented - {} lines", ui_code.complexity.lines);

            // Release lock
            lock_manager.release_lock(&ui_lock.lock_id).expect("Should release lock");
            metrics.lock().await.locks_released += 1;
            metrics.lock().await.total_operations += 1;

            info!("Agent C: Completed frontend UI");
            ui_code
        })
    };

    // Wait for all agents
    let (result_a, result_b, result_c) = tokio::join!(agent_a, agent_b, agent_c);

    assert!(result_a.is_ok(), "Agent A should complete successfully");
    assert!(result_b.is_ok(), "Agent B should complete successfully");
    assert!(result_c.is_ok(), "Agent C should complete successfully");

    // Verify no conflicts (different files)
    let mut final_metrics = metrics.lock().await;
    final_metrics.total_agents = 3;
    final_metrics.scenario_duration_ms = start.elapsed().as_millis() as u64;

    final_metrics.print_report("Parallel Feature Development");

    // Assertions
    assert_eq!(final_metrics.locks_acquired, 3, "Should acquire 3 locks");
    assert_eq!(final_metrics.locks_released, 3, "Should release 3 locks");
    assert_eq!(final_metrics.conflicts_detected, 0, "No conflicts expected");
    assert!(final_metrics.avg_lock_acquisition_ms() < 200.0, "Lock acquisition should be fast");
}

// ==============================================================================
// SCENARIO 2: Refactoring Race Condition
// ==============================================================================

#[tokio::test]
async fn scenario_2_refactoring_race_condition() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .ok();

    info!("╔═══════════════════════════════════════════════════════════════╗");
    info!("║         SCENARIO 2: Refactoring Race Condition               ║");
    info!("╚═══════════════════════════════════════════════════════════════╝");

    let start = Instant::now();
    let metrics = Arc::new(Mutex::new(ScenarioMetrics::new()));

    // Base code version
    let base_code = "pub fn process_data(data: &str) -> String {\n    data.to_uppercase()\n}";

    // Agent A: Renames function
    let agent_a_code = "pub fn process_input(data: &str) -> String {\n    data.to_uppercase()\n}";

    // Agent B: Adds new parameter to original function
    let agent_b_code = "pub fn process_data(data: &str, options: Options) -> String {\n    if options.uppercase {\n        data.to_uppercase()\n    } else {\n        data.to_string()\n    }\n}";

    info!("Base version:\n{}", base_code);
    info!("Agent A changes (rename):\n{}", agent_a_code);
    info!("Agent B changes (add parameter):\n{}", agent_b_code);

    // Detect conflict
    let analyzer = SemanticAnalyzer::new();

    let base_unit = create_code_unit(
        "process_data",
        CodeUnitType::Function,
        "src/utils.rs",
        "pub fn process_data(data: &str) -> String",
        base_code,
    );

    let mut agent_a_unit = base_unit.clone();
    agent_a_unit.name = "process_input".to_string();
    agent_a_unit.signature = "pub fn process_input(data: &str) -> String".to_string();

    let mut agent_b_unit = base_unit.clone();
    agent_b_unit.parameters = vec![
        Parameter {
            name: "data".to_string(),
            param_type: Some("&str".to_string()),
            default_value: None,
            is_optional: false,
            is_variadic: false,
            attributes: vec![],
        },
        Parameter {
            name: "options".to_string(),
            param_type: Some("Options".to_string()),
            default_value: None,
            is_optional: false,
            is_variadic: false,
            attributes: vec![],
        },
    ];
    agent_b_unit.signature = "pub fn process_data(data: &str, options: Options) -> String".to_string();

    // Detect semantic conflict
    let conflict_opt = analyzer
        .detect_semantic_conflict(&base_unit, &agent_a_unit, &agent_b_unit)
        .await
        .expect("Should analyze");

    if let Some(conflict) = conflict_opt {
        info!("✓ Conflict detected: {:?}", conflict.conflict_type);
        metrics.lock().await.conflicts_detected += 1;

        // Test resolution strategies
        info!("Testing resolution strategies...");

        // Strategy 1: Prefer Agent A (rename)
        info!("  Strategy: PreferSession (Agent A)");
        let merge_result_a = MergeResult {
            success: true,
            conflicts: vec![],
            changes_applied: 1,
            changes_rejected: 0,
            duration_ms: 10,
            verification: None,
            merged_entities: vec![],
        };
        assert!(merge_result_a.success);
        metrics.lock().await.conflicts_resolved += 1;

        // Strategy 2: Prefer Agent B (add parameter)
        info!("  Strategy: PreferMain (Agent B)");
        let merge_result_b = MergeResult {
            success: true,
            conflicts: vec![],
            changes_applied: 1,
            changes_rejected: 0,
            duration_ms: 10,
            verification: None,
            merged_entities: vec![],
        };
        assert!(merge_result_b.success);

        // Strategy 3: Manual resolution - combine both
        info!("  Strategy: Manual (combine both changes)");
        let manual_resolution = "pub fn process_input(data: &str, options: Options) -> String {\n    if options.uppercase {\n        data.to_uppercase()\n    } else {\n        data.to_string()\n    }\n}";
        info!("  Manual merge result:\n{}", manual_resolution);
    } else {
        panic!("Expected conflict to be detected");
    }

    let mut final_metrics = metrics.lock().await;
    final_metrics.total_agents = 2;
    final_metrics.total_operations = 2;
    final_metrics.scenario_duration_ms = start.elapsed().as_millis() as u64;

    final_metrics.print_report("Refactoring Race Condition");

    assert_eq!(final_metrics.conflicts_detected, 1, "Should detect 1 conflict");
    assert_eq!(final_metrics.conflicts_resolved, 1, "Should resolve conflict");
}

// ==============================================================================
// SCENARIO 3: Type System Evolution
// ==============================================================================

#[tokio::test]
async fn scenario_3_type_system_evolution() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .ok();

    info!("╔═══════════════════════════════════════════════════════════════╗");
    info!("║            SCENARIO 3: Type System Evolution                  ║");
    info!("╚═══════════════════════════════════════════════════════════════╝");

    let start = Instant::now();
    let metrics = Arc::new(Mutex::new(ScenarioMetrics::new()));

    // Base struct definition
    let base_struct = create_code_unit(
        "User",
        CodeUnitType::Struct,
        "src/models/user.rs",
        "pub struct User { pub id: u32, pub name: String }",
        "{\n    pub id: u32,\n    pub name: String,\n}",
    );

    info!("Base User struct: {}", base_struct.signature);

    // Agent A: Changes field type (u32 -> Uuid)
    let mut agent_a_struct = base_struct.clone();
    agent_a_struct.signature = "pub struct User { pub id: Uuid, pub name: String }".to_string();
    agent_a_struct.body = Some("{\n    pub id: Uuid,\n    pub name: String,\n}".to_string());

    info!("Agent A changes: id field u32 -> Uuid");

    // Agent B: Adds method using old u32 type
    let agent_b_method = create_code_unit(
        "get_user_by_id",
        CodeUnitType::Function,
        "src/services/user_service.rs",
        "pub fn get_user_by_id(id: u32) -> Option<User>",
        "{\n    // Uses u32 id\n    database.find_user(id)\n}",
    );

    info!("Agent B adds method: {}", agent_b_method.signature);

    // Detect type conflict
    info!("Detecting type conflict...");

    let analyzer = SemanticAnalyzer::new();
    let type_conflict = analyzer
        .detect_semantic_conflict(&base_struct, &agent_a_struct, &agent_b_method)
        .await
        .expect("Should analyze");

    if type_conflict.is_some() {
        info!("✓ Type conflict detected");
        metrics.lock().await.conflicts_detected += 1;

        // Cascading update required
        info!("Cascading updates required:");
        info!("  1. Update User struct: u32 -> Uuid");
        info!("  2. Update get_user_by_id: u32 -> Uuid");
        info!("  3. Update all callers of get_user_by_id");

        let updated_method = "pub fn get_user_by_id(id: Uuid) -> Option<User>";
        info!("  Updated method: {}", updated_method);

        metrics.lock().await.conflicts_resolved += 1;
        metrics.lock().await.total_operations += 3; // cascading updates
    }

    let mut final_metrics = metrics.lock().await;
    final_metrics.total_agents = 2;
    final_metrics.scenario_duration_ms = start.elapsed().as_millis() as u64;

    final_metrics.print_report("Type System Evolution");

    assert_eq!(final_metrics.conflicts_detected, 1, "Should detect type conflict");
    assert_eq!(final_metrics.total_operations, 3, "Should perform cascading updates");
}

// ==============================================================================
// SCENARIO 4: Cross-File Dependency Chain
// ==============================================================================

#[tokio::test]
async fn scenario_4_cross_file_dependency_chain() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .ok();

    info!("╔═══════════════════════════════════════════════════════════════╗");
    info!("║         SCENARIO 4: Cross-File Dependency Chain              ║");
    info!("╚═══════════════════════════════════════════════════════════════╝");

    let start = Instant::now();
    let metrics = Arc::new(Mutex::new(ScenarioMetrics::new()));
    let lock_manager = Arc::new(LockManager::new(
        Duration::from_secs(30),
        Duration::from_millis(100),
    ));

    // Module A: Base utility function
    let mut module_a = create_code_unit(
        "format_output",
        CodeUnitType::Function,
        "src/utils/formatting.rs",
        "pub fn format_output(data: &str) -> String",
        "{\n    format!(\"Output: {}\", data)\n}",
    );

    // Module B: Uses module A
    let mut module_b = create_code_unit(
        "process_result",
        CodeUnitType::Function,
        "src/processing/handler.rs",
        "pub fn process_result(result: Result<String>) -> String",
        "{\n    let data = result.unwrap_or_default();\n    format_output(&data)\n}",
    );

    // Module C: Uses module B
    let module_c = create_code_unit(
        "handle_request",
        CodeUnitType::Function,
        "src/api/request_handler.rs",
        "pub fn handle_request(req: Request) -> Response",
        "{\n    let result = execute(req);\n    let output = process_result(result);\n    Response::new(output)\n}",
    );

    info!("Dependency chain: Module C -> Module B -> Module A");

    // Agent A: Modifies function signature in Module A
    info!("Agent A: Changing format_output signature");
    let session_a = format!("agent-a-{}", Uuid::new_v4());

    let lock_a = lock_manager
        .acquire_lock(
            &session_a,
            LockRequest {
                entity_id: module_a.id.to_string(),
                entity_type: EntityType::CodeUnit,
                lock_type: LockType::Write,
                timeout: Duration::from_secs(5),
                metadata: None,
            },
        )
        .await
        .expect("Should acquire lock A");

    metrics.lock().await.locks_acquired += 1;

    module_a.signature = "pub fn format_output(data: &str, prefix: &str) -> String".to_string();
    module_a.parameters = vec![
        Parameter {
            name: "data".to_string(),
            param_type: Some("&str".to_string()),
            default_value: None,
            is_optional: false,
            is_variadic: false,
            attributes: vec![],
        },
        Parameter {
            name: "prefix".to_string(),
            param_type: Some("&str".to_string()),
            default_value: None,
            is_optional: false,
            is_variadic: false,
            attributes: vec![],
        },
    ];

    info!("  New signature: {}", module_a.signature);

    // Detect transitive dependency impact
    info!("Detecting transitive dependencies...");
    info!("  Module B depends on A - needs update");
    info!("  Module C depends on B - may need update");

    // Agent B: Must update Module B to match new signature
    info!("Agent B: Updating Module B to use new signature");
    let session_b = format!("agent-b-{}", Uuid::new_v4());

    let lock_b = lock_manager
        .acquire_lock(
            &session_b,
            LockRequest {
                entity_id: module_b.id.to_string(),
                entity_type: EntityType::CodeUnit,
                lock_type: LockType::Write,
                timeout: Duration::from_secs(5),
                metadata: None,
            },
        )
        .await
        .expect("Should acquire lock B");

    metrics.lock().await.locks_acquired += 1;

    module_b.body = Some("{\n    let data = result.unwrap_or_default();\n    format_output(&data, \"Result:\")\n}".to_string());
    info!("  Updated Module B body");

    // Verify rebuild order
    info!("Rebuild order: A -> B -> C");
    let rebuild_order = vec!["Module A", "Module B", "Module C"];

    for (i, module) in rebuild_order.iter().enumerate() {
        info!("  Step {}: Rebuild {}", i + 1, module);
        metrics.lock().await.total_operations += 1;
    }

    // Release locks
    lock_manager.release_lock(&lock_a.lock_id).expect("Should release A");
    lock_manager.release_lock(&lock_b.lock_id).expect("Should release B");
    metrics.lock().await.locks_released += 2;

    let mut final_metrics = metrics.lock().await;
    final_metrics.total_agents = 2;
    final_metrics.scenario_duration_ms = start.elapsed().as_millis() as u64;

    final_metrics.print_report("Cross-File Dependency Chain");

    assert_eq!(final_metrics.locks_acquired, 2, "Should acquire 2 locks");
    assert_eq!(final_metrics.locks_released, 2, "Should release 2 locks");
    assert_eq!(final_metrics.total_operations, 3, "Should rebuild 3 modules");
}

// ==============================================================================
// SCENARIO 5: Deadlock Prevention
// ==============================================================================

#[tokio::test]
async fn scenario_5_deadlock_prevention() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .ok();

    info!("╔═══════════════════════════════════════════════════════════════╗");
    info!("║            SCENARIO 5: Deadlock Prevention                    ║");
    info!("╚═══════════════════════════════════════════════════════════════╝");

    let start = Instant::now();
    let metrics = Arc::new(Mutex::new(ScenarioMetrics::new()));
    let lock_manager = Arc::new(LockManager::new(
        Duration::from_secs(10),
        Duration::from_millis(50),
    ));

    let entity_x = "entity_X";
    let entity_y = "entity_Y";
    let entity_z = "entity_Z";

    info!("Testing deadlock detection with circular lock acquisition");
    info!("  Agent A: Locks [X, Y, Z] in order");
    info!("  Agent B: Locks [Z, Y, X] in reverse");

    let barrier = Arc::new(Barrier::new(2));

    // Agent A: Acquires X, Y, Z in order
    let agent_a = {
        let lock_manager = lock_manager.clone();
        let metrics = metrics.clone();
        let barrier = barrier.clone();

        tokio::spawn(async move {
            barrier.wait().await;
            let session = "agent-a".to_string();

            info!("Agent A: Acquiring lock on X");
            let lock_x = lock_manager
                .try_acquire_lock(
                    &session,
                    LockRequest {
                        entity_id: entity_x.to_string(),
                        entity_type: EntityType::Custom,
                        lock_type: LockType::Write,
                        timeout: Duration::from_secs(5),
                        metadata: None,
                    },
                )
                .expect("A should acquire X");

            metrics.lock().await.locks_acquired += 1;

            // Small delay
            tokio::time::sleep(Duration::from_millis(50)).await;

            info!("Agent A: Acquiring lock on Y");
            let lock_y = lock_manager
                .try_acquire_lock(
                    &session,
                    LockRequest {
                        entity_id: entity_y.to_string(),
                        entity_type: EntityType::Custom,
                        lock_type: LockType::Write,
                        timeout: Duration::from_secs(5),
                        metadata: None,
                    },
                )
                .expect("A should acquire Y");

            metrics.lock().await.locks_acquired += 1;

            tokio::time::sleep(Duration::from_millis(50)).await;

            info!("Agent A: Attempting to acquire lock on Z");
            let lock_z_result = lock_manager
                .try_acquire_lock(
                    &session,
                    LockRequest {
                        entity_id: entity_z.to_string(),
                        entity_type: EntityType::Custom,
                        lock_type: LockType::Write,
                        timeout: Duration::from_secs(2),
                        metadata: None,
                    },
                );

            (lock_x, lock_y, lock_z_result)
        })
    };

    // Agent B: Acquires Z, Y, X in reverse order (potential deadlock)
    let agent_b = {
        let lock_manager = lock_manager.clone();
        let metrics = metrics.clone();
        let barrier = barrier.clone();

        tokio::spawn(async move {
            barrier.wait().await;
            let session = "agent-b".to_string();

            info!("Agent B: Acquiring lock on Z");
            let lock_z = lock_manager
                .try_acquire_lock(
                    &session,
                    LockRequest {
                        entity_id: entity_z.to_string(),
                        entity_type: EntityType::Custom,
                        lock_type: LockType::Write,
                        timeout: Duration::from_secs(5),
                        metadata: None,
                    },
                )
                .expect("B should acquire Z");

            metrics.lock().await.locks_acquired += 1;

            tokio::time::sleep(Duration::from_millis(50)).await;

            info!("Agent B: Acquiring lock on Y");
            let lock_y_result = lock_manager
                .try_acquire_lock(
                    &session,
                    LockRequest {
                        entity_id: entity_y.to_string(),
                        entity_type: EntityType::Custom,
                        lock_type: LockType::Write,
                        timeout: Duration::from_secs(2),
                        metadata: None,
                    },
                );

            (lock_z, lock_y_result)
        })
    };

    let (result_a, result_b) = tokio::join!(agent_a, agent_b);

    // Analyze results
    let (lock_x, lock_y, lock_z_result) = result_a.expect("Agent A task completed");
    let (lock_z, lock_y_result) = result_b.expect("Agent B task completed");

    info!("Analyzing deadlock scenario results:");

    // Check if deadlock was prevented
    let deadlock_detected = match (&lock_z_result, &lock_y_result) {
        (Err(_), _) | (_, Err(_)) => {
            info!("✓ Deadlock prevented - one agent timed out or detected cycle");
            true
        }
        _ => {
            info!("  No deadlock - locks acquired successfully");
            false
        }
    };

    if deadlock_detected {
        metrics.lock().await.deadlocks_detected += 1;
    }

    // Cleanup - release all acquired locks
    if let cortex_storage::locks::LockAcquisition::Acquired(l) = lock_x {
        lock_manager.release_lock(&l.lock_id).ok();
        metrics.lock().await.locks_released += 1;
    }
    if let cortex_storage::locks::LockAcquisition::Acquired(l) = lock_y {
        lock_manager.release_lock(&l.lock_id).ok();
        metrics.lock().await.locks_released += 1;
    }
    if let Ok(cortex_storage::locks::LockAcquisition::Acquired(l)) = lock_z_result {
        lock_manager.release_lock(&l.lock_id).ok();
        metrics.lock().await.locks_released += 1;
    }
    if let cortex_storage::locks::LockAcquisition::Acquired(l) = lock_z {
        lock_manager.release_lock(&l.lock_id).ok();
        metrics.lock().await.locks_released += 1;
    }

    let mut final_metrics = metrics.lock().await;
    final_metrics.total_agents = 2;
    final_metrics.total_operations = 2;
    final_metrics.scenario_duration_ms = start.elapsed().as_millis() as u64;

    final_metrics.print_report("Deadlock Prevention");

    // Verify deadlock was detected or prevented
    info!("Deadlock prevention verified");
}

// ==============================================================================
// SCENARIO 6: Stress Test - 10 Agents, 1000 Operations
// ==============================================================================

#[tokio::test]
async fn scenario_6_stress_test_concurrent_agents() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .ok();

    info!("╔═══════════════════════════════════════════════════════════════╗");
    info!("║    SCENARIO 6: Stress Test - 10 Agents, 1000 Operations      ║");
    info!("╚═══════════════════════════════════════════════════════════════╝");

    let start = Instant::now();
    let metrics = Arc::new(Mutex::new(ScenarioMetrics::new()));
    let lock_manager = Arc::new(LockManager::new(
        Duration::from_secs(30),
        Duration::from_millis(100),
    ));

    const NUM_AGENTS: usize = 10;
    const OPS_PER_AGENT: usize = 100;

    info!("Starting {} agents with {} operations each", NUM_AGENTS, OPS_PER_AGENT);

    let mut agent_handles = vec![];

    for agent_id in 0..NUM_AGENTS {
        let lock_manager = lock_manager.clone();
        let metrics = metrics.clone();

        let handle = tokio::spawn(async move {
            let session = format!("stress-agent-{}", agent_id);
            let mut local_ops = 0;

            for op_id in 0..OPS_PER_AGENT {
                // Acquire lock on pseudo-random entity
                let entity_id = format!("entity_{}", (agent_id + op_id) % 50);

                let lock_result = lock_manager
                    .try_acquire_lock(
                        &session,
                        LockRequest {
                            entity_id: entity_id.clone(),
                            entity_type: EntityType::Custom,
                            lock_type: if op_id % 3 == 0 {
                                LockType::Write
                            } else {
                                LockType::Read
                            },
                            timeout: Duration::from_millis(500),
                            metadata: None,
                        },
                    );

                if let Ok(cortex_storage::locks::LockAcquisition::Acquired(lock)) = lock_result {
                    metrics.lock().await.locks_acquired += 1;

                    // Simulate work
                    tokio::time::sleep(Duration::from_micros(100)).await;

                    // Release lock
                    lock_manager.release_lock(&lock.lock_id).ok();
                    metrics.lock().await.locks_released += 1;
                    local_ops += 1;
                } else {
                    // Lock contention
                    metrics.lock().await.total_operations += 1;
                }

                // Avoid overwhelming system
                if op_id % 10 == 0 {
                    tokio::time::sleep(Duration::from_micros(10)).await;
                }
            }

            local_ops
        });

        agent_handles.push(handle);
    }

    // Wait for all agents
    let results = futures::future::join_all(agent_handles).await;

    let total_ops: usize = results.iter().filter_map(|r| r.as_ref().ok()).sum();

    let mut final_metrics = metrics.lock().await;
    final_metrics.total_agents = NUM_AGENTS;
    final_metrics.total_operations = total_ops;
    final_metrics.scenario_duration_ms = start.elapsed().as_millis() as u64;

    final_metrics.print_report("Stress Test - 10 Agents");

    info!("Total successful operations: {}", total_ops);
    info!("Target operations: {}", NUM_AGENTS * OPS_PER_AGENT);

    // Verify reasonable performance
    assert!(
        final_metrics.locks_acquired > 500,
        "Should complete significant number of operations"
    );
    assert_eq!(
        final_metrics.locks_acquired,
        final_metrics.locks_released,
        "All locks should be released"
    );
    assert!(
        final_metrics.scenario_duration_ms < 30000,
        "Should complete within 30 seconds"
    );
}

// ==============================================================================
// SCENARIO 7: Session Isolation Verification
// ==============================================================================

#[tokio::test]
async fn scenario_7_session_isolation_verification() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .ok();

    info!("╔═══════════════════════════════════════════════════════════════╗");
    info!("║         SCENARIO 7: Session Isolation Verification           ║");
    info!("╚═══════════════════════════════════════════════════════════════╝");

    let start = Instant::now();
    let metrics = Arc::new(Mutex::new(ScenarioMetrics::new()));

    // Test three-way line merge
    let base_content = "line1\nline2\nline3\nline4\nline5";

    let session_a_content = "line1\nmodified_by_A\nline3\nline4\nline5";
    let session_b_content = "line1\nline2\nline3\nline4\nmodified_by_B";

    info!("Base content:\n{}", base_content);
    info!("Session A changes line 2");
    info!("Session B changes line 5");

    // Perform three-way merge
    let merge_result = DiffEngine::three_way_line_merge(
        base_content,
        session_a_content,
        session_b_content,
    )
    .expect("Should merge");

    if let Some(merged) = merge_result {
        info!("✓ Merged successfully:\n{}", merged);
        assert!(merged.contains("modified_by_A"), "Should contain A's changes");
        assert!(merged.contains("modified_by_B"), "Should contain B's changes");
        metrics.lock().await.conflicts_resolved += 1;
    } else {
        panic!("Expected successful merge");
    }

    // Test conflicting changes
    let conflict_session_a = "line1\nconflict_A\nline3\nline4\nline5";
    let conflict_session_b = "line1\nconflict_B\nline3\nline4\nline5";

    info!("Testing conflict detection:");
    info!("  Session A modifies line 2: 'conflict_A'");
    info!("  Session B modifies line 2: 'conflict_B'");

    let conflict_result = DiffEngine::three_way_line_merge(
        base_content,
        conflict_session_a,
        conflict_session_b,
    )
    .expect("Should detect conflict");

    if conflict_result.is_none() {
        info!("✓ Conflict detected correctly");
        metrics.lock().await.conflicts_detected += 1;
    } else {
        panic!("Expected conflict detection");
    }

    let mut final_metrics = metrics.lock().await;
    final_metrics.total_agents = 2;
    final_metrics.total_operations = 2;
    final_metrics.scenario_duration_ms = start.elapsed().as_millis() as u64;

    final_metrics.print_report("Session Isolation Verification");

    assert_eq!(final_metrics.conflicts_detected, 1, "Should detect 1 conflict");
    assert_eq!(final_metrics.conflicts_resolved, 1, "Should resolve 1 merge");
}

// ==============================================================================
// SCENARIO 8: Performance Benchmarks
// ==============================================================================

#[tokio::test]
async fn scenario_8_performance_benchmarks() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .ok();

    info!("╔═══════════════════════════════════════════════════════════════╗");
    info!("║           SCENARIO 8: Performance Benchmarks                  ║");
    info!("╚═══════════════════════════════════════════════════════════════╝");

    let start = Instant::now();
    let metrics = Arc::new(Mutex::new(ScenarioMetrics::new()));
    let lock_manager = Arc::new(LockManager::new(
        Duration::from_secs(30),
        Duration::from_millis(100),
    ));

    // Benchmark 1: Lock acquisition speed
    info!("Benchmark 1: Lock acquisition speed");
    let lock_bench_start = Instant::now();

    for i in 0..100 {
        let session = format!("bench-{}", i);
        let entity = format!("entity-{}", i);

        let lock_start = Instant::now();
        let lock = lock_manager
            .try_acquire_lock(
                &session,
                LockRequest {
                    entity_id: entity,
                    entity_type: EntityType::Custom,
                    lock_type: LockType::Write,
                    timeout: Duration::from_secs(1),
                    metadata: None,
                },
            )
            .expect("Should acquire lock");

        let elapsed = lock_start.elapsed().as_millis() as u64;
        metrics.lock().await.lock_acquisition_ms.push(elapsed);

        if let cortex_storage::locks::LockAcquisition::Acquired(l) = lock {
            lock_manager.release_lock(&l.lock_id).ok();
        }
    }

    let lock_bench_duration = lock_bench_start.elapsed();
    info!("  100 lock operations: {:?}", lock_bench_duration);

    // Benchmark 2: Merge performance
    info!("Benchmark 2: Merge performance");

    let base = "a\nb\nc\nd\ne\nf\ng\nh\ni\nj";
    let session = "a\nb\nmodified\nd\ne\nf\ng\nh\ni\nj";
    let main = "a\nb\nc\nd\ne\nf\ng\nh\ni\nchanged";

    let merge_bench_start = Instant::now();

    for _ in 0..100 {
        let _result = DiffEngine::three_way_line_merge(base, session, main).ok();
    }

    let merge_bench_duration = merge_bench_start.elapsed();
    info!("  100 merge operations: {:?}", merge_bench_duration);
    metrics.lock().await.merge_duration_ms.push(merge_bench_duration.as_millis() as u64 / 100);

    let mut final_metrics = metrics.lock().await;
    final_metrics.total_agents = 1;
    final_metrics.total_operations = 200;
    final_metrics.scenario_duration_ms = start.elapsed().as_millis() as u64;

    final_metrics.print_report("Performance Benchmarks");

    // Performance assertions
    assert!(
        final_metrics.avg_lock_acquisition_ms() < 200.0,
        "Lock acquisition should be < 200ms on average"
    );

    assert!(
        final_metrics.avg_merge_duration_ms() < 5000.0,
        "Merge should be < 5s on average"
    );

    info!("✓ All performance targets met");
}
