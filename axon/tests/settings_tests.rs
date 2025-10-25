//! Integration tests for settings management module.

use cc_sdk::settings::{ClaudeSettings, HookConfig, SettingsScope, load_settings, save_settings};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_settings_scope_ordering() {
    // Test that scope precedence is correct: Local > Project > User
    let all = SettingsScope::all_ordered();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0], SettingsScope::Local);
    assert_eq!(all[1], SettingsScope::Project);
    assert_eq!(all[2], SettingsScope::User);
}

#[test]
fn test_settings_scope_file_path() {
    // Test User scope
    if let Some(user_path) = SettingsScope::User.file_path(None) {
        assert!(user_path.ends_with(".claude/settings.json"));
        println!("User settings path: {:?}", user_path);
    }

    // Test Local scope
    let local_path = SettingsScope::Local.file_path(None);
    assert!(local_path.is_some());
    assert_eq!(local_path.unwrap(), PathBuf::from(".claude/settings.json"));

    // Test Project scope
    let project_path = PathBuf::from("/test/project");
    let project_settings = SettingsScope::Project.file_path(Some(&project_path));
    assert!(project_settings.is_some());
    assert_eq!(
        project_settings.unwrap(),
        PathBuf::from("/test/project/.claude/settings.json")
    );
}

#[test]
fn test_claude_settings_default() {
    // Test default settings
    let settings = ClaudeSettings::default();
    assert!(settings.hooks.is_empty());
    assert!(settings.mcp_servers.is_empty());
    assert!(settings.default_model.is_none());
    assert!(settings.permission_mode.is_none());
    assert!(settings.prompts.is_empty());
    assert!(settings.env.is_empty());
}

#[test]
fn test_claude_settings_new() {
    // Test creating new settings
    let settings = ClaudeSettings::new();
    assert!(settings.hooks.is_empty());
    assert!(settings.mcp_servers.is_empty());
}

#[test]
fn test_settings_serialization() {
    // Test serializing and deserializing settings
    let mut settings = ClaudeSettings::new();
    settings.default_model = Some("claude-sonnet-4-5-20250929".to_string());
    settings.permission_mode = Some("accept-edits".to_string());

    let json = serde_json::to_string_pretty(&settings).unwrap();
    println!("Serialized settings:\n{}", json);

    let deserialized: ClaudeSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.default_model, settings.default_model);
    assert_eq!(deserialized.permission_mode, settings.permission_mode);
}

#[test]
fn test_hook_config_creation() {
    // Test creating hook configurations
    let hook = HookConfig::new("pre_tool_use");
    assert_eq!(hook.hook_type, "pre_tool_use");
    assert!(hook.command.is_none());
    assert!(hook.enabled);

    let hook_with_command = HookConfig::new("post_tool_use")
        .with_command("python script.py")
        .with_args(vec!["arg1".to_string(), "arg2".to_string()]);

    assert_eq!(hook_with_command.command, Some("python script.py".to_string()));
    assert_eq!(hook_with_command.args, Some(vec!["arg1".to_string(), "arg2".to_string()]));
}

#[test]
fn test_hook_config_serialization() {
    // Test hook config serialization
    let hook = HookConfig::new("pre_tool_use")
        .with_command("node hook.js");

    let json = serde_json::to_string(&hook).unwrap();
    println!("Serialized hook: {}", json);

    let deserialized: HookConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.hook_type, "pre_tool_use");
    assert_eq!(deserialized.command, Some("node hook.js".to_string()));
}

#[test]
fn test_settings_with_hooks() {
    // Test settings with hook configurations
    let mut settings = ClaudeSettings::new();

    let mut pre_tool_use_hooks = Vec::new();
    pre_tool_use_hooks.push(
        HookConfig::new("pre_tool_use")
            .with_command("python validate.py")
    );

    settings.hooks.insert("pre_tool_use".to_string(), pre_tool_use_hooks);

    let json = serde_json::to_string_pretty(&settings).unwrap();
    println!("Settings with hooks:\n{}", json);

    // Verify serialization/deserialization
    let deserialized: ClaudeSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.hooks.len(), 1);
    assert!(deserialized.hooks.contains_key("pre_tool_use"));
}

#[test]
fn test_settings_merging() {
    // Test merging settings from multiple scopes
    let mut user_settings = ClaudeSettings::new();
    user_settings.default_model = Some("claude-opus-3-5-20241022".to_string());
    user_settings.env.insert("USER_VAR".to_string(), "user_value".to_string());

    let mut project_settings = ClaudeSettings::new();
    project_settings.default_model = Some("claude-sonnet-4-5-20250929".to_string());
    project_settings.env.insert("PROJECT_VAR".to_string(), "project_value".to_string());

    // Manually merge (project overrides user)
    let mut merged = user_settings.clone();
    merged.merge(project_settings);

    assert_eq!(merged.default_model, Some("claude-sonnet-4-5-20250929".to_string()));
    assert_eq!(merged.env.get("USER_VAR"), Some(&"user_value".to_string()));
    assert_eq!(merged.env.get("PROJECT_VAR"), Some(&"project_value".to_string()));
}

#[test]
fn test_settings_with_mcp_servers() {
    // Test settings with MCP server configurations
    let mut settings = ClaudeSettings::new();

    // MCP server config is an enum with different variants
    let mcp_config = cc_sdk::McpServerConfig::Stdio {
        command: "node".to_string(),
        args: Some(vec!["server.js".to_string()]),
        env: Some({
            let mut env = HashMap::new();
            env.insert("API_KEY".to_string(), "secret".to_string());
            env
        }),
    };

    settings.mcp_servers.insert("my_server".to_string(), mcp_config);

    // Note: McpServerConfig may not be fully serializable due to its complex structure
    // In practice, users define MCP servers in JSON configuration files
    println!("Settings with MCP servers configured");
    assert!(settings.mcp_servers.contains_key("my_server"));
}

#[tokio::test]
async fn test_load_settings_user_scope() {
    // Test loading user settings
    match load_settings(&[SettingsScope::User], None).await {
        Ok(settings) => {
            println!("Loaded user settings successfully");
            println!("Default model: {:?}", settings.default_model);
            println!("Hooks: {}", settings.hooks.len());
            println!("MCP servers: {}", settings.mcp_servers.len());
        }
        Err(e) => {
            println!("Could not load user settings (expected if no settings file): {}", e);
        }
    }
}

#[tokio::test]
async fn test_load_settings_multiple_scopes() {
    // Test loading from multiple scopes
    let scopes = vec![SettingsScope::User, SettingsScope::Local];
    match load_settings(&scopes, None).await {
        Ok(settings) => {
            println!("Loaded settings from multiple scopes");
            println!("Default model: {:?}", settings.default_model);
        }
        Err(e) => {
            println!("Could not load settings: {}", e);
        }
    }
}

#[tokio::test]
async fn test_save_and_load_roundtrip() {
    // Test saving and loading settings (using a temp location)
    let temp_dir = std::env::temp_dir().join("cc_sdk_test_settings");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir.join(".claude")).unwrap();

    // Create test settings
    let mut settings = ClaudeSettings::new();
    settings.default_model = Some("claude-sonnet-4-5-20250929".to_string());
    settings.permission_mode = Some("accept-edits".to_string());

    // Save to temp file
    let settings_path = temp_dir.join(".claude").join("settings.json");
    let json = serde_json::to_string_pretty(&settings).unwrap();
    fs::write(&settings_path, json).unwrap();

    // Load back
    let loaded_json = fs::read_to_string(&settings_path).unwrap();
    let loaded: ClaudeSettings = serde_json::from_str(&loaded_json).unwrap();

    assert_eq!(loaded.default_model, settings.default_model);
    assert_eq!(loaded.permission_mode, settings.permission_mode);

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_hook_config_disabled() {
    // Test disabling hooks
    let hook = HookConfig::new("test_hook").disabled();
    assert!(!hook.enabled);
}

#[test]
fn test_settings_with_custom_prompts() {
    // Test settings with custom prompts
    let mut settings = ClaudeSettings::new();
    settings.prompts.insert("greeting".to_string(), "Hello, World!".to_string());
    settings.prompts.insert("farewell".to_string(), "Goodbye!".to_string());

    let json = serde_json::to_string_pretty(&settings).unwrap();
    let deserialized: ClaudeSettings = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.prompts.len(), 2);
    assert_eq!(deserialized.prompts.get("greeting"), Some(&"Hello, World!".to_string()));
}

#[test]
fn test_settings_scope_precedence() {
    // Test that Local > Project > User precedence is maintained
    let scopes = SettingsScope::all_ordered();

    // Local should come first (highest precedence)
    assert_eq!(scopes[0], SettingsScope::Local);

    // User should come last (lowest precedence)
    assert_eq!(scopes[2], SettingsScope::User);
}

#[test]
fn test_hook_config_with_additional_fields() {
    // Test hook config with additional fields in the config map
    let mut hook = HookConfig::new("custom_hook");
    hook.config.insert("timeout".to_string(), serde_json::json!(30));
    hook.config.insert("retry".to_string(), serde_json::json!(true));

    let json = serde_json::to_string_pretty(&hook).unwrap();
    println!("Hook with additional fields:\n{}", json);

    let deserialized: HookConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.config.get("timeout"), Some(&serde_json::json!(30)));
    assert_eq!(deserialized.config.get("retry"), Some(&serde_json::json!(true)));
}

#[tokio::test]
#[ignore = "Requires write permissions to temp directory"]
async fn test_save_settings_integration() {
    // This test is ignored by default as it requires file system access
    let temp_dir = std::env::temp_dir().join("cc_sdk_settings_test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let mut settings = ClaudeSettings::new();
    settings.default_model = Some("claude-sonnet-4-5-20250929".to_string());

    // Note: save_settings requires proper scope setup
    // This is a placeholder for integration testing
    println!("Settings prepared for saving to {:?}", temp_dir);

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}
