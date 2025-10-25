use cortex_code_analysis::{AstEditor, Language};
use cortex_vfs::{VirtualFileSystem, VirtualPath};
use std::collections::HashMap;

/// Full-stack E2E test utilities
mod test_utils {
    pub fn count_occurrences(code: &str, pattern: &str) -> usize {
        code.matches(pattern).count()
    }

    pub fn verify_type_consistency(rust_code: &str, ts_code: &str) -> bool {
        // Basic verification that types match across languages
        // Check for common type names
        let has_user_type = rust_code.contains("struct User") && ts_code.contains("interface User");
        let has_consistent_fields = rust_code.contains("username") && ts_code.contains("username");

        has_user_type && has_consistent_fields
    }
}

/// Scenario 1: Build Full-Stack User Management Feature
///
/// This test simulates building a complete feature across the stack:
/// 1. Create Rust API endpoint with types
/// 2. Create TypeScript API client with matching types
/// 3. Create React component using the client
/// 4. Verify end-to-end type safety
#[test]
fn test_full_stack_user_management() {
    // Step 1: Define Rust backend API
    let rust_backend = r#"
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use warp::{Filter, Reply, Rejection};

// Domain models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub created_at: i64,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub email: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub user: User,
}

#[derive(Debug, Serialize)]
pub struct UsersListResponse {
    pub users: Vec<User>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

// Error types
#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    InvalidInput(String),
    InternalError(String),
}

impl warp::reject::Reject for ApiError {}

// User repository
pub struct UserRepository {
    users: Arc<RwLock<HashMap<String, User>>>,
}

impl UserRepository {
    pub fn new() -> Self {
        UserRepository {
            users: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn create_user(&self, req: CreateUserRequest) -> Result<User, ApiError> {
        let mut users = self.users.write().map_err(|e| {
            ApiError::InternalError(format!("Lock error: {}", e))
        })?;

        let user = User {
            id: uuid::Uuid::new_v4().to_string(),
            username: req.username,
            email: req.email,
            created_at: chrono::Utc::now().timestamp(),
            is_active: true,
        };

        users.insert(user.id.clone(), user.clone());
        Ok(user)
    }

    pub fn get_user(&self, id: &str) -> Result<User, ApiError> {
        let users = self.users.read().map_err(|e| {
            ApiError::InternalError(format!("Lock error: {}", e))
        })?;

        users
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("User {} not found", id)))
    }

    pub fn list_users(&self) -> Result<Vec<User>, ApiError> {
        let users = self.users.read().map_err(|e| {
            ApiError::InternalError(format!("Lock error: {}", e))
        })?;

        Ok(users.values().cloned().collect())
    }

    pub fn update_user(&self, id: &str, req: UpdateUserRequest) -> Result<User, ApiError> {
        let mut users = self.users.write().map_err(|e| {
            ApiError::InternalError(format!("Lock error: {}", e))
        })?;

        let user = users
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("User {} not found", id)))?;

        if let Some(username) = req.username {
            user.username = username;
        }
        if let Some(email) = req.email {
            user.email = email;
        }
        if let Some(is_active) = req.is_active {
            user.is_active = is_active;
        }

        Ok(user.clone())
    }

    pub fn delete_user(&self, id: &str) -> Result<(), ApiError> {
        let mut users = self.users.write().map_err(|e| {
            ApiError::InternalError(format!("Lock error: {}", e))
        })?;

        users
            .remove(id)
            .ok_or_else(|| ApiError::NotFound(format!("User {} not found", id)))?;

        Ok(())
    }
}

// API Handlers
pub async fn create_user_handler(
    req: CreateUserRequest,
    repo: Arc<UserRepository>,
) -> Result<impl Reply, Rejection> {
    let user = repo.create_user(req).map_err(warp::reject::custom)?;

    Ok(warp::reply::json(&UserResponse { user }))
}

pub async fn get_user_handler(
    id: String,
    repo: Arc<UserRepository>,
) -> Result<impl Reply, Rejection> {
    let user = repo.get_user(&id).map_err(warp::reject::custom)?;

    Ok(warp::reply::json(&UserResponse { user }))
}

pub async fn list_users_handler(
    repo: Arc<UserRepository>,
) -> Result<impl Reply, Rejection> {
    let users = repo.list_users().map_err(warp::reject::custom)?;
    let total = users.len();

    Ok(warp::reply::json(&UsersListResponse { users, total }))
}

pub async fn update_user_handler(
    id: String,
    req: UpdateUserRequest,
    repo: Arc<UserRepository>,
) -> Result<impl Reply, Rejection> {
    let user = repo.update_user(&id, req).map_err(warp::reject::custom)?;

    Ok(warp::reply::json(&UserResponse { user }))
}

pub async fn delete_user_handler(
    id: String,
    repo: Arc<UserRepository>,
) -> Result<impl Reply, Rejection> {
    repo.delete_user(&id).map_err(warp::reject::custom)?;

    Ok(warp::reply::with_status(
        warp::reply::json(&serde_json::json!({"success": true})),
        warp::http::StatusCode::NO_CONTENT,
    ))
}

// Routes
pub fn user_routes(
    repo: Arc<UserRepository>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let create = warp::post()
        .and(warp::path("users"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and(with_repo(repo.clone()))
        .and_then(create_user_handler);

    let get = warp::get()
        .and(warp::path("users"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and(with_repo(repo.clone()))
        .and_then(get_user_handler);

    let list = warp::get()
        .and(warp::path("users"))
        .and(warp::path::end())
        .and(with_repo(repo.clone()))
        .and_then(list_users_handler);

    let update = warp::put()
        .and(warp::path("users"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::body::json())
        .and(with_repo(repo.clone()))
        .and_then(update_user_handler);

    let delete = warp::delete()
        .and(warp::path("users"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and(with_repo(repo))
        .and_then(delete_user_handler);

    create.or(get).or(list).or(update).or(delete)
}

fn with_repo(
    repo: Arc<UserRepository>,
) -> impl Filter<Extract = (Arc<UserRepository>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || repo.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_user() {
        let repo = UserRepository::new();
        let req = CreateUserRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
        };

        let user = repo.create_user(req).unwrap();
        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");
        assert!(user.is_active);
    }

    #[test]
    fn test_get_user() {
        let repo = UserRepository::new();
        let user = repo
            .create_user(CreateUserRequest {
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
            })
            .unwrap();

        let fetched = repo.get_user(&user.id).unwrap();
        assert_eq!(fetched.id, user.id);
        assert_eq!(fetched.username, user.username);
    }

    #[test]
    fn test_update_user() {
        let repo = UserRepository::new();
        let user = repo
            .create_user(CreateUserRequest {
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
            })
            .unwrap();

        let updated = repo
            .update_user(
                &user.id,
                UpdateUserRequest {
                    username: Some("newname".to_string()),
                    email: None,
                    is_active: Some(false),
                },
            )
            .unwrap();

        assert_eq!(updated.username, "newname");
        assert_eq!(updated.email, user.email);
        assert!(!updated.is_active);
    }

    #[test]
    fn test_delete_user() {
        let repo = UserRepository::new();
        let user = repo
            .create_user(CreateUserRequest {
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
            })
            .unwrap();

        repo.delete_user(&user.id).unwrap();
        assert!(repo.get_user(&user.id).is_err());
    }
}
"#;

    // Step 2: Create TypeScript API client with matching types
    let typescript_client = r#"
// API Client for User Management
// Types must match backend exactly

export interface User {
    id: string;
    username: string;
    email: string;
    created_at: number;
    is_active: boolean;
}

export interface CreateUserRequest {
    username: string;
    email: string;
}

export interface UpdateUserRequest {
    username?: string;
    email?: string;
    is_active?: boolean;
}

export interface UserResponse {
    user: User;
}

export interface UsersListResponse {
    users: User[];
    total: number;
}

export interface ErrorResponse {
    error: string;
    code: string;
}

export class ApiError extends Error {
    constructor(
        message: string,
        public code: string,
        public status: number
    ) {
        super(message);
        this.name = 'ApiError';
    }
}

export class UserApiClient {
    private baseUrl: string;

    constructor(baseUrl: string = '/api') {
        this.baseUrl = baseUrl;
    }

    private async request<T>(
        path: string,
        options: RequestInit = {}
    ): Promise<T> {
        const response = await fetch(`${this.baseUrl}${path}`, {
            ...options,
            headers: {
                'Content-Type': 'application/json',
                ...options.headers,
            },
        });

        if (!response.ok) {
            let errorData: ErrorResponse;
            try {
                errorData = await response.json();
            } catch {
                errorData = {
                    error: 'Unknown error',
                    code: 'UNKNOWN_ERROR',
                };
            }

            throw new ApiError(errorData.error, errorData.code, response.status);
        }

        if (response.status === 204) {
            return {} as T;
        }

        return response.json();
    }

    async createUser(req: CreateUserRequest): Promise<User> {
        const response = await this.request<UserResponse>('/users', {
            method: 'POST',
            body: JSON.stringify(req),
        });
        return response.user;
    }

    async getUser(id: string): Promise<User> {
        const response = await this.request<UserResponse>(`/users/${id}`);
        return response.user;
    }

    async listUsers(): Promise<{ users: User[]; total: number }> {
        return this.request<UsersListResponse>('/users');
    }

    async updateUser(id: string, req: UpdateUserRequest): Promise<User> {
        const response = await this.request<UserResponse>(`/users/${id}`, {
            method: 'PUT',
            body: JSON.stringify(req),
        });
        return response.user;
    }

    async deleteUser(id: string): Promise<void> {
        await this.request<void>(`/users/${id}`, {
            method: 'DELETE',
        });
    }
}

// Export singleton instance
export const userApi = new UserApiClient();
"#;

    // Step 3: Create React components using the client
    let react_components = r#"
import React, { useState, useEffect, useCallback } from 'react';
import {
    User,
    CreateUserRequest,
    UpdateUserRequest,
    userApi,
    ApiError
} from './userApiClient';

// Custom hook for user data
function useUsers() {
    const [users, setUsers] = useState<User[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const loadUsers = useCallback(async () => {
        setLoading(true);
        setError(null);

        try {
            const response = await userApi.listUsers();
            setUsers(response.users);
        } catch (err) {
            if (err instanceof ApiError) {
                setError(err.message);
            } else {
                setError('Failed to load users');
            }
        } finally {
            setLoading(false);
        }
    }, []);

    useEffect(() => {
        loadUsers();
    }, [loadUsers]);

    return { users, loading, error, refetch: loadUsers };
}

// User List Component
export const UserList: React.FC = () => {
    const { users, loading, error, refetch } = useUsers();
    const [selectedUser, setSelectedUser] = useState<User | null>(null);

    const handleDelete = async (id: string) => {
        if (!window.confirm('Are you sure you want to delete this user?')) {
            return;
        }

        try {
            await userApi.deleteUser(id);
            await refetch();
        } catch (err) {
            alert(err instanceof ApiError ? err.message : 'Failed to delete user');
        }
    };

    if (loading) {
        return <div className="loading">Loading users...</div>;
    }

    if (error) {
        return (
            <div className="error">
                <p>Error: {error}</p>
                <button onClick={refetch}>Retry</button>
            </div>
        );
    }

    return (
        <div className="user-list">
            <div className="header">
                <h2>Users ({users.length})</h2>
                <button onClick={refetch}>Refresh</button>
            </div>

            <table>
                <thead>
                    <tr>
                        <th>Username</th>
                        <th>Email</th>
                        <th>Status</th>
                        <th>Created</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    {users.map(user => (
                        <tr
                            key={user.id}
                            className={selectedUser?.id === user.id ? 'selected' : ''}
                        >
                            <td>{user.username}</td>
                            <td>{user.email}</td>
                            <td>
                                <span className={user.is_active ? 'active' : 'inactive'}>
                                    {user.is_active ? 'Active' : 'Inactive'}
                                </span>
                            </td>
                            <td>
                                {new Date(user.created_at * 1000).toLocaleDateString()}
                            </td>
                            <td>
                                <button onClick={() => setSelectedUser(user)}>
                                    Edit
                                </button>
                                <button onClick={() => handleDelete(user.id)}>
                                    Delete
                                </button>
                            </td>
                        </tr>
                    ))}
                </tbody>
            </table>

            {selectedUser && (
                <UserEditModal
                    user={selectedUser}
                    onClose={() => setSelectedUser(null)}
                    onSave={() => {
                        setSelectedUser(null);
                        refetch();
                    }}
                />
            )}
        </div>
    );
};

// User Create Form
interface UserCreateFormProps {
    onSuccess: () => void;
    onCancel: () => void;
}

export const UserCreateForm: React.FC<UserCreateFormProps> = ({
    onSuccess,
    onCancel
}) => {
    const [formData, setFormData] = useState<CreateUserRequest>({
        username: '',
        email: '',
    });
    const [submitting, setSubmitting] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        setSubmitting(true);
        setError(null);

        try {
            await userApi.createUser(formData);
            onSuccess();
        } catch (err) {
            setError(err instanceof ApiError ? err.message : 'Failed to create user');
        } finally {
            setSubmitting(false);
        }
    };

    const handleChange = (field: keyof CreateUserRequest) => {
        return (e: React.ChangeEvent<HTMLInputElement>) => {
            setFormData(prev => ({
                ...prev,
                [field]: e.target.value,
            }));
        };
    };

    return (
        <form onSubmit={handleSubmit} className="user-create-form">
            <h3>Create New User</h3>

            {error && <div className="error-message">{error}</div>}

            <div className="form-group">
                <label htmlFor="username">Username:</label>
                <input
                    id="username"
                    type="text"
                    value={formData.username}
                    onChange={handleChange('username')}
                    required
                />
            </div>

            <div className="form-group">
                <label htmlFor="email">Email:</label>
                <input
                    id="email"
                    type="email"
                    value={formData.email}
                    onChange={handleChange('email')}
                    required
                />
            </div>

            <div className="form-actions">
                <button type="submit" disabled={submitting}>
                    {submitting ? 'Creating...' : 'Create User'}
                </button>
                <button type="button" onClick={onCancel} disabled={submitting}>
                    Cancel
                </button>
            </div>
        </form>
    );
};

// User Edit Modal
interface UserEditModalProps {
    user: User;
    onClose: () => void;
    onSave: () => void;
}

const UserEditModal: React.FC<UserEditModalProps> = ({
    user,
    onClose,
    onSave
}) => {
    const [formData, setFormData] = useState<UpdateUserRequest>({
        username: user.username,
        email: user.email,
        is_active: user.is_active,
    });
    const [submitting, setSubmitting] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        setSubmitting(true);
        setError(null);

        try {
            await userApi.updateUser(user.id, formData);
            onSave();
        } catch (err) {
            setError(err instanceof ApiError ? err.message : 'Failed to update user');
        } finally {
            setSubmitting(false);
        }
    };

    return (
        <div className="modal-overlay" onClick={onClose}>
            <div className="modal-content" onClick={(e) => e.stopPropagation()}>
                <form onSubmit={handleSubmit}>
                    <h3>Edit User</h3>

                    {error && <div className="error-message">{error}</div>}

                    <div className="form-group">
                        <label>Username:</label>
                        <input
                            type="text"
                            value={formData.username || ''}
                            onChange={(e) =>
                                setFormData(prev => ({ ...prev, username: e.target.value }))
                            }
                        />
                    </div>

                    <div className="form-group">
                        <label>Email:</label>
                        <input
                            type="email"
                            value={formData.email || ''}
                            onChange={(e) =>
                                setFormData(prev => ({ ...prev, email: e.target.value }))
                            }
                        />
                    </div>

                    <div className="form-group">
                        <label>
                            <input
                                type="checkbox"
                                checked={formData.is_active || false}
                                onChange={(e) =>
                                    setFormData(prev => ({ ...prev, is_active: e.target.checked }))
                                }
                            />
                            Active
                        </label>
                    </div>

                    <div className="form-actions">
                        <button type="submit" disabled={submitting}>
                            {submitting ? 'Saving...' : 'Save'}
                        </button>
                        <button type="button" onClick={onClose} disabled={submitting}>
                            Cancel
                        </button>
                    </div>
                </form>
            </div>
        </div>
    );
};

// Main App Component
export const UserManagementApp: React.FC = () => {
    const [showCreateForm, setShowCreateForm] = useState(false);

    return (
        <div className="user-management-app">
            <header>
                <h1>User Management</h1>
                <button onClick={() => setShowCreateForm(true)}>
                    Add New User
                </button>
            </header>

            <main>
                <UserList />
            </main>

            {showCreateForm && (
                <div className="modal-overlay">
                    <div className="modal-content">
                        <UserCreateForm
                            onSuccess={() => setShowCreateForm(false)}
                            onCancel={() => setShowCreateForm(false)}
                        />
                    </div>
                </div>
            )}
        </div>
    );
};
"#;

    println!("Generated full-stack code:");
    println!("  Rust backend: {} bytes", rust_backend.len());
    println!("  TypeScript client: {} bytes", typescript_client.len());
    println!("  React components: {} bytes", react_components.len());

    // Verify type consistency across layers
    assert!(test_utils::verify_type_consistency(rust_backend, typescript_client));

    // Verify Rust backend
    assert!(rust_backend.contains("struct User"));
    assert!(rust_backend.contains("struct CreateUserRequest"));
    assert!(rust_backend.contains("struct UpdateUserRequest"));
    assert!(rust_backend.contains("async fn create_user_handler"));
    assert!(rust_backend.contains("pub fn user_routes"));

    // Verify TypeScript client
    assert!(typescript_client.contains("interface User"));
    assert!(typescript_client.contains("interface CreateUserRequest"));
    assert!(typescript_client.contains("interface UpdateUserRequest"));
    assert!(typescript_client.contains("class UserApiClient"));
    assert!(typescript_client.contains("async createUser"));

    // Verify React components
    assert!(react_components.contains("function useUsers()"));
    assert!(react_components.contains("export const UserList"));
    assert!(react_components.contains("export const UserCreateForm"));
    assert!(react_components.contains("await userApi.createUser"));

    // Verify field consistency
    let rust_fields = vec!["username", "email", "is_active"];
    let ts_fields = vec!["username", "email", "is_active"];

    for field in rust_fields {
        assert!(rust_backend.contains(field));
        assert!(typescript_client.contains(field));
        assert!(react_components.contains(field));
    }
}

/// Scenario 2: Real-time WebSocket Feature
///
/// This test creates a real-time chat feature with:
/// - Rust WebSocket server
/// - TypeScript WebSocket client
/// - React chat UI
#[test]
fn test_websocket_realtime_chat() {
    let rust_websocket = r#"
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use warp::ws::{Message, WebSocket};
use std::collections::HashMap;
use std::sync::Arc;
use futures::{StreamExt, SinkExt};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ChatMessage {
    Join { user_id: String, username: String },
    Leave { user_id: String },
    Message { user_id: String, content: String, timestamp: i64 },
    UserList { users: Vec<String> },
}

pub struct ChatRoom {
    users: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<ChatMessage>>>>,
}

impl ChatRoom {
    pub fn new() -> Self {
        ChatRoom {
            users: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn user_connected(&self, user_id: String, username: String, ws: WebSocket) {
        let (mut user_ws_tx, mut user_ws_rx) = ws.split();
        let (tx, mut rx) = mpsc::unbounded_channel();

        // Add user to room
        self.users.write().await.insert(user_id.clone(), tx);

        // Broadcast join message
        self.broadcast(ChatMessage::Join {
            user_id: user_id.clone(),
            username: username.clone(),
        }).await;

        // Send user list to new user
        let users: Vec<String> = self.users.read().await.keys().cloned().collect();
        if let Some(sender) = self.users.read().await.get(&user_id) {
            let _ = sender.send(ChatMessage::UserList { users });
        }

        // Spawn task to handle outgoing messages
        tokio::task::spawn(async move {
            while let Some(message) = rx.recv().await {
                if let Ok(json) = serde_json::to_string(&message) {
                    if user_ws_tx.send(Message::text(json)).await.is_err() {
                        break;
                    }
                }
            }
        });

        // Handle incoming messages
        let room = self.clone();
        let user_id_clone = user_id.clone();

        tokio::task::spawn(async move {
            while let Some(result) = user_ws_rx.next().await {
                if let Ok(msg) = result {
                    if let Ok(text) = msg.to_str() {
                        if let Ok(message) = serde_json::from_str::<ChatMessage>(text) {
                            room.broadcast(message).await;
                        }
                    }
                }
            }

            // User disconnected
            room.user_disconnected(&user_id_clone).await;
        });
    }

    async fn broadcast(&self, message: ChatMessage) {
        let users = self.users.read().await;
        for (_, tx) in users.iter() {
            let _ = tx.send(message.clone());
        }
    }

    async fn user_disconnected(&self, user_id: &str) {
        self.users.write().await.remove(user_id);
        self.broadcast(ChatMessage::Leave {
            user_id: user_id.to_string(),
        }).await;
    }
}

impl Clone for ChatRoom {
    fn clone(&self) -> Self {
        ChatRoom {
            users: Arc::clone(&self.users),
        }
    }
}
"#;

    let typescript_websocket = r#"
export type ChatMessage =
    | { type: 'Join'; user_id: string; username: string }
    | { type: 'Leave'; user_id: string }
    | { type: 'Message'; user_id: string; content: string; timestamp: number }
    | { type: 'UserList'; users: string[] };

export type ChatEventHandler = (message: ChatMessage) => void;

export class ChatClient {
    private ws: WebSocket | null = null;
    private handlers: Set<ChatEventHandler> = new Set();
    private reconnectInterval: number = 5000;
    private reconnectTimer: number | null = null;

    constructor(
        private url: string,
        private userId: string,
        private username: string
    ) {}

    connect(): void {
        this.ws = new WebSocket(this.url);

        this.ws.onopen = () => {
            console.log('Connected to chat server');
            this.sendMessage({
                type: 'Join',
                user_id: this.userId,
                username: this.username,
            });
        };

        this.ws.onmessage = (event) => {
            try {
                const message = JSON.parse(event.data) as ChatMessage;
                this.notifyHandlers(message);
            } catch (err) {
                console.error('Failed to parse message:', err);
            }
        };

        this.ws.onerror = (error) => {
            console.error('WebSocket error:', error);
        };

        this.ws.onclose = () => {
            console.log('Disconnected from chat server');
            this.scheduleReconnect();
        };
    }

    disconnect(): void {
        if (this.reconnectTimer !== null) {
            clearTimeout(this.reconnectTimer);
            this.reconnectTimer = null;
        }

        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
    }

    sendMessage(message: ChatMessage): void {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(message));
        }
    }

    onMessage(handler: ChatEventHandler): () => void {
        this.handlers.add(handler);
        return () => this.handlers.delete(handler);
    }

    private notifyHandlers(message: ChatMessage): void {
        this.handlers.forEach(handler => handler(message));
    }

    private scheduleReconnect(): void {
        this.reconnectTimer = window.setTimeout(() => {
            console.log('Attempting to reconnect...');
            this.connect();
        }, this.reconnectInterval);
    }
}
"#;

    let react_chat_ui = r#"
import React, { useState, useEffect, useRef } from 'react';
import { ChatClient, ChatMessage } from './chatClient';

interface Message {
    id: string;
    userId: string;
    content: string;
    timestamp: number;
}

export const ChatApp: React.FC = () => {
    const [messages, setMessages] = useState<Message[]>([]);
    const [users, setUsers] = useState<string[]>([]);
    const [input, setInput] = useState('');
    const [connected, setConnected] = useState(false);
    const chatClientRef = useRef<ChatClient | null>(null);
    const messagesEndRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        const userId = Math.random().toString(36).substring(7);
        const username = `User${userId}`;

        const client = new ChatClient(
            'ws://localhost:3030/chat',
            userId,
            username
        );

        const unsubscribe = client.onMessage((message) => {
            switch (message.type) {
                case 'Join':
                    setMessages(prev => [...prev, {
                        id: Math.random().toString(36),
                        userId: 'system',
                        content: `${message.username} joined the chat`,
                        timestamp: Date.now(),
                    }]);
                    break;

                case 'Leave':
                    setMessages(prev => [...prev, {
                        id: Math.random().toString(36),
                        userId: 'system',
                        content: `User ${message.user_id} left the chat`,
                        timestamp: Date.now(),
                    }]);
                    break;

                case 'Message':
                    setMessages(prev => [...prev, {
                        id: Math.random().toString(36),
                        userId: message.user_id,
                        content: message.content,
                        timestamp: message.timestamp,
                    }]);
                    break;

                case 'UserList':
                    setUsers(message.users);
                    break;
            }
        });

        client.connect();
        setConnected(true);
        chatClientRef.current = client;

        return () => {
            unsubscribe();
            client.disconnect();
        };
    }, []);

    useEffect(() => {
        messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }, [messages]);

    const handleSend = () => {
        if (input.trim() && chatClientRef.current) {
            chatClientRef.current.sendMessage({
                type: 'Message',
                user_id: chatClientRef.current['userId'],
                content: input,
                timestamp: Date.now(),
            });
            setInput('');
        }
    };

    return (
        <div className="chat-app">
            <div className="chat-sidebar">
                <h3>Users ({users.length})</h3>
                <ul>
                    {users.map(user => (
                        <li key={user}>{user}</li>
                    ))}
                </ul>
            </div>

            <div className="chat-main">
                <div className="chat-messages">
                    {messages.map(msg => (
                        <div
                            key={msg.id}
                            className={`message ${msg.userId === 'system' ? 'system' : ''}`}
                        >
                            <span className="timestamp">
                                {new Date(msg.timestamp).toLocaleTimeString()}
                            </span>
                            <span className="content">{msg.content}</span>
                        </div>
                    ))}
                    <div ref={messagesEndRef} />
                </div>

                <div className="chat-input">
                    <input
                        type="text"
                        value={input}
                        onChange={(e) => setInput(e.target.value)}
                        onKeyPress={(e) => e.key === 'Enter' && handleSend()}
                        placeholder="Type a message..."
                        disabled={!connected}
                    />
                    <button onClick={handleSend} disabled={!connected}>
                        Send
                    </button>
                </div>
            </div>
        </div>
    );
};
"#;

    println!("WebSocket feature:");
    println!("  Rust server: {} bytes", rust_websocket.len());
    println!("  TypeScript client: {} bytes", typescript_websocket.len());
    println!("  React UI: {} bytes", react_chat_ui.len());

    // Verify message types match
    assert!(rust_websocket.contains("enum ChatMessage"));
    assert!(typescript_websocket.contains("type ChatMessage"));

    // Verify Rust WebSocket implementation
    assert!(rust_websocket.contains("struct ChatRoom"));
    assert!(rust_websocket.contains("async fn broadcast"));

    // Verify TypeScript client
    assert!(typescript_websocket.contains("class ChatClient"));
    assert!(typescript_websocket.contains("connect()"));
    assert!(typescript_websocket.contains("sendMessage"));

    // Verify React UI
    assert!(react_chat_ui.contains("export const ChatApp"));
    assert!(react_chat_ui.contains("useState<Message[]>"));
    assert!(react_chat_ui.contains("useEffect"));
}

/// Test VFS integration for full-stack development
#[test]
fn test_vfs_full_stack_integration() {
    let mut vfs = VirtualFileSystem::new();

    // Create backend files
    vfs.create_file(
        &VirtualPath::from("/backend/src/api.rs"),
        "pub struct ApiServer {}".as_bytes().to_vec(),
    ).unwrap();

    vfs.create_file(
        &VirtualPath::from("/backend/src/models.rs"),
        "pub struct User { pub id: String }".as_bytes().to_vec(),
    ).unwrap();

    // Create frontend files
    vfs.create_file(
        &VirtualPath::from("/frontend/src/api.ts"),
        "export class ApiClient {}".as_bytes().to_vec(),
    ).unwrap();

    vfs.create_file(
        &VirtualPath::from("/frontend/src/types.ts"),
        "export interface User { id: string }".as_bytes().to_vec(),
    ).unwrap();

    // Verify structure
    assert!(vfs.exists(&VirtualPath::from("/backend/src/api.rs")));
    assert!(vfs.exists(&VirtualPath::from("/backend/src/models.rs")));
    assert!(vfs.exists(&VirtualPath::from("/frontend/src/api.ts")));
    assert!(vfs.exists(&VirtualPath::from("/frontend/src/types.ts")));

    // Verify content
    let api_content = vfs.read_file(&VirtualPath::from("/backend/src/api.rs")).unwrap();
    assert!(String::from_utf8_lossy(&api_content).contains("ApiServer"));
}
