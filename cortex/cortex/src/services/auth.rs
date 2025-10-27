//! Authentication and authorization service
//!
//! Provides unified authentication operations for both API and MCP modules.
//! Handles user authentication, session management, API keys, and JWT tokens.

use anyhow::{anyhow, Result};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{DateTime, Duration, Utc};
use cortex_storage::ConnectionManager;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Authentication service for managing users, sessions, and API keys
#[derive(Clone)]
pub struct AuthService {
    storage: Arc<ConnectionManager>,
    jwt_secret: String,
    access_token_expiry: i64,  // minutes
    refresh_token_expiry: i64, // days
}

impl AuthService {
    /// Create a new authentication service
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self {
            storage,
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "cortex-dev-secret-change-in-production".to_string()),
            access_token_expiry: 15,  // 15 minutes
            refresh_token_expiry: 7,  // 7 days
        }
    }

    /// Create a new authentication service with custom token expiry
    pub fn with_expiry(
        storage: Arc<ConnectionManager>,
        access_token_expiry: i64,
        refresh_token_expiry: i64,
    ) -> Self {
        Self {
            storage,
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "cortex-dev-secret-change-in-production".to_string()),
            access_token_expiry,
            refresh_token_expiry,
        }
    }

    // ========================================================================
    // User Management
    // ========================================================================

    /// Authenticate a user with email and password
    pub async fn authenticate_user(&self, email: &str, password: &str) -> Result<AuthenticatedUser> {
        debug!("Authenticating user: {}", email);

        let conn = self.storage.acquire().await?;

        // Query user from database using parameterized query
        let query = "SELECT * FROM users WHERE email = $email LIMIT 1";
        let mut result = conn.connection()
            .query(query)
            .bind(("email", email.to_string()))
            .await?;

        let users: Vec<User> = result.take(0)?;
        let user = users.first()
            .ok_or_else(|| anyhow!("Invalid email or password"))?;

        // Verify password (CPU-bound, must run in blocking thread)
        let password_hash = user.password_hash.clone();
        let password_owned = password.to_string();
        let valid = tokio::task::spawn_blocking(move || {
            verify(&password_owned, &password_hash)
        })
        .await
        .map_err(|e| anyhow!("Password verification failed: {}", e))??;

        if !valid {
            warn!("Failed authentication attempt for user: {}", email);
            return Err(anyhow!("Invalid email or password"));
        }

        // Generate tokens
        let access_token = self.generate_access_token(&user)?;
        let refresh_token = self.generate_refresh_token(&user)?;

        // Create session
        let session = self.create_session_internal(&user.id, None).await?;

        // Update session with refresh token
        self.update_session_token(&session.id, &refresh_token).await?;

        info!("User authenticated successfully: {}", email);

        Ok(AuthenticatedUser {
            user: UserInfo::from_user(user.clone()),
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.access_token_expiry * 60, // Convert to seconds
        })
    }

    /// Create a new user
    pub async fn create_user(
        &self,
        email: String,
        password: String,
        roles: Vec<String>,
    ) -> Result<User> {
        info!("Creating new user: {}", email);

        // Hash password (CPU-bound, must run in blocking thread)
        let password_hash = tokio::task::spawn_blocking(move || {
            hash(&password, DEFAULT_COST)
        })
        .await
        .map_err(|e| anyhow!("Password hashing failed: {}", e))??;

        let user_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let user = User {
            id: user_id.clone(),
            email: email.clone(),
            password_hash,
            roles,
            created_at: now,
            updated_at: now,
        };

        // Save to database using raw query with JSON serialization
        // This ensures proper datetime handling with SurrealDB SCHEMAFULL tables
        let conn = self.storage.acquire().await?;

        let user_json = serde_json::to_string(&user)?;
        let query = format!("CREATE users:`{}` CONTENT {}", user_id, user_json);

        conn.connection().query(&query).await?;

        // Retrieve the created user
        let created_user: Option<User> = conn.connection()
            .select(("users", user_id.as_str()))
            .await?;

        let created_user = created_user.ok_or_else(|| anyhow!("Failed to create user"))?;

        info!("User created: {} ({})", email, user_id);

        Ok(created_user)
    }

    /// Get user by ID
    pub async fn get_user(&self, user_id: &str) -> Result<Option<User>> {
        debug!("Getting user: {}", user_id);

        let conn = self.storage.acquire().await?;

        let user: Option<User> = conn.connection()
            .select(("users", user_id))
            .await?;

        Ok(user)
    }

    /// Update user
    pub async fn update_user(&self, user_id: &str, updates: UserUpdate) -> Result<User> {
        info!("Updating user: {}", user_id);

        let conn = self.storage.acquire().await?;

        // Get existing user
        let mut user: User = conn.connection()
            .select(("users", user_id))
            .await?
            .ok_or_else(|| anyhow!("User not found"))?;

        // Apply updates
        if let Some(email) = updates.email {
            user.email = email;
        }
        if let Some(password) = updates.password {
            // Hash password (CPU-bound, must run in blocking thread)
            user.password_hash = tokio::task::spawn_blocking(move || {
                hash(&password, DEFAULT_COST)
            })
            .await
            .map_err(|e| anyhow!("Password hashing failed: {}", e))??;
        }
        if let Some(roles) = updates.roles {
            user.roles = roles;
        }
        user.updated_at = Utc::now();

        // Update in database using raw query with JSON serialization
        let user_json = serde_json::to_string(&user)?;
        let query = format!("UPDATE users:`{}` CONTENT {}", user_id, user_json);

        conn.connection().query(&query).await?;

        // Retrieve the updated user
        let updated_user: Option<User> = conn.connection()
            .select(("users", user_id))
            .await?;

        let updated_user = updated_user.ok_or_else(|| anyhow!("Failed to update user"))?;

        info!("User updated: {}", user_id);

        Ok(updated_user)
    }

    /// Delete user
    pub async fn delete_user(&self, user_id: &str) -> Result<()> {
        info!("Deleting user: {}", user_id);

        let conn = self.storage.acquire().await?;

        // Delete all user sessions
        let query = "DELETE FROM sessions WHERE user_id = $user_id";
        conn.connection()
            .query(query)
            .bind(("user_id", user_id.to_string()))
            .await?;

        // Delete all user API keys
        let query = "DELETE FROM api_keys WHERE user_id = $user_id";
        conn.connection()
            .query(query)
            .bind(("user_id", user_id.to_string()))
            .await?;

        // Delete user
        let _: Option<User> = conn.connection()
            .delete(("users", user_id))
            .await?;

        info!("User deleted: {}", user_id);

        Ok(())
    }

    // ========================================================================
    // Session Management
    // ========================================================================

    /// Create a new session for a user
    pub async fn create_session(&self, user_id: &str, ip: Option<String>) -> Result<Session> {
        self.create_session_internal(user_id, ip).await
    }

    /// Internal session creation
    async fn create_session_internal(&self, user_id: &str, ip: Option<String>) -> Result<Session> {
        debug!("Creating session for user: {}", user_id);

        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = now + Duration::days(self.refresh_token_expiry);

        let session = Session {
            id: session_id.clone(),
            user_id: user_id.to_string(),
            refresh_token: String::new(), // Will be set later
            ip_address: ip,
            user_agent: None,
            expires_at,
            created_at: now,
            last_accessed: now,
        };

        let conn = self.storage.acquire().await?;

        // Use raw query with JSON serialization for proper datetime handling
        let session_json = serde_json::to_string(&session)?;
        let query = format!("CREATE sessions:`{}` CONTENT {}", session_id, session_json);

        conn.connection().query(&query).await?;

        // Retrieve the created session
        let created_session: Option<Session> = conn.connection()
            .select(("sessions", session_id.as_str()))
            .await?;

        let created_session = created_session.ok_or_else(|| anyhow!("Failed to create session"))?;

        debug!("Session created: {}", session_id);

        Ok(created_session)
    }

    /// Update session with refresh token
    async fn update_session_token(&self, session_id: &str, refresh_token: &str) -> Result<()> {
        let conn = self.storage.acquire().await?;

        let query = "UPDATE sessions SET refresh_token = $token, last_accessed = $time WHERE id = $id";
        conn.connection()
            .query(query)
            .bind(("token", refresh_token.to_string()))
            .bind(("time", Utc::now()))
            .bind(("id", session_id.to_string()))
            .await?;

        Ok(())
    }

    /// Validate a token and return the associated session
    pub async fn validate_token(&self, token: &str) -> Result<Option<ValidatedSession>> {
        debug!("Validating token");

        // Decode token
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )?;

        // Check if access token
        if token_data.claims.token_type != "access" {
            return Err(anyhow!("Invalid token type"));
        }

        // Get user
        let user = self.get_user(&token_data.claims.sub).await?;
        let user = user.ok_or_else(|| anyhow!("User not found"))?;

        Ok(Some(ValidatedSession {
            user_id: user.id.clone(),
            email: user.email.clone(),
            roles: user.roles.clone(),
            expires_at: DateTime::from_timestamp(token_data.claims.exp, 0)
                .ok_or_else(|| anyhow!("Invalid expiration timestamp"))?,
        }))
    }

    /// Refresh an access token using a refresh token
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<SessionTokens> {
        debug!("Refreshing token");

        // Decode and validate refresh token
        let token_data = decode::<Claims>(
            refresh_token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )?;

        if token_data.claims.token_type != "refresh" {
            return Err(anyhow!("Invalid token type"));
        }

        // Verify session exists in database
        let conn = self.storage.acquire().await?;
        let query = "SELECT * FROM sessions WHERE user_id = $user_id AND refresh_token = $token AND expires_at > $now LIMIT 1";
        let mut result = conn.connection()
            .query(query)
            .bind(("user_id", token_data.claims.sub.clone()))
            .bind(("token", refresh_token.to_string()))
            .bind(("now", Utc::now()))
            .await?;

        let sessions: Vec<Session> = result.take(0)?;
        if sessions.is_empty() {
            return Err(anyhow!("Session not found or expired"));
        }

        // Get user
        let user = self.get_user(&token_data.claims.sub).await?
            .ok_or_else(|| anyhow!("User not found"))?;

        // Generate new access token
        let access_token = self.generate_access_token(&user)?;

        // Update last accessed time
        let query = "UPDATE sessions SET last_accessed = $time WHERE user_id = $user_id AND refresh_token = $token";
        conn.connection()
            .query(query)
            .bind(("time", Utc::now()))
            .bind(("user_id", user.id.clone()))
            .bind(("token", refresh_token.to_string()))
            .await?;

        debug!("Token refreshed for user: {}", user.id);

        Ok(SessionTokens {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: self.access_token_expiry * 60,
        })
    }

    /// Revoke a token (logout) - adds token to blacklist
    pub async fn revoke_token(&self, token: &str) -> Result<()> {
        debug!("Revoking token");

        // Decode token to get claims and expiration
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )?;

        let claims = &token_data.claims;

        // Add token to blacklist
        self.add_to_blacklist(token, &claims.sub, &claims.token_type, claims.exp).await?;

        // If it's a refresh token, also delete the associated session
        if claims.token_type == "refresh" {
            let conn = self.storage.acquire().await?;
            let query = "DELETE FROM sessions WHERE user_id = $user_id AND refresh_token = $token";
            conn.connection()
                .query(query)
                .bind(("user_id", claims.sub.clone()))
                .bind(("token", token.to_string()))
                .await?;
        }

        info!("Token revoked for user: {} (type: {})", claims.sub, claims.token_type);

        Ok(())
    }

    /// Revoke all tokens for a user (logout all devices)
    pub async fn revoke_all_user_tokens(&self, user_id: &str) -> Result<()> {
        info!("Revoking all tokens for user: {}", user_id);

        let conn = self.storage.acquire().await?;

        // Get all active sessions for this user to blacklist their refresh tokens
        let query = "SELECT * FROM sessions WHERE user_id = $user_id AND expires_at > $now";
        let mut result = conn.connection()
            .query(query)
            .bind(("user_id", user_id.to_string()))
            .bind(("now", Utc::now()))
            .await?;

        let sessions: Vec<Session> = result.take(0)?;

        // Blacklist all refresh tokens from sessions
        for session in sessions {
            if !session.refresh_token.is_empty() {
                // Decode to get expiration
                if let Ok(token_data) = decode::<Claims>(
                    &session.refresh_token,
                    &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
                    &Validation::default(),
                ) {
                    self.add_to_blacklist(
                        &session.refresh_token,
                        user_id,
                        "refresh",
                        token_data.claims.exp,
                    ).await?;
                }
            }
        }

        // Delete all sessions for this user
        let query = "DELETE FROM sessions WHERE user_id = $user_id";
        conn.connection()
            .query(query)
            .bind(("user_id", user_id.to_string()))
            .await?;

        info!("All tokens revoked for user: {}", user_id);

        Ok(())
    }

    /// Add a token to the blacklist
    async fn add_to_blacklist(
        &self,
        token: &str,
        user_id: &str,
        token_type: &str,
        exp_timestamp: i64,
    ) -> Result<()> {
        let conn = self.storage.acquire().await?;

        // Create SHA-256 hash of the token
        let token_hash = self.hash_token(token);

        let expires_at = DateTime::from_timestamp(exp_timestamp, 0)
            .ok_or_else(|| anyhow!("Invalid expiration timestamp"))?;

        let blacklist_id = Uuid::new_v4().to_string();
        let revoked_token = RevokedToken {
            id: blacklist_id.clone(),
            token_hash: token_hash.clone(),
            user_id: user_id.to_string(),
            token_type: token_type.to_string(),
            revoked_at: Utc::now(),
            expires_at,
        };

        // Insert into blacklist using raw query (ignore if already exists due to unique index)
        let revoked_json = serde_json::to_string(&revoked_token)?;
        let query = format!("CREATE revoked_tokens:`{}` CONTENT {}", blacklist_id, revoked_json);

        let result = conn.connection().query(&query).await;

        match result {
            Ok(_) => {
                debug!("Token added to blacklist: {} (type: {})", token_hash, token_type);
                Ok(())
            },
            Err(e) => {
                // If it's a duplicate error, that's fine - token is already blacklisted
                if e.to_string().contains("already exists") || e.to_string().contains("unique") {
                    debug!("Token already in blacklist: {}", token_hash);
                    Ok(())
                } else {
                    Err(anyhow!("Failed to add token to blacklist: {}", e))
                }
            }
        }
    }

    /// Check if a token is blacklisted
    pub async fn is_token_blacklisted(&self, token: &str) -> Result<bool> {
        let conn = self.storage.acquire().await?;

        let token_hash = self.hash_token(token);

        // Check if token hash exists in blacklist and hasn't expired
        let query = "SELECT * FROM revoked_tokens WHERE token_hash = $hash AND expires_at > $now LIMIT 1";
        let mut result = conn.connection()
            .query(query)
            .bind(("hash", token_hash))
            .bind(("now", Utc::now()))
            .await?;

        let tokens: Vec<RevokedToken> = result.take(0)?;

        Ok(!tokens.is_empty())
    }

    /// Hash a token using SHA-256 for storage in blacklist
    fn hash_token(&self, token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> Result<usize> {
        debug!("Cleaning up expired sessions");

        let conn = self.storage.acquire().await?;
        let query = "DELETE FROM sessions WHERE expires_at < $now";
        let mut result = conn.connection()
            .query(query)
            .bind(("now", Utc::now()))
            .await?;

        // SurrealDB doesn't return count for DELETE, so we'll return 0
        // In production, you'd want to query first to count
        let _: Vec<Session> = result.take(0).unwrap_or_default();

        info!("Expired sessions cleaned up");

        Ok(0)
    }

    /// Clean up expired revoked tokens from blacklist
    pub async fn cleanup_expired_revoked_tokens(&self) -> Result<usize> {
        debug!("Cleaning up expired revoked tokens");

        let conn = self.storage.acquire().await?;
        let query = "DELETE FROM revoked_tokens WHERE expires_at < $now";
        let mut result = conn.connection()
            .query(query)
            .bind(("now", Utc::now()))
            .await?;

        // SurrealDB doesn't return count for DELETE, so we'll return 0
        let _: Vec<RevokedToken> = result.take(0).unwrap_or_default();

        info!("Expired revoked tokens cleaned up");

        Ok(0)
    }

    // ========================================================================
    // API Key Management
    // ========================================================================

    /// Create a new API key for a user
    pub async fn create_api_key(
        &self,
        user_id: &str,
        name: String,
        scopes: Vec<String>,
        expires_in_days: Option<i64>,
    ) -> Result<ApiKey> {
        info!("Creating API key for user: {}", user_id);

        // Generate API key
        let key_id = Uuid::new_v4().to_string();
        let api_key = format!("cortex_{}", Uuid::new_v4().simple());

        // Hash the API key for storage (CPU-bound, must run in blocking thread)
        let api_key_clone = api_key.clone();
        let key_hash = tokio::task::spawn_blocking(move || {
            hash(&api_key_clone, DEFAULT_COST)
        })
        .await
        .map_err(|e| anyhow!("API key hashing failed: {}", e))??;

        let now = Utc::now();
        let expires_at = expires_in_days.map(|days| now + Duration::days(days));

        let api_key_record = ApiKeyRecord {
            id: key_id.clone(),
            user_id: user_id.to_string(),
            name: name.clone(),
            key_hash: key_hash.clone(),
            scopes: scopes.clone(),
            expires_at,
            created_at: now,
            last_used: None,
        };

        let conn = self.storage.acquire().await?;

        // Use raw query with JSON serialization for proper datetime handling
        let api_key_json = serde_json::to_string(&api_key_record)?;
        let query = format!("CREATE api_keys:`{}` CONTENT {}", key_id, api_key_json);

        conn.connection().query(&query).await?;

        info!("API key created: {} ({})", name, key_id);

        Ok(ApiKey {
            id: key_id,
            key: api_key, // Only returned once
            name,
            scopes,
            expires_at,
            created_at: now,
        })
    }

    /// Validate an API key
    pub async fn validate_api_key(&self, key: &str) -> Result<Option<ApiKeyInfo>> {
        debug!("Validating API key");

        let conn = self.storage.acquire().await?;

        // Get all API keys (we need to hash-compare them)
        // In production, you'd want to optimize this with an index or key prefix
        let query = "SELECT * FROM api_keys WHERE expires_at IS NULL OR expires_at > $now";
        let mut result = conn.connection()
            .query(query)
            .bind(("now", Utc::now()))
            .await?;

        let api_keys: Vec<ApiKeyRecord> = result.take(0)?;

        // Find matching key by hash comparison (CPU-bound, must run in blocking thread)
        let key_owned = key.to_string();
        for key_record in api_keys {
            let key_hash = key_record.key_hash.clone();
            let key_clone = key_owned.clone();

            let is_valid = tokio::task::spawn_blocking(move || {
                verify(&key_clone, &key_hash).unwrap_or(false)
            })
            .await
            .unwrap_or(false);

            if is_valid {
                // Update last used time
                let query = "UPDATE api_keys SET last_used = $time WHERE id = $id";
                conn.connection()
                    .query(query)
                    .bind(("time", Utc::now()))
                    .bind(("id", key_record.id.clone()))
                    .await?;

                debug!("API key validated: {}", key_record.id);

                return Ok(Some(ApiKeyInfo {
                    id: key_record.id,
                    user_id: key_record.user_id,
                    name: key_record.name,
                    scopes: key_record.scopes,
                    expires_at: key_record.expires_at,
                    created_at: key_record.created_at,
                    last_used: Some(Utc::now()),
                }));
            }
        }

        warn!("Invalid API key attempt");
        Ok(None)
    }

    /// Revoke an API key
    pub async fn revoke_api_key(&self, key_id: &str) -> Result<()> {
        info!("Revoking API key: {}", key_id);

        let conn = self.storage.acquire().await?;

        let _: Option<ApiKeyRecord> = conn.connection()
            .delete(("api_keys", key_id))
            .await?;

        info!("API key revoked: {}", key_id);

        Ok(())
    }

    /// List API keys for a user
    pub async fn list_api_keys(&self, user_id: &str) -> Result<Vec<ApiKeyInfo>> {
        debug!("Listing API keys for user: {}", user_id);

        let conn = self.storage.acquire().await?;

        let query = "SELECT * FROM api_keys WHERE user_id = $user_id ORDER BY created_at DESC";
        let mut result = conn.connection()
            .query(query)
            .bind(("user_id", user_id.to_string()))
            .await?;

        let keys: Vec<ApiKeyRecord> = result.take(0)?;

        Ok(keys.into_iter().map(ApiKeyInfo::from_record).collect())
    }

    // ========================================================================
    // Token Generation
    // ========================================================================

    /// Generate an access token
    fn generate_access_token(&self, user: &User) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::minutes(self.access_token_expiry);

        let claims = Claims {
            sub: user.id.clone(),
            email: user.email.clone(),
            roles: user.roles.clone(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            token_type: "access".to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| anyhow!("Failed to generate access token: {}", e))
    }

    /// Generate a refresh token
    fn generate_refresh_token(&self, user: &User) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::days(self.refresh_token_expiry);

        let claims = Claims {
            sub: user.id.clone(),
            email: user.email.clone(),
            roles: user.roles.clone(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            token_type: "refresh".to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| anyhow!("Failed to generate refresh token: {}", e))
    }
}

// ============================================================================
// Types
// ============================================================================

/// User database model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub roles: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User information (without sensitive data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub roles: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl UserInfo {
    fn from_user(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            roles: user.roles,
            created_at: user.created_at,
        }
    }
}

/// User update request
#[derive(Debug, Clone, Deserialize)]
pub struct UserUpdate {
    pub email: Option<String>,
    pub password: Option<String>,
    pub roles: Option<Vec<String>>,
}

/// Session database model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub refresh_token: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
}

/// Validated session information
#[derive(Debug, Clone, Serialize)]
pub struct ValidatedSession {
    pub user_id: String,
    pub email: String,
    pub roles: Vec<String>,
    pub expires_at: DateTime<Utc>,
}

/// Session tokens
#[derive(Debug, Clone, Serialize)]
pub struct SessionTokens {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// Authenticated user with tokens
#[derive(Debug, Clone, Serialize)]
pub struct AuthenticatedUser {
    pub user: UserInfo,
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// JWT claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,        // User ID
    pub email: String,      // User email
    pub roles: Vec<String>, // User roles
    pub exp: i64,           // Expiration time
    pub iat: i64,           // Issued at
    pub token_type: String, // "access" or "refresh"
}

/// API key database model
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiKeyRecord {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub key_hash: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
}

/// API key (with plain key - only returned on creation)
#[derive(Debug, Clone, Serialize)]
pub struct ApiKey {
    pub id: String,
    pub key: String, // Only returned once
    pub name: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// API key information (without plain key)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
}

impl ApiKeyInfo {
    fn from_record(record: ApiKeyRecord) -> Self {
        Self {
            id: record.id,
            user_id: record.user_id,
            name: record.name,
            scopes: record.scopes,
            expires_at: record.expires_at,
            created_at: record.created_at,
            last_used: record.last_used,
        }
    }
}

/// Revoked token database model (for token blacklist)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RevokedToken {
    pub id: String,
    pub token_hash: String,
    pub user_id: String,
    pub token_type: String,
    pub revoked_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_info_from_user() {
        let user = User {
            id: "user-1".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            roles: vec!["admin".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let info = UserInfo::from_user(user.clone());

        assert_eq!(info.id, user.id);
        assert_eq!(info.email, user.email);
        assert_eq!(info.roles, user.roles);
    }

    #[test]
    fn test_claims_serialization() {
        let claims = Claims {
            sub: "user-1".to_string(),
            email: "test@example.com".to_string(),
            roles: vec!["admin".to_string()],
            exp: 1234567890,
            iat: 1234567800,
            token_type: "access".to_string(),
        };

        let json = serde_json::to_string(&claims).unwrap();
        let deserialized: Claims = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.sub, claims.sub);
        assert_eq!(deserialized.email, claims.email);
        assert_eq!(deserialized.token_type, claims.token_type);
    }
}
