//! Example demonstrating the use of the Cortex configuration system.
//!
//! This example shows how to:
//! - Load or create default configuration
//! - Access and modify configuration values
//! - Save configuration changes
//! - Use environment variable overrides
//! - Validate configuration
//! - Use configuration profiles (dev, prod, test)
//! - Hot-reload configuration changes
//! - Import/export configuration
//! - Migrate configuration versions
//! - Use the thread-safe ConfigManager singleton

use cortex_core::config::{GlobalConfig, ConfigManager, ConfigProfile};
use cortex_core::Result;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Cortex Configuration Example ===\n");

    // 1. Load or create default configuration
    println!("1. Loading or creating default configuration...");
    let mut config = GlobalConfig::load_or_create_default().await?;
    println!("   Configuration loaded from: {}", GlobalConfig::config_path()?.display());
    println!("   Current log level: {}", config.general().log_level);
    println!("   Database mode: {}", config.database().mode);
    println!();

    // 2. Display all directory paths
    println!("2. Configuration directories:");
    println!("   Base:       {}", GlobalConfig::base_dir()?.display());
    println!("   SurrealDB:  {}", GlobalConfig::surrealdb_dir()?.display());
    println!("   Cache:      {}", GlobalConfig::cache_dir()?.display());
    println!("   Sessions:   {}", GlobalConfig::sessions_dir()?.display());
    println!("   Temp:       {}", GlobalConfig::temp_dir()?.display());
    println!("   Data:       {}", GlobalConfig::data_dir()?.display());
    println!("   Logs:       {}", GlobalConfig::logs_dir()?.display());
    println!("   Run:        {}", GlobalConfig::run_dir()?.display());
    println!("   Workspaces: {}", GlobalConfig::workspaces_dir()?.display());
    println!();

    // 3. Access configuration sections
    println!("3. Current configuration:");
    println!("   General:");
    println!("     - Version:   {}", config.general().version);
    println!("     - Log Level: {}", config.general().log_level);
    println!();

    println!("   Database:");
    println!("     - Mode:      {}", config.database().mode);
    println!("     - Bind:      {}", config.database().local_bind);
    println!("     - Namespace: {}", config.database().namespace);
    println!("     - Database:  {}", config.database().database);
    println!();

    println!("   Pool:");
    println!("     - Min Connections: {}", config.pool().min_connections);
    println!("     - Max Connections: {}", config.pool().max_connections);
    println!();

    println!("   Cache:");
    println!("     - Memory Size: {} MB", config.cache().memory_size_mb);
    println!("     - TTL:         {} seconds", config.cache().ttl_seconds);
    println!();

    println!("   VFS:");
    println!("     - Max File Size: {} MB", config.vfs().max_file_size_mb);
    println!("     - Auto Flush:    {}", config.vfs().auto_flush);
    println!();

    println!("   Ingestion:");
    println!("     - Workers:     {}", config.ingestion().parallel_workers);
    println!("     - Chunk Size:  {}", config.ingestion().chunk_size);
    println!("     - Embeddings:  {}", config.ingestion().generate_embeddings);
    println!("     - Model:       {}", config.ingestion().embedding_model);
    println!();

    println!("   MCP:");
    println!("     - Server Bind: {}", config.mcp().server_bind);
    println!("     - CORS:        {}", config.mcp().cors_enabled);
    println!();

    // 4. Modify configuration
    println!("4. Modifying configuration...");
    config.general_mut().log_level = "debug".to_string();
    config.pool_mut().max_connections = 20;
    config.cache_mut().memory_size_mb = 1024;
    println!("   - Set log level to 'debug'");
    println!("   - Increased max connections to 20");
    println!("   - Increased cache size to 1024 MB");
    println!();

    // 5. Validate configuration
    println!("5. Validating configuration...");
    match config.validate() {
        Ok(_) => println!("   ✓ Configuration is valid"),
        Err(e) => println!("   ✗ Configuration error: {}", e),
    }
    println!();

    // 6. Save configuration
    println!("6. Saving configuration...");
    config.save().await?;
    println!("   ✓ Configuration saved successfully");
    println!();

    // 7. Demonstrate environment variable overrides
    println!("7. Environment variable overrides:");
    println!("   Set the following environment variables to override configuration:");
    println!("   - CORTEX_LOG_LEVEL=trace");
    println!("   - CORTEX_DB_MODE=remote");
    println!("   - CORTEX_DB_URL=ws://remote.example.com:8000");
    println!("   - CORTEX_CACHE_SIZE_MB=2048");
    println!();

    // 8. Example: Production configuration
    println!("8. Example production configuration:");
    let mut prod_config = GlobalConfig::default();

    // Set production values
    prod_config.general_mut().log_level = "warn".to_string();
    prod_config.database_mut().mode = "hybrid".to_string();
    prod_config.database_mut().remote_urls = vec![
        "ws://primary.example.com:8000".to_string(),
        "ws://backup.example.com:8000".to_string(),
    ];
    prod_config.pool_mut().min_connections = 5;
    prod_config.pool_mut().max_connections = 50;
    prod_config.cache_mut().memory_size_mb = 4096;
    prod_config.cache_mut().redis_url = "redis://cache.example.com:6379".to_string();
    prod_config.ingestion_mut().parallel_workers = 16;
    prod_config.mcp_mut().max_request_size_mb = 50;

    println!("   Production settings:");
    println!("   - Log level: {}", prod_config.general().log_level);
    println!("   - Database mode: {}", prod_config.database().mode);
    println!("   - Remote URLs: {:?}", prod_config.database().remote_urls);
    println!("   - Pool size: {}-{}", prod_config.pool().min_connections, prod_config.pool().max_connections);
    println!("   - Cache: {} MB + Redis", prod_config.cache().memory_size_mb);
    println!("   - Workers: {}", prod_config.ingestion().parallel_workers);
    println!();

    // Validate production config
    match prod_config.validate() {
        Ok(_) => println!("   ✓ Production configuration is valid"),
        Err(e) => println!("   ✗ Production configuration error: {}", e),
    }
    println!();

    // 9. Example: Development configuration
    println!("9. Example development configuration:");
    let mut dev_config = GlobalConfig::default();

    dev_config.general_mut().log_level = "debug".to_string();
    dev_config.database_mut().mode = "local".to_string();
    dev_config.pool_mut().max_connections = 5;
    dev_config.cache_mut().memory_size_mb = 256;
    dev_config.ingestion_mut().parallel_workers = 2;

    println!("   Development settings:");
    println!("   - Log level: {}", dev_config.general().log_level);
    println!("   - Database mode: {}", dev_config.database().mode);
    println!("   - Pool size: {}", dev_config.pool().max_connections);
    println!("   - Cache: {} MB", dev_config.cache().memory_size_mb);
    println!("   - Workers: {}", dev_config.ingestion().parallel_workers);
    println!();

    // 10. Configuration Profiles
    println!("10. Configuration Profiles:");

    let dev_profile = GlobalConfig::with_profile(ConfigProfile::Dev);
    println!("   Dev Profile:");
    println!("     - Log Level: {}", dev_profile.general().log_level);
    println!("     - Hot Reload: {}", dev_profile.general().hot_reload);
    println!("     - Max Connections: {}", dev_profile.pool().max_connections);
    println!("     - Cache Size: {} MB", dev_profile.cache().memory_size_mb);
    println!();

    let prod_profile = GlobalConfig::with_profile(ConfigProfile::Prod);
    println!("   Prod Profile:");
    println!("     - Log Level: {}", prod_profile.general().log_level);
    println!("     - Hot Reload: {}", prod_profile.general().hot_reload);
    println!("     - Max Connections: {}", prod_profile.pool().max_connections);
    println!("     - Cache Size: {} MB", prod_profile.cache().memory_size_mb);
    println!();

    let test_profile = GlobalConfig::with_profile(ConfigProfile::Test);
    println!("   Test Profile:");
    println!("     - Log Level: {}", test_profile.general().log_level);
    println!("     - Database Namespace: {}", test_profile.database().namespace);
    println!("     - Max Connections: {}", test_profile.pool().max_connections);
    println!("     - Cache Size: {} MB", test_profile.cache().memory_size_mb);
    println!();

    // 11. Import/Export Configuration
    println!("11. Import/Export Configuration:");

    // Export to JSON
    let json_export = config.export_json()?;
    println!("   ✓ Exported to JSON ({} bytes)", json_export.len());

    // Export to TOML
    let toml_export = config.export_toml()?;
    println!("   ✓ Exported to TOML ({} bytes)", toml_export.len());

    // Import from JSON
    let imported_config = GlobalConfig::import_json(&json_export)?;
    println!("   ✓ Imported from JSON");
    println!("     - Version: {}", imported_config.general().version);
    println!();

    // 12. Configuration Migration
    println!("12. Configuration Migration:");
    let migrated = config.clone().migrate()?;
    println!("   ✓ Migration check completed");
    println!("     - Current version: {}", migrated.general().version);
    println!();

    // 13. Configuration Metadata
    println!("13. Configuration Metadata:");
    let metadata = config.metadata();
    println!("   - Version: {}", metadata.version);
    println!("   - Profile: {}", metadata.profile);
    println!("   - Created at: {}", metadata.created_at);
    println!();

    // 14. Thread-Safe ConfigManager
    println!("14. Thread-Safe ConfigManager:");
    println!("   Creating ConfigManager for concurrent access...");

    let manager = Arc::new(ConfigManager::new(
        config.clone(),
        GlobalConfig::config_path()?,
    ));

    // Read access
    {
        let read_config = manager.read().await;
        println!("   - Read access acquired");
        println!("     - Log level: {}", read_config.general().log_level);
    }

    // Write access
    {
        let mut write_config = manager.write().await;
        write_config.general_mut().log_level = "trace".to_string();
        println!("   - Write access acquired");
        println!("     - Updated log level to: trace");
    }

    // Update with closure
    manager.update(|cfg| {
        cfg.pool_mut().max_connections = 30;
        Ok(())
    }).await?;
    println!("   - Updated max_connections to 30 using closure");
    println!();

    // 15. Hot-Reload Support
    println!("15. Hot-Reload Support:");
    println!("   Hot-reload is configured with:");
    println!("     - Enabled: {}", config.general().hot_reload);
    println!("     - Check interval: {} seconds", config.general().hot_reload_interval_secs);
    println!("   To start hot-reload monitoring:");
    println!("     let manager = Arc::new(ConfigManager::new(config, path));");
    println!("     manager.clone().start_hot_reload().await?;");
    println!();

    println!("=== Example Complete ===");
    println!();
    println!("Additional Features:");
    println!("- Set CORTEX_CONFIG_PROFILE=prod to use production profile");
    println!("- Set CORTEX_CONFIG_PATH to override config location");
    println!("- Use ConfigManager::global() for singleton access");
    println!("- Configuration automatically validates on load and save");
    println!("- Atomic writes ensure configuration consistency");

    Ok(())
}
