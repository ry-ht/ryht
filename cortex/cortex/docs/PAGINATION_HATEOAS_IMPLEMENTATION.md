# REST API v3 Pagination & HATEOAS Implementation Summary

This document summarizes the implementation of cursor-based pagination and HATEOAS links in the Cortex REST API v3.

## Overview

The REST API has been standardized to match the specification requirements:

1. **Cursor-based pagination** for all list endpoints
2. **HATEOAS links** for resource navigation
3. **Complete metadata** in all responses
4. **Backward compatibility** with legacy offset-based pagination

## Implementation Details

### 1. Core Type Updates (`types.rs`)

#### New Structures

**PaginationInfo:**
```rust
pub struct PaginationInfo {
    pub cursor: Option<String>,      // Next page cursor (base64-encoded)
    pub has_more: bool,               // Whether more results exist
    pub total: Option<usize>,         // Total count (if available)
    pub count: usize,                 // Items in current page
    pub limit: usize,                 // Page size limit
}
```

**HateoasLinks:**
```rust
pub struct HateoasLinks {
    pub self_link: String,                              // Current resource URL
    pub next: Option<String>,                           // Next page URL
    pub prev: Option<String>,                           // Previous page URL (reserved)
    pub related: Option<HashMap<String, String>>,       // Related resources
}
```

**PaginationParams:**
```rust
pub struct PaginationParams {
    pub cursor: Option<String>,       // Opaque cursor string
    pub limit: usize,                 // Items per page (10-100, default 20)
}
```

**CursorData (Internal):**
```rust
pub struct CursorData {
    pub last_id: String,              // Last item ID from previous page
    pub last_timestamp: DateTime<Utc>, // Last item timestamp
    pub offset: usize,                // Cumulative offset
}
```

#### Updated ApiResponse

```rust
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub metadata: ApiMetadata,
    pub pagination: Option<PaginationInfo>,  // NEW: Optional pagination
    pub links: Option<HateoasLinks>,         // NEW: Optional HATEOAS links
}
```

New helper method:
```rust
impl<T> ApiResponse<T> {
    pub fn success_with_pagination(
        data: T,
        request_id: String,
        duration_ms: u64,
        pagination: PaginationInfo,
        links: HateoasLinks,
    ) -> Self { ... }
}
```

### 2. Pagination Helper Module (`pagination.rs`)

#### Cursor Management

- `encode_cursor(data: &CursorData) -> Result<String, String>`
  - Encodes cursor data to base64 JSON string

- `decode_cursor(cursor: &str) -> Result<CursorData, String>`
  - Decodes base64 cursor back to CursorData

- `generate_next_cursor(last_id: String, last_timestamp: DateTime<Utc>, offset: usize) -> Option<String>`
  - Creates next page cursor from last item

#### Pagination Info Builder

- `build_pagination_info(items_count: usize, limit: usize, total: Option<usize>, next_cursor: Option<String>) -> PaginationInfo`
  - Constructs pagination metadata

#### LinkBuilder

The `LinkBuilder` provides methods for generating HATEOAS links:

**Generic Methods:**
```rust
LinkBuilder::new(base_url) -> Self
LinkBuilder::from_path(path) -> Self
builder.build_list_links(cursor, next_cursor, limit) -> HateoasLinks
builder.build_resource_links(resource_id, related) -> HateoasLinks
```

**Resource-Specific Methods:**
```rust
LinkBuilder::build_workspace_links(workspace_id) -> HateoasLinks
LinkBuilder::build_file_links(file_id, workspace_id) -> HateoasLinks
LinkBuilder::build_session_links(session_id, workspace_id) -> HateoasLinks
LinkBuilder::build_unit_links(unit_id, workspace_id) -> HateoasLinks
LinkBuilder::build_episode_links(episode_id) -> HateoasLinks
LinkBuilder::build_task_links(task_id, workspace_id) -> HateoasLinks
```

### 3. Updated Endpoints

#### Workspaces

**`GET /api/v3/workspaces`** (List)
- Added `PaginationParams` query parameter support
- Implements cursor-based pagination with database queries
- Returns pagination metadata and HATEOAS links
- Query: `ORDER BY created_at DESC, id DESC LIMIT n+1`
- Cursor filtering: `WHERE created_at < $timestamp OR (created_at = $timestamp AND id < $id)`

**`GET /api/v3/workspaces/:id`** (Single)
- Added HATEOAS links with related resources:
  - `files`: `/api/v3/workspaces/:id/files`
  - `units`: `/api/v3/workspaces/:id/units`
  - `dependencies`: `/api/v3/workspaces/:id/dependencies`
  - `tree`: `/api/v3/workspaces/:id/tree`
  - `sync`: `/api/v3/workspaces/:id/sync`

#### Files

**`GET /api/v3/workspaces/:id/files`** (List)
- Updated `FileListRequest` to include cursor-based pagination
- Maintains backward compatibility with `offset` parameter
- Implements in-memory cursor filtering after VFS query
- Sorts results by `created_at DESC, id DESC`
- Returns pagination metadata only when using cursor-based pagination

**Request Parameters:**
```rust
pub struct FileListRequest {
    pub recursive: bool,
    pub file_type: Option<String>,
    pub language: Option<String>,
    pub cursor: Option<String>,          // NEW: Cursor-based
    pub limit: usize,                    // NEW: Default 20
    pub offset: Option<usize>,           // LEGACY: Deprecated
}
```

#### Code Units

**`GET /api/v3/workspaces/:id/units`** (List)
- Updated `CodeUnitListRequest` with cursor support
- Similar structure to `FileListRequest`

```rust
pub struct CodeUnitListRequest {
    pub unit_type: Option<String>,
    pub visibility: Option<String>,
    pub language: Option<String>,
    pub min_complexity: Option<u32>,
    pub max_complexity: Option<u32>,
    pub has_tests: Option<bool>,
    pub has_docs: Option<bool>,
    pub cursor: Option<String>,          // NEW
    pub limit: usize,                    // NEW: Default 20
    pub offset: Option<usize>,           // LEGACY
}
```

## Backward Compatibility

### Optional Fields Strategy

All new fields are optional and skip serialization when null:
```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub pagination: Option<PaginationInfo>,

#[serde(skip_serializing_if = "Option::is_none")]
pub links: Option<HateoasLinks>,
```

### Legacy Support

1. **Offset-based pagination** still works on endpoints that previously supported it
2. Responses using offset-based pagination **do NOT** include pagination metadata
3. Endpoints automatically detect which pagination method is being used:
   - If `cursor` is present → cursor-based with metadata
   - If only `offset` is present → legacy offset-based without metadata
   - If neither is present → cursor-based with metadata (default)

### Migration Path

**Old Client (No Changes Required):**
```bash
GET /api/v3/workspaces/ws-123/files?offset=20&limit=10
# Response: Legacy format without pagination metadata
```

**New Client (Recommended):**
```bash
GET /api/v3/workspaces/ws-123/files?limit=20
# Response: Includes pagination.cursor for next page

GET /api/v3/workspaces/ws-123/files?cursor=eyJ...&limit=20
# Response: Next page with updated cursor
```

## Example Responses

### List Endpoint with Pagination

```json
{
  "success": true,
  "data": [
    {
      "id": "ws-001",
      "name": "my-project",
      "workspace_type": "code",
      "source_type": "local",
      "namespace": "ws_001",
      "source_path": "/path/to/project",
      "read_only": false,
      "created_at": "2025-10-22T10:00:00Z",
      "updated_at": "2025-10-22T14:00:00Z"
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
    "cursor": "eyJsYXN0X2lkIjoid3MtMDAxIiwibGFzdF90aW1lc3RhbXAiOiIyMDI1LTEwLTIyVDEwOjAwOjAwWiIsIm9mZnNldCI6MjB9",
    "has_more": true,
    "total": null,
    "count": 20,
    "limit": 20
  },
  "links": {
    "self": "/api/v3/workspaces?limit=20",
    "next": "/api/v3/workspaces?cursor=eyJ...&limit=20"
  }
}
```

### Single Resource with HATEOAS

```json
{
  "success": true,
  "data": {
    "id": "ws-001",
    "name": "my-project",
    "workspace_type": "code",
    "source_type": "local",
    "namespace": "ws_001",
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
    "self": "/api/v3/workspaces/ws-001",
    "related": {
      "files": "/api/v3/workspaces/ws-001/files",
      "units": "/api/v3/workspaces/ws-001/units",
      "dependencies": "/api/v3/workspaces/ws-001/dependencies",
      "tree": "/api/v3/workspaces/ws-001/tree",
      "sync": "/api/v3/workspaces/ws-001/sync"
    }
  }
}
```

## Files Modified

1. **`cortex/src/api/types.rs`**
   - Added `PaginationInfo`, `HateoasLinks`, `PaginationParams`, `CursorData`
   - Updated `ApiResponse` with optional pagination and links fields
   - Updated `FileListRequest` and `CodeUnitListRequest` with cursor support
   - Added `success_with_pagination()` helper method

2. **`cortex/src/api/pagination.rs`** (NEW)
   - Cursor encoding/decoding functions
   - Pagination info builder
   - LinkBuilder with resource-specific link generators
   - Comprehensive test suite

3. **`cortex/src/api/mod.rs`**
   - Added pagination module export
   - Exported new types and LinkBuilder

4. **`cortex/src/api/routes/workspaces.rs`**
   - Updated `list_workspaces()` with cursor-based pagination
   - Updated `get_workspace()` with HATEOAS links
   - Database queries now support cursor filtering

5. **`cortex/src/api/routes/vfs.rs`**
   - Updated `list_files()` with cursor-based pagination
   - Maintained backward compatibility with offset-based pagination
   - Conditional response format based on pagination method

## Performance Considerations

### Cursor-Based Pagination Benefits

1. **Consistent Results**: No duplicate or missing items when data changes
2. **Efficient Queries**: Uses indexed fields (created_at, id) for filtering
3. **Scalable**: Performance doesn't degrade with offset size
4. **Stateless**: Cursor contains all necessary information

### Database Query Pattern

```sql
-- First page
SELECT * FROM workspace
ORDER BY created_at DESC, id DESC
LIMIT 21;  -- Fetch n+1 to check has_more

-- Next page
SELECT * FROM workspace
WHERE created_at < $timestamp
   OR (created_at = $timestamp AND id < $id)
ORDER BY created_at DESC, id DESC
LIMIT 21;
```

### Total Count Trade-off

The `total` field is optional because:
- Computing total count requires additional COUNT(*) query
- On large datasets, this can be expensive
- Most UIs don't require exact count (infinite scroll, "Load More", etc.)
- Can be added per-endpoint if needed

## Testing

### Unit Tests

The `pagination.rs` module includes comprehensive tests:
- Cursor encoding/decoding round-trip
- Pagination info building
- Link generation for lists and resources
- Resource-specific link builders

### Integration Testing

To verify the implementation:

```bash
# Test cursor-based pagination
curl "http://localhost:8080/api/v3/workspaces?limit=2"
# Extract cursor from response, use in next request
curl "http://localhost:8080/api/v3/workspaces?cursor=eyJ...&limit=2"

# Test legacy offset-based (should work without pagination metadata)
curl "http://localhost:8080/api/v3/workspaces?offset=10&limit=5"

# Test HATEOAS links
curl "http://localhost:8080/api/v3/workspaces/ws-123"
# Follow related links from response

# Test file listing with filters
curl "http://localhost:8080/api/v3/workspaces/ws-123/files?file_type=file&limit=20"
```

## Future Enhancements

1. **Previous Page Support**: Add `prev` link for bidirectional navigation
2. **Total Count Optimization**: Add caching for total counts
3. **Custom Sort Orders**: Allow client-specified sorting
4. **Filter Composition**: More complex filter combinations
5. **Link Templates**: RFC 6570 URI templates for related resources
6. **ETag Support**: Add caching headers for resources
7. **Sparse Fieldsets**: Allow clients to request specific fields

## Compliance Checklist

✅ **Pagination Support** (Spec lines 62-85, 74-84)
- ✅ Cursor-based pagination (not offset-based)
- ✅ `cursor` field in PaginationInfo
- ✅ `has_more` boolean flag
- ✅ `total` count (optional)
- ✅ Query parameters: `cursor` (string), `limit` (10-100, default 20)
- ✅ Pagination metadata in list responses

✅ **HATEOAS Links**
- ✅ `links` object in ApiResponse
- ✅ `self` link (current resource URL)
- ✅ `next` link (next page for lists)
- ✅ `related` object with related resource URLs
- ✅ Resource-specific links for:
  - ✅ Workspace → files, units, dependencies
  - ✅ File → workspace
  - ✅ Session → workspace, files
  - ✅ Code Unit → workspace, references
  - ✅ Episode → self
  - ✅ Task → workspace

✅ **Metadata Completeness**
- ✅ `request_id` (UUID)
- ✅ `timestamp` (ISO 8601)
- ✅ `version` ("v3")
- ✅ `duration_ms` (processing time)

✅ **Backward Compatibility**
- ✅ All new fields are optional
- ✅ Legacy offset-based pagination still works
- ✅ Existing clients unaffected

## References

- API Response Format Specification: `cortex/docs/api-response-format.md`
- Implementation Code: `cortex/src/api/pagination.rs`
- Type Definitions: `cortex/src/api/types.rs`
