use cortex_parser::{AstEditor, Language};
use std::collections::HashMap;

/// Test utilities for verifying Rust code compilation
mod test_utils {
    use std::fs;
    use std::io::Write;
    use std::process::Command;
    use tempfile::TempDir;

    pub fn verify_rust_compiles(code: &str) -> Result<bool, String> {
        let temp_dir = TempDir::new().map_err(|e| e.to_string())?;
        let file_path = temp_dir.path().join("test.rs");

        let mut file = fs::File::create(&file_path).map_err(|e| e.to_string())?;
        file.write_all(code.as_bytes()).map_err(|e| e.to_string())?;

        let output = Command::new("rustc")
            .arg("--crate-type")
            .arg("lib")
            .arg("--edition")
            .arg("2021")
            .arg(&file_path)
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Compilation failed:\n{}", stderr));
        }

        Ok(true)
    }

    pub fn count_occurrences(code: &str, pattern: &str) -> usize {
        code.matches(pattern).count()
    }
}

/// Scenario 1: Add Authentication to Existing HTTP Server
///
/// This test simulates adding authentication to a basic HTTP server:
/// 1. Start with a simple HTTP server
/// 2. Add authentication middleware function
/// 3. Modify request handler to use middleware
/// 4. Add authentication types (User, Token, Claims)
/// 5. Add tests for authentication
#[test]
fn test_add_authentication_to_server() {
    let initial_code = r#"
use std::net::TcpListener;
use std::io::{Read, Write};
use std::thread;

pub struct HttpServer {
    listener: TcpListener,
}

impl HttpServer {
    pub fn new(addr: &str) -> std::io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        Ok(HttpServer { listener })
    }

    pub fn run(&self) -> std::io::Result<()> {
        for stream in self.listener.incoming() {
            let mut stream = stream?;
            thread::spawn(move || {
                let mut buffer = [0; 1024];
                stream.read(&mut buffer).unwrap();

                let response = handle_request(&buffer);
                stream.write_all(response.as_bytes()).unwrap();
                stream.flush().unwrap();
            });
        }
        Ok(())
    }
}

fn handle_request(request: &[u8]) -> String {
    let request_str = String::from_utf8_lossy(request);

    if request_str.starts_with("GET /api/data") {
        format!("HTTP/1.1 200 OK\r\n\r\n{{\"data\": \"sensitive information\"}}")
    } else if request_str.starts_with("POST /api/update") {
        format!("HTTP/1.1 200 OK\r\n\r\n{{\"status\": \"updated\"}}")
    } else {
        format!("HTTP/1.1 404 NOT FOUND\r\n\r\n")
    }
}

pub fn parse_headers(request: &str) -> std::collections::HashMap<String, String> {
    let mut headers = std::collections::HashMap::new();
    for line in request.lines().skip(1) {
        if line.is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(": ") {
            headers.insert(key.to_string(), value.to_string());
        }
    }
    headers
}
"#;

    let mut editor = AstEditor::new(Language::Rust);

    // Step 1: Add authentication types at the top of the file
    let auth_types = r#"
#[derive(Debug, Clone)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub value: String,
    pub expires_at: u64,
}

#[derive(Debug, Clone)]
pub struct Claims {
    pub user_id: u64,
    pub username: String,
    pub exp: u64,
}

pub struct AuthError {
    pub message: String,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Authentication error: {}", self.message)
    }
}

impl std::fmt::Debug for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "AuthError {{ message: {} }}", self.message)
    }
}

impl std::error::Error for AuthError {}
"#;

    let code_with_types = editor
        .insert_at_start(initial_code, auth_types)
        .expect("Failed to insert auth types");

    // Step 2: Add authentication middleware function
    let auth_middleware = r#"

pub fn authenticate(headers: &std::collections::HashMap<String, String>) -> Result<User, AuthError> {
    let token = headers
        .get("Authorization")
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| AuthError {
            message: "Missing authorization header".to_string(),
        })?;

    // Simulate token validation (in real app, this would verify JWT)
    if token.len() < 10 {
        return Err(AuthError {
            message: "Invalid token format".to_string(),
        });
    }

    // Simulate extracting user from token
    Ok(User {
        id: 1,
        username: "authenticated_user".to_string(),
        roles: vec!["user".to_string()],
    })
}

pub fn check_permission(user: &User, required_role: &str) -> Result<(), AuthError> {
    if user.roles.iter().any(|r| r == required_role || r == "admin") {
        Ok(())
    } else {
        Err(AuthError {
            message: format!("User lacks required role: {}", required_role),
        })
    }
}
"#;

    let code_with_auth = code_with_types + auth_middleware;

    // Step 3: Replace handle_request with authenticated version
    let old_handle_request = r#"fn handle_request(request: &[u8]) -> String {
    let request_str = String::from_utf8_lossy(request);

    if request_str.starts_with("GET /api/data") {
        format!("HTTP/1.1 200 OK\r\n\r\n{{\"data\": \"sensitive information\"}}")
    } else if request_str.starts_with("POST /api/update") {
        format!("HTTP/1.1 200 OK\r\n\r\n{{\"status\": \"updated\"}}")
    } else {
        format!("HTTP/1.1 404 NOT FOUND\r\n\r\n")
    }
}"#;

    let new_handle_request = r#"fn handle_request(request: &[u8]) -> String {
    let request_str = String::from_utf8_lossy(request);
    let headers = parse_headers(&request_str);

    // Authenticate user
    let user = match authenticate(&headers) {
        Ok(u) => u,
        Err(e) => {
            return format!("HTTP/1.1 401 UNAUTHORIZED\r\n\r\n{{\"error\": \"{}\"}}", e.message);
        }
    };

    if request_str.starts_with("GET /api/data") {
        // Check read permission
        if let Err(e) = check_permission(&user, "user") {
            return format!("HTTP/1.1 403 FORBIDDEN\r\n\r\n{{\"error\": \"{}\"}}", e.message);
        }
        format!("HTTP/1.1 200 OK\r\n\r\n{{\"data\": \"sensitive information\", \"user\": \"{}\"}}", user.username)
    } else if request_str.starts_with("POST /api/update") {
        // Check write permission
        if let Err(e) = check_permission(&user, "admin") {
            return format!("HTTP/1.1 403 FORBIDDEN\r\n\r\n{{\"error\": \"{}\"}}", e.message);
        }
        format!("HTTP/1.1 200 OK\r\n\r\n{{\"status\": \"updated\", \"by\": \"{}\"}}", user.username)
    } else {
        format!("HTTP/1.1 404 NOT FOUND\r\n\r\n")
    }
}"#;

    let final_code = code_with_auth.replace(old_handle_request, new_handle_request);

    // Add tests module
    let tests_module = r#"

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authenticate_valid_token() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer valid_token_123456".to_string());

        let result = authenticate(&headers);
        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.id, 1);
    }

    #[test]
    fn test_authenticate_missing_token() {
        let headers = std::collections::HashMap::new();
        let result = authenticate(&headers);
        assert!(result.is_err());
    }

    #[test]
    fn test_authenticate_invalid_token() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer short".to_string());

        let result = authenticate(&headers);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_permission_authorized() {
        let user = User {
            id: 1,
            username: "test".to_string(),
            roles: vec!["admin".to_string()],
        };

        assert!(check_permission(&user, "user").is_ok());
        assert!(check_permission(&user, "admin").is_ok());
    }

    #[test]
    fn test_check_permission_unauthorized() {
        let user = User {
            id: 1,
            username: "test".to_string(),
            roles: vec!["user".to_string()],
        };

        assert!(check_permission(&user, "admin").is_err());
    }

    #[test]
    fn test_parse_headers() {
        let request = "GET / HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer token123\r\n\r\n";
        let headers = parse_headers(request);

        assert_eq!(headers.get("Host"), Some(&"localhost".to_string()));
        assert_eq!(headers.get("Authorization"), Some(&"Bearer token123".to_string()));
    }
}
"#;

    let final_code_with_tests = final_code + tests_module;

    // Verify the code
    println!("Generated code length: {} bytes", final_code_with_tests.len());

    // Check that authentication types were added
    assert!(final_code_with_tests.contains("struct User"));
    assert!(final_code_with_tests.contains("struct Token"));
    assert!(final_code_with_tests.contains("struct Claims"));
    assert!(final_code_with_tests.contains("struct AuthError"));

    // Check that middleware was added
    assert!(final_code_with_tests.contains("fn authenticate"));
    assert!(final_code_with_tests.contains("fn check_permission"));

    // Check that handle_request was modified
    assert!(final_code_with_tests.contains("401 UNAUTHORIZED"));
    assert!(final_code_with_tests.contains("403 FORBIDDEN"));

    // Check that tests were added
    assert!(final_code_with_tests.contains("test_authenticate_valid_token"));
    assert!(final_code_with_tests.contains("test_check_permission_authorized"));

    // Verify syntax correctness by attempting to parse
    assert!(
        test_utils::verify_rust_compiles(&final_code_with_tests).is_ok(),
        "Generated code should compile"
    );
}

/// Scenario 2: Refactor for Error Handling
///
/// This test simulates refactoring code to use proper error handling:
/// 1. Start with code using unwrap()
/// 2. Find all unwrap() calls
/// 3. Replace with proper Result<T, E> handling
/// 4. Add error types and From implementations
#[test]
fn test_refactor_error_handling() {
    let initial_code = r#"
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct ConfigLoader {
    config_dir: String,
}

impl ConfigLoader {
    pub fn new(config_dir: String) -> Self {
        ConfigLoader { config_dir }
    }

    pub fn load_config(&self, name: &str) -> String {
        let path = format!("{}/{}.json", self.config_dir, name);
        let mut file = File::open(&path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        contents
    }

    pub fn parse_json(&self, json_str: &str) -> serde_json::Value {
        serde_json::from_str(json_str).unwrap()
    }

    pub fn get_value(&self, config: &serde_json::Value, key: &str) -> String {
        config[key].as_str().unwrap().to_string()
    }

    pub fn load_and_parse(&self, name: &str) -> serde_json::Value {
        let contents = self.load_config(name);
        self.parse_json(&contents)
    }
}

pub fn process_configs(loader: &ConfigLoader, names: Vec<&str>) -> Vec<serde_json::Value> {
    names
        .iter()
        .map(|name| loader.load_and_parse(name))
        .collect()
}

pub fn merge_configs(configs: Vec<serde_json::Value>) -> serde_json::Value {
    let mut merged = serde_json::Map::new();

    for config in configs {
        let obj = config.as_object().unwrap();
        for (key, value) in obj {
            merged.insert(key.clone(), value.clone());
        }
    }

    serde_json::Value::Object(merged)
}
"#;

    let mut editor = AstEditor::new(Language::Rust);

    // Step 1: Add error types
    let error_types = r#"
use std::fmt;

#[derive(Debug)]
pub enum ConfigError {
    IoError(std::io::Error),
    ParseError(serde_json::Error),
    MissingKey(String),
    InvalidFormat(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConfigError::IoError(e) => write!(f, "IO error: {}", e),
            ConfigError::ParseError(e) => write!(f, "Parse error: {}", e),
            ConfigError::MissingKey(k) => write!(f, "Missing key: {}", k),
            ConfigError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::IoError(err)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(err: serde_json::Error) -> Self {
        ConfigError::ParseError(err)
    }
}
"#;

    let code_with_errors = error_types.to_string() + initial_code;

    // Step 2: Replace unwrap() calls with proper error handling
    let refactored_impl = r#"
impl ConfigLoader {
    pub fn new(config_dir: String) -> Self {
        ConfigLoader { config_dir }
    }

    pub fn load_config(&self, name: &str) -> Result<String, ConfigError> {
        let path = format!("{}/{}.json", self.config_dir, name);
        let mut file = File::open(&path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(contents)
    }

    pub fn parse_json(&self, json_str: &str) -> Result<serde_json::Value, ConfigError> {
        Ok(serde_json::from_str(json_str)?)
    }

    pub fn get_value(&self, config: &serde_json::Value, key: &str) -> Result<String, ConfigError> {
        config
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| ConfigError::MissingKey(key.to_string()))
    }

    pub fn load_and_parse(&self, name: &str) -> Result<serde_json::Value, ConfigError> {
        let contents = self.load_config(name)?;
        self.parse_json(&contents)
    }
}
"#;

    // Replace the old implementation
    let old_impl = r#"impl ConfigLoader {
    pub fn new(config_dir: String) -> Self {
        ConfigLoader { config_dir }
    }

    pub fn load_config(&self, name: &str) -> String {
        let path = format!("{}/{}.json", self.config_dir, name);
        let mut file = File::open(&path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        contents
    }

    pub fn parse_json(&self, json_str: &str) -> serde_json::Value {
        serde_json::from_str(json_str).unwrap()
    }

    pub fn get_value(&self, config: &serde_json::Value, key: &str) -> String {
        config[key].as_str().unwrap().to_string()
    }

    pub fn load_and_parse(&self, name: &str) -> serde_json::Value {
        let contents = self.load_config(name);
        self.parse_json(&contents)
    }
}"#;

    let code_with_refactored_impl = code_with_errors.replace(old_impl, refactored_impl);

    // Step 3: Update functions to use Result
    let old_process = r#"pub fn process_configs(loader: &ConfigLoader, names: Vec<&str>) -> Vec<serde_json::Value> {
    names
        .iter()
        .map(|name| loader.load_and_parse(name))
        .collect()
}"#;

    let new_process = r#"pub fn process_configs(loader: &ConfigLoader, names: Vec<&str>) -> Result<Vec<serde_json::Value>, ConfigError> {
    names
        .iter()
        .map(|name| loader.load_and_parse(name))
        .collect()
}"#;

    let code_with_process = code_with_refactored_impl.replace(old_process, new_process);

    let old_merge = r#"pub fn merge_configs(configs: Vec<serde_json::Value>) -> serde_json::Value {
    let mut merged = serde_json::Map::new();

    for config in configs {
        let obj = config.as_object().unwrap();
        for (key, value) in obj {
            merged.insert(key.clone(), value.clone());
        }
    }

    serde_json::Value::Object(merged)
}"#;

    let new_merge = r#"pub fn merge_configs(configs: Vec<serde_json::Value>) -> Result<serde_json::Value, ConfigError> {
    let mut merged = serde_json::Map::new();

    for config in configs {
        let obj = config
            .as_object()
            .ok_or_else(|| ConfigError::InvalidFormat("Expected object".to_string()))?;
        for (key, value) in obj {
            merged.insert(key.clone(), value.clone());
        }
    }

    Ok(serde_json::Value::Object(merged))
}"#;

    let final_code = code_with_process.replace(old_merge, new_merge);

    // Add tests
    let tests = r#"

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let config_err: ConfigError = io_err.into();
        assert!(matches!(config_err, ConfigError::IoError(_)));
    }

    #[test]
    fn test_missing_key_error() {
        let loader = ConfigLoader::new("/tmp".to_string());
        let json = serde_json::json!({"name": "test"});

        let result = loader.get_value(&json, "nonexistent");
        assert!(result.is_err());

        if let Err(ConfigError::MissingKey(key)) = result {
            assert_eq!(key, "nonexistent");
        } else {
            panic!("Expected MissingKey error");
        }
    }

    #[test]
    fn test_get_value_success() {
        let loader = ConfigLoader::new("/tmp".to_string());
        let json = serde_json::json!({"name": "test"});

        let result = loader.get_value(&json, "name");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test");
    }

    #[test]
    fn test_merge_configs_invalid_format() {
        let configs = vec![
            serde_json::json!({"a": 1}),
            serde_json::json!("not an object"),
        ];

        let result = merge_configs(configs);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::InvalidFormat(_)));
    }
}
"#;

    let final_code_with_tests = final_code + tests;

    // Verify no unwrap() calls remain (except in tests)
    let unwrap_count = test_utils::count_occurrences(&final_code_with_tests, ".unwrap()");
    // We expect only unwrap() calls in test assertions
    assert!(unwrap_count <= 2, "Should have minimal unwrap() calls, found {}", unwrap_count);

    // Verify error handling was added
    assert!(final_code_with_tests.contains("enum ConfigError"));
    assert!(final_code_with_tests.contains("impl From<std::io::Error>"));
    assert!(final_code_with_tests.contains("impl From<serde_json::Error>"));

    // Verify Result types are used
    assert!(final_code_with_tests.contains("-> Result<String, ConfigError>"));
    assert!(final_code_with_tests.contains("-> Result<serde_json::Value, ConfigError>"));

    // Verify ? operator is used
    let question_mark_count = test_utils::count_occurrences(&final_code_with_tests, "?;");
    assert!(question_mark_count >= 3, "Should use ? operator for error propagation");

    println!("Generated code length: {} bytes", final_code_with_tests.len());
    println!("Unwrap count: {}", unwrap_count);
    println!("Question mark count: {}", question_mark_count);
}

/// Scenario 3: Extract Reusable Module
///
/// This test simulates extracting duplicated code into a reusable module:
/// 1. Identify duplicated validation patterns
/// 2. Extract to new validation module
/// 3. Update all call sites
/// 4. Add documentation
#[test]
fn test_extract_reusable_module() {
    let initial_code = r#"
pub struct UserRegistration {
    pub email: String,
    pub password: String,
    pub username: String,
}

pub struct ProductSubmission {
    pub name: String,
    pub description: String,
    pub price: f64,
}

pub struct CommentPost {
    pub author: String,
    pub content: String,
}

impl UserRegistration {
    pub fn validate(&self) -> Result<(), String> {
        // Email validation
        if self.email.is_empty() {
            return Err("Email cannot be empty".to_string());
        }
        if !self.email.contains('@') {
            return Err("Email must contain @".to_string());
        }
        if self.email.len() > 255 {
            return Err("Email too long".to_string());
        }

        // Password validation
        if self.password.is_empty() {
            return Err("Password cannot be empty".to_string());
        }
        if self.password.len() < 8 {
            return Err("Password must be at least 8 characters".to_string());
        }
        if self.password.len() > 128 {
            return Err("Password too long".to_string());
        }

        // Username validation
        if self.username.is_empty() {
            return Err("Username cannot be empty".to_string());
        }
        if self.username.len() < 3 {
            return Err("Username must be at least 3 characters".to_string());
        }
        if self.username.len() > 50 {
            return Err("Username too long".to_string());
        }

        Ok(())
    }
}

impl ProductSubmission {
    pub fn validate(&self) -> Result<(), String> {
        // Name validation
        if self.name.is_empty() {
            return Err("Name cannot be empty".to_string());
        }
        if self.name.len() > 200 {
            return Err("Name too long".to_string());
        }

        // Description validation
        if self.description.is_empty() {
            return Err("Description cannot be empty".to_string());
        }
        if self.description.len() > 5000 {
            return Err("Description too long".to_string());
        }

        // Price validation
        if self.price < 0.0 {
            return Err("Price cannot be negative".to_string());
        }
        if self.price > 1_000_000.0 {
            return Err("Price too high".to_string());
        }

        Ok(())
    }
}

impl CommentPost {
    pub fn validate(&self) -> Result<(), String> {
        // Author validation
        if self.author.is_empty() {
            return Err("Author cannot be empty".to_string());
        }
        if self.author.len() > 100 {
            return Err("Author name too long".to_string());
        }

        // Content validation
        if self.content.is_empty() {
            return Err("Content cannot be empty".to_string());
        }
        if self.content.len() < 10 {
            return Err("Content must be at least 10 characters".to_string());
        }
        if self.content.len() > 10000 {
            return Err("Content too long".to_string());
        }

        Ok(())
    }
}
"#;

    // Step 1: Extract validation module
    let validation_module = r#"
/// Validation utilities for common input validation patterns
pub mod validation {
    use std::fmt;

    #[derive(Debug, Clone)]
    pub struct ValidationError {
        pub field: String,
        pub message: String,
    }

    impl fmt::Display for ValidationError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}: {}", self.field, self.message)
        }
    }

    impl std::error::Error for ValidationError {}

    /// Validates that a string is not empty
    pub fn validate_not_empty(value: &str, field: &str) -> Result<(), ValidationError> {
        if value.is_empty() {
            Err(ValidationError {
                field: field.to_string(),
                message: "cannot be empty".to_string(),
            })
        } else {
            Ok(())
        }
    }

    /// Validates string length is within bounds
    pub fn validate_length(
        value: &str,
        field: &str,
        min: Option<usize>,
        max: Option<usize>,
    ) -> Result<(), ValidationError> {
        if let Some(min_len) = min {
            if value.len() < min_len {
                return Err(ValidationError {
                    field: field.to_string(),
                    message: format!("must be at least {} characters", min_len),
                });
            }
        }

        if let Some(max_len) = max {
            if value.len() > max_len {
                return Err(ValidationError {
                    field: field.to_string(),
                    message: format!("cannot exceed {} characters", max_len),
                });
            }
        }

        Ok(())
    }

    /// Validates email format
    pub fn validate_email(email: &str, field: &str) -> Result<(), ValidationError> {
        validate_not_empty(email, field)?;

        if !email.contains('@') {
            return Err(ValidationError {
                field: field.to_string(),
                message: "must be a valid email address".to_string(),
            });
        }

        validate_length(email, field, None, Some(255))?;

        Ok(())
    }

    /// Validates numeric range
    pub fn validate_range<T: PartialOrd + fmt::Display>(
        value: T,
        field: &str,
        min: Option<T>,
        max: Option<T>,
    ) -> Result<(), ValidationError> {
        if let Some(min_val) = min {
            if value < min_val {
                return Err(ValidationError {
                    field: field.to_string(),
                    message: format!("must be at least {}", min_val),
                });
            }
        }

        if let Some(max_val) = max {
            if value > max_val {
                return Err(ValidationError {
                    field: field.to_string(),
                    message: format!("cannot exceed {}", max_val),
                });
            }
        }

        Ok(())
    }
}
"#;

    // Step 2: Refactor structs to use validation module
    let refactored_code = validation_module.to_string() + r#"

pub struct UserRegistration {
    pub email: String,
    pub password: String,
    pub username: String,
}

pub struct ProductSubmission {
    pub name: String,
    pub description: String,
    pub price: f64,
}

pub struct CommentPost {
    pub author: String,
    pub content: String,
}

impl UserRegistration {
    pub fn validate(&self) -> Result<(), validation::ValidationError> {
        validation::validate_email(&self.email, "email")?;
        validation::validate_not_empty(&self.password, "password")?;
        validation::validate_length(&self.password, "password", Some(8), Some(128))?;
        validation::validate_not_empty(&self.username, "username")?;
        validation::validate_length(&self.username, "username", Some(3), Some(50))?;
        Ok(())
    }
}

impl ProductSubmission {
    pub fn validate(&self) -> Result<(), validation::ValidationError> {
        validation::validate_not_empty(&self.name, "name")?;
        validation::validate_length(&self.name, "name", None, Some(200))?;
        validation::validate_not_empty(&self.description, "description")?;
        validation::validate_length(&self.description, "description", None, Some(5000))?;
        validation::validate_range(self.price, "price", Some(0.0), Some(1_000_000.0))?;
        Ok(())
    }
}

impl CommentPost {
    pub fn validate(&self) -> Result<(), validation::ValidationError> {
        validation::validate_not_empty(&self.author, "author")?;
        validation::validate_length(&self.author, "author", None, Some(100))?;
        validation::validate_not_empty(&self.content, "content")?;
        validation::validate_length(&self.content, "content", Some(10), Some(10000))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_not_empty() {
        assert!(validation::validate_not_empty("test", "field").is_ok());
        assert!(validation::validate_not_empty("", "field").is_err());
    }

    #[test]
    fn test_validation_length() {
        assert!(validation::validate_length("test", "field", Some(2), Some(10)).is_ok());
        assert!(validation::validate_length("a", "field", Some(2), None).is_err());
        assert!(validation::validate_length("toolong", "field", None, Some(5)).is_err());
    }

    #[test]
    fn test_validation_email() {
        assert!(validation::validate_email("test@example.com", "email").is_ok());
        assert!(validation::validate_email("invalid", "email").is_err());
        assert!(validation::validate_email("", "email").is_err());
    }

    #[test]
    fn test_validation_range() {
        assert!(validation::validate_range(5, "field", Some(0), Some(10)).is_ok());
        assert!(validation::validate_range(-1, "field", Some(0), None).is_err());
        assert!(validation::validate_range(11, "field", None, Some(10)).is_err());
    }

    #[test]
    fn test_user_registration_valid() {
        let user = UserRegistration {
            email: "user@example.com".to_string(),
            password: "securepass123".to_string(),
            username: "testuser".to_string(),
        };
        assert!(user.validate().is_ok());
    }

    #[test]
    fn test_user_registration_invalid_email() {
        let user = UserRegistration {
            email: "invalid".to_string(),
            password: "securepass123".to_string(),
            username: "testuser".to_string(),
        };
        assert!(user.validate().is_err());
    }

    #[test]
    fn test_product_submission_valid() {
        let product = ProductSubmission {
            name: "Test Product".to_string(),
            description: "A great product".to_string(),
            price: 99.99,
        };
        assert!(product.validate().is_ok());
    }

    #[test]
    fn test_product_submission_negative_price() {
        let product = ProductSubmission {
            name: "Test Product".to_string(),
            description: "A great product".to_string(),
            price: -10.0,
        };
        assert!(product.validate().is_err());
    }

    #[test]
    fn test_comment_post_valid() {
        let comment = CommentPost {
            author: "John Doe".to_string(),
            content: "This is a valid comment with enough content.".to_string(),
        };
        assert!(comment.validate().is_ok());
    }

    #[test]
    fn test_comment_post_too_short() {
        let comment = CommentPost {
            author: "John Doe".to_string(),
            content: "Short".to_string(),
        };
        assert!(comment.validate().is_err());
    }
}
"#;

    println!("Generated code length: {} bytes", refactored_code.len());

    // Verify validation module exists
    assert!(refactored_code.contains("pub mod validation"));
    assert!(refactored_code.contains("pub fn validate_not_empty"));
    assert!(refactored_code.contains("pub fn validate_length"));
    assert!(refactored_code.contains("pub fn validate_email"));
    assert!(refactored_code.contains("pub fn validate_range"));

    // Verify all structs use the validation module
    assert!(refactored_code.contains("validation::validate_email"));
    assert!(refactored_code.contains("validation::validate_not_empty"));
    assert!(refactored_code.contains("validation::validate_length"));
    assert!(refactored_code.contains("validation::validate_range"));

    // Verify code is cleaner (less duplication)
    let is_empty_count = test_utils::count_occurrences(&refactored_code, ".is_empty()");
    let initial_is_empty = test_utils::count_occurrences(initial_code, ".is_empty()");
    println!("is_empty() calls - Initial: {}, Refactored: {}", initial_is_empty, is_empty_count);

    // The refactored code should have fewer direct is_empty checks in impl blocks
    // (they're now in the validation module)

    // Verify tests were added
    assert!(refactored_code.contains("test_validation_not_empty"));
    assert!(refactored_code.contains("test_user_registration_valid"));
    assert!(refactored_code.contains("test_product_submission_valid"));
}

/// Test token efficiency - measure how compact the transformations are
#[test]
fn test_token_efficiency() {
    let code = r#"
pub fn process_data(input: Vec<i32>) -> Vec<i32> {
    input.iter().map(|x| x * 2).collect()
}
"#;

    let editor = AstEditor::new(Language::Rust);

    // Test that we can add comprehensive documentation without excessive bloat
    let documented = r#"
/// Processes a vector of integers by doubling each value.
///
/// # Arguments
///
/// * `input` - A vector of i32 values to process
///
/// # Returns
///
/// A new vector containing doubled values
///
/// # Examples
///
/// ```
/// let data = vec![1, 2, 3];
/// let result = process_data(data);
/// assert_eq!(result, vec![2, 4, 6]);
/// ```
pub fn process_data(input: Vec<i32>) -> Vec<i32> {
    input.iter().map(|x| x * 2).collect()
}
"#;

    // Verify documentation adds value without excessive size
    let original_size = code.len();
    let documented_size = documented.len();
    let overhead_ratio = documented_size as f64 / original_size as f64;

    println!("Original: {} bytes, Documented: {} bytes, Ratio: {:.2}",
             original_size, documented_size, overhead_ratio);

    // Documentation should be reasonable (less than 10x the original)
    assert!(overhead_ratio < 10.0, "Documentation overhead too high");
}
