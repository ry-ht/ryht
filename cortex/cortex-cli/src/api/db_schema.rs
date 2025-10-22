//! Database schema initialization for authentication and sessions

use cortex_storage::ConnectionManager;
use std::sync::Arc;
use tracing::info;

/// Initialize authentication database schema
pub async fn initialize_auth_schema(storage: &Arc<ConnectionManager>) -> anyhow::Result<()> {
    info!("Initializing authentication database schema");

    let conn = storage.acquire().await?;

    // Define users table
    let users_schema = r#"
        DEFINE TABLE users SCHEMAFULL;

        DEFINE FIELD id ON TABLE users TYPE string;
        DEFINE FIELD email ON TABLE users TYPE string
            ASSERT string::is::email($value);
        DEFINE FIELD password_hash ON TABLE users TYPE string;
        DEFINE FIELD roles ON TABLE users TYPE array<string> DEFAULT [];
        DEFINE FIELD created_at ON TABLE users TYPE datetime DEFAULT time::now();
        DEFINE FIELD updated_at ON TABLE users TYPE datetime DEFAULT time::now();

        DEFINE INDEX users_email_idx ON TABLE users COLUMNS email UNIQUE;
    "#;

    // Define sessions table
    let sessions_schema = r#"
        DEFINE TABLE sessions SCHEMAFULL;

        DEFINE FIELD id ON TABLE sessions TYPE string;
        DEFINE FIELD user_id ON TABLE sessions TYPE string;
        DEFINE FIELD refresh_token ON TABLE sessions TYPE string;
        DEFINE FIELD expires_at ON TABLE sessions TYPE datetime;
        DEFINE FIELD created_at ON TABLE sessions TYPE datetime DEFAULT time::now();

        DEFINE INDEX sessions_user_idx ON TABLE sessions COLUMNS user_id;
        DEFINE INDEX sessions_token_idx ON TABLE sessions COLUMNS refresh_token;
    "#;

    // Define api_keys table
    let api_keys_schema = r#"
        DEFINE TABLE api_keys SCHEMAFULL;

        DEFINE FIELD id ON TABLE api_keys TYPE string;
        DEFINE FIELD user_id ON TABLE api_keys TYPE string;
        DEFINE FIELD name ON TABLE api_keys TYPE string;
        DEFINE FIELD key_hash ON TABLE api_keys TYPE string;
        DEFINE FIELD scopes ON TABLE api_keys TYPE array<string> DEFAULT [];
        DEFINE FIELD expires_at ON TABLE api_keys TYPE option<datetime>;
        DEFINE FIELD created_at ON TABLE api_keys TYPE datetime DEFAULT time::now();
        DEFINE FIELD last_used_at ON TABLE api_keys TYPE option<datetime>;

        DEFINE INDEX api_keys_user_idx ON TABLE api_keys COLUMNS user_id;
    "#;

    // Execute schema definitions
    conn.connection().query(users_schema).await?;
    info!("Created users table schema");

    conn.connection().query(sessions_schema).await?;
    info!("Created sessions table schema");

    conn.connection().query(api_keys_schema).await?;
    info!("Created api_keys table schema");

    info!("Authentication schema initialized successfully");

    Ok(())
}

/// Create a default admin user if none exists
pub async fn create_default_admin(storage: &Arc<ConnectionManager>) -> anyhow::Result<()> {
    use bcrypt::{hash, DEFAULT_COST};
    use chrono::Utc;
    use uuid::Uuid;

    let conn = storage.acquire().await?;

    // Check if any users exist
    let mut result = conn.connection().query("SELECT * FROM users LIMIT 1").await?;
    let users: Vec<serde_json::Value> = result.take(0)?;

    if users.is_empty() {
        info!("No users found, creating default admin user");

        let admin_id = Uuid::new_v4().to_string();
        let admin_email = "admin@cortex.local";
        let admin_password = "admin123"; // Should be changed on first login

        let password_hash = hash(admin_password, DEFAULT_COST)?;

        let admin_user = serde_json::json!({
            "id": admin_id,
            "email": admin_email,
            "password_hash": password_hash,
            "roles": vec!["admin", "user"],
            "created_at": Utc::now(),
            "updated_at": Utc::now(),
        });

        let query = format!(
            "CREATE users:{} CONTENT {}",
            admin_id,
            serde_json::to_string(&admin_user)?
        );

        conn.connection().query(&query).await?;

        info!("Default admin user created:");
        info!("  Email: {}", admin_email);
        info!("  Password: {}", admin_password);
        info!("  ⚠️  IMPORTANT: Change this password immediately!");
    }

    Ok(())
}

/// Cleanup expired sessions and API keys
pub async fn cleanup_expired_auth_data(storage: &Arc<ConnectionManager>) -> anyhow::Result<()> {
    info!("Cleaning up expired authentication data");

    let conn = storage.acquire().await?;

    // Delete expired sessions
    let delete_sessions = "DELETE FROM sessions WHERE expires_at < time::now()";
    conn.connection().query(delete_sessions).await?;

    // Delete expired API keys
    let delete_keys = "DELETE FROM api_keys WHERE expires_at IS NOT NULL AND expires_at < time::now()";
    conn.connection().query(delete_keys).await?;

    info!("Expired authentication data cleaned up");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_schema_initialization() {
        // This would require a test database connection
        // Skipping for now, but you can add integration tests
    }
}
