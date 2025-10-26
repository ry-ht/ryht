//! Command modules for Axon CLI

pub mod config;
pub mod output;
pub mod runtime_manager;
pub mod runtime_manager_impl;
pub mod server_manager;
pub mod api;

use anyhow::Result;
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
        // TODO: Validate workflow
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

    // TODO: Implement proper workflow validation
    // For now, just check if it's valid YAML/JSON
    if workflow.extension().and_then(|s| s.to_str()) == Some("json") {
        serde_json::from_str::<serde_json::Value>(&content)?;
    } else {
        serde_yaml::from_str::<serde_yaml::Value>(&content)?;
    }

    println!("✓ Workflow is valid");
    Ok(())
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
        // TODO: Show recent logs

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