# JWT Authentication Middleware Implementation

## Overview

This document describes the JWT authentication middleware implementation for the Cortex REST API. The middleware protects API routes and provides role-based access control.

## Features

1. **JWT Token Validation**: Validates Bearer tokens from Authorization header
2. **API Key Support**: Validates API keys for programmatic access
3. **Role-Based Access Control**: Supports multiple user roles
4. **Proper Error Responses**: Returns 401/403 with WWW-Authenticate header
5. **User Context**: Extracts and stores user information in request extensions

## Architecture

### Components

1. **AuthUser** (`middleware/auth.rs`):
   - User information extracted from JWT claims
   - Stored in request extensions for use in handlers
   - Provides helper methods for role checking

2. **AuthMiddleware** (`middleware/auth.rs`):
   - `validate()`: Required authentication
   - `optional()`: Optional authentication
   - `require_role()`: Role-based access control
   - `require_admin()`: Admin-only access
   - `require_any_role()`: Multiple role support

3. **AuthState** (`middleware/auth.rs`):
   - Shared state for JWT validation
   - Contains storage connection and JWT secret

## Route Protection

### Public Routes (No Authentication)
- `POST /api/v3/auth/login` - User login
- `POST /api/v3/auth/refresh` - Refresh access token
- `GET /api/v3/health` - Health check
- `GET /api/v3/metrics` - System metrics

### Protected Routes (Authentication Required)
All other routes require a valid JWT token or API key:
- Workspaces management
- Files and VFS operations
- Sessions and search
- Memory and code units
- Analysis and build
- Dashboard
- Tasks
- Export/Import

### WebSocket Routes
- `WS /api/v3/ws` - Optional authentication (token checked if provided)

## Authentication Methods

### 1. Bearer Token (JWT)
```http
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

### 2. API Key
```http
Authorization: ApiKey cortex_a1b2c3d4e5f6...
```

## User Roles

The system supports the following roles:

- **admin**: Full access to all operations
- **developer**: Standard developer access
- **viewer**: Read-only access
- **ci_cd**: CI/CD pipeline access

Admins have automatic access to all role-protected routes.

## Usage in Route Handlers

### Basic Authentication
```rust
use crate::api::middleware::AuthUser;

async fn list_workspaces(
    auth_user: AuthUser,  // Extract authenticated user
    State(ctx): State<WorkspaceContext>,
) -> ApiResult<Json<ApiResponse<Vec<WorkspaceResponse>>>> {
    tracing::info!(
        user_id = %auth_user.user_id,
        email = %auth_user.email,
        roles = ?auth_user.roles,
        "User listing workspaces"
    );

    // Handler logic...
}
```

### Admin-Only Operations
```rust
async fn delete_workspace(
    auth_user: AuthUser,
    State(ctx): State<WorkspaceContext>,
    Path(workspace_id): Path<String>,
) -> ApiResult<Json<ApiResponse<()>>> {
    // Check if user is admin
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Only administrators can delete workspaces".to_string()
        ));
    }

    // Deletion logic...
}
```

### Role-Based Access
```rust
async fn build_trigger(
    auth_user: AuthUser,
    State(ctx): State<BuildContext>,
) -> ApiResult<Json<ApiResponse<BuildResponse>>> {
    // Check if user has required role
    if !auth_user.has_any_role(&["developer", "ci_cd"]) {
        return Err(ApiError::Forbidden(
            "Build access requires developer or ci_cd role".to_string()
        ));
    }

    // Build logic...
}
```

## Error Responses

### 401 Unauthorized
Returned when authentication is required but missing or invalid:

```json
{
  "success": false,
  "error": {
    "code": "UNAUTHORIZED",
    "message": "Authentication required"
  }
}
```

Headers:
```
WWW-Authenticate: Bearer realm="Cortex API"
```

### 403 Forbidden
Returned when authenticated but lacking required permissions:

```json
{
  "success": false,
  "error": {
    "code": "FORBIDDEN",
    "message": "Insufficient permissions. Required role: admin"
  }
}
```

## Implementation Details

### JWT Validation
- Uses `jsonwebtoken` crate for validation
- Checks token signature with shared secret
- Verifies token type (`access` vs `refresh`)
- Validates expiration time

### API Key Validation
- Hashes API key with bcrypt
- Compares against stored hash in database
- Checks expiration time
- Loads user information from database

### Request Flow
1. Request arrives at server
2. Middleware extracts Authorization header
3. Token/key is validated
4. User information is stored in request extensions
5. Route handler extracts AuthUser
6. Handler performs authorization checks
7. Handler executes business logic

## Configuration

### JWT Secret
Set via environment variable:
```bash
export JWT_SECRET="your-secret-key-here"
```

Default (development only):
```
cortex-dev-secret-change-in-production
```

### Token Expiry
- Access tokens: 15 minutes
- Refresh tokens: 7 days
- API keys: Configurable per key

## Security Considerations

1. **Always use HTTPS in production** - Tokens are bearer credentials
2. **Rotate JWT secrets regularly** - Use strong secrets in production
3. **Implement rate limiting** - Prevent brute force attacks
4. **Log authentication events** - Monitor for suspicious activity
5. **Validate token expiration** - Ensure tokens expire properly
6. **Use secure password hashing** - bcrypt with appropriate cost
7. **Implement token revocation** - Store sessions in database

## Testing

### Login Flow
```bash
# Login
curl -X POST http://localhost:8080/api/v3/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@example.com","password":"admin123"}'

# Use token
curl -X GET http://localhost:8080/api/v3/workspaces \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
```

### API Key Flow
```bash
# Create API key (requires authentication)
curl -X POST http://localhost:8080/api/v3/auth/api-key \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"name":"CI/CD Key","scopes":["read","write"],"expires_in_days":90}'

# Use API key
curl -X GET http://localhost:8080/api/v3/workspaces \
  -H "Authorization: ApiKey cortex_a1b2c3d4e5f6..."
```

### Test Authentication Failures
```bash
# No token - should return 401
curl -X GET http://localhost:8080/api/v3/workspaces

# Invalid token - should return 401
curl -X GET http://localhost:8080/api/v3/workspaces \
  -H "Authorization: Bearer invalid-token"

# Valid token, insufficient permissions - should return 403
curl -X DELETE http://localhost:8080/api/v3/workspaces/123 \
  -H "Authorization: Bearer <viewer-token>"
```

## Migration Notes

### Before This Implementation
- All routes were publicly accessible
- No authentication or authorization
- No user context in handlers

### After This Implementation
- Public routes: `/health`, `/metrics`, `/auth/login`, `/auth/refresh`
- Protected routes: All other endpoints require authentication
- User context available in all handlers
- Role-based access control available

## Future Enhancements

1. **OAuth2/OIDC Support**: External identity providers
2. **Token Refresh Rotation**: More secure refresh token handling
3. **Multi-Factor Authentication**: Additional security layer
4. **Fine-Grained Permissions**: Beyond role-based access
5. **Audit Logging**: Comprehensive authentication audit trail
6. **Token Blacklisting**: Explicit token revocation
7. **Rate Limiting per User**: Prevent abuse
8. **Session Management UI**: Admin panel for session management

## References

- JWT Specification: https://datatracker.ietf.org/doc/html/rfc7519
- Bearer Token Specification: https://datatracker.ietf.org/doc/html/rfc6750
- Axum Middleware: https://docs.rs/axum/latest/axum/middleware/
- jsonwebtoken crate: https://docs.rs/jsonwebtoken/
