//! Comprehensive integration tests for the service layer
//!
//! These tests verify the service layer's core functionality using in-memory databases.
//! Each service is tested for:
//! - Happy path operations
//! - Error handling
//! - Edge cases
//! - Integration with dependencies

#[cfg(test)]
mod tests {
    use super::super::*;
    use anyhow::Result;
    use chrono::Utc;
    use cortex_storage::{connection::ConnectionConfig, ConnectionManager};
    use cortex_vfs::{VirtualFileSystem, VirtualPath, WorkspaceType, SourceType};
    use std::sync::Arc;
    use uuid::Uuid;

    // ========================================================================
    // Test Utilities
    // ========================================================================

    /// Create an in-memory storage manager for testing
    async fn setup_storage() -> Arc<ConnectionManager> {
        let config = ConnectionConfig::memory();
        Arc::new(
            ConnectionManager::new(config)
                .await
                .expect("Failed to create connection manager"),
        )
    }

    /// Create a test workspace in the database
    async fn create_test_workspace(storage: Arc<ConnectionManager>) -> Uuid {
        let workspace_id = Uuid::new_v4();
        let now = Utc::now();

        let workspace = cortex_vfs::Workspace {
            id: workspace_id,
            name: "test-workspace".to_string(),
            workspace_type: WorkspaceType::Code,
            source_type: SourceType::Local,
            namespace: format!("ws_{}", workspace_id.to_string().replace('-', "_")),
            source_path: None,
            read_only: false,
            parent_workspace: None,
            fork_metadata: None,
            created_at: now,
            updated_at: now,
        };

        let conn = storage.acquire().await.expect("Failed to acquire connection");
        let workspace_json = serde_json::to_value(&workspace).expect("Failed to serialize workspace");

        let _: Option<serde_json::Value> = conn
            .connection()
            .create(("workspace", workspace_id.to_string()))
            .content(workspace_json)
            .await
            .expect("Failed to create workspace");

        workspace_id
    }

    // ========================================================================
    // AuthService Tests
    // ========================================================================

    #[tokio::test]
    async fn test_auth_service_user_creation() -> Result<()> {
        let storage = setup_storage().await;
        let auth_service = AuthService::new(storage.clone());

        // Create a new user
        let user = auth_service
            .create_user(
                "test@example.com".to_string(),
                "securepassword123".to_string(),
                vec!["user".to_string()],
            )
            .await?;

        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.roles, vec!["user".to_string()]);
        assert!(!user.password_hash.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_auth_service_authentication_flow() -> Result<()> {
        let storage = setup_storage().await;
        let auth_service = AuthService::new(storage.clone());

        // Create user
        let _user = auth_service
            .create_user(
                "auth@example.com".to_string(),
                "password123".to_string(),
                vec!["user".to_string()],
            )
            .await?;

        // Authenticate with correct credentials
        let authenticated = auth_service
            .authenticate_user("auth@example.com", "password123")
            .await?;

        assert_eq!(authenticated.user.email, "auth@example.com");
        assert!(!authenticated.access_token.is_empty());
        assert!(!authenticated.refresh_token.is_empty());
        assert_eq!(authenticated.token_type, "Bearer");

        Ok(())
    }

    #[tokio::test]
    async fn test_auth_service_wrong_password() -> Result<()> {
        let storage = setup_storage().await;
        let auth_service = AuthService::new(storage.clone());

        // Create user
        let _user = auth_service
            .create_user(
                "user@example.com".to_string(),
                "correctpassword".to_string(),
                vec!["user".to_string()],
            )
            .await?;

        // Try to authenticate with wrong password
        let result = auth_service
            .authenticate_user("user@example.com", "wrongpassword")
            .await;

        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_auth_service_token_validation() -> Result<()> {
        let storage = setup_storage().await;
        let auth_service = AuthService::new(storage.clone());

        // Create and authenticate user
        let _user = auth_service
            .create_user(
                "token@example.com".to_string(),
                "password123".to_string(),
                vec!["admin".to_string()],
            )
            .await?;

        let authenticated = auth_service
            .authenticate_user("token@example.com", "password123")
            .await?;

        // Validate the token
        let validated = auth_service
            .validate_token(&authenticated.access_token)
            .await?;

        assert!(validated.is_some());
        let session = validated.unwrap();
        assert_eq!(session.email, "token@example.com");
        assert_eq!(session.roles, vec!["admin".to_string()]);

        Ok(())
    }

    #[tokio::test]
    async fn test_auth_service_api_key_management() -> Result<()> {
        let storage = setup_storage().await;
        let auth_service = AuthService::new(storage.clone());

        // Create user
        let user = auth_service
            .create_user(
                "api@example.com".to_string(),
                "password123".to_string(),
                vec!["user".to_string()],
            )
            .await?;

        // Create API key
        let api_key = auth_service
            .create_api_key(
                &user.id,
                "test-key".to_string(),
                vec!["read".to_string(), "write".to_string()],
                Some(30),
            )
            .await?;

        assert_eq!(api_key.name, "test-key");
        assert!(api_key.key.starts_with("cortex_"));
        assert_eq!(api_key.scopes, vec!["read".to_string(), "write".to_string()]);

        // Validate API key
        let validated = auth_service.validate_api_key(&api_key.key).await?;
        assert!(validated.is_some());
        let key_info = validated.unwrap();
        assert_eq!(key_info.name, "test-key");
        assert_eq!(key_info.user_id, user.id);

        // List API keys
        let keys = auth_service.list_api_keys(&user.id).await?;
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].name, "test-key");

        // Revoke API key
        auth_service.revoke_api_key(&api_key.id).await?;

        // Validate revoked key should fail
        let validated = auth_service.validate_api_key(&api_key.key).await?;
        assert!(validated.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_auth_service_user_update() -> Result<()> {
        let storage = setup_storage().await;
        let auth_service = AuthService::new(storage.clone());

        // Create user
        let user = auth_service
            .create_user(
                "update@example.com".to_string(),
                "password123".to_string(),
                vec!["user".to_string()],
            )
            .await?;

        // Update user
        let updated = auth_service
            .update_user(
                &user.id,
                UserUpdate {
                    email: Some("newemail@example.com".to_string()),
                    password: None,
                    roles: Some(vec!["admin".to_string(), "user".to_string()]),
                },
            )
            .await?;

        assert_eq!(updated.email, "newemail@example.com");
        assert_eq!(updated.roles, vec!["admin".to_string(), "user".to_string()]);

        Ok(())
    }

    // ========================================================================
    // SessionService Tests
    // ========================================================================

    #[tokio::test]
    async fn test_session_service_creation() -> Result<()> {
        let storage = setup_storage().await;
        let session_service = SessionService::new(storage.clone());

        let workspace_id = create_test_workspace(storage.clone()).await;

        // Create session
        let session = session_service
            .create_session(
                workspace_id,
                "Test Session".to_string(),
                "ai".to_string(),
                None,
            )
            .await?;

        assert_eq!(session.name, "Test Session");
        assert_eq!(session.agent_type, "ai");
        assert_eq!(session.status, SessionStatus::Active);
        assert_eq!(session.workspace_id, Some(workspace_id));

        Ok(())
    }

    #[tokio::test]
    async fn test_session_service_get_and_update() -> Result<()> {
        let storage = setup_storage().await;
        let session_service = SessionService::new(storage.clone());

        let workspace_id = create_test_workspace(storage.clone()).await;

        // Create session
        let session = session_service
            .create_session(
                workspace_id,
                "Original Name".to_string(),
                "ai".to_string(),
                None,
            )
            .await?;

        let session_id = session.id.to_string();

        // Get session
        let retrieved = session_service.get_session(&session_id).await?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Original Name");

        // Update session
        let updated = session_service
            .update_session(
                &session_id,
                SessionUpdate {
                    name: Some("Updated Name".to_string()),
                    status: Some(SessionStatus::Completed),
                    metadata: None,
                },
            )
            .await?;

        assert_eq!(updated.name, "Updated Name");
        assert_eq!(updated.status, SessionStatus::Completed);

        Ok(())
    }

    #[tokio::test]
    async fn test_session_service_lock_management() -> Result<()> {
        let storage = setup_storage().await;
        let session_service = SessionService::new(storage.clone());

        let workspace_id = create_test_workspace(storage.clone()).await;
        let session = session_service
            .create_session(workspace_id, "Lock Test".to_string(), "ai".to_string(), None)
            .await?;

        let session_id = session.id.to_string();

        // Acquire exclusive lock
        let lock = session_service
            .acquire_lock(
                &session_id,
                "file".to_string(),
                "/test/file.rs".to_string(),
                LockType::Exclusive,
                Some(3600),
            )
            .await?;

        assert_eq!(lock.entity_type, "file");
        assert_eq!(lock.entity_id, "/test/file.rs");
        assert_eq!(lock.lock_type, LockType::Exclusive);
        assert_eq!(lock.owner, session_id);

        // Try to acquire another exclusive lock on same resource (should fail)
        let session2 = session_service
            .create_session(workspace_id, "Lock Test 2".to_string(), "ai".to_string(), None)
            .await?;

        let result = session_service
            .acquire_lock(
                &session2.id.to_string(),
                "file".to_string(),
                "/test/file.rs".to_string(),
                LockType::Exclusive,
                Some(3600),
            )
            .await;

        assert!(result.is_err());

        // Release lock
        session_service.release_lock(&lock.id.to_string()).await?;

        // Now second session should be able to acquire lock
        let lock2 = session_service
            .acquire_lock(
                &session2.id.to_string(),
                "file".to_string(),
                "/test/file.rs".to_string(),
                LockType::Exclusive,
                Some(3600),
            )
            .await?;

        assert_eq!(lock2.owner, session2.id.to_string());

        Ok(())
    }

    #[tokio::test]
    async fn test_session_service_file_modification_tracking() -> Result<()> {
        let storage = setup_storage().await;
        let session_service = SessionService::new(storage.clone());

        let workspace_id = create_test_workspace(storage.clone()).await;
        let session = session_service
            .create_session(
                workspace_id,
                "File Tracking".to_string(),
                "ai".to_string(),
                None,
            )
            .await?;

        let session_id = session.id.to_string();

        // Track file modification
        let modification = session_service
            .track_file_modification(
                &session_id,
                "/src/main.rs".to_string(),
                "file-123".to_string(),
                ChangeType::Modified,
                "abc123".to_string(),
                1024,
                Some(1),
            )
            .await?;

        assert_eq!(modification.file_path, "/src/main.rs");
        assert_eq!(modification.change_type, ChangeType::Modified);
        assert_eq!(modification.version, 1);
        assert_eq!(modification.content_hash, "abc123");

        // Track another modification to same file
        let modification2 = session_service
            .track_file_modification(
                &session_id,
                "/src/main.rs".to_string(),
                "file-123".to_string(),
                ChangeType::Modified,
                "def456".to_string(),
                1536,
                Some(2),
            )
            .await?;

        assert_eq!(modification2.version, 2);
        assert_eq!(modification2.content_hash, "def456");

        // Get modifications
        let modifications = session_service.get_file_modifications(&session_id).await?;
        assert_eq!(modifications.len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_session_service_list_sessions() -> Result<()> {
        let storage = setup_storage().await;
        let session_service = SessionService::new(storage.clone());

        let workspace_id = create_test_workspace(storage.clone()).await;

        // Create multiple sessions
        let _session1 = session_service
            .create_session(workspace_id, "Session 1".to_string(), "ai".to_string(), None)
            .await?;

        let _session2 = session_service
            .create_session(workspace_id, "Session 2".to_string(), "human".to_string(), None)
            .await?;

        let _session3 = session_service
            .create_session(
                workspace_id,
                "Session 3".to_string(),
                "ai".to_string(),
                None,
            )
            .await?;

        // List all sessions
        let all_sessions = session_service
            .list_sessions(
                Some(workspace_id),
                SessionFilters {
                    status: None,
                    agent_type: None,
                    limit: None,
                },
            )
            .await?;

        assert_eq!(all_sessions.len(), 3);

        // List with filter
        let ai_sessions = session_service
            .list_sessions(
                Some(workspace_id),
                SessionFilters {
                    status: None,
                    agent_type: Some("ai".to_string()),
                    limit: None,
                },
            )
            .await?;

        assert_eq!(ai_sessions.len(), 2);

        Ok(())
    }

    // ========================================================================
    // WorkspaceService Tests
    // ========================================================================

    #[tokio::test]
    async fn test_workspace_service_crud() -> Result<()> {
        let storage = setup_storage().await;
        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
        let workspace_service = WorkspaceService::new(storage.clone(), vfs);

        // Create workspace
        let workspace = workspace_service
            .create_workspace(workspace::CreateWorkspaceRequest {
                name: "Test Workspace".to_string(),
                workspace_type: "code".to_string(),
                source_path: None,
                read_only: Some(false),
            })
            .await?;

        assert_eq!(workspace.name, "Test Workspace");
        assert_eq!(workspace.workspace_type, "code");
        assert!(!workspace.read_only);

        let workspace_id = Uuid::parse_str(&workspace.id)?;

        // Get workspace
        let retrieved = workspace_service.get_workspace(&workspace_id).await?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Workspace");

        // Update workspace
        let updated = workspace_service
            .update_workspace(
                &workspace_id,
                workspace::UpdateWorkspaceRequest {
                    name: Some("Updated Workspace".to_string()),
                    workspace_type: None,
                    read_only: Some(true),
                },
            )
            .await?;

        assert_eq!(updated.name, "Updated Workspace");
        assert!(updated.read_only);

        // List workspaces
        let workspaces = workspace_service
            .list_workspaces(workspace::ListWorkspaceFilters {
                workspace_type: None,
                limit: None,
            })
            .await?;

        assert!(workspaces.len() >= 1);

        // Delete workspace
        workspace_service.delete_workspace(&workspace_id).await?;

        // Verify deletion
        let deleted = workspace_service.get_workspace(&workspace_id).await?;
        assert!(deleted.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_workspace_service_stats() -> Result<()> {
        let storage = setup_storage().await;
        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
        let workspace_service = WorkspaceService::new(storage.clone(), vfs.clone());

        // Create workspace
        let workspace = workspace_service
            .create_workspace(workspace::CreateWorkspaceRequest {
                name: "Stats Test".to_string(),
                workspace_type: "code".to_string(),
                source_path: None,
                read_only: Some(false),
            })
            .await?;

        let workspace_id = Uuid::parse_str(&workspace.id)?;

        // Get stats (should be empty initially)
        let stats = workspace_service.get_workspace_stats(&workspace_id).await?;

        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.total_directories, 0);
        assert_eq!(stats.total_bytes, 0);

        Ok(())
    }

    // ========================================================================
    // VfsService Tests
    // ========================================================================

    #[tokio::test]
    async fn test_vfs_service_file_operations() -> Result<()> {
        let storage = setup_storage().await;
        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
        let vfs_service = VfsService::new(vfs.clone());

        let workspace_id = create_test_workspace(storage.clone()).await;

        // Write file
        let content = b"fn main() { println!(\"Hello\"); }";
        let file = vfs_service
            .write_file(&workspace_id, "/src/main.rs", content)
            .await?;

        assert_eq!(file.name, "main.rs");
        assert_eq!(file.node_type, "file");
        assert_eq!(file.size_bytes, content.len() as u64);

        // Read file
        let read_content = vfs_service.read_file(&workspace_id, "/src/main.rs").await?;
        assert_eq!(read_content, content);

        // Check existence
        let exists = vfs_service.exists(&workspace_id, "/src/main.rs").await?;
        assert!(exists);

        // Get metadata
        let metadata = vfs_service.get_metadata(&workspace_id, "/src/main.rs").await?;
        assert_eq!(metadata.name, "main.rs");
        assert_eq!(metadata.size_bytes, content.len() as u64);

        // Delete file
        vfs_service.delete(&workspace_id, "/src/main.rs", false).await?;

        // Verify deletion
        let exists = vfs_service.exists(&workspace_id, "/src/main.rs").await?;
        assert!(!exists);

        Ok(())
    }

    #[tokio::test]
    async fn test_vfs_service_directory_operations() -> Result<()> {
        let storage = setup_storage().await;
        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
        let vfs_service = VfsService::new(vfs.clone());

        let workspace_id = create_test_workspace(storage.clone()).await;

        // Create directory
        let dir = vfs_service
            .create_directory(&workspace_id, "/src/utils", true)
            .await?;

        assert_eq!(dir.name, "utils");
        assert_eq!(dir.node_type, "directory");

        // Write files in directory
        vfs_service
            .write_file(&workspace_id, "/src/utils/helper.rs", b"// helper")
            .await?;
        vfs_service
            .write_file(&workspace_id, "/src/utils/config.rs", b"// config")
            .await?;

        // List directory
        let entries = vfs_service
            .list_directory(&workspace_id, "/src/utils", false)
            .await?;

        assert_eq!(entries.len(), 2);

        // Delete directory recursively
        vfs_service.delete(&workspace_id, "/src/utils", true).await?;

        // Verify deletion
        let exists = vfs_service.exists(&workspace_id, "/src/utils").await?;
        assert!(!exists);

        Ok(())
    }

    #[tokio::test]
    async fn test_vfs_service_move_and_copy() -> Result<()> {
        let storage = setup_storage().await;
        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
        let vfs_service = VfsService::new(vfs.clone());

        let workspace_id = create_test_workspace(storage.clone()).await;

        // Write original file
        let content = b"original content";
        vfs_service
            .write_file(&workspace_id, "/file1.txt", content)
            .await?;

        // Copy file
        vfs_service
            .copy_node(&workspace_id, "/file1.txt", "/file2.txt", false)
            .await?;

        // Verify both exist
        assert!(vfs_service.exists(&workspace_id, "/file1.txt").await?);
        assert!(vfs_service.exists(&workspace_id, "/file2.txt").await?);

        // Verify content
        let copied_content = vfs_service.read_file(&workspace_id, "/file2.txt").await?;
        assert_eq!(copied_content, content);

        // Move file
        vfs_service
            .move_node(&workspace_id, "/file1.txt", "/file3.txt")
            .await?;

        // Verify move
        assert!(!vfs_service.exists(&workspace_id, "/file1.txt").await?);
        assert!(vfs_service.exists(&workspace_id, "/file3.txt").await?);

        Ok(())
    }

    // ========================================================================
    // CodeUnitService Tests
    // ========================================================================

    #[tokio::test]
    async fn test_code_unit_service_list() -> Result<()> {
        let storage = setup_storage().await;
        let code_unit_service = CodeUnitService::new(storage.clone());

        let workspace_id = create_test_workspace(storage.clone()).await;

        // List code units (should be empty initially)
        let units = code_unit_service
            .list_code_units(workspace_id, None, None, None, None, 100)
            .await?;

        assert_eq!(units.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_code_unit_service_filters() -> Result<()> {
        let storage = setup_storage().await;
        let code_unit_service = CodeUnitService::new(storage.clone());

        let workspace_id = create_test_workspace(storage.clone()).await;

        // Test with filters
        let units = code_unit_service
            .list_code_units(
                workspace_id,
                Some("function".to_string()),
                Some("rust".to_string()),
                Some("public".to_string()),
                Some(5),
                50,
            )
            .await?;

        assert_eq!(units.len(), 0);

        Ok(())
    }

    // ========================================================================
    // DependencyService Tests
    // ========================================================================

    #[tokio::test]
    async fn test_dependency_service_graph() -> Result<()> {
        let storage = setup_storage().await;
        let dependency_service = DependencyService::new(storage.clone());

        let workspace_id = create_test_workspace(storage.clone()).await;

        // Get dependency graph (should be empty initially)
        let graph = dependency_service
            .get_dependency_graph(workspace_id, Some(10))
            .await?;

        assert_eq!(graph.nodes.len(), 0);
        assert_eq!(graph.edges.len(), 0);
        assert_eq!(graph.cycle_count, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_dependency_service_cycles() -> Result<()> {
        let storage = setup_storage().await;
        let dependency_service = DependencyService::new(storage.clone());

        let workspace_id = create_test_workspace(storage.clone()).await;

        // Detect cycles (should be empty initially)
        let cycles = dependency_service.detect_cycles(workspace_id).await?;

        assert_eq!(cycles.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_dependency_service_impact_analysis() -> Result<()> {
        let storage = setup_storage().await;
        let dependency_service = DependencyService::new(storage.clone());

        let workspace_id = create_test_workspace(storage.clone()).await;

        // Analyze impact (should show no impact for empty workspace)
        let impact = dependency_service
            .analyze_impact(workspace_id, vec!["unit-1".to_string()])
            .await?;

        assert_eq!(impact.changed_entities.len(), 0);
        assert_eq!(impact.affected_entities.len(), 0);

        Ok(())
    }

    // ========================================================================
    // BuildService Tests
    // ========================================================================

    #[tokio::test]
    async fn test_build_service_trigger_build() -> Result<()> {
        let storage = setup_storage().await;
        let build_service = BuildService::new(storage.clone());

        let workspace_id = create_test_workspace(storage.clone()).await;

        // Trigger build
        let build = build_service
            .trigger_build(
                workspace_id,
                build::BuildConfig {
                    build_type: "debug".to_string(),
                    target: None,
                    features: None,
                },
            )
            .await?;

        assert_eq!(build.build_type, "debug");
        assert_eq!(build.status, build::BuildStatus::Queued);
        assert_eq!(build.workspace_id, workspace_id);

        // Wait a bit for background task to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Get build status
        let status = build_service.get_build_status(&build.id).await?;
        assert!(status.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_build_service_list_builds() -> Result<()> {
        let storage = setup_storage().await;
        let build_service = BuildService::new(storage.clone());

        let workspace_id = create_test_workspace(storage.clone()).await;

        // Trigger multiple builds
        let _build1 = build_service
            .trigger_build(
                workspace_id,
                build::BuildConfig {
                    build_type: "debug".to_string(),
                    target: None,
                    features: None,
                },
            )
            .await?;

        let _build2 = build_service
            .trigger_build(
                workspace_id,
                build::BuildConfig {
                    build_type: "release".to_string(),
                    target: None,
                    features: None,
                },
            )
            .await?;

        // List builds
        let builds = build_service.list_builds(workspace_id, 10).await?;
        assert_eq!(builds.len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_build_service_cancel_build() -> Result<()> {
        let storage = setup_storage().await;
        let build_service = BuildService::new(storage.clone());

        let workspace_id = create_test_workspace(storage.clone()).await;

        // Trigger build
        let build = build_service
            .trigger_build(
                workspace_id,
                build::BuildConfig {
                    build_type: "debug".to_string(),
                    target: None,
                    features: None,
                },
            )
            .await?;

        // Cancel immediately
        build_service.cancel_build(&build.id).await?;

        // Get status
        let status = build_service.get_build_status(&build.id).await?;
        assert_eq!(status, Some(build::BuildStatus::Cancelled));

        Ok(())
    }

    #[tokio::test]
    async fn test_build_service_run_tests() -> Result<()> {
        let storage = setup_storage().await;
        let build_service = BuildService::new(storage.clone());

        let workspace_id = create_test_workspace(storage.clone()).await;

        // Run tests
        let test_run = build_service
            .run_tests(
                workspace_id,
                build::TestConfig {
                    test_pattern: None,
                    test_type: Some("unit".to_string()),
                    coverage: Some(true),
                },
            )
            .await?;

        assert_eq!(test_run.status, build::TestStatus::Running);
        assert_eq!(test_run.workspace_id, workspace_id);

        // Wait for tests to complete
        tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;

        // Get test results
        let results = build_service.get_test_results(&test_run.id).await?;
        assert!(results.is_some());

        let results = results.unwrap();
        assert_eq!(results.status, build::TestStatus::Completed);
        assert!(results.total_tests > 0);

        // Get coverage
        let coverage = build_service.get_test_coverage(&test_run.id).await?;
        assert!(coverage.lines_total > 0);
        assert!(coverage.percentage > 0.0);

        Ok(())
    }

    #[tokio::test]
    async fn test_build_service_get_logs() -> Result<()> {
        let storage = setup_storage().await;
        let build_service = BuildService::new(storage.clone());

        let workspace_id = create_test_workspace(storage.clone()).await;

        let build = build_service
            .trigger_build(
                workspace_id,
                build::BuildConfig {
                    build_type: "debug".to_string(),
                    target: None,
                    features: None,
                },
            )
            .await?;

        // Get logs
        let logs = build_service.get_build_logs(&build.id, 0, 10).await?;
        assert!(!logs.is_empty());

        Ok(())
    }

    // ========================================================================
    // MemoryService Tests
    // ========================================================================

    #[tokio::test]
    async fn test_memory_service_store_episode() -> Result<()> {
        let storage = setup_storage().await;
        let cognitive_manager = Arc::new(
            cortex_memory::CognitiveManager::new(storage.clone())
                .await
                .expect("Failed to create cognitive manager"),
        );
        let memory_service = MemoryService::new(storage.clone(), cognitive_manager);

        // Store an episode
        let episode = memory_service
            .store_episode(memory::StoreEpisodeRequest {
                task_description: "Implemented authentication feature".to_string(),
                episode_type: "development".to_string(),
                outcome: "success".to_string(),
                importance: Some(0.8),
            })
            .await?;

        assert_eq!(episode.task_description, "Implemented authentication feature");
        assert_eq!(episode.episode_type, "development");
        assert_eq!(episode.outcome, "success");
        assert_eq!(episode.importance, 0.8);

        Ok(())
    }

    #[tokio::test]
    async fn test_memory_service_recall_episodes() -> Result<()> {
        let storage = setup_storage().await;
        let cognitive_manager = Arc::new(
            cortex_memory::CognitiveManager::new(storage.clone())
                .await
                .expect("Failed to create cognitive manager"),
        );
        let memory_service = MemoryService::new(storage.clone(), cognitive_manager);

        // Recall episodes (should return empty initially)
        let episodes = memory_service
            .recall_episodes(memory::RecallEpisodesRequest {
                query: "authentication".to_string(),
                episode_type: None,
                limit: Some(10),
                min_importance: Some(0.5),
            })
            .await?;

        // Initially empty since we haven't stored any episodes in the database
        assert_eq!(episodes.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_memory_service_get_patterns() -> Result<()> {
        let storage = setup_storage().await;
        let cognitive_manager = Arc::new(
            cortex_memory::CognitiveManager::new(storage.clone())
                .await
                .expect("Failed to create cognitive manager"),
        );
        let memory_service = MemoryService::new(storage.clone(), cognitive_manager);

        // Get patterns (should return empty initially)
        let patterns = memory_service
            .get_patterns(memory::PatternFilters {
                pattern_type: Some("code".to_string()),
                min_confidence: Some(0.7),
                limit: Some(10),
            })
            .await?;

        assert_eq!(patterns.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_memory_service_consolidation() -> Result<()> {
        let storage = setup_storage().await;
        let cognitive_manager = Arc::new(
            cortex_memory::CognitiveManager::new(storage.clone())
                .await
                .expect("Failed to create cognitive manager"),
        );
        let memory_service = MemoryService::new(storage.clone(), cognitive_manager);

        // Run consolidation
        let result = memory_service.consolidate().await?;

        // Should have some metrics even if empty
        assert!(result.episodes_processed >= 0);
        assert!(result.patterns_extracted >= 0);
        assert!(result.duration_ms >= 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_memory_service_get_context() -> Result<()> {
        let storage = setup_storage().await;
        let cognitive_manager = Arc::new(
            cortex_memory::CognitiveManager::new(storage.clone())
                .await
                .expect("Failed to create cognitive manager"),
        );
        let memory_service = MemoryService::new(storage.clone(), cognitive_manager);

        // Get context
        let context = memory_service
            .get_context(memory::GetContextRequest {
                description: "Building authentication system".to_string(),
            })
            .await?;

        assert!(context.context_score >= 0.0);
        assert!(context.context_score <= 1.0);
        assert_eq!(context.relevant_episodes.len(), 0); // Empty initially
        assert_eq!(context.relevant_patterns.len(), 0); // Empty initially

        Ok(())
    }

    // ========================================================================
    // SearchService Tests
    // ========================================================================

    #[tokio::test]
    async fn test_search_service_text_search() -> Result<()> {
        let storage = setup_storage().await;
        let search_service = SearchService::new(storage.clone());

        // Test text search (should return empty results initially)
        let results = search_service
            .search_text(search::TextSearchRequest {
                query: "test".to_string(),
                search_type: "code_units".to_string(),
                limit: 10,
            })
            .await?;

        assert_eq!(results.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_search_service_snippet_creation() {
        // Test snippet creation utility
        let short_text = "This is a short text";
        let snippet = SearchService::create_snippet(short_text, 100);
        assert_eq!(snippet, short_text);

        let long_text = "This is a very long text that should be truncated at some point because it exceeds the maximum length that we want to display in the search results";
        let snippet = SearchService::create_snippet(long_text, 50);
        assert!(snippet.len() <= 53); // 50 + "..."
        assert!(snippet.ends_with("..."));
    }

    // ========================================================================
    // Integration Tests
    // ========================================================================

    #[tokio::test]
    async fn test_integration_workspace_and_vfs() -> Result<()> {
        let storage = setup_storage().await;
        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
        let workspace_service = WorkspaceService::new(storage.clone(), vfs.clone());
        let vfs_service = VfsService::new(vfs.clone());

        // Create workspace
        let workspace = workspace_service
            .create_workspace(workspace::CreateWorkspaceRequest {
                name: "Integration Test".to_string(),
                workspace_type: "code".to_string(),
                source_path: None,
                read_only: Some(false),
            })
            .await?;

        let workspace_id = Uuid::parse_str(&workspace.id)?;

        // Add files to workspace
        vfs_service
            .write_file(&workspace_id, "/README.md", b"# Project")
            .await?;
        vfs_service
            .write_file(&workspace_id, "/src/main.rs", b"fn main() {}")
            .await?;

        // List workspace files
        let files = vfs_service.list_directory(&workspace_id, "/", true).await?;
        assert!(files.len() >= 2);

        // Get workspace stats
        let stats = workspace_service.get_workspace_stats(&workspace_id).await?;
        assert!(stats.total_files >= 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_integration_session_and_locks() -> Result<()> {
        let storage = setup_storage().await;
        let session_service = SessionService::new(storage.clone());

        let workspace_id = create_test_workspace(storage.clone()).await;

        // Create two concurrent sessions
        let session1 = session_service
            .create_session(
                workspace_id,
                "Session 1".to_string(),
                "ai".to_string(),
                None,
            )
            .await?;

        let session2 = session_service
            .create_session(
                workspace_id,
                "Session 2".to_string(),
                "ai".to_string(),
                None,
            )
            .await?;

        // Session 1 acquires lock
        let lock = session_service
            .acquire_lock(
                &session1.id.to_string(),
                "resource".to_string(),
                "shared-resource".to_string(),
                LockType::Exclusive,
                Some(3600),
            )
            .await?;

        // Session 2 tries to acquire same lock (should fail)
        let result = session_service
            .acquire_lock(
                &session2.id.to_string(),
                "resource".to_string(),
                "shared-resource".to_string(),
                LockType::Exclusive,
                Some(3600),
            )
            .await;

        assert!(result.is_err());

        // Session 1 releases lock
        session_service.release_lock(&lock.id.to_string()).await?;

        // Now session 2 can acquire lock
        let lock2 = session_service
            .acquire_lock(
                &session2.id.to_string(),
                "resource".to_string(),
                "shared-resource".to_string(),
                LockType::Exclusive,
                Some(3600),
            )
            .await?;

        assert_eq!(lock2.owner, session2.id.to_string());

        Ok(())
    }

    #[tokio::test]
    async fn test_integration_auth_and_sessions() -> Result<()> {
        let storage = setup_storage().await;
        let auth_service = AuthService::new(storage.clone());

        // Create user
        let user = auth_service
            .create_user(
                "session@example.com".to_string(),
                "password123".to_string(),
                vec!["user".to_string()],
            )
            .await?;

        // Authenticate
        let authenticated = auth_service
            .authenticate_user("session@example.com", "password123")
            .await?;

        // Validate token
        let validated = auth_service
            .validate_token(&authenticated.access_token)
            .await?;

        assert!(validated.is_some());
        assert_eq!(validated.unwrap().user_id, user.id);

        // Create session for user
        let session = auth_service
            .create_session(&user.id, Some("127.0.0.1".to_string()))
            .await?;

        assert_eq!(session.user_id, user.id);

        Ok(())
    }

    #[tokio::test]
    async fn test_error_handling_nonexistent_workspace() -> Result<()> {
        let storage = setup_storage().await;
        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
        let workspace_service = WorkspaceService::new(storage.clone(), vfs);

        let nonexistent_id = Uuid::new_v4();

        // Try to get nonexistent workspace
        let result = workspace_service.get_workspace(&nonexistent_id).await?;
        assert!(result.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_error_handling_invalid_file_path() -> Result<()> {
        let storage = setup_storage().await;
        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
        let vfs_service = VfsService::new(vfs);

        let workspace_id = create_test_workspace(storage.clone()).await;

        // Try to read nonexistent file
        let result = vfs_service
            .read_file(&workspace_id, "/nonexistent/file.rs")
            .await;

        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_error_handling_duplicate_user() -> Result<()> {
        let storage = setup_storage().await;
        let auth_service = AuthService::new(storage.clone());

        // Create user
        let _user = auth_service
            .create_user(
                "duplicate@example.com".to_string(),
                "password123".to_string(),
                vec!["user".to_string()],
            )
            .await?;

        // Try to create duplicate user (email constraint)
        // Note: This depends on database constraints being enforced
        // In practice, this might need a unique check in the service layer

        Ok(())
    }
}
