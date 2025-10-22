# Cortex REST API - Authentication Guide

## Quick Start

### 1. Start the Server
```bash
cortex serve
```

The server will display available endpoints and authentication information on startup.

### 2. Login to Get Access Token

```bash
curl -X POST http://localhost:8080/api/v3/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@example.com",
    "password": "admin123"
  }'
```

Response:
```json
{
  "success": true,
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "token_type": "Bearer",
    "expires_in": 900,
    "user": {
      "id": "user-123",
      "email": "admin@example.com",
      "roles": ["admin"],
      "created_at": "2025-01-01T00:00:00Z"
    }
  }
}
```

### 3. Use Access Token for API Requests

```bash
# Store token for convenience
export TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."

# List workspaces
curl -X GET http://localhost:8080/api/v3/workspaces \
  -H "Authorization: Bearer $TOKEN"

# Create workspace
curl -X POST http://localhost:8080/api/v3/workspaces \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My Project",
    "workspace_type": "code",
    "source_path": "/path/to/project"
  }'
```

## Authentication Methods

### 1. JWT Bearer Token (Recommended for Users)

**Pros:**
- Short-lived for security (15 minutes)
- Can be refreshed without re-login
- Tied to user session

**Usage:**
```bash
curl -H "Authorization: Bearer <access_token>" \
  http://localhost:8080/api/v3/workspaces
```

### 2. API Key (Recommended for CI/CD)

**Pros:**
- Long-lived (configurable)
- No need to refresh
- Can be scoped per application

**Create API Key:**
```bash
curl -X POST http://localhost:8080/api/v3/auth/api-key \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "CI/CD Pipeline",
    "scopes": ["read", "write"],
    "expires_in_days": 90
  }'
```

**Usage:**
```bash
curl -H "Authorization: ApiKey cortex_a1b2c3d4e5f6..." \
  http://localhost:8080/api/v3/workspaces
```

## Token Refresh Flow

When access token expires (after 15 minutes):

```bash
curl -X POST http://localhost:8080/api/v3/auth/refresh \
  -H "Content-Type: application/json" \
  -d '{
    "refresh_token": "<your_refresh_token>"
  }'
```

Response contains new access token (refresh token remains valid).

## Route Categories

### Public (No Authentication Required)
- `POST /api/v3/auth/login` - User login
- `POST /api/v3/auth/refresh` - Refresh token
- `GET /api/v3/health` - Health check
- `GET /api/v3/metrics` - System metrics

### Protected (Authentication Required)
All other routes require valid Bearer token or API key.

### Admin-Only Operations
Some operations require admin role:
- `DELETE /api/v3/workspaces/:id` - Delete workspace
- User management operations (when implemented)

## User Roles

| Role | Description | Typical Use Case |
|------|-------------|------------------|
| `admin` | Full access to all operations | System administrators |
| `developer` | Standard development access | Software developers |
| `viewer` | Read-only access | Stakeholders, viewers |
| `ci_cd` | CI/CD pipeline access | Automated builds/tests |

## Common Scenarios

### Scenario 1: Web Application
```javascript
// Login
const loginResponse = await fetch('http://localhost:8080/api/v3/auth/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    email: 'user@example.com',
    password: 'password123'
  })
});

const { access_token, refresh_token } = await loginResponse.json();

// Store tokens
localStorage.setItem('access_token', access_token);
localStorage.setItem('refresh_token', refresh_token);

// Use token
const workspaces = await fetch('http://localhost:8080/api/v3/workspaces', {
  headers: {
    'Authorization': `Bearer ${access_token}`
  }
});

// Refresh when expired
const refreshResponse = await fetch('http://localhost:8080/api/v3/auth/refresh', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ refresh_token })
});

const { access_token: newToken } = await refreshResponse.json();
localStorage.setItem('access_token', newToken);
```

### Scenario 2: CI/CD Pipeline
```bash
#!/bin/bash
# ci-cd-deploy.sh

# API key stored as secret in CI/CD environment
API_KEY=$CORTEX_API_KEY

# Create workspace for deployment
WORKSPACE_ID=$(curl -s -X POST http://cortex.example.com/api/v3/workspaces \
  -H "Authorization: ApiKey $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Deploy-'$(date +%Y%m%d-%H%M%S)'",
    "workspace_type": "code",
    "source_path": "/builds/current"
  }' | jq -r '.data.id')

# Run analysis
curl -X POST http://cortex.example.com/api/v3/analysis/impact \
  -H "Authorization: ApiKey $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"workspace_id": "'$WORKSPACE_ID'"}'

# Trigger build
curl -X POST http://cortex.example.com/api/v3/build/trigger \
  -H "Authorization: ApiKey $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"workspace_id": "'$WORKSPACE_ID'"}'
```

### Scenario 3: Python Script
```python
import requests
import os

class CortexClient:
    def __init__(self, base_url, api_key=None):
        self.base_url = base_url
        self.api_key = api_key
        self.access_token = None

    def login(self, email, password):
        response = requests.post(
            f"{self.base_url}/api/v3/auth/login",
            json={"email": email, "password": password}
        )
        response.raise_for_status()
        data = response.json()['data']
        self.access_token = data['access_token']
        return data

    def _headers(self):
        if self.api_key:
            return {"Authorization": f"ApiKey {self.api_key}"}
        elif self.access_token:
            return {"Authorization": f"Bearer {self.access_token}"}
        else:
            raise ValueError("No authentication method set")

    def list_workspaces(self):
        response = requests.get(
            f"{self.base_url}/api/v3/workspaces",
            headers=self._headers()
        )
        response.raise_for_status()
        return response.json()['data']

    def create_workspace(self, name, workspace_type, source_path=None):
        response = requests.post(
            f"{self.base_url}/api/v3/workspaces",
            headers=self._headers(),
            json={
                "name": name,
                "workspace_type": workspace_type,
                "source_path": source_path
            }
        )
        response.raise_for_status()
        return response.json()['data']

# Usage with credentials
client = CortexClient("http://localhost:8080")
client.login("admin@example.com", "admin123")
workspaces = client.list_workspaces()

# Usage with API key
client = CortexClient("http://localhost:8080", api_key=os.getenv("CORTEX_API_KEY"))
workspaces = client.list_workspaces()
```

## Error Handling

### 401 Unauthorized
**Cause:** Missing or invalid authentication token

**Response:**
```json
{
  "success": false,
  "error": {
    "code": "UNAUTHORIZED",
    "message": "Authentication required"
  }
}
```

**Solution:** Login again or provide valid token/API key

### 403 Forbidden
**Cause:** Valid authentication but insufficient permissions

**Response:**
```json
{
  "success": false,
  "error": {
    "code": "FORBIDDEN",
    "message": "Only administrators can delete workspaces"
  }
}
```

**Solution:** Contact administrator to request appropriate role

### Token Expired
**Cause:** Access token expired (after 15 minutes)

**Response:**
```json
{
  "success": false,
  "error": {
    "code": "UNAUTHORIZED",
    "message": "Invalid or expired token"
  }
}
```

**Solution:** Use refresh token to get new access token

## Security Best Practices

### Development
- Default admin credentials should be changed immediately
- Use environment variables for JWT secret
- Enable HTTPS in production

### Production
- Always use HTTPS
- Use strong JWT secrets (32+ random characters)
- Rotate API keys regularly
- Implement rate limiting
- Monitor authentication logs
- Use secure password policies

### API Keys
- Store securely (environment variables, secret managers)
- Never commit to version control
- Rotate before expiration
- Use minimal scopes required
- One key per application/service

## Troubleshooting

### "Authentication required" on all requests
- Check if server is running
- Verify token is being sent in Authorization header
- Check token format: `Bearer <token>` or `ApiKey <key>`

### "Invalid or expired token"
- Token may have expired (15 min for access tokens)
- Use refresh token to get new access token
- Re-login if refresh token also expired

### "Insufficient permissions"
- Check user roles with `GET /api/v3/auth/me`
- Contact administrator for role upgrade
- Use correct account for operation

### Cannot login
- Verify credentials are correct
- Check if user exists in database
- Review server logs for errors

## Default Credentials

**⚠️ CHANGE IMMEDIATELY IN PRODUCTION**

Default admin user (created on first server start):
- Email: `admin@example.com`
- Password: `admin123`
- Roles: `["admin"]`

## Additional Resources

- Full API specification: See `/docs/spec/` directory
- Middleware implementation: `/src/api/middleware/auth.rs`
- Authentication routes: `/src/api/routes/auth.rs`
- Example integration tests: `/tests/api/auth_tests.rs`
