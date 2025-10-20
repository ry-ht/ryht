# Cortex: REST API Specification

## 🔴 Implementation Status: NOT IMPLEMENTED (0%)

**Last Updated**: 2025-10-20
**Priority**: High (Priority 1 for completion)
**Estimated Effort**: 2-3 days

### Current State
- ✅ Specification: 100% Complete
- 🔴 Implementation: 0% (Not started)
- ❌ Tests: None
- ❌ Documentation: Specification only

### Blockers
None - Core systems are complete and ready for REST API implementation.

### Dependencies
- ✅ cortex-core (100% complete)
- ✅ cortex-storage (100% complete)
- ✅ cortex-vfs (100% complete)
- ✅ cortex-memory (100% complete)
- ✅ cortex-mcp (100% complete)

All foundation components are implemented and ready to expose via REST API.

---

## Overview

The Cortex REST API provides comprehensive access to all system capabilities through HTTP endpoints. This enables external systems, dashboards, CI/CD pipelines, and third-party tools to interact with the Cortex.

## API Design Principles

1. **RESTful Design**: Resources-based URLs with standard HTTP methods
2. **Consistent Response Format**: Uniform JSON structure across all endpoints
3. **Versioning**: API version in URL path for backward compatibility
4. **Pagination**: Cursor-based pagination for large datasets
5. **Filtering**: Flexible query parameters for filtering results
6. **Real-time**: WebSocket support for live updates
7. **Security**: JWT authentication with role-based access control
8. **Rate Limiting**: Configurable limits per endpoint
9. **Idempotency**: Safe retries with idempotency keys
10. **HATEOAS**: Hypermedia links for API discoverability

## Base Configuration

### Base URL
```
https://api.cortex.dev
```

### Request Headers
```http
Content-Type: application/json
Authorization: Bearer <jwt_token>
X-Request-ID: <unique_request_id>
X-Idempotency-Key: <idempotency_key>
X-Agent-ID: <agent_identifier>
```

### Response Format
```json
{
  "success": true,
  "data": { /* response data */ },
  "error": null,
  "metadata": {
    "request_id": "req_abc123",
    "timestamp": "2024-01-01T00:00:00Z",
    "version": "v3",
    "duration_ms": 45
  },
  "pagination": {
    "cursor": "eyJpZCI6MTAwfQ==",
    "has_more": true,
    "total": 1234
  },
  "links": {
    "self": "/resource/123",
    "next": "/resource?cursor=xyz",
    "related": { /* HATEOAS links */ }
  }
}
```

### Error Response
```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "RESOURCE_NOT_FOUND",
    "message": "The requested resource was not found",
    "details": {
      "resource_type": "workspace",
      "resource_id": "ws_123"
    },
    "documentation_url": "https://docs.cortex.dev/errors/RESOURCE_NOT_FOUND"
  },
  "metadata": { /* standard metadata */ }
}
```

## Authentication & Authorization

### Authentication Endpoints

#### POST /auth/login
```json
Request:
{
  "email": "user@example.com",
  "password": "secure_password",
  "mfa_token": "123456"  // Optional
}

Response:
{
  "success": true,
  "data": {
    "access_token": "eyJ...",
    "refresh_token": "eyJ...",
    "token_type": "Bearer",
    "expires_in": 3600,
    "user": {
      "id": "user_123",
      "email": "user@example.com",
      "roles": ["developer", "admin"]
    }
  }
}
```

#### POST /auth/refresh
```json
Request:
{
  "refresh_token": "eyJ..."
}

Response:
{
  "success": true,
  "data": {
    "access_token": "eyJ...",
    "expires_in": 3600
  }
}
```

#### POST /auth/api-key
Create API key for programmatic access
```json
Request:
{
  "name": "CI/CD Pipeline",
  "scopes": ["workspace:read", "code:write"],
  "expires_at": "2025-01-01T00:00:00Z"
}

Response:
{
  "success": true,
  "data": {
    "api_key": "mk_live_abc123...",
    "api_key_id": "key_123",
    "name": "CI/CD Pipeline",
    "created_at": "2024-01-01T00:00:00Z",
    "expires_at": "2025-01-01T00:00:00Z"
  }
}
```

## Core Resource Endpoints

### Workspaces

#### GET /workspaces
List all workspaces
```
Query Parameters:
- status: active|archived|all
- type: rust_cargo|typescript_turborepo|typescript_nx|mixed
- cursor: pagination cursor
- limit: 10-100 (default: 20)
- sort: name|created_at|updated_at
- order: asc|desc

Response:
{
  "data": {
    "workspaces": [
      {
        "id": "ws_123",
        "name": "my-project",
        "type": "rust_cargo",
        "status": "active",
        "stats": {
          "files": 1234,
          "code_units": 5678,
          "size_bytes": 10485760
        },
        "created_at": "2024-01-01T00:00:00Z",
        "links": {
          "self": "/workspaces/ws_123",
          "files": "/workspaces/ws_123/files",
          "units": "/workspaces/ws_123/units"
        }
      }
    ]
  }
}
```

#### GET /workspaces/{id}
Get workspace details
```json
Response:
{
  "data": {
    "id": "ws_123",
    "name": "my-project",
    "type": "rust_cargo",
    "root_path": "/projects/my-project",
    "config": {
      "build_system": "cargo",
      "language_version": "1.75"
    },
    "metadata": {
      "description": "My awesome project",
      "tags": ["backend", "api"],
      "owner": "team_123"
    },
    "stats": {
      "files": 1234,
      "directories": 456,
      "code_units": 5678,
      "languages": {
        "rust": 0.85,
        "toml": 0.10,
        "markdown": 0.05
      },
      "complexity": {
        "average": 3.2,
        "max": 15
      }
    },
    "git": {
      "remote": "https://github.com/user/project",
      "branch": "main",
      "commit": "abc123"
    }
  }
}
```

#### POST /workspaces
Create new workspace
```json
Request:
{
  "name": "new-project",
  "type": "typescript_turborepo",
  "root_path": "/projects/new-project",
  "import_options": {
    "scan_depth": -1,
    "include_hidden": true,
    "auto_analyze": true
  }
}

Response:
{
  "data": {
    "id": "ws_456",
    "name": "new-project",
    "status": "importing",
    "import_job_id": "job_789"
  }
}
```

#### PUT /workspaces/{id}
Update workspace
```json
Request:
{
  "name": "renamed-project",
  "metadata": {
    "description": "Updated description"
  }
}
```

#### DELETE /workspaces/{id}
Delete workspace
```
Query Parameters:
- archive: true|false (default: true) - Archive instead of permanent delete
```

#### POST /workspaces/{id}/sync
Synchronize workspace with filesystem
```json
Request:
{
  "mode": "full|incremental",
  "auto_resolve_conflicts": false
}

Response:
{
  "data": {
    "sync_id": "sync_123",
    "status": "running",
    "changes_detected": 42,
    "estimated_time_seconds": 10
  }
}
```

### Virtual Filesystem

#### GET /workspaces/{id}/files
Browse virtual filesystem
```
Query Parameters:
- path: /src (default: /)
- recursive: true|false
- type: file|directory|all
- language: rust|typescript|javascript
- include_content: true|false
- include_metadata: true|false

Response:
{
  "data": {
    "files": [
      {
        "id": "vn_123",
        "path": "/src/main.rs",
        "type": "file",
        "name": "main.rs",
        "size_bytes": 1024,
        "language": "rust",
        "permissions": "644",
        "content": "fn main() {...}",  // if requested
        "metadata": { /* if requested */ },
        "modified_at": "2024-01-01T00:00:00Z",
        "version": 42
      }
    ]
  }
}
```

#### GET /files/{id}
Get file details
```json
Response:
{
  "data": {
    "id": "vn_123",
    "path": "/src/main.rs",
    "content": "fn main() {\n    println!(\"Hello\");\n}",
    "language": "rust",
    "encoding": "utf-8",
    "size_bytes": 35,
    "line_count": 3,
    "hash": "sha256:abc123...",
    "ast": { /* tree-sitter AST if requested */ },
    "units": [ /* code units in file */ ],
    "version": 42,
    "history": [ /* version history */ ]
  }
}
```

#### POST /workspaces/{id}/files
Create new file
```json
Request:
{
  "path": "/src/new_module.rs",
  "content": "pub fn new_function() {}",
  "encoding": "utf-8"
}
```

#### PUT /files/{id}
Update file content
```json
Request:
{
  "content": "updated content",
  "expected_version": 42  // For optimistic locking
}
```

#### DELETE /files/{id}
Delete file
```
Query Parameters:
- expected_version: 42 (for safe deletion)
```

### Code Units (Semantic Entities)

#### GET /workspaces/{id}/units
List code units
```
Query Parameters:
- type: function|class|struct|interface|trait|enum
- visibility: public|private|protected
- language: rust|typescript
- file: /src/main.rs
- complexity_min: 1
- complexity_max: 10
- has_tests: true|false
- has_docs: true|false

Response:
{
  "data": {
    "units": [
      {
        "id": "cu_123",
        "type": "function",
        "name": "calculate_tax",
        "qualified_name": "finance::tax::calculate_tax",
        "signature": "fn calculate_tax(income: f64) -> f64",
        "file": "/src/finance/tax.rs",
        "lines": {
          "start": 10,
          "end": 20
        },
        "visibility": "public",
        "complexity": {
          "cyclomatic": 3,
          "cognitive": 5
        },
        "has_tests": true,
        "test_coverage": 0.85,
        "has_documentation": true
      }
    ]
  }
}
```

#### GET /units/{id}
Get unit details
```json
Response:
{
  "data": {
    "id": "cu_123",
    "type": "function",
    "name": "calculate_tax",
    "qualified_name": "finance::tax::calculate_tax",
    "signature": "fn calculate_tax(income: f64) -> f64",
    "body": "{\n    income * 0.2\n}",
    "docstring": "/// Calculates tax at 20% rate",
    "parameters": [
      {
        "name": "income",
        "type": "f64",
        "optional": false
      }
    ],
    "return_type": "f64",
    "modifiers": [],
    "complexity": {
      "cyclomatic": 1,
      "cognitive": 1,
      "nesting": 0
    },
    "dependencies": [
      {
        "type": "calls",
        "target": "cu_456",
        "name": "validate_income"
      }
    ],
    "dependents": [
      {
        "type": "called_by",
        "source": "cu_789",
        "name": "process_payment"
      }
    ],
    "tests": [
      {
        "id": "cu_999",
        "name": "test_calculate_tax",
        "coverage": 0.85
      }
    ]
  }
}
```

#### PUT /units/{id}
Update code unit
```json
Request:
{
  "body": "{\n    income * 0.25  // Updated rate\n}",
  "docstring": "/// Calculates tax at 25% rate",
  "expected_version": 5
}
```

### Semantic Search

#### POST /search/semantic
Semantic code search
```json
Request:
{
  "query": "function that handles user authentication",
  "workspace_id": "ws_123",  // Optional, search all if not specified
  "filters": {
    "types": ["function", "method"],
    "languages": ["rust", "typescript"],
    "visibility": ["public"],
    "min_relevance": 0.7
  },
  "limit": 20
}

Response:
{
  "data": {
    "results": [
      {
        "unit_id": "cu_123",
        "type": "function",
        "name": "authenticate_user",
        "qualified_name": "auth::authenticate_user",
        "signature": "async fn authenticate_user(credentials: Credentials) -> Result<User>",
        "relevance_score": 0.92,
        "file": "/src/auth/mod.rs",
        "snippet": "async fn authenticate_user(credentials: Credentials) -> Result<User> {\n    // Verify credentials\n    ...\n}",
        "highlights": [
          {
            "start": 10,
            "end": 26,
            "text": "authenticate_user"
          }
        ]
      }
    ]
  }
}
```

#### POST /search/pattern
AST pattern search
```json
Request:
{
  "pattern": "(function_item name: (identifier) @name parameters: (parameters) @params)",
  "language": "rust",
  "workspace_id": "ws_123"
}
```

#### GET /search/references/{unit_id}
Find all references to a code unit
```json
Response:
{
  "data": {
    "target": {
      "id": "cu_123",
      "name": "calculate_tax",
      "qualified_name": "finance::tax::calculate_tax"
    },
    "references": [
      {
        "id": "ref_456",
        "type": "function_call",
        "location": {
          "file": "/src/payment.rs",
          "line": 42,
          "column": 15
        },
        "context": "let tax = calculate_tax(amount);"
      }
    ],
    "total": 5
  }
}
```

### Dependencies & Graph

#### GET /workspaces/{id}/dependencies
Get dependency graph
```
Query Parameters:
- format: json|dot|mermaid
- level: file|unit|package
- max_depth: 10
- include_external: true|false

Response:
{
  "data": {
    "nodes": [
      {
        "id": "node_123",
        "type": "file",
        "name": "/src/main.rs",
        "metrics": {
          "in_degree": 0,
          "out_degree": 5
        }
      }
    ],
    "edges": [
      {
        "from": "node_123",
        "to": "node_456",
        "type": "imports",
        "weight": 1
      }
    ],
    "statistics": {
      "total_nodes": 100,
      "total_edges": 450,
      "average_degree": 4.5,
      "max_degree": 23,
      "cycles_detected": 2
    }
  }
}
```

#### POST /analysis/impact
Analyze impact of changes
```json
Request:
{
  "changed_entities": ["cu_123", "cu_456"],
  "analysis_type": "full|direct|transitive",
  "max_depth": 10
}

Response:
{
  "data": {
    "directly_affected": [
      {
        "id": "cu_789",
        "name": "process_payment",
        "impact_type": "compilation",
        "severity": "high"
      }
    ],
    "transitively_affected": [
      {
        "id": "cu_999",
        "name": "generate_invoice",
        "impact_type": "behavior",
        "severity": "medium"
      }
    ],
    "risk_assessment": {
      "level": "medium",
      "score": 6.5,
      "breaking_changes": 1,
      "test_coverage": 0.75
    }
  }
}
```

#### GET /analysis/cycles
Detect circular dependencies
```json
Response:
{
  "data": {
    "cycles": [
      {
        "id": "cycle_1",
        "path": ["module_a", "module_b", "module_c", "module_a"],
        "severity": "high",
        "suggestion": "Extract common functionality to separate module"
      }
    ]
  }
}
```

### Sessions & Multi-Agent

#### POST /sessions
Create agent session
```json
Request:
{
  "agent_id": "agent_123",
  "workspace_id": "ws_123",
  "scope": {
    "paths": ["/src/feature"],
    "read_only_paths": ["/tests"]
  },
  "isolation_level": "snapshot|read_committed|serializable",
  "ttl_seconds": 3600
}

Response:
{
  "data": {
    "session_id": "sess_123",
    "token": "st_abc123...",
    "expires_at": "2024-01-01T01:00:00Z",
    "base_version": 42
  }
}
```

#### GET /sessions/{id}
Get session status
```json
Response:
{
  "data": {
    "session_id": "sess_123",
    "agent_id": "agent_123",
    "status": "active",
    "changes": {
      "created": 5,
      "modified": 10,
      "deleted": 2
    },
    "conflicts": [],
    "created_at": "2024-01-01T00:00:00Z",
    "expires_at": "2024-01-01T01:00:00Z"
  }
}
```

#### GET /sessions/{id}/files
List files in session scope

Returns all files visible within the session scope, including both workspace files and session-specific modifications.

**Query Parameters:**
- `path`: Filter by path prefix (e.g., `/src`)
- `recursive`: `true|false` - Include subdirectories (default: `false`)
- `modified_only`: `true|false` - Only show files modified in this session (default: `false`)
- `type`: `file|directory|all` - Filter by type (default: `all`)
- `include_content`: `true|false` - Include file content in response (default: `false`)

**Example Request:**
```http
GET /sessions/sess_123/files?path=/src&recursive=true&modified_only=true
Authorization: Bearer eyJ...
```

**Example Response:**
```json
{
  "success": true,
  "data": {
    "files": [
      {
        "id": "vn_456",
        "path": "/src/main.rs",
        "type": "file",
        "name": "main.rs",
        "size_bytes": 2048,
        "language": "rust",
        "modified_in_session": true,
        "change_type": "modified",
        "session_version": 3,
        "base_version": 42,
        "modified_at": "2024-01-01T00:15:00Z"
      },
      {
        "id": "vn_789",
        "path": "/src/lib.rs",
        "type": "file",
        "name": "lib.rs",
        "size_bytes": 1024,
        "language": "rust",
        "modified_in_session": true,
        "change_type": "created",
        "session_version": 1,
        "base_version": null,
        "modified_at": "2024-01-01T00:10:00Z"
      }
    ],
    "total": 2,
    "session_id": "sess_123"
  },
  "error": null,
  "metadata": {
    "request_id": "req_abc123",
    "timestamp": "2024-01-01T00:20:00Z",
    "version": "v3",
    "duration_ms": 12
  }
}
```

**Error Codes:**
- `404 NOT_FOUND`: Session does not exist
- `401 UNAUTHORIZED`: Invalid session token
- `403 FORBIDDEN`: Session expired or insufficient permissions
- `400 VALIDATION_ERROR`: Invalid query parameters

#### GET /sessions/{id}/files/{path}
Read file content from session scope

Retrieves file content as it appears within the session scope, including any uncommitted modifications.

**Path Parameters:**
- `id`: Session ID (e.g., `sess_123`)
- `path`: URL-encoded file path (e.g., `/src/main.rs` → `%2Fsrc%2Fmain.rs`)

**Query Parameters:**
- `include_metadata`: `true|false` - Include file metadata (default: `false`)
- `include_ast`: `true|false` - Include parsed AST if available (default: `false`)
- `version`: Specific session version to read (default: latest)

**Example Request:**
```http
GET /sessions/sess_123/files/%2Fsrc%2Fmain.rs?include_metadata=true
Authorization: Bearer eyJ...
```

**Example Response:**
```json
{
  "success": true,
  "data": {
    "id": "vn_456",
    "path": "/src/main.rs",
    "content": "fn main() {\n    println!(\"Hello, world!\");\n    // New feature added\n    process_data();\n}",
    "language": "rust",
    "encoding": "utf-8",
    "size_bytes": 2048,
    "line_count": 5,
    "hash": "sha256:def456...",
    "modified_in_session": true,
    "change_type": "modified",
    "session_version": 3,
    "base_version": 42,
    "metadata": {
      "created_at": "2023-12-15T10:00:00Z",
      "modified_at": "2024-01-01T00:15:00Z",
      "permissions": "644"
    }
  },
  "error": null,
  "metadata": {
    "request_id": "req_def456",
    "timestamp": "2024-01-01T00:20:00Z",
    "version": "v3",
    "duration_ms": 8
  }
}
```

**Error Codes:**
- `404 NOT_FOUND`: Session or file does not exist
- `401 UNAUTHORIZED`: Invalid session token
- `403 FORBIDDEN`: Session expired, insufficient permissions, or path outside session scope
- `400 VALIDATION_ERROR`: Invalid path format

#### PUT /sessions/{id}/files/{path}
Write or update file in session scope

Creates or modifies a file within the session scope. Changes are isolated to the session until merged.

**Path Parameters:**
- `id`: Session ID (e.g., `sess_123`)
- `path`: URL-encoded file path (e.g., `/src/new_module.rs`)

**Request Body:**
```json
{
  "content": "pub fn new_function() -> Result<()> {\n    Ok(())\n}",
  "encoding": "utf-8",
  "expected_version": 42,
  "create_if_missing": true,
  "metadata": {
    "description": "New utility module",
    "tags": ["utility", "helpers"]
  }
}
```

**Request Parameters:**
- `content` (required): File content as string
- `encoding` (optional): Character encoding (default: `utf-8`)
- `expected_version` (optional): Base version for optimistic locking
- `create_if_missing` (optional): Create new file if it doesn't exist (default: `true`)
- `metadata` (optional): Additional metadata for the file

**Example Request:**
```http
PUT /sessions/sess_123/files/%2Fsrc%2Fnew_module.rs
Authorization: Bearer eyJ...
Content-Type: application/json

{
  "content": "pub fn new_function() -> Result<()> {\n    Ok(())\n}",
  "create_if_missing": true
}
```

**Example Response (Success - Created):**
```json
{
  "success": true,
  "data": {
    "id": "vn_999",
    "path": "/src/new_module.rs",
    "change_type": "created",
    "session_version": 4,
    "base_version": null,
    "size_bytes": 54,
    "hash": "sha256:xyz789...",
    "modified_at": "2024-01-01T00:25:00Z",
    "session_id": "sess_123"
  },
  "error": null,
  "metadata": {
    "request_id": "req_xyz789",
    "timestamp": "2024-01-01T00:25:00Z",
    "version": "v3",
    "duration_ms": 15
  }
}
```

**Example Response (Success - Modified):**
```json
{
  "success": true,
  "data": {
    "id": "vn_456",
    "path": "/src/main.rs",
    "change_type": "modified",
    "session_version": 5,
    "base_version": 42,
    "previous_version": 3,
    "size_bytes": 2150,
    "hash": "sha256:abc999...",
    "modified_at": "2024-01-01T00:26:00Z",
    "session_id": "sess_123",
    "diff": {
      "lines_added": 3,
      "lines_removed": 1,
      "lines_changed": 2
    }
  },
  "error": null,
  "metadata": {
    "request_id": "req_abc999",
    "timestamp": "2024-01-01T00:26:00Z",
    "version": "v3",
    "duration_ms": 18
  }
}
```

**Example Response (Error - Version Conflict):**
```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "VERSION_CONFLICT",
    "message": "File has been modified since expected version",
    "details": {
      "expected_version": 42,
      "current_version": 45,
      "path": "/src/main.rs",
      "session_id": "sess_123"
    },
    "documentation_url": "https://docs.cortex.dev/errors/VERSION_CONFLICT"
  },
  "metadata": {
    "request_id": "req_err123",
    "timestamp": "2024-01-01T00:27:00Z",
    "version": "v3",
    "duration_ms": 5
  }
}
```

**Error Codes:**
- `404 NOT_FOUND`: Session does not exist
- `401 UNAUTHORIZED`: Invalid session token
- `403 FORBIDDEN`: Session expired, read-only path, or path outside session scope
- `409 VERSION_CONFLICT`: File version mismatch (optimistic locking failure)
- `400 VALIDATION_ERROR`: Invalid content, encoding, or path format
- `413 PAYLOAD_TOO_LARGE`: File content exceeds maximum size limit
- `507 INSUFFICIENT_STORAGE`: Session storage quota exceeded

#### POST /sessions/{id}/merge
Merge session changes
```json
Request:
{
  "strategy": "auto|manual|theirs|mine",
  "conflict_resolution": {
    "file_1": "mine",
    "file_2": "theirs"
  }
}

Response:
{
  "data": {
    "merge_id": "merge_123",
    "status": "success",
    "changes_merged": 17,
    "conflicts_resolved": 2,
    "new_version": 43
  }
}
```

#### GET /locks
List active locks
```json
Response:
{
  "data": {
    "locks": [
      {
        "id": "lock_123",
        "entity_type": "file",
        "entity_id": "vn_456",
        "lock_type": "exclusive",
        "owner": "agent_123",
        "acquired_at": "2024-01-01T00:00:00Z",
        "expires_at": "2024-01-01T00:05:00Z"
      }
    ]
  }
}
```

### Memory & Episodes

#### GET /memory/episodes
List development episodes
```
Query Parameters:
- agent_id: agent_123
- outcome: success|partial|failure
- from: 2024-01-01
- to: 2024-12-31
- task_type: feature|bugfix|refactor

Response:
{
  "data": {
    "episodes": [
      {
        "id": "ep_123",
        "task_description": "Implement authentication",
        "agent_id": "agent_123",
        "outcome": "success",
        "duration_seconds": 1800,
        "solution_summary": "Implemented JWT-based auth with refresh tokens",
        "entities_modified": ["cu_123", "cu_456"],
        "files_touched": ["/src/auth/mod.rs"],
        "patterns_learned": ["jwt_implementation"],
        "created_at": "2024-01-01T00:00:00Z"
      }
    ]
  }
}
```

#### POST /memory/search
Search similar episodes
```json
Request:
{
  "query": "implement REST API endpoints",
  "limit": 10,
  "min_similarity": 0.7
}

Response:
{
  "data": {
    "episodes": [
      {
        "id": "ep_456",
        "task_description": "Create CRUD API for users",
        "similarity_score": 0.85,
        "solution_summary": "Used actix-web with diesel ORM",
        "success_metrics": {
          "tests_passed": 42,
          "coverage": 0.90
        }
      }
    ]
  }
}
```

#### GET /memory/patterns
Get learned patterns
```json
Response:
{
  "data": {
    "patterns": [
      {
        "id": "pat_123",
        "name": "error_handling_pattern",
        "description": "Consistent error handling with Result type",
        "frequency": 42,
        "success_rate": 0.95,
        "example_episodes": ["ep_123", "ep_456"]
      }
    ]
  }
}
```

### Tasks & Workflow

#### GET /tasks
List tasks
```
Query Parameters:
- status: pending|in_progress|blocked|done|cancelled
- priority: critical|high|medium|low
- assigned_to: agent_123
- tags: backend,api

Response:
{
  "data": {
    "tasks": [
      {
        "id": "task_123",
        "title": "Implement user authentication",
        "description": "Add JWT-based authentication",
        "status": "in_progress",
        "priority": "high",
        "assigned_to": ["agent_123"],
        "estimated_hours": 8,
        "actual_hours": 6.5,
        "progress": 0.75,
        "tags": ["auth", "security"],
        "dependencies": ["task_456"],
        "created_at": "2024-01-01T00:00:00Z"
      }
    ]
  }
}
```

#### POST /tasks
Create task
```json
Request:
{
  "title": "Add logging system",
  "description": "Implement structured logging",
  "priority": "medium",
  "estimated_hours": 4,
  "tags": ["infrastructure"],
  "spec_reference": {
    "spec_name": "logging-spec",
    "section": "implementation"
  }
}
```

#### PUT /tasks/{id}
Update task
```json
Request:
{
  "status": "done",
  "actual_hours": 3.5,
  "completion_note": "Implemented with slog library"
}
```

### Dashboard Endpoints

#### GET /dashboard/overview
Dashboard overview data
```json
Response:
{
  "data": {
    "workspaces": {
      "total": 5,
      "active": 4,
      "archived": 1
    },
    "code_metrics": {
      "total_files": 12345,
      "total_units": 45678,
      "total_lines": 234567,
      "languages": {
        "rust": 0.60,
        "typescript": 0.30,
        "other": 0.10
      }
    },
    "quality_metrics": {
      "average_complexity": 3.2,
      "test_coverage": 0.78,
      "documentation_coverage": 0.65,
      "code_duplication": 0.05
    },
    "activity": {
      "active_sessions": 3,
      "tasks_in_progress": 12,
      "episodes_today": 45,
      "changes_today": 234
    },
    "trends": {
      "complexity_trend": "decreasing",
      "coverage_trend": "increasing",
      "productivity_trend": "stable"
    }
  }
}
```

#### GET /dashboard/activity
Real-time activity feed
```json
Response:
{
  "data": {
    "activities": [
      {
        "id": "act_123",
        "type": "code_change",
        "agent_id": "agent_123",
        "description": "Modified calculate_tax function",
        "details": {
          "file": "/src/finance/tax.rs",
          "unit": "cu_123",
          "change_type": "update"
        },
        "timestamp": "2024-01-01T00:00:00Z"
      }
    ]
  }
}
```

#### GET /dashboard/metrics
Detailed metrics
```
Query Parameters:
- workspace_id: ws_123
- from: 2024-01-01
- to: 2024-12-31
- granularity: hour|day|week|month

Response:
{
  "data": {
    "time_series": [
      {
        "timestamp": "2024-01-01T00:00:00Z",
        "metrics": {
          "code_changes": 45,
          "tests_run": 234,
          "coverage": 0.78,
          "complexity": 3.2,
          "episodes": 12,
          "tasks_completed": 5
        }
      }
    ],
    "aggregates": {
      "total_changes": 12345,
      "average_coverage": 0.78,
      "average_complexity": 3.2
    }
  }
}
```

#### GET /dashboard/health
System health status
```json
Response:
{
  "data": {
    "status": "healthy",
    "components": {
      "database": {
        "status": "healthy",
        "latency_ms": 2,
        "connections": 45
      },
      "storage": {
        "status": "healthy",
        "used_gb": 123,
        "available_gb": 877
      },
      "memory": {
        "status": "healthy",
        "used_mb": 2048,
        "available_mb": 6144
      },
      "indexer": {
        "status": "healthy",
        "queue_size": 12,
        "processing_rate": 100
      }
    },
    "uptime_seconds": 864000,
    "version": "3.0.0",
    "last_backup": "2024-01-01T00:00:00Z"
  }
}
```

### Build & CI/CD

#### POST /build/trigger
Trigger build
```json
Request:
{
  "workspace_id": "ws_123",
  "build_type": "debug|release|test",
  "target": "x86_64-unknown-linux-gnu",
  "flush_first": true
}

Response:
{
  "data": {
    "build_id": "build_123",
    "status": "queued",
    "estimated_duration_seconds": 60
  }
}
```

#### GET /build/{id}/status
Get build status
```json
Response:
{
  "data": {
    "build_id": "build_123",
    "status": "running",
    "progress": 0.45,
    "current_step": "Compiling dependencies",
    "logs_url": "/build/build_123/logs"
  }
}
```

#### POST /test/run
Run tests
```json
Request:
{
  "workspace_id": "ws_123",
  "test_pattern": "test_*",
  "coverage": true
}

Response:
{
  "data": {
    "test_run_id": "test_123",
    "status": "running",
    "tests_total": 234
  }
}
```

### Export & Import

#### POST /export
Export workspace
```json
Request:
{
  "workspace_id": "ws_123",
  "format": "tar.gz|zip|git",
  "include_history": true,
  "include_metadata": true
}

Response:
{
  "data": {
    "export_id": "exp_123",
    "status": "processing",
    "estimated_size_mb": 456
  }
}
```

#### GET /export/{id}/download
Download export
```
Response: Binary stream
Content-Type: application/octet-stream
Content-Disposition: attachment; filename="workspace_export.tar.gz"
```

#### POST /import
Import workspace
```json
Request:
{
  "source_type": "file|git|url",
  "source": "https://github.com/user/repo.git",
  "name": "imported-project",
  "type": "rust_cargo"
}

Response:
{
  "data": {
    "import_id": "imp_123",
    "status": "processing",
    "workspace_id": "ws_789"
  }
}
```

## WebSocket API

### Connection
```javascript
const ws = new WebSocket('wss://api.cortex.dev/ws');

// Authenticate after connection
ws.send(JSON.stringify({
  type: 'auth',
  token: 'Bearer eyJ...'
}));

// Subscribe to events
ws.send(JSON.stringify({
  type: 'subscribe',
  channels: ['workspace:ws_123', 'sessions', 'builds']
}));
```

### Event Types

#### Code Changes
```json
{
  "type": "code_change",
  "channel": "workspace:ws_123",
  "data": {
    "change_type": "update",
    "file_id": "vn_456",
    "path": "/src/main.rs",
    "agent_id": "agent_123",
    "timestamp": "2024-01-01T00:00:00Z"
  }
}
```

#### Session Updates
```json
{
  "type": "session_update",
  "channel": "sessions",
  "data": {
    "session_id": "sess_123",
    "status": "merging",
    "changes_pending": 15
  }
}
```

#### Build Progress
```json
{
  "type": "build_progress",
  "channel": "builds",
  "data": {
    "build_id": "build_123",
    "progress": 0.75,
    "current_step": "Running tests"
  }
}
```

#### System Alerts
```json
{
  "type": "alert",
  "channel": "system",
  "data": {
    "severity": "warning",
    "message": "High memory usage detected",
    "component": "indexer"
  }
}
```

## Rate Limiting

### Headers
```http
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1640995200
```

### Limits by Endpoint

| Endpoint Category | Rate Limit | Window |
|-------------------|------------|--------|
| Authentication | 10 req | 1 minute |
| Read Operations | 1000 req | 1 minute |
| Write Operations | 100 req | 1 minute |
| Search | 100 req | 1 minute |
| Analysis | 50 req | 1 minute |
| Build/Test | 10 req | 1 minute |
| Export/Import | 5 req | 1 hour |

### Rate Limit Response
```json
{
  "success": false,
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "Rate limit exceeded",
    "details": {
      "limit": 1000,
      "remaining": 0,
      "reset_at": "2024-01-01T00:01:00Z"
    }
  }
}
```

## OpenAPI Specification

### OpenAPI Document
```yaml
openapi: 3.0.0
info:
  title: Cortex API
  version: 3.0.0
  description: Cognitive memory system for multi-agent development
  contact:
    email: api@cortex.dev
  license:
    name: MIT
    url: https://opensource.org/licenses/MIT

servers:
  - url: https://api.cortex.dev
    description: Production
  - url: https://staging.api.cortex.dev
    description: Staging
  - url: http://localhost:8080
    description: Local

security:
  - bearerAuth: []
  - apiKeyAuth: []

components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
    apiKeyAuth:
      type: apiKey
      in: header
      name: X-API-Key

  schemas:
    Workspace:
      type: object
      properties:
        id:
          type: string
          pattern: '^ws_[a-zA-Z0-9]+$'
        name:
          type: string
        type:
          type: string
          enum: [rust_cargo, typescript_turborepo, typescript_nx, mixed]
        # ... full schema

paths:
  /workspaces:
    get:
      summary: List workspaces
      operationId: listWorkspaces
      tags: [Workspaces]
      parameters:
        - name: status
          in: query
          schema:
            type: string
            enum: [active, archived, all]
      responses:
        '200':
          description: Success
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/WorkspaceList'
        '401':
          $ref: '#/components/responses/Unauthorized'
    # ... all endpoints
```

## SDK Support

### TypeScript/JavaScript
```typescript
import { CortexClient } from '@cortex/sdk';

const client = new CortexClient({
  apiKey: process.env.CORTEX_API_KEY,
  baseUrl: 'https://api.cortex.dev'
});

// List workspaces
const workspaces = await client.workspaces.list({
  status: 'active',
  limit: 20
});

// Semantic search
const results = await client.search.semantic({
  query: 'authentication handler',
  workspace_id: 'ws_123'
});

// WebSocket connection
const ws = client.websocket();
ws.on('code_change', (event) => {
  console.log('Code changed:', event.path);
});
```

### Python
```python
from cortex import CortexClient

client = CortexClient(
    api_key=os.environ['CORTEX_API_KEY'],
    base_url='https://api.cortex.dev'
)

# List workspaces
workspaces = client.workspaces.list(status='active')

# Create session
session = client.sessions.create(
    agent_id='agent_123',
    workspace_id='ws_123',
    isolation_level='snapshot'
)

# Async operations
async def search_code():
    results = await client.search.semantic(
        query='database connection',
        limit=10
    )
    return results
```

### Rust
```rust
use cortex_sdk::{Client, SearchQuery};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new()
        .api_key(std::env::var("CORTEX_API_KEY")?)
        .build()?;

    // List workspaces
    let workspaces = client.workspaces()
        .status(Status::Active)
        .list()
        .await?;

    // Semantic search
    let results = client.search()
        .semantic(SearchQuery {
            query: "error handling".to_string(),
            workspace_id: Some("ws_123".to_string()),
            limit: Some(20),
            ..Default::default()
        })
        .await?;

    Ok(())
}
```

### CLI Tool
```bash
# Configure
cortex config set api_key mk_live_abc123
cortex config set endpoint https://api.cortex.dev

# List workspaces
cortex workspace list --status active

# Search code
cortex search semantic "user authentication" --workspace ws_123

# Create session
cortex session create --agent agent_123 --workspace ws_123

# Run build
cortex build trigger --workspace ws_123 --type release

# Export workspace
cortex export create --workspace ws_123 --format tar.gz
```

## Security

### Authentication Methods
1. **JWT Bearer Tokens**: Short-lived tokens for user sessions
2. **API Keys**: Long-lived keys for programmatic access
3. **OAuth 2.0**: Third-party application authorization (planned)
4. **mTLS**: Mutual TLS for enterprise deployments

### Permissions Model
```json
{
  "roles": {
    "admin": {
      "permissions": ["*"]
    },
    "developer": {
      "permissions": [
        "workspace:read",
        "workspace:write",
        "code:*",
        "session:*",
        "build:*"
      ]
    },
    "viewer": {
      "permissions": [
        "workspace:read",
        "code:read",
        "search:*"
      ]
    },
    "ci_cd": {
      "permissions": [
        "workspace:read",
        "build:*",
        "test:*",
        "export:read"
      ]
    }
  }
}
```

### API Security Headers
```http
Content-Security-Policy: default-src 'self'
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 1; mode=block
Strict-Transport-Security: max-age=31536000; includeSubDomains
```

## Monitoring & Metrics

### Prometheus Metrics Endpoint
```
GET /metrics

# HELP cortex_api_requests_total Total API requests
# TYPE cortex_api_requests_total counter
cortex_api_requests_total{method="GET",endpoint="/workspaces",status="200"} 12345

# HELP cortex_api_request_duration_seconds API request duration
# TYPE cortex_api_request_duration_seconds histogram
cortex_api_request_duration_seconds_bucket{le="0.1"} 10234
cortex_api_request_duration_seconds_bucket{le="0.5"} 11456
cortex_api_request_duration_seconds_bucket{le="1.0"} 11890
```

### Health Check
```
GET /health

Response:
{
  "status": "healthy",
  "checks": {
    "database": "pass",
    "storage": "pass",
    "cache": "pass"
  },
  "version": "3.0.0",
  "uptime": 864000
}
```

## Error Codes

| Code | Description | HTTP Status |
|------|-------------|-------------|
| RESOURCE_NOT_FOUND | Resource does not exist | 404 |
| UNAUTHORIZED | Authentication required | 401 |
| FORBIDDEN | Insufficient permissions | 403 |
| VALIDATION_ERROR | Invalid request data | 400 |
| CONFLICT | Resource version conflict | 409 |
| RATE_LIMIT_EXCEEDED | Too many requests | 429 |
| INTERNAL_ERROR | Server error | 500 |
| SERVICE_UNAVAILABLE | Temporary unavailable | 503 |

## Conclusion

The Cortex REST API provides comprehensive access to all system capabilities with:

1. **Complete Coverage**: All MCP tools exposed via REST
2. **Dashboard Support**: Specialized endpoints for visualization
3. **Real-time Updates**: WebSocket support for live data
4. **Enterprise Ready**: Authentication, rate limiting, monitoring
5. **Developer Friendly**: SDKs for major languages
6. **OpenAPI Spec**: Full API documentation and client generation

This API enables seamless integration with CI/CD pipelines, IDEs, monitoring systems, and custom dashboards while maintaining the security and performance required for production deployments.