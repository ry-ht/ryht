# REST API v3 Response Format Specification

This document describes the standardized API response format for Cortex REST API v3, including pagination and HATEOAS link support.

## Standard Response Structure

All API responses follow this structure:

```json
{
  "success": true,
  "data": <response-data>,
  "error": null,
  "metadata": {
    "request_id": "550e8400-e29b-41d4-a716-446655440000",
    "timestamp": "2025-10-22T14:30:00Z",
    "version": "v3",
    "duration_ms": 42
  },
  "pagination": {  // Optional, only for list endpoints
    "cursor": "eyJsYXN0X2lkIjoi...",
    "has_more": true,
    "total": 150,
    "count": 20,
    "limit": 20
  },
  "links": {  // Optional, HATEOAS links
    "self": "/api/v3/workspaces?limit=20",
    "next": "/api/v3/workspaces?cursor=eyJsYXN0X2lkIjoi...&limit=20",
    "related": {
      "files": "/api/v3/workspaces/ws-123/files",
      "units": "/api/v3/workspaces/ws-123/units"
    }
  }
}
```

## Response Fields

### Core Fields

- **success** (boolean): Indicates if the request was successful
- **data** (any): The response payload (null on error)
- **error** (string|null): Error message if success is false
- **metadata** (object): Request metadata

### Metadata Object

- **request_id** (string): Unique identifier for this request (UUID)
- **timestamp** (datetime): When the response was generated (ISO 8601)
- **version** (string): API version (always "v3")
- **duration_ms** (integer): Request processing time in milliseconds

### Pagination Object (Optional)

Only present on list endpoints that support cursor-based pagination:

- **cursor** (string|null): Opaque cursor for the next page (base64-encoded)
- **has_more** (boolean): Whether more results exist beyond this page
- **total** (integer|null): Total count of items (if available)
- **count** (integer): Number of items in current page
- **limit** (integer): Maximum items per page

### HATEOAS Links Object (Optional)

Provides hypermedia links for resource navigation:

- **self** (string): URL to the current resource
- **next** (string|null): URL to the next page (for paginated lists)
- **prev** (string|null): URL to the previous page (reserved for future use)
- **related** (object|null): Links to related resources

## Pagination

### Cursor-Based Pagination

All list endpoints support cursor-based pagination (preferred method):

**Query Parameters:**
- `cursor` (string, optional): Opaque pagination cursor
- `limit` (integer, optional): Items per page (10-100, default 20)

**Example Request:**
```bash
GET /api/v3/workspaces?limit=20
GET /api/v3/workspaces?cursor=eyJsYXN0X2lkIjoi...&limit=20
```

**Cursor Format:**
Cursors are base64-encoded JSON objects containing:
```json
{
  "last_id": "550e8400-e29b-41d4-a716-446655440000",
  "last_timestamp": "2025-10-22T14:30:00Z",
  "offset": 20
}
```

**Important:** Cursors are opaque and should not be decoded or manipulated by clients.

### Legacy Offset-Based Pagination (Deprecated)

Some endpoints still support offset-based pagination for backward compatibility:

**Query Parameters:**
- `offset` (integer): Number of items to skip
- `limit` (integer): Items per page

**Note:** Offset-based pagination does NOT include pagination metadata in responses.

## HATEOAS Links

### Resource-Specific Links

Different resources provide different related links:

#### Workspace Links
```json
{
  "self": "/api/v3/workspaces/ws-123",
  "related": {
    "files": "/api/v3/workspaces/ws-123/files",
    "units": "/api/v3/workspaces/ws-123/units",
    "dependencies": "/api/v3/workspaces/ws-123/dependencies",
    "tree": "/api/v3/workspaces/ws-123/tree",
    "sync": "/api/v3/workspaces/ws-123/sync"
  }
}
```

#### File Links
```json
{
  "self": "/api/v3/files/file-456",
  "related": {
    "workspace": "/api/v3/workspaces/ws-123"
  }
}
```

#### Session Links
```json
{
  "self": "/api/v3/sessions/sess-789",
  "related": {
    "workspace": "/api/v3/workspaces/ws-123",
    "files": "/api/v3/workspaces/ws-123/files?session_id=sess-789"
  }
}
```

#### Code Unit Links
```json
{
  "self": "/api/v3/units/unit-abc",
  "related": {
    "workspace": "/api/v3/workspaces/ws-123",
    "references": "/api/v3/search/references/unit-abc"
  }
}
```

## Example Responses

### List Workspaces (Paginated)

**Request:**
```bash
GET /api/v3/workspaces?limit=2
```

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "my-project",
      "workspace_type": "code",
      "source_type": "local",
      "namespace": "ws_550e8400_e29b_41d4_a716_446655440000",
      "source_path": "/path/to/project",
      "read_only": false,
      "created_at": "2025-10-22T10:00:00Z",
      "updated_at": "2025-10-22T14:00:00Z"
    },
    {
      "id": "660e8400-e29b-41d4-a716-446655440001",
      "name": "docs-project",
      "workspace_type": "documentation",
      "source_type": "local",
      "namespace": "ws_660e8400_e29b_41d4_a716_446655440001",
      "source_path": "/path/to/docs",
      "read_only": true,
      "created_at": "2025-10-21T10:00:00Z",
      "updated_at": "2025-10-22T12:00:00Z"
    }
  ],
  "error": null,
  "metadata": {
    "request_id": "req-12345",
    "timestamp": "2025-10-22T14:30:00Z",
    "version": "v3",
    "duration_ms": 15
  },
  "pagination": {
    "cursor": "eyJsYXN0X2lkIjoiNjYwZTg0MDAtZTI5Yi00MWQ0LWE3MTYtNDQ2NjU1NDQwMDAxIiwibGFzdF90aW1lc3RhbXAiOiIyMDI1LTEwLTIxVDEwOjAwOjAwWiIsIm9mZnNldCI6Mn0=",
    "has_more": true,
    "total": null,
    "count": 2,
    "limit": 2
  },
  "links": {
    "self": "/api/v3/workspaces?limit=2",
    "next": "/api/v3/workspaces?cursor=eyJsYXN0X2lkIjoi...&limit=2"
  }
}
```

### Get Single Workspace (With HATEOAS)

**Request:**
```bash
GET /api/v3/workspaces/550e8400-e29b-41d4-a716-446655440000
```

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "my-project",
    "workspace_type": "code",
    "source_type": "local",
    "namespace": "ws_550e8400_e29b_41d4_a716_446655440000",
    "source_path": "/path/to/project",
    "read_only": false,
    "created_at": "2025-10-22T10:00:00Z",
    "updated_at": "2025-10-22T14:00:00Z"
  },
  "error": null,
  "metadata": {
    "request_id": "req-67890",
    "timestamp": "2025-10-22T14:31:00Z",
    "version": "v3",
    "duration_ms": 8
  },
  "links": {
    "self": "/api/v3/workspaces/550e8400-e29b-41d4-a716-446655440000",
    "related": {
      "files": "/api/v3/workspaces/550e8400-e29b-41d4-a716-446655440000/files",
      "units": "/api/v3/workspaces/550e8400-e29b-41d4-a716-446655440000/units",
      "dependencies": "/api/v3/workspaces/550e8400-e29b-41d4-a716-446655440000/dependencies",
      "tree": "/api/v3/workspaces/550e8400-e29b-41d4-a716-446655440000/tree",
      "sync": "/api/v3/workspaces/550e8400-e29b-41d4-a716-446655440000/sync"
    }
  }
}
```

### List Files (Paginated with Filters)

**Request:**
```bash
GET /api/v3/workspaces/550e8400-e29b-41d4-a716-446655440000/files?file_type=file&limit=20
```

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "id": "file-001",
      "name": "main.rs",
      "path": "/src/main.rs",
      "file_type": "file",
      "size": 1024,
      "language": "rust",
      "content": null,
      "created_at": "2025-10-22T10:00:00Z",
      "updated_at": "2025-10-22T14:00:00Z"
    }
  ],
  "error": null,
  "metadata": {
    "request_id": "req-11111",
    "timestamp": "2025-10-22T14:32:00Z",
    "version": "v3",
    "duration_ms": 22
  },
  "pagination": {
    "cursor": null,
    "has_more": false,
    "total": 1,
    "count": 1,
    "limit": 20
  },
  "links": {
    "self": "/api/v3/workspaces/550e8400-e29b-41d4-a716-446655440000/files?limit=20"
  }
}
```

### Error Response

**Request:**
```bash
GET /api/v3/workspaces/invalid-id
```

**Response:**
```json
{
  "success": false,
  "data": null,
  "error": "Invalid workspace ID",
  "metadata": {
    "request_id": "req-error-1",
    "timestamp": "2025-10-22T14:33:00Z",
    "version": "v3",
    "duration_ms": 2
  }
}
```

## Backward Compatibility

### Optional Fields
All new fields (`pagination` and `links`) are optional and only included when relevant:
- Existing clients that don't expect these fields can safely ignore them
- Legacy endpoints using offset-based pagination continue to work without pagination metadata

### Migration Guide

**Before (Legacy):**
```bash
GET /api/v3/workspaces/ws-123/files?offset=20&limit=10
```

**After (Cursor-based):**
```bash
# First page
GET /api/v3/workspaces/ws-123/files?limit=10

# Next page (use cursor from response)
GET /api/v3/workspaces/ws-123/files?cursor=eyJsYXN0X2lkIjoi...&limit=10
```

## Supported Endpoints

The following endpoints now support cursor-based pagination and HATEOAS links:

### List Endpoints (Paginated)
- `GET /api/v3/workspaces` - List workspaces
- `GET /api/v3/workspaces/:id/files` - List workspace files
- `GET /api/v3/workspaces/:id/units` - List code units
- `GET /api/v3/sessions` - List sessions
- `GET /api/v3/memory/episodes` - List memory episodes
- `GET /api/v3/tasks` - List tasks

### Single Resource Endpoints (HATEOAS Links)
- `GET /api/v3/workspaces/:id` - Get workspace details
- `GET /api/v3/files/:id` - Get file details
- `GET /api/v3/sessions/:id` - Get session details
- `GET /api/v3/units/:id` - Get code unit details
- `GET /api/v3/memory/episodes/:id` - Get episode details
- `GET /api/v3/tasks/:id` - Get task details

## Best Practices

1. **Always use cursor-based pagination** for new integrations
2. **Follow HATEOAS links** instead of constructing URLs manually
3. **Check `has_more`** before fetching the next page
4. **Store cursors opaquely** - don't decode or manipulate them
5. **Respect the `limit` range** (10-100, default 20)
6. **Use `request_id`** for debugging and support requests
7. **Monitor `duration_ms`** for performance tracking

## Performance Considerations

- Cursor-based pagination is more efficient than offset-based for large datasets
- The `total` count may be omitted on some endpoints to improve performance
- HATEOAS links are generated dynamically based on resource relationships
- Limit values are capped at 100 to prevent excessive memory usage
