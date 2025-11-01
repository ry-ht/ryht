//! Command modules for Axon CLI

pub mod config;
pub mod output;
pub mod runtime_manager;
pub mod runtime_manager_impl;
pub mod server_manager;
pub mod api;

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::process::Command;
use tracing::info;
use serde_json::json;

use crate::agents::AgentType;
use crate::commands::output::OutputFormatArg;
use crate::commands::runtime_manager_impl::RuntimeManager;
use crate::commands::server_manager::ServerManager;

// Global runtime manager
lazy_static::lazy_static! {
    static ref RUNTIME_MANAGER: Arc<RuntimeManager> = Arc::new(RuntimeManager::new());
    static ref SERVER_MANAGER: Arc<ServerManager> = Arc::new(ServerManager::new());
}

/// Initialize a new Axon workspace
pub async fn init_workspace(name: String, path: Option<PathBuf>) -> Result<()> {
    let workspace_path = path.unwrap_or_else(|| PathBuf::from("."));
    let workspace_dir = workspace_path.join(&name);

    info!("Initializing Axon workspace: {}", name);

    // Create workspace directory structure
    fs::create_dir_all(&workspace_dir).await?;
    fs::create_dir_all(workspace_dir.join("agents")).await?;
    fs::create_dir_all(workspace_dir.join("workflows")).await?;
    fs::create_dir_all(workspace_dir.join("configs")).await?;
    fs::create_dir_all(workspace_dir.join("logs")).await?;
    fs::create_dir_all(workspace_dir.join("data")).await?;

    // Create default configuration
    let config = json!({
        "name": name,
        "version": "1.0.0",
        "agents": {
            "default_model": "gpt-4",
            "max_concurrent": 10,
            "timeout_seconds": 300
        },
        "runtime": {
            "memory_limit_mb": 1024,
            "cpu_limit_percent": 80,
            "max_tool_calls": 100
        },
        "cortex": {
            "enabled": true,
            "mcp_mode": "stdio",
            "binary_path": "cortex"
        }
    });

    let config_path = workspace_dir.join("axon.json");
    let config_str = serde_json::to_string_pretty(&config)?;
    fs::write(&config_path, config_str).await?;

    // Create README
    let readme = format!(
        "# {} Workspace\n\n\
         This is an Axon multi-agent system workspace.\n\n\
         ## Getting Started\n\n\
         1. Start an agent: `axon agent start orchestrator --name main`\n\
         2. Run a workflow: `axon workflow run workflow.yaml`\n\
         3. Check status: `axon status`\n\n\
         ## Directory Structure\n\n\
         - `agents/` - Agent configurations\n\
         - `workflows/` - Workflow definitions\n\
         - `configs/` - Configuration files\n\
         - `logs/` - System logs\n\
         - `data/` - Persistent data\n",
        name
    );

    fs::write(workspace_dir.join("README.md"), readme).await?;

    info!("Workspace initialized successfully at: {}", workspace_dir.display());
    println!("✓ Workspace '{}' created at: {}", name, workspace_dir.display());
    println!("\nNext steps:");
    println!("  cd {}", workspace_dir.display());
    println!("  axon agent start orchestrator --name main");

    Ok(())
}

// Agent commands
pub async fn agent_start(
    agent_type: AgentType,
    name: String,
    capabilities: Option<String>,
    model: Option<String>,
    max_tasks: usize,
) -> Result<()> {
    info!("Starting agent: {} (type: {:?})", name, agent_type);

    let caps = capabilities
        .map(|c| c.split(',').map(String::from).collect())
        .unwrap_or_default();

    RUNTIME_MANAGER.start_agent(
        name.clone(),
        agent_type,
        caps,
        model,
        max_tasks,
    ).await?;

    println!("✓ Agent '{}' started successfully", name);
    Ok(())
}

pub async fn agent_stop(agent_id: String, force: bool) -> Result<()> {
    info!("Stopping agent: {} (force: {})", agent_id, force);

    if force {
        RUNTIME_MANAGER.force_stop_agent(&agent_id).await?;
        println!("✓ Agent '{}' force stopped", agent_id);
    } else {
        RUNTIME_MANAGER.stop_agent(&agent_id).await?;
        println!("✓ Agent '{}' stopped gracefully", agent_id);
    }

    Ok(())
}

pub async fn agent_list(
    agent_type: Option<AgentType>,
    detailed: bool,
    format: OutputFormatArg,
) -> Result<()> {
    let agents = RUNTIME_MANAGER.list_agents(agent_type).await?;

    match format {
        OutputFormatArg::Json => {
            println!("{}", serde_json::to_string_pretty(&agents)?);
        }
        OutputFormatArg::Plain => {
            for agent in agents {
                println!("{}\t{}\t{}", agent.id, agent.name, agent.status);
            }
        }
        OutputFormatArg::Human => {
            if agents.is_empty() {
                println!("No agents running");
            } else {
                println!("Running Agents:");
                println!("{:<20} {:<15} {:<10} {:<15}", "ID", "Name", "Type", "Status");
                println!("{}", "-".repeat(60));

                for agent in agents {
                    println!(
                        "{:<20} {:<15} {:<10} {:<15}",
                        agent.id,
                        agent.name,
                        format!("{:?}", agent.agent_type),
                        agent.status
                    );

                    if detailed {
                        println!("  Model: {}", agent.model.as_deref().unwrap_or("default"));
                        println!("  Capabilities: {}", agent.capabilities.join(", "));
                        println!("  Started: {}", agent.started_at);
                        if let Some(metrics) = agent.metrics {
                            println!("  Tasks: {}", metrics.tasks_completed);
                            println!("  Errors: {}", metrics.errors);
                        }
                        println!();
                    }
                }
            }
        }
    }

    Ok(())
}

pub async fn agent_info(agent_id: String, format: OutputFormatArg) -> Result<()> {
    let info = RUNTIME_MANAGER.get_agent_info(&agent_id).await?;

    match format {
        OutputFormatArg::Json => {
            println!("{}", serde_json::to_string_pretty(&info)?);
        }
        OutputFormatArg::Plain | OutputFormatArg::Human => {
            println!("Agent Information");
            println!("================");
            println!("ID: {}", info.id);
            println!("Name: {}", info.name);
            println!("Type: {:?}", info.agent_type);
            println!("Status: {}", info.status);
            println!("Model: {}", info.model.as_deref().unwrap_or("default"));
            println!("Capabilities: {}", info.capabilities.join(", "));
            println!("Started: {}", info.started_at);

            if let Some(metrics) = info.metrics {
                println!("\nMetrics:");
                println!("  Tasks Completed: {}", metrics.tasks_completed);
                println!("  Tasks Failed: {}", metrics.tasks_failed);
                println!("  Avg Response Time: {:.2}ms", metrics.avg_response_time_ms);
                println!("  Errors: {}", metrics.errors);
                println!("  Memory Usage: {}MB", metrics.memory_usage_mb);
                println!("  CPU Usage: {:.1}%", metrics.cpu_usage_percent);
            }
        }
    }

    Ok(())
}

pub async fn agent_pause(agent_id: String) -> Result<()> {
    RUNTIME_MANAGER.pause_agent(&agent_id).await?;
    println!("✓ Agent '{}' paused", agent_id);
    Ok(())
}

pub async fn agent_resume(agent_id: String) -> Result<()> {
    RUNTIME_MANAGER.resume_agent(&agent_id).await?;
    println!("✓ Agent '{}' resumed", agent_id);
    Ok(())
}

pub async fn agent_logs(agent_id: String, follow: bool, lines: usize) -> Result<()> {
    let logs = RUNTIME_MANAGER.get_agent_logs(&agent_id, lines).await?;

    for line in logs {
        println!("{}", line);
    }

    if follow {
        // Follow mode - stream new logs
        let mut stream = RUNTIME_MANAGER.stream_agent_logs(&agent_id).await?;
        while let Some(line) = stream.recv().await {
            println!("{}", line);
        }
    }

    Ok(())
}

// Workflow commands
pub async fn workflow_run(
    workflow: PathBuf,
    input: Option<String>,
    dry_run: bool,
) -> Result<()> {
    info!("Running workflow: {}", workflow.display());

    if !workflow.exists() {
        return Err(anyhow::anyhow!("Workflow file not found: {}", workflow.display()));
    }

    let workflow_content = fs::read_to_string(&workflow).await?;

    if dry_run {
        println!("Dry run - validating workflow...");

        // Validate workflow structure
        if let Err(e) = validate_workflow_content(&workflow_content) {
            return Err(anyhow::anyhow!("Workflow validation failed: {}", e));
        }

        println!("✓ Workflow is valid");
        return Ok(());
    }

    let input_data = input
        .map(|i| serde_json::from_str(&i))
        .transpose()?
        .unwrap_or_else(|| json!({}));

    let workflow_id = RUNTIME_MANAGER.run_workflow(workflow_content, input_data).await?;

    println!("✓ Workflow started with ID: {}", workflow_id);
    println!("Use 'axon workflow status {}' to check progress", workflow_id);

    Ok(())
}

pub async fn workflow_list(status: Option<String>, format: OutputFormatArg) -> Result<()> {
    let workflows = RUNTIME_MANAGER.list_workflows(status).await?;

    match format {
        OutputFormatArg::Json => {
            println!("{}", serde_json::to_string_pretty(&workflows)?);
        }
        OutputFormatArg::Plain => {
            for wf in workflows {
                println!("{}\t{}\t{}", wf.id, wf.name, wf.status);
            }
        }
        OutputFormatArg::Human => {
            if workflows.is_empty() {
                println!("No workflows found");
            } else {
                println!("Workflows:");
                println!("{:<36} {:<20} {:<10} {:<20}", "ID", "Name", "Status", "Started");
                println!("{}", "-".repeat(86));

                for wf in workflows {
                    println!(
                        "{:<36} {:<20} {:<10} {:<20}",
                        wf.id, wf.name, wf.status, wf.started_at
                    );
                }
            }
        }
    }

    Ok(())
}

pub async fn workflow_status(workflow_id: String, format: OutputFormatArg) -> Result<()> {
    let status = RUNTIME_MANAGER.get_workflow_status(&workflow_id).await?;

    match format {
        OutputFormatArg::Json => {
            println!("{}", serde_json::to_string_pretty(&status)?);
        }
        OutputFormatArg::Plain | OutputFormatArg::Human => {
            println!("Workflow Status");
            println!("==============");
            println!("ID: {}", status.id);
            println!("Name: {}", status.name);
            println!("Status: {}", status.status);
            println!("Started: {}", status.started_at);
            if let Some(completed) = status.completed_at {
                println!("Completed: {}", completed);
            }
            println!("Progress: {}/{} tasks", status.tasks_completed, status.total_tasks);

            if !status.current_tasks.is_empty() {
                println!("\nCurrent Tasks:");
                for task in &status.current_tasks {
                    println!("  - {}: {}", task.agent, task.description);
                }
            }

            if let Some(error) = status.error {
                println!("\nError: {}", error);
            }
        }
    }

    Ok(())
}

pub async fn workflow_cancel(workflow_id: String) -> Result<()> {
    RUNTIME_MANAGER.cancel_workflow(&workflow_id).await?;
    println!("✓ Workflow '{}' cancelled", workflow_id);
    Ok(())
}

pub async fn workflow_validate(workflow: PathBuf) -> Result<()> {
    if !workflow.exists() {
        return Err(anyhow::anyhow!("Workflow file not found: {}", workflow.display()));
    }

    let content = fs::read_to_string(&workflow).await?;

    // Validate workflow structure
    validate_workflow_content(&content)?;

    println!("✓ Workflow is valid");
    Ok(())
}

/// Validate workflow content structure
fn validate_workflow_content(content: &str) -> Result<()> {
    // Try to parse as JSON first
    let workflow: crate::orchestration::workflow::Workflow =
        if let Ok(wf) = serde_json::from_str(content) {
            wf
        } else {
            // Try YAML
            serde_yaml::from_str(content)
                .map_err(|e| anyhow::anyhow!("Failed to parse workflow as JSON or YAML: {}", e))?
        };

    // Validate workflow structure
    if workflow.name.is_empty() {
        return Err(anyhow::anyhow!("Workflow name cannot be empty"));
    }

    if workflow.tasks.is_empty() {
        return Err(anyhow::anyhow!("Workflow must have at least one task"));
    }

    // Validate each task
    for task in &workflow.tasks {
        if task.id.is_empty() {
            return Err(anyhow::anyhow!("Task ID cannot be empty"));
        }
        if task.name.is_empty() {
            return Err(anyhow::anyhow!("Task name cannot be empty (task: {})", task.id));
        }
    }

    // Validate dependencies reference existing tasks
    let task_ids: std::collections::HashSet<_> = workflow.tasks.iter().map(|t| t.id.as_str()).collect();
    for (task_id, deps) in &workflow.dependencies {
        if !task_ids.contains(task_id.as_str()) {
            return Err(anyhow::anyhow!("Dependency references non-existent task: {}", task_id));
        }
        for dep in deps {
            if !task_ids.contains(dep.as_str()) {
                return Err(anyhow::anyhow!("Task {} depends on non-existent task: {}", task_id, dep));
            }
        }
    }

    // Check for circular dependencies
    if has_circular_dependencies(&workflow.dependencies) {
        return Err(anyhow::anyhow!("Workflow contains circular dependencies"));
    }

    Ok(())
}

/// Check for circular dependencies using depth-first search
fn has_circular_dependencies(dependencies: &std::collections::HashMap<String, Vec<String>>) -> bool {
    use std::collections::{HashMap, HashSet};

    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();

    fn dfs(
        node: &str,
        dependencies: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(deps) = dependencies.get(node) {
            for dep in deps {
                if !visited.contains(dep) {
                    if dfs(dep, dependencies, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(dep) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }

    for task_id in dependencies.keys() {
        if !visited.contains(task_id) {
            if dfs(task_id, dependencies, &mut visited, &mut rec_stack) {
                return true;
            }
        }
    }

    false
}

// Server commands
pub async fn server_start(host: String, port: u16, _workers: Option<usize>) -> Result<()> {
    info!("Starting API server on {}:{}", host, port);

    // Fork a new process for the server
    let cmd = Command::new(std::env::current_exe()?)
        .arg("internal-server-run")
        .arg("--host")
        .arg(&host)
        .arg("--port")
        .arg(port.to_string())
        .spawn()?;

    // Save PID
    let pid = cmd.id().unwrap_or(0);
    SERVER_MANAGER.set_server_pid(pid).await;

    println!("✓ API server started on http://{}:{}", host, port);
    println!("  PID: {}", pid);
    println!("  Use 'axon server stop' to stop the server");

    Ok(())
}

pub async fn server_run_blocking(host: String, port: u16, workers: Option<usize>) -> Result<()> {
    use crate::commands::api::server::start_server;

    info!("Running API server on {}:{}", host, port);

    // This runs in the forked process
    start_server(host, port, workers).await?;

    Ok(())
}

pub async fn server_stop() -> Result<()> {
    let pid = SERVER_MANAGER.get_server_pid().await
        .ok_or_else(|| anyhow::anyhow!("No server running"))?;

    // Send SIGTERM
    unsafe {
        libc::kill(pid as i32, libc::SIGTERM);
    }

    SERVER_MANAGER.clear_server_pid().await;

    println!("✓ API server stopped");
    Ok(())
}

pub async fn server_status() -> Result<()> {
    if let Some(pid) = SERVER_MANAGER.get_server_pid().await {
        // Check if process is still running
        let running = unsafe { libc::kill(pid as i32, 0) == 0 };

        if running {
            println!("✓ API server is running (PID: {})", pid);
        } else {
            println!("✗ API server process not found (stale PID: {})", pid);
            SERVER_MANAGER.clear_server_pid().await;
        }
    } else {
        println!("✗ API server is not running");
    }

    Ok(())
}

// Status command
pub async fn show_status(detailed: bool, format: OutputFormatArg) -> Result<()> {
    let agents = RUNTIME_MANAGER.list_agents(None).await?;
    let workflows = RUNTIME_MANAGER.list_workflows(None).await?;
    let server_running = SERVER_MANAGER.get_server_pid().await.is_some();

    let status = json!({
        "agents": {
            "total": agents.len(),
            "running": agents.iter().filter(|a| a.status == "running").count(),
            "paused": agents.iter().filter(|a| a.status == "paused").count(),
        },
        "workflows": {
            "total": workflows.len(),
            "running": workflows.iter().filter(|w| w.status == "running").count(),
            "completed": workflows.iter().filter(|w| w.status == "completed").count(),
            "failed": workflows.iter().filter(|w| w.status == "failed").count(),
        },
        "server": {
            "running": server_running,
            "pid": SERVER_MANAGER.get_server_pid().await,
        }
    });

    match format {
        OutputFormatArg::Json => {
            println!("{}", serde_json::to_string_pretty(&status)?);
        }
        OutputFormatArg::Plain | OutputFormatArg::Human => {
            println!("Axon System Status");
            println!("==================");
            println!();
            println!("Agents:");
            println!("  Total: {}", status["agents"]["total"]);
            println!("  Running: {}", status["agents"]["running"]);
            println!("  Paused: {}", status["agents"]["paused"]);
            println!();
            println!("Workflows:");
            println!("  Total: {}", status["workflows"]["total"]);
            println!("  Running: {}", status["workflows"]["running"]);
            println!("  Completed: {}", status["workflows"]["completed"]);
            println!("  Failed: {}", status["workflows"]["failed"]);
            println!();
            println!("API Server:");
            if server_running {
                println!("  Status: Running");
                println!("  PID: {}", status["server"]["pid"]);
            } else {
                println!("  Status: Stopped");
            }

            if detailed {
                println!();
                println!("Running Agents:");
                for agent in &agents {
                    if agent.status == "running" {
                        println!("  - {} ({})", agent.name, agent.id);
                    }
                }

                println!();
                println!("Active Workflows:");
                for wf in &workflows {
                    if wf.status == "running" {
                        println!("  - {} ({})", wf.name, wf.id);
                    }
                }
            }
        }
    }

    Ok(())
}

// Config commands
pub async fn config_get(key: String) -> Result<()> {
    let config = config::ConfigManager::load().await?;

    let value = config.get(&key)
        .ok_or_else(|| anyhow::anyhow!("Configuration key not found: {}", key))?;

    println!("{}", value);
    Ok(())
}

pub async fn config_set(key: String, value: String, global: bool) -> Result<()> {
    let mut config = config::ConfigManager::load().await?;

    config.set(&key, value.clone())?;
    config.save(global).await?;

    println!("✓ Configuration updated: {} = {}", key, value);
    Ok(())
}

pub async fn config_list(format: OutputFormatArg) -> Result<()> {
    let config = config::ConfigManager::load().await?;

    match format {
        OutputFormatArg::Json => {
            println!("{}", serde_json::to_string_pretty(&config.all())?);
        }
        OutputFormatArg::Plain | OutputFormatArg::Human => {
            println!("Configuration:");
            for (key, value) in config.all() {
                println!("  {}: {}", key, value);
            }
        }
    }

    Ok(())
}

// Monitor commands
pub async fn monitor_dashboard(refresh: u64) -> Result<()> {
    use std::time::Duration;

    loop {
        // Clear screen
        print!("\x1B[2J\x1B[1;1H");

        // Show status
        show_status(true, OutputFormatArg::Human).await?;

        // Show recent logs
        println!();
        println!("Recent Activity:");
        println!("================");

        // Try to show recent logs from any available agent
        let agents = RUNTIME_MANAGER.list_agents(None).await?;
        if !agents.is_empty() {
            // Show logs from the first agent as an example
            let agent_id = &agents[0].id;
            match RUNTIME_MANAGER.get_agent_logs(agent_id, 5).await {
                Ok(logs) => {
                    for log in logs {
                        println!("  {}", log);
                    }
                }
                Err(_) => {
                    println!("  No recent activity");
                }
            }
        } else {
            println!("  No agents running");
        }

        tokio::time::sleep(Duration::from_secs(refresh)).await;
    }
}

pub async fn monitor_metrics(agent_id: Option<String>, format: OutputFormatArg) -> Result<()> {
    let metrics = if let Some(id) = agent_id {
        RUNTIME_MANAGER.get_agent_metrics(&id).await?
    } else {
        RUNTIME_MANAGER.get_all_metrics().await?
    };

    match format {
        OutputFormatArg::Json => {
            println!("{}", serde_json::to_string_pretty(&metrics)?);
        }
        OutputFormatArg::Plain | OutputFormatArg::Human => {
            println!("System Metrics");
            println!("=============");
            println!();

            for (agent_id, agent_metrics) in metrics {
                println!("Agent: {}", agent_id);
                println!("  Tasks Completed: {}", agent_metrics.tasks_completed);
                println!("  Tasks Failed: {}", agent_metrics.tasks_failed);
                println!("  Avg Response Time: {:.2}ms", agent_metrics.avg_response_time_ms);
                println!("  Memory Usage: {}MB", agent_metrics.memory_usage_mb);
                println!("  CPU Usage: {:.1}%", agent_metrics.cpu_usage_percent);
                println!();
            }
        }
    }

    Ok(())
}

pub async fn monitor_telemetry(range: u64, format: OutputFormatArg) -> Result<()> {
    let telemetry = RUNTIME_MANAGER.get_telemetry(range).await?;

    match format {
        OutputFormatArg::Json => {
            println!("{}", serde_json::to_string_pretty(&telemetry)?);
        }
        OutputFormatArg::Plain | OutputFormatArg::Human => {
            println!("Telemetry Data (last {} minutes)", range);
            println!("================================");
            println!();

            println!("Request Rate: {:.1} req/min", telemetry.request_rate);
            println!("Error Rate: {:.1}%", telemetry.error_rate);
            println!("Avg Latency: {:.2}ms", telemetry.avg_latency_ms);
            println!("Active Agents: {}", telemetry.active_agents);
            println!("Active Workflows: {}", telemetry.active_workflows);
            println!();

            if !telemetry.top_errors.is_empty() {
                println!("Top Errors:");
                for (error, count) in &telemetry.top_errors {
                    println!("  {} ({}x)", error, count);
                }
            }
        }
    }

    Ok(())
}

// Doctor commands
pub async fn doctor_check(fix: bool) -> Result<()> {
    println!("Running diagnostic checks...");
    println!();

    let mut issues = Vec::new();

    // Check Cortex binary
    print!("Checking Cortex binary... ");
    match Command::new("cortex").arg("--version").output().await {
        Ok(output) if output.status.success() => {
            println!("✓");
        }
        _ => {
            println!("✗");
            issues.push("Cortex binary not found in PATH");
        }
    }

    // Check config file
    print!("Checking configuration... ");
    if PathBuf::from("axon.json").exists() {
        println!("✓");
    } else {
        println!("✗");
        issues.push("Configuration file not found");

        if fix {
            // Create default config
            let config = json!({
                "agents": {
                    "default_model": "gpt-4",
                    "max_concurrent": 10
                }
            });
            fs::write("axon.json", serde_json::to_string_pretty(&config)?).await?;
            println!("  → Created default configuration");
        }
    }

    // Check directories
    print!("Checking workspace directories... ");
    let dirs = ["agents", "workflows", "logs", "data"];
    let mut missing_dirs = Vec::new();

    for dir in &dirs {
        if !PathBuf::from(dir).exists() {
            missing_dirs.push(*dir);
        }
    }

    if missing_dirs.is_empty() {
        println!("✓");
    } else {
        println!("✗");
        issues.push("Missing workspace directories");

        if fix {
            for dir in missing_dirs {
                fs::create_dir_all(dir).await?;
                println!("  → Created directory: {}", dir);
            }
        }
    }

    // Check server connectivity
    print!("Checking API server... ");
    if SERVER_MANAGER.get_server_pid().await.is_some() {
        // Try to connect
        match reqwest::get("http://localhost:3000/api/v1/health").await {
            Ok(resp) if resp.status().is_success() => {
                println!("✓");
            }
            _ => {
                println!("✗");
                issues.push("API server not responding");
            }
        }
    } else {
        println!("✗ (not running)");
    }

    println!();

    if issues.is_empty() {
        println!("✅ All checks passed!");
    } else {
        println!("Issues found:");
        for issue in &issues {
            println!("  - {}", issue);
        }

        if !fix {
            println!();
            println!("Run with --fix to attempt automatic repairs");
        }
    }

    Ok(())
}

pub async fn doctor_health() -> Result<()> {
    let health = json!({
        "status": "healthy",
        "agents": RUNTIME_MANAGER.list_agents(None).await?.len(),
        "workflows": RUNTIME_MANAGER.list_workflows(None).await?.len(),
        "server": SERVER_MANAGER.get_server_pid().await.is_some(),
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    println!("{}", serde_json::to_string_pretty(&health)?);
    Ok(())
}

// Export commands
pub async fn export_metrics(output: PathBuf, format: String) -> Result<()> {
    let metrics = RUNTIME_MANAGER.get_all_metrics().await?;

    let content = match format.as_str() {
        "json" => serde_json::to_string_pretty(&metrics)?,
        "yaml" => serde_yaml::to_string(&metrics)?,
        "csv" => {
            let mut csv = String::from("agent_id,tasks_completed,tasks_failed,avg_response_time_ms,memory_mb,cpu_percent\n");
            for (id, m) in metrics {
                csv.push_str(&format!(
                    "{},{},{},{:.2},{},{:.1}\n",
                    id, m.tasks_completed, m.tasks_failed,
                    m.avg_response_time_ms, m.memory_usage_mb, m.cpu_usage_percent
                ));
            }
            csv
        }
        _ => return Err(anyhow::anyhow!("Unsupported format: {}", format)),
    };

    fs::write(&output, content).await?;
    println!("✓ Metrics exported to: {}", output.display());

    Ok(())
}

pub async fn export_workflows(output: PathBuf, format: String) -> Result<()> {
    let workflows = RUNTIME_MANAGER.list_workflows(None).await?;

    let content = match format.as_str() {
        "json" => serde_json::to_string_pretty(&workflows)?,
        "yaml" => serde_yaml::to_string(&workflows)?,
        "csv" => {
            let mut csv = String::from("id,name,status,started_at,completed_at\n");
            for wf in workflows {
                csv.push_str(&format!(
                    "{},{},{},{},{}\n",
                    wf.id, wf.name, wf.status, wf.started_at,
                    wf.completed_at.as_deref().unwrap_or("")
                ));
            }
            csv
        }
        _ => return Err(anyhow::anyhow!("Unsupported format: {}", format)),
    };

    fs::write(&output, content).await?;
    println!("✓ Workflows exported to: {}", output.display());

    Ok(())
}

// Interactive mode
pub async fn interactive_mode(mode: String) -> Result<()> {
    match mode.as_str() {
        "wizard" => {
            println!("Welcome to Axon Setup Wizard!");
            println!("=============================");
            println!();

            // Interactive setup wizard
            use dialoguer::{Input, Select, Confirm};

            let name: String = Input::new()
                .with_prompt("Workspace name")
                .default("my-workspace".into())
                .interact_text()?;

            let _model = Select::new()
                .with_prompt("Default model")
                .items(&["gpt-4", "gpt-3.5-turbo", "claude-2", "llama-2"])
                .default(0)
                .interact()?;

            let _enable_cortex = Confirm::new()
                .with_prompt("Enable Cortex cognitive memory?")
                .default(true)
                .interact()?;

            init_workspace(name, None).await?;

            println!();
            println!("Setup complete! You can now start using Axon.");
        }
        "dashboard" => {
            monitor_dashboard(5).await?;
        }
        _ => {
            return Err(anyhow::anyhow!("Unknown interactive mode: {}", mode));
        }
    }

    Ok(())
}

// ============================================================================
// MCP Commands
// ============================================================================

/// Initialize file-only logging (no stdout/stderr output)
fn init_file_logging(log_file: &str, log_level: &str) -> Result<()> {
    use tracing_subscriber::fmt;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

    // Create log directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(log_file).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Open log file for appending
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;

    // Create filter from log level
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

    // Initialize file-only subscriber (NO stdout/stderr!)
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(Arc::new(file)))
        .try_init()
        .map_err(|e| anyhow::anyhow!("Failed to initialize logging: {}", e))?;

    Ok(())
}

/// Start MCP server in stdio mode
pub async fn mcp_stdio() -> Result<()> {
    // Get log file path - use a default location in the user's home directory
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let log_file = format!("{}/.axon/logs/mcp-stdio.log", home);
    let log_level = "axon=info,warn";

    // Initialize file logging for stdio mode (NO stdout/stderr output!)
    init_file_logging(&log_file, log_level)?;

    tracing::info!("Starting Axon MCP Server (stdio mode)");
    tracing::info!("Log file: {}", log_file);

    // Get Cortex URL from environment or use default
    let cortex_url = std::env::var("CORTEX_MCP_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    // Parse URL to extract host and port for auto-start
    let parsed_url = url::Url::parse(&cortex_url)
        .context("Failed to parse CORTEX_MCP_URL")?;
    let host = parsed_url.host_str().unwrap_or("localhost");
    let port = parsed_url.port().unwrap_or(8080);

    // Ensure Cortex HTTP server is running (auto-start if needed)
    ensure_cortex_server_running(&cortex_url, host, port).await?;

    let working_dir = std::env::current_dir()?;

    // Create MCP server configuration
    let config = crate::mcp_server::McpServerConfig {
        name: "axon-mcp".to_string(),
        version: crate::VERSION.to_string(),
        cortex_url: cortex_url.clone(),
        working_dir,
        max_concurrent_agents: 10,
        default_timeout_secs: 3600,
    };

    // Initialize Cortex bridge
    let cortex_config = crate::cortex_bridge::CortexConfig {
        base_url: cortex_url,
        ..Default::default()
    };

    let cortex = match crate::cortex_bridge::CortexBridge::new(cortex_config).await {
        Ok(bridge) => {
            tracing::info!("Successfully connected to Cortex");
            Arc::new(bridge)
        }
        Err(e) => {
            tracing::error!("Failed to connect to Cortex after auto-start: {}", e);
            return Err(anyhow::anyhow!("Cortex bridge initialization failed: {}", e));
        }
    };

    // Create and run MCP server
    let server = crate::mcp_server::AxonMcpServer::new(config, cortex);

    tracing::info!("MCP server started successfully, listening on stdio");

    server.run().await?;
    Ok(())
}

/// Start MCP server in HTTP mode
pub async fn mcp_http(address: String, port: u16) -> Result<()> {
    // Get log file path
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let log_file = format!("{}/.axon/logs/mcp-http.log", home);
    let log_level = "axon=info,warn";

    // Initialize file logging for HTTP mode
    init_file_logging(&log_file, log_level)?;

    println!("Starting Axon MCP Server (HTTP mode)");
    println!("Address: {}", address);
    println!("Port: {}", port);
    println!("Initializing server...");

    // Get Cortex URL from environment or use default
    let cortex_url = std::env::var("CORTEX_MCP_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    // Parse URL to extract host and port for auto-start
    let parsed_url = url::Url::parse(&cortex_url)
        .context("Failed to parse CORTEX_MCP_URL")?;
    let cortex_host = parsed_url.host_str().unwrap_or("localhost");
    let cortex_port = parsed_url.port().unwrap_or(8080);

    // Ensure Cortex HTTP server is running (auto-start if needed)
    println!("Checking Cortex HTTP server at {}...", cortex_url);
    ensure_cortex_server_running(&cortex_url, cortex_host, cortex_port).await?;

    let working_dir = std::env::current_dir()?;

    // Create MCP server configuration
    let config = crate::mcp_server::McpServerConfig {
        name: "axon-mcp".to_string(),
        version: crate::VERSION.to_string(),
        cortex_url: cortex_url.clone(),
        working_dir,
        max_concurrent_agents: 10,
        default_timeout_secs: 3600,
    };

    // Initialize Cortex bridge
    let cortex_config = crate::cortex_bridge::CortexConfig {
        base_url: cortex_url,
        ..Default::default()
    };
    let cortex = Arc::new(crate::cortex_bridge::CortexBridge::new(cortex_config).await?);

    // Create MCP server
    let server = crate::mcp_server::AxonMcpServer::new(config, cortex);

    let bind_addr = format!("{}:{}", address, port);

    println!("✓ MCP server started successfully");
    println!("Listening on http://{}", bind_addr);
    println!("Log file: {}", log_file);
    println!("Press Ctrl+C to stop");

    tracing::info!("MCP HTTP server started on {}", bind_addr);
    tracing::info!("Log file: {}", log_file);

    server.serve_http(&bind_addr).await?;
    Ok(())
}

/// Show information about available MCP tools
pub async fn mcp_info(detailed: bool, category: Option<String>) -> Result<()> {
    println!("Axon MCP Server - Tools Information");
    println!("====================================");
    println!();

    // Tool categories and descriptions
    let categories = vec![
        ("Agent Management", 3, vec![
            ("axon.agent.launch", "Launch a specialized agent (developer, tester, reviewer, etc.) to perform a specific task"),
            ("axon.agent.status", "Check the status of a running agent by agent_id"),
            ("axon.agent.stop", "Stop a running agent by agent_id"),
        ]),
        ("Orchestration", 1, vec![
            ("axon.orchestrate.task", "Orchestrate a complex task across multiple specialized agents"),
        ]),
        ("Knowledge Integration", 1, vec![
            ("axon.cortex.query", "Query the Cortex knowledge graph for code, patterns, and semantic information"),
        ]),
        ("Session Management", 2, vec![
            ("axon.session.create", "Create an isolated work session for experimental changes"),
            ("axon.session.merge", "Merge a session's changes back into the main workspace"),
        ]),
    ];

    // Filter by category if specified
    let filtered_categories: Vec<_> = if let Some(ref filter) = category {
        categories.into_iter()
            .filter(|(cat, _, _)| cat.to_lowercase().contains(&filter.to_lowercase()))
            .collect()
    } else {
        categories
    };

    if filtered_categories.is_empty() {
        println!("No categories found matching: {}", category.unwrap());
        return Ok(());
    }

    // Calculate total tools
    let total_tools: usize = filtered_categories.iter().map(|(_, count, _)| count).sum();

    println!("Total Tools: {}", total_tools);
    println!("Categories: {}", filtered_categories.len());
    println!();

    for (cat_name, count, tools) in filtered_categories {
        println!("## {} ({} tools)", cat_name, count);
        println!();

        if detailed {
            for (tool_name, description) in tools {
                println!("  - {}", tool_name);
                println!("    {}", description);
                println!();
            }
        } else {
            for (tool_name, _) in tools {
                println!("  - {}", tool_name);
            }
            println!();
        }
    }

    println!("Usage:");
    println!("  axon mcp stdio              Start MCP server in stdio mode");
    println!("  axon mcp http               Start MCP server in HTTP mode");
    println!("  axon mcp info --detailed    Show detailed tool information");
    println!();

    Ok(())
}

// ============================================================================
// Cortex Server Auto-Start Helpers
// ============================================================================

/// Check if Cortex HTTP server is available at the given URL
async fn check_cortex_http_available(url: &str) -> bool {
    let health_url = format!("{}/v3/health", url);

    match reqwest::get(&health_url).await {
        Ok(response) => {
            if response.status().is_success() {
                tracing::info!("Cortex HTTP server is available at {}", url);
                true
            } else {
                tracing::debug!("Cortex HTTP server returned status: {}", response.status());
                false
            }
        }
        Err(e) => {
            tracing::debug!("Cortex HTTP server not available at {}: {}", url, e);
            false
        }
    }
}

/// Find the cortex binary in various locations
fn find_cortex_binary() -> Result<PathBuf> {
    // Check if cortex is in PATH
    if let Ok(cortex_path) = which::which("cortex") {
        tracing::info!("Found cortex binary in PATH: {}", cortex_path.display());
        return Ok(cortex_path);
    }

    // Check common locations relative to current directory
    let locations = vec![
        PathBuf::from("./dist/cortex"),
        PathBuf::from("./target/release/cortex"),
        PathBuf::from("../dist/cortex"),
        PathBuf::from("../target/release/cortex"),
    ];

    for location in locations {
        if location.exists() {
            tracing::info!("Found cortex binary at: {}", location.display());
            return Ok(location);
        }
    }

    Err(anyhow::anyhow!(
        "Cortex binary not found. Please ensure 'cortex' is in PATH or built in ./dist or ./target/release"
    ))
}

/// Start the Cortex HTTP server in the background
async fn start_cortex_server(host: &str, port: u16) -> Result<()> {
    let cortex_binary = find_cortex_binary()?;

    tracing::info!("Starting Cortex HTTP server on {}:{}", host, port);

    // Start cortex server in background
    let child = Command::new(&cortex_binary)
        .arg("server")
        .arg("start")
        .arg("--host")
        .arg(host)
        .arg("--port")
        .arg(port.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("Failed to spawn cortex server process")?;

    let pid = child.id().unwrap_or(0);
    tracing::info!("Cortex HTTP server started with PID: {}", pid);

    Ok(())
}

/// Ensure Cortex HTTP server is running, starting it if necessary
async fn ensure_cortex_server_running(url: &str, host: &str, port: u16) -> Result<()> {
    // First check if already running
    if check_cortex_http_available(url).await {
        tracing::info!("Cortex HTTP server is already running at {}", url);
        return Ok(());
    }

    tracing::info!("Cortex HTTP server not detected at {}, attempting to start it...", url);

    // Start the server
    start_cortex_server(host, port).await?;

    // Wait for server to become available (max 30 seconds)
    let max_attempts = 30;
    let delay_secs = 1;

    for attempt in 1..=max_attempts {
        tracing::debug!("Checking if Cortex is ready (attempt {}/{})", attempt, max_attempts);

        tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs)).await;

        if check_cortex_http_available(url).await {
            tracing::info!("Cortex HTTP server is now running at {}", url);
            return Ok(());
        }
    }

    Err(anyhow::anyhow!(
        "Timeout waiting for Cortex HTTP server to start at {}. Check logs for details.",
        url
    ))
}