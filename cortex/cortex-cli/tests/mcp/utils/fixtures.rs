//! Test fixtures for MCP tools
//!
//! Provides reusable test data:
//! - Sample projects in various languages (Rust, TypeScript, Python, Go)
//! - Code snippets and templates
//! - Mock data for different scenarios
//! - Language-specific utilities

use std::path::{Path, PathBuf};
use tokio::fs;

/// Supported language types for fixtures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageType {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
}

impl LanguageType {
    /// Get file extension for this language
    pub fn extension(&self) -> &str {
        match self {
            Self::Rust => "rs",
            Self::TypeScript => "ts",
            Self::JavaScript => "js",
            Self::Python => "py",
            Self::Go => "go",
        }
    }

    /// Get main source directory name
    pub fn src_dir(&self) -> &str {
        match self {
            Self::Rust => "src",
            Self::TypeScript | Self::JavaScript => "src",
            Self::Python => "src",
            Self::Go => ".",
        }
    }
}

/// A complete project fixture
pub struct ProjectFixture {
    pub language: LanguageType,
    pub name: String,
    pub files: Vec<FileTemplate>,
}

/// A file template with path and content
#[derive(Clone)]
pub struct FileTemplate {
    pub path: String,
    pub content: String,
}

impl ProjectFixture {
    /// Create a new project fixture
    pub fn new(language: LanguageType, name: impl Into<String>) -> Self {
        let name = name.into();
        let files = match language {
            LanguageType::Rust => Self::rust_files(&name),
            LanguageType::TypeScript => Self::typescript_files(&name),
            LanguageType::JavaScript => Self::javascript_files(&name),
            LanguageType::Python => Self::python_files(&name),
            LanguageType::Go => Self::go_files(&name),
        };

        Self {
            language,
            name,
            files,
        }
    }

    /// Write the fixture to a directory
    pub async fn write_to(&self, base_dir: &Path) -> std::io::Result<PathBuf> {
        let project_dir = base_dir.join(&self.name);
        fs::create_dir_all(&project_dir).await?;

        for file in &self.files {
            let file_path = project_dir.join(&file.path);

            // Create parent directories if needed
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).await?;
            }

            fs::write(&file_path, &file.content).await?;
        }

        Ok(project_dir)
    }

    /// Rust project files
    fn rust_files(name: &str) -> Vec<FileTemplate> {
        vec![
            FileTemplate {
                path: "Cargo.toml".to_string(),
                content: format!(
                    r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = {{ version = "1.0", features = ["derive"] }}
tokio = {{ version = "1.0", features = ["full"] }}
anyhow = "1.0"

[dev-dependencies]
tempfile = "3.0"
"#,
                    name
                ),
            },
            FileTemplate {
                path: "src/lib.rs".to_string(),
                content: RUST_LIB_CONTENT.to_string(),
            },
            FileTemplate {
                path: "src/main.rs".to_string(),
                content: RUST_MAIN_CONTENT.to_string(),
            },
            FileTemplate {
                path: "src/models.rs".to_string(),
                content: RUST_MODELS_CONTENT.to_string(),
            },
            FileTemplate {
                path: "src/utils.rs".to_string(),
                content: RUST_UTILS_CONTENT.to_string(),
            },
            FileTemplate {
                path: ".gitignore".to_string(),
                content: "target/\nCargo.lock\n.DS_Store\n".to_string(),
            },
        ]
    }

    /// TypeScript project files
    fn typescript_files(name: &str) -> Vec<FileTemplate> {
        vec![
            FileTemplate {
                path: "package.json".to_string(),
                content: format!(
                    r#"{{
  "name": "{}",
  "version": "1.0.0",
  "description": "Test TypeScript project",
  "main": "dist/index.js",
  "scripts": {{
    "build": "tsc",
    "test": "jest",
    "dev": "ts-node src/index.ts"
  }},
  "dependencies": {{
    "express": "^4.18.0",
    "dotenv": "^16.0.0"
  }},
  "devDependencies": {{
    "@types/node": "^20.0.0",
    "@types/express": "^4.17.0",
    "typescript": "^5.0.0",
    "jest": "^29.0.0",
    "ts-node": "^10.0.0"
  }}
}}
"#,
                    name
                ),
            },
            FileTemplate {
                path: "tsconfig.json".to_string(),
                content: TYPESCRIPT_CONFIG.to_string(),
            },
            FileTemplate {
                path: "src/index.ts".to_string(),
                content: TYPESCRIPT_INDEX_CONTENT.to_string(),
            },
            FileTemplate {
                path: "src/models.ts".to_string(),
                content: TYPESCRIPT_MODELS_CONTENT.to_string(),
            },
            FileTemplate {
                path: "src/services.ts".to_string(),
                content: TYPESCRIPT_SERVICES_CONTENT.to_string(),
            },
            FileTemplate {
                path: "src/utils.ts".to_string(),
                content: TYPESCRIPT_UTILS_CONTENT.to_string(),
            },
            FileTemplate {
                path: ".gitignore".to_string(),
                content: "node_modules/\ndist/\n.env\n.DS_Store\n".to_string(),
            },
        ]
    }

    /// JavaScript project files
    fn javascript_files(name: &str) -> Vec<FileTemplate> {
        vec![
            FileTemplate {
                path: "package.json".to_string(),
                content: format!(
                    r#"{{
  "name": "{}",
  "version": "1.0.0",
  "description": "Test JavaScript project",
  "main": "src/index.js",
  "scripts": {{
    "start": "node src/index.js",
    "test": "jest"
  }},
  "dependencies": {{
    "express": "^4.18.0"
  }},
  "devDependencies": {{
    "jest": "^29.0.0"
  }}
}}
"#,
                    name
                ),
            },
            FileTemplate {
                path: "src/index.js".to_string(),
                content: JAVASCRIPT_INDEX_CONTENT.to_string(),
            },
            FileTemplate {
                path: "src/utils.js".to_string(),
                content: JAVASCRIPT_UTILS_CONTENT.to_string(),
            },
        ]
    }

    /// Python project files
    fn python_files(name: &str) -> Vec<FileTemplate> {
        vec![
            FileTemplate {
                path: "setup.py".to_string(),
                content: format!(
                    r#"from setuptools import setup, find_packages

setup(
    name="{}",
    version="0.1.0",
    packages=find_packages(),
    install_requires=[
        "requests>=2.28.0",
        "pydantic>=2.0.0",
    ],
)
"#,
                    name
                ),
            },
            FileTemplate {
                path: "pyproject.toml".to_string(),
                content: PYTHON_PROJECT_TOML.to_string(),
            },
            FileTemplate {
                path: "src/__init__.py".to_string(),
                content: "".to_string(),
            },
            FileTemplate {
                path: "src/main.py".to_string(),
                content: PYTHON_MAIN_CONTENT.to_string(),
            },
            FileTemplate {
                path: "src/models.py".to_string(),
                content: PYTHON_MODELS_CONTENT.to_string(),
            },
            FileTemplate {
                path: "src/utils.py".to_string(),
                content: PYTHON_UTILS_CONTENT.to_string(),
            },
            FileTemplate {
                path: ".gitignore".to_string(),
                content: "__pycache__/\n*.pyc\n.pytest_cache/\n.env\n".to_string(),
            },
        ]
    }

    /// Go project files
    fn go_files(name: &str) -> Vec<FileTemplate> {
        vec![
            FileTemplate {
                path: "go.mod".to_string(),
                content: format!(
                    r#"module github.com/example/{}

go 1.21

require (
    github.com/gin-gonic/gin v1.9.0
)
"#,
                    name
                ),
            },
            FileTemplate {
                path: "main.go".to_string(),
                content: GO_MAIN_CONTENT.to_string(),
            },
            FileTemplate {
                path: "models.go".to_string(),
                content: GO_MODELS_CONTENT.to_string(),
            },
            FileTemplate {
                path: "utils.go".to_string(),
                content: GO_UTILS_CONTENT.to_string(),
            },
            FileTemplate {
                path: ".gitignore".to_string(),
                content: "*.exe\n*.test\n*.out\n".to_string(),
            },
        ]
    }
}

/// Code snippet fixture
pub struct CodeFixture {
    pub language: LanguageType,
    pub content: String,
}

impl CodeFixture {
    /// Create a function definition
    pub fn function(language: LanguageType, name: &str, params: &str, body: &str) -> Self {
        let content = match language {
            LanguageType::Rust => {
                format!("pub fn {}({}) {{\n    {}\n}}", name, params, body)
            }
            LanguageType::TypeScript | LanguageType::JavaScript => {
                format!("function {}({}) {{\n    {}\n}}", name, params, body)
            }
            LanguageType::Python => {
                format!("def {}({}):\n    {}", name, params, body)
            }
            LanguageType::Go => {
                format!("func {}({}) {{\n    {}\n}}", name, params, body)
            }
        };

        Self { language, content }
    }

    /// Create a class/struct definition
    pub fn class(language: LanguageType, name: &str, fields: &[(&str, &str)]) -> Self {
        let content = match language {
            LanguageType::Rust => {
                let field_defs: Vec<String> = fields
                    .iter()
                    .map(|(n, t)| format!("    pub {}: {}", n, t))
                    .collect();
                format!("pub struct {} {{\n{}\n}}", name, field_defs.join(",\n"))
            }
            LanguageType::TypeScript => {
                let field_defs: Vec<String> = fields
                    .iter()
                    .map(|(n, t)| format!("    {}: {}", n, t))
                    .collect();
                format!("class {} {{\n{}\n}}", name, field_defs.join(";\n"))
            }
            LanguageType::Python => {
                let field_defs: Vec<String> = fields
                    .iter()
                    .map(|(n, _)| format!("        self.{} = {}", n, n))
                    .collect();
                let params: Vec<String> = fields.iter().map(|(n, _)| n.to_string()).collect();
                format!(
                    "class {}:\n    def __init__(self, {}):\n{}",
                    name,
                    params.join(", "),
                    field_defs.join("\n")
                )
            }
            LanguageType::Go => {
                let field_defs: Vec<String> = fields
                    .iter()
                    .map(|(n, t)| format!("    {} {}", n, t))
                    .collect();
                format!("type {} struct {{\n{}\n}}", name, field_defs.join("\n"))
            }
            _ => String::new(),
        };

        Self { language, content }
    }
}

// Content templates for different languages

const RUST_LIB_CONTENT: &str = r#"//! Main library module

pub mod models;
pub mod utils;

pub use models::{User, Task};
pub use utils::{calculate_total, validate_email};

/// Library version
pub const VERSION: &str = "0.1.0";

/// Initialize the library
pub fn init() -> Result<(), String> {
    println!("Initializing library v{}", VERSION);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        assert!(init().is_ok());
    }
}
"#;

const RUST_MAIN_CONTENT: &str = r#"use anyhow::Result;

fn main() -> Result<()> {
    test_project::init()?;

    let user = test_project::User::new(
        1,
        "Alice".to_string(),
        "alice@example.com".to_string(),
    );

    println!("User: {:?}", user);
    println!("Email valid: {}", test_project::validate_email(&user.email));

    Ok(())
}
"#;

const RUST_MODELS_CONTENT: &str = r#"//! Data models

use serde::{Deserialize, Serialize};

/// A user in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
}

impl User {
    /// Create a new user
    pub fn new(id: u64, name: String, email: String) -> Self {
        Self { id, name, email }
    }

    /// Get user display name
    pub fn display_name(&self) -> String {
        format!("{} ({})", self.name, self.email)
    }
}

/// A task in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub completed: bool,
    pub user_id: u64,
}

impl Task {
    /// Create a new task
    pub fn new(id: u64, title: String, user_id: u64) -> Self {
        Self {
            id,
            title,
            completed: false,
            user_id,
        }
    }

    /// Mark task as completed
    pub fn complete(&mut self) {
        self.completed = true;
    }
}
"#;

const RUST_UTILS_CONTENT: &str = r#"//! Utility functions

/// Validate an email address
pub fn validate_email(email: &str) -> bool {
    email.contains('@') && email.contains('.')
}

/// Calculate total from a list of values
pub fn calculate_total(values: &[f64]) -> f64 {
    values.iter().sum()
}

/// Format a name
pub fn format_name(first: &str, last: &str) -> String {
    format!("{} {}", first, last)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_email() {
        assert!(validate_email("test@example.com"));
        assert!(!validate_email("invalid"));
    }

    #[test]
    fn test_calculate_total() {
        assert_eq!(calculate_total(&[1.0, 2.0, 3.0]), 6.0);
    }
}
"#;

const TYPESCRIPT_CONFIG: &str = r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "lib": ["ES2020"],
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "declaration": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}
"#;

const TYPESCRIPT_INDEX_CONTENT: &str = r#"import express from 'express';
import { UserService } from './services';
import { User } from './models';

const app = express();
const userService = new UserService();

app.use(express.json());

app.post('/users', (req, res) => {
  const { name, email } = req.body;
  const user = userService.createUser(name, email);
  res.json(user);
});

app.get('/users/:id', (req, res) => {
  const id = parseInt(req.params.id);
  const user = userService.getUser(id);
  if (user) {
    res.json(user);
  } else {
    res.status(404).json({ error: 'User not found' });
  }
});

app.get('/users', (req, res) => {
  res.json(userService.getAllUsers());
});

const PORT = process.env.PORT || 3000;
app.listen(PORT, () => {
  console.log(`Server running on port ${PORT}`);
});

export { app };
"#;

const TYPESCRIPT_MODELS_CONTENT: &str = r#"export interface User {
  id: number;
  name: string;
  email: string;
}

export interface Task {
  id: number;
  title: string;
  completed: boolean;
  userId: number;
}

export class UserModel implements User {
  constructor(
    public id: number,
    public name: string,
    public email: string
  ) {}

  displayName(): string {
    return `${this.name} (${this.email})`;
  }

  validateEmail(): boolean {
    return this.email.includes('@');
  }
}
"#;

const TYPESCRIPT_SERVICES_CONTENT: &str = r#"import { User, UserModel } from './models';

export class UserService {
  private users: Map<number, User> = new Map();
  private nextId = 1;

  createUser(name: string, email: string): User {
    const user = new UserModel(this.nextId++, name, email);
    this.users.set(user.id, user);
    return user;
  }

  getUser(id: number): User | undefined {
    return this.users.get(id);
  }

  getAllUsers(): User[] {
    return Array.from(this.users.values());
  }

  deleteUser(id: number): boolean {
    return this.users.delete(id);
  }
}
"#;

const TYPESCRIPT_UTILS_CONTENT: &str = r#"export function validateEmail(email: string): boolean {
  return email.includes('@') && email.includes('.');
}

export function calculateTotal(values: number[]): number {
  return values.reduce((sum, val) => sum + val, 0);
}

export function formatName(first: string, last: string): string {
  return `${first} ${last}`;
}
"#;

const JAVASCRIPT_INDEX_CONTENT: &str = r#"const express = require('express');

const app = express();
app.use(express.json());

const users = new Map();
let nextId = 1;

app.post('/users', (req, res) => {
  const { name, email } = req.body;
  const user = { id: nextId++, name, email };
  users.set(user.id, user);
  res.json(user);
});

app.get('/users/:id', (req, res) => {
  const id = parseInt(req.params.id);
  const user = users.get(id);
  if (user) {
    res.json(user);
  } else {
    res.status(404).json({ error: 'User not found' });
  }
});

const PORT = process.env.PORT || 3000;
app.listen(PORT, () => {
  console.log(`Server running on port ${PORT}`);
});

module.exports = { app };
"#;

const JAVASCRIPT_UTILS_CONTENT: &str = r#"function validateEmail(email) {
  return email.includes('@') && email.includes('.');
}

function calculateTotal(values) {
  return values.reduce((sum, val) => sum + val, 0);
}

module.exports = { validateEmail, calculateTotal };
"#;

const PYTHON_PROJECT_TOML: &str = r#"[build-system]
requires = ["setuptools>=42", "wheel"]
build-backend = "setuptools.build_meta"

[project]
name = "test-project"
version = "0.1.0"
dependencies = [
    "requests>=2.28.0",
    "pydantic>=2.0.0",
]
"#;

const PYTHON_MAIN_CONTENT: &str = r#"""Main module"""
from src.models import User, Task
from src.utils import validate_email, calculate_total

def main():
    user = User(id=1, name="Alice", email="alice@example.com")
    print(f"User: {user}")
    print(f"Email valid: {validate_email(user.email)}")

    task = Task(id=1, title="Complete project", user_id=user.id)
    print(f"Task: {task}")

if __name__ == "__main__":
    main()
"#;

const PYTHON_MODELS_CONTENT: &str = r#"""Data models"""
from pydantic import BaseModel, EmailStr
from typing import Optional

class User(BaseModel):
    id: int
    name: str
    email: EmailStr

    def display_name(self) -> str:
        return f"{self.name} ({self.email})"

class Task(BaseModel):
    id: int
    title: str
    completed: bool = False
    user_id: int

    def complete(self):
        self.completed = True
"#;

const PYTHON_UTILS_CONTENT: &str = r#"""Utility functions"""
from typing import List

def validate_email(email: str) -> bool:
    """Validate an email address"""
    return "@" in email and "." in email

def calculate_total(values: List[float]) -> float:
    """Calculate total from a list of values"""
    return sum(values)

def format_name(first: str, last: str) -> str:
    """Format a name"""
    return f"{first} {last}"
"#;

const GO_MAIN_CONTENT: &str = r#"package main

import (
    "fmt"
)

func main() {
    user := NewUser(1, "Alice", "alice@example.com")
    fmt.Printf("User: %+v\n", user)
    fmt.Printf("Email valid: %v\n", ValidateEmail(user.Email))
}
"#;

const GO_MODELS_CONTENT: &str = r#"package main

type User struct {
    ID    int64
    Name  string
    Email string
}

func NewUser(id int64, name, email string) *User {
    return &User{
        ID:    id,
        Name:  name,
        Email: email,
    }
}

func (u *User) DisplayName() string {
    return fmt.Sprintf("%s (%s)", u.Name, u.Email)
}

type Task struct {
    ID        int64
    Title     string
    Completed bool
    UserID    int64
}

func NewTask(id int64, title string, userID int64) *Task {
    return &Task{
        ID:        id,
        Title:     title,
        Completed: false,
        UserID:    userID,
    }
}

func (t *Task) Complete() {
    t.Completed = true
}
"#;

const GO_UTILS_CONTENT: &str = r#"package main

import "strings"

func ValidateEmail(email string) bool {
    return strings.Contains(email, "@") && strings.Contains(email, ".")
}

func CalculateTotal(values []float64) float64 {
    total := 0.0
    for _, v := range values {
        total += v
    }
    return total
}

func FormatName(first, last string) string {
    return first + " " + last
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rust_project() {
        let fixture = ProjectFixture::new(LanguageType::Rust, "test-rust");
        assert_eq!(fixture.language, LanguageType::Rust);
        assert!(!fixture.files.is_empty());
    }

    #[test]
    fn test_code_fixture_function() {
        let fixture = CodeFixture::function(
            LanguageType::Rust,
            "test_fn",
            "x: i32",
            "x + 1",
        );
        assert!(fixture.content.contains("pub fn test_fn"));
    }
}
