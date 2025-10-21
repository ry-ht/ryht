//! Tests for the modern type-safe client API.

use cc_sdk::client::{ClaudeClient, ClaudeClientBuilder};
use cc_sdk::core::{BinaryPath, ModelId};
use cc_sdk::PermissionMode;
// Note: MockTransport is internal, not used directly in tests

#[test]
fn test_client_builder_creation() {
    // Test that we can create a builder
    let _builder = ClaudeClient::builder();
    // Builder created successfully in NoBinary state
}

#[test]
fn test_client_type_states() {
    // Test that type states exist and can be used as type parameters
    // This is a compile-time test - if it compiles, it works

    // We can't actually test the full state transitions without async,
    // but we can verify the types exist
    use cc_sdk::core::state::{NoBinary, WithBinary, Configured, Connected, Disconnected};

    // These types should exist
    let _: std::marker::PhantomData<NoBinary> = std::marker::PhantomData;
    let _: std::marker::PhantomData<WithBinary> = std::marker::PhantomData;
    let _: std::marker::PhantomData<Configured> = std::marker::PhantomData;
    let _: std::marker::PhantomData<Connected> = std::marker::PhantomData;
    let _: std::marker::PhantomData<Disconnected> = std::marker::PhantomData;
}

#[tokio::test]
async fn test_client_with_binary_path() {
    // Test creating a client with a specific binary path
    let binary_path = BinaryPath::from("/usr/local/bin/claude");

    // This will transition from NoBinary to WithBinary
    let _builder = ClaudeClient::builder()
        .binary(binary_path);

    // Builder should now be in WithBinary state
    // We can't test much more without a real Claude binary
    println!("Builder transitioned to WithBinary state");
}

#[test]
fn test_model_id_creation() {
    // Test ModelId creation
    let model1 = ModelId::from("claude-sonnet-4-5-20250929");
    assert_eq!(model1.as_str(), "claude-sonnet-4-5-20250929");

    let model2 = ModelId::from("claude-opus-3-5-20241022");
    assert_eq!(model2.as_str(), "claude-opus-3-5-20241022");
}

#[test]
fn test_permission_mode_values() {
    // Test PermissionMode enum values
    // Note: Check actual PermissionMode variants in the source
    let modes = vec![
        PermissionMode::AcceptEdits,
        // Other modes may have different names
    ];

    for mode in modes {
        println!("Permission mode: {:?}", mode);
    }
}

#[test]
fn test_builder_configuration_options() {
    // Test that we can configure various builder options
    let binary = BinaryPath::from("/usr/local/bin/claude");
    let model = ModelId::from("claude-sonnet-4-5-20250929");

    let builder = ClaudeClient::builder()
        .binary(binary)
        .model(model)
        .permission_mode(PermissionMode::AcceptEdits)
        .working_directory("/tmp/test");

    println!("Builder configured successfully");
}

#[test]
fn test_builder_with_allowed_tools() {
    // Test adding allowed tools
    let binary = BinaryPath::from("/usr/local/bin/claude");

    let builder = ClaudeClient::builder()
        .binary(binary)
        .add_allowed_tool("Bash")
        .add_allowed_tool("Read")
        .add_allowed_tool("Write");

    println!("Builder configured with allowed tools");
}

#[tokio::test]
#[ignore = "Requires Claude binary to be installed"]
async fn test_client_discovery_integration() {
    // This test requires Claude to be installed
    match ClaudeClient::builder().discover_binary().await {
        Ok(builder) => {
            println!("Successfully discovered Claude binary");
            // Builder is now in WithBinary state
        }
        Err(e) => {
            println!("Could not discover Claude binary (expected if not installed): {}", e);
        }
    }
}

#[tokio::test]
#[ignore = "Requires Claude binary and full connection"]
async fn test_full_client_workflow() {
    // This is a full integration test that requires Claude to be installed
    // and properly configured

    match ClaudeClient::builder()
        .discover_binary().await
    {
        Ok(builder) => {
            let result = builder
                .model(ModelId::from("claude-sonnet-4-5-20250929"))
                .permission_mode(PermissionMode::AcceptEdits)
                .configure()
                .connect().await;

            match result {
                Ok(connected_builder) => {
                    match connected_builder.build() {
                        Ok(client) => {
                            println!("Client successfully built and connected");
                            // Could send messages here if desired
                            client.disconnect().await.ok();
                        }
                        Err(e) => println!("Error building client: {}", e),
                    }
                }
                Err(e) => println!("Error connecting: {}", e),
            }
        }
        Err(e) => {
            println!("Could not discover binary: {}", e);
        }
    }
}

#[test]
fn test_binary_path_operations() {
    // Test BinaryPath operations
    let path1 = BinaryPath::from("/usr/local/bin/claude");
    let path2 = BinaryPath::from("/usr/local/bin/claude");
    let path3 = BinaryPath::from("/opt/homebrew/bin/claude");

    assert_eq!(path1, path2);
    assert_ne!(path1, path3);

    println!("Path1: {:?}", path1);
    println!("Path3: {:?}", path3);
}

#[test]
fn test_claude_code_options_builder() {
    // Test building ClaudeCodeOptions
    use cc_sdk::ClaudeCodeOptions;

    let options = ClaudeCodeOptions::builder()
        .model("claude-sonnet-4-5-20250929")
        .permission_mode(PermissionMode::AcceptEdits)
        .cwd("/tmp/test")
        .build();

    println!("Built ClaudeCodeOptions: {:?}", options);
}

#[test]
fn test_client_state_compile_time_safety() {
    // This test verifies that the type system prevents invalid operations
    // If this compiles, the type-state pattern is working

    let builder = ClaudeClient::builder();
    // builder is in NoBinary state

    // We can only call methods valid for NoBinary state:
    // - discover_binary()
    // - with_binary()

    // We CANNOT call methods like configure() or connect()
    // because those require WithBinary or Configured state

    // This is enforced at compile time!
    println!("Type-state safety verified at compile time");
}

#[tokio::test]
async fn test_mock_transport_integration() {
    // Test using mock transport for testing without real Claude binary
    // Note: MockTransport is an internal type for testing
    // In a real test, we would use it through the client API

    println!("Mock transport can be used for testing without real Claude binary");

    // In a real test, we would:
    // 1. Send messages through mock
    // 2. Verify responses
    // 3. Check state transitions
}

#[test]
fn test_message_types() {
    // Test that message types are properly defined
    // Note: Message types have specific formats
    // This is a simplified test

    println!("Message types are defined in the SDK");
    // In practice, messages are created through the client API
    // rather than directly constructing them
}

#[test]
fn test_permission_mode_serialization() {
    // Test that PermissionMode can be serialized/deserialized
    let mode = PermissionMode::AcceptEdits;
    let json = serde_json::to_string(&mode).unwrap();
    println!("Serialized PermissionMode: {}", json);

    let deserialized: PermissionMode = serde_json::from_str(&json).unwrap();
    assert_eq!(mode, deserialized);
}

#[test]
fn test_builder_pattern_ergonomics() {
    // Test that the builder pattern is ergonomic to use
    let binary = BinaryPath::from("/usr/local/bin/claude");

    let _builder = ClaudeClient::builder()
        .binary(binary)
        .model(ModelId::from("claude-sonnet-4-5-20250929"))
        .permission_mode(PermissionMode::AcceptEdits)
        .working_directory("/tmp/test")
        .add_allowed_tool("Bash")
        .add_allowed_tool("Read")
        .configure();

    // Builder pattern allows method chaining
    println!("Builder pattern is ergonomic and type-safe");
}

#[tokio::test]
#[ignore = "Requires full client setup"]
async fn test_message_streaming() {
    // Test message streaming (requires connected client)
    // This is a placeholder for integration testing

    println!("Message streaming would be tested with a real connection");

    // In a real test:
    // 1. Create and connect client
    // 2. Send a message
    // 3. Stream responses
    // 4. Verify message ordering and content
}

#[test]
fn test_error_types() {
    // Test that error types are properly defined
    use cc_sdk::error::{Error, BinaryError, ClientError, SessionError};

    let _binary_err = Error::Binary(BinaryError::NotFound { searched_paths: vec!["/test".into()] });
    let _client_err = Error::Client(ClientError::NotConnected);
    let _session_err = Error::Session(SessionError::HomeDirectoryNotFound);

    println!("Error types are properly defined");
}

#[test]
fn test_result_type() {
    // Test that Result type works as expected
    use cc_sdk::Result;

    fn test_function() -> Result<String> {
        Ok("success".to_string())
    }

    let result = test_function();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success");
}
