# Documentation MCP Tools

Complete reference for Cortex MCP documentation management tools.

## Overview

The documentation system provides 26 comprehensive tools for creating, managing, and organizing technical documentation with full integration to the codebase.

**Total Tools:** 26

## Tool Categories

- [Document CRUD](#document-crud) (8 tools)
- [Section Management](#section-management) (5 tools)
- [Link Management](#link-management) (3 tools)
- [Search & Discovery](#search--discovery) (3 tools)
- [Advanced Operations](#advanced-operations) (3 tools)
- [Versioning](#versioning) (3 tools)

---

## Document CRUD

### cortex.document.create

Create a new document with metadata and content.

**Input:**
- `title` (string, required): Document title
- `content` (string, required): Document content in Markdown
- `doc_type` (string, optional): Document type - `guide`, `api`, `architecture`, `tutorial`, `explanation`, `troubleshooting`, `faq`, `release_notes`, `example`, `general`
- `description` (string, optional): Brief description
- `parent_id` (string, optional): Parent document ID for hierarchy
- `tags` (array, optional): Tag list
- `keywords` (array, optional): Keywords for search
- `author` (string, optional): Author name/email
- `language` (string, optional): Language code (default: "en")
- `workspace_id` (string, optional): Associated workspace
- `metadata` (object, optional): Custom metadata

**Output:**
- `document_id`: Created document ID
- `title`: Document title
- `slug`: URL-friendly slug
- `created_at`: Creation timestamp

---

### cortex.document.get

Get a document by its ID with full metadata.

**Input:**
- `document_id` (string, required): Document ID

**Output:**
- Complete document object with all fields

---

### cortex.document.get-by-slug

Get a document by its URL slug for user-friendly access.

**Input:**
- `slug` (string, required): URL slug

**Output:**
- Complete document object

---

### cortex.document.update

Update document content and metadata.

**Input:**
- `document_id` (string, required): Document ID
- `title` (string, optional): New title
- `content` (string, optional): New content
- `description` (string, optional): New description
- `doc_type` (string, optional): New document type
- `tags` (array, optional): New tags
- `keywords` (array, optional): New keywords
- `metadata` (object, optional): New metadata

**Output:**
- `document_id`: Updated document ID
- `updated`: Boolean success flag
- `version`: Updated version string

---

### cortex.document.delete

Delete a document permanently.

**Input:**
- `document_id` (string, required): Document ID

**Output:**
- `document_id`: Deleted document ID
- `deleted`: Boolean success flag

---

### cortex.document.list

List documents with optional filtering.

**Input:**
- `status` (string, optional): Filter by status - `draft`, `review`, `published`, `archived`
- `doc_type` (string, optional): Filter by document type
- `parent_id` (string, optional): Filter by parent document
- `workspace_id` (string, optional): Filter by workspace
- `limit` (number, optional): Maximum results (default: 50)

**Output:**
- `documents`: Array of document summaries
- `total_count`: Total matching documents

---

### cortex.document.publish

Publish a document (changes status to Published, sets published_at timestamp).

**Input:**
- `document_id` (string, required): Document ID

**Output:**
- `document_id`: Document ID
- `status`: New status (Published)
- `published_at`: Publication timestamp

---

### cortex.document.archive

Archive a document (changes status to Archived).

**Input:**
- `document_id` (string, required): Document ID

**Output:**
- `document_id`: Document ID
- `status`: New status (Archived)

---

## Section Management

### cortex.document.section.create

Create a new section within a document.

**Input:**
- `document_id` (string, required): Parent document ID
- `title` (string, required): Section title
- `content` (string, required): Section content
- `level` (number, required): Heading level (1-6)
- `parent_section_id` (string, optional): Parent section for nesting
- `order` (number, optional): Display order

**Output:**
- `section_id`: Created section ID
- `document_id`: Parent document ID
- `title`: Section title
- `level`: Heading level
- `order`: Display order

---

### cortex.document.section.get

Get a specific section by ID with full content.

**Input:**
- `section_id` (string, required): Section ID

**Output:**
- Complete section object with content

---

### cortex.document.section.update

Update section content and metadata.

**Input:**
- `section_id` (string, required): Section ID
- `title` (string, optional): New title
- `content` (string, optional): New content
- `order` (number, optional): New display order

**Output:**
- `section_id`: Updated section ID
- `updated`: Boolean success flag

---

### cortex.document.section.delete

Delete a section from a document.

**Input:**
- `section_id` (string, required): Section ID

**Output:**
- `section_id`: Deleted section ID
- `deleted`: Boolean success flag

---

### cortex.document.section.list

List all sections for a document in hierarchical order.

**Input:**
- `document_id` (string, required): Document ID

**Output:**
- `sections`: Array of section summaries
- `total_count`: Total sections

---

## Link Management

### cortex.document.link.create

Create a link from a document to another resource (document, code unit, file, or external URL).

**Input:**
- `source_document_id` (string, required): Source document ID
- `link_type` (string, required): Link type - `reference`, `related`, `prerequisite`, `next`, `previous`, `parent`, `child`, `external`, `api`, `example`
- `target_type` (string, required): Target type - `document`, `codeunit`, `external`, `file`
- `target_id` (string, required): Target identifier (document ID, code unit ID, URL, or file path)

**Output:**
- `link_id`: Created link ID
- `source_document_id`: Source document
- `link_type`: Link relationship type
- `target`: Target description

---

### cortex.document.link.list

List all links for a document.

**Input:**
- `document_id` (string, required): Document ID

**Output:**
- `links`: Array of link summaries
- `total_count`: Total links

---

### cortex.document.link.delete

Delete a link from a document.

**Input:**
- `link_id` (string, required): Link ID

**Output:**
- `link_id`: Deleted link ID
- `deleted`: Boolean success flag

---

## Search & Discovery

### cortex.document.search

Full-text search across documents by title, content, and keywords.

**Input:**
- `query` (string, required): Search query
- `limit` (number, optional): Maximum results (default: 20)

**Output:**
- `documents`: Array of matching document summaries
- `total_count`: Total matches
- `query`: Original query string

---

### cortex.document.tree

Get document hierarchy tree with children, sections count, and links count.

**Input:**
- `document_id` (string, required): Root document ID

**Output:**
- `document`: Full document object
- `children`: Array of child documents
- `sections_count`: Number of sections
- `links_count`: Number of links

---

### cortex.document.related

Get all related documents through link relationships.

**Input:**
- `document_id` (string, required): Document ID

**Output:**
- `related_documents`: Array of related documents with link metadata
- `total_count`: Total related documents

---

## Advanced Operations

### cortex.document.clone

Clone a document with all its sections (creates as Draft status).

**Input:**
- `document_id` (string, required): Source document ID
- `new_title` (string, required): New document title

**Output:**
- `new_document_id`: Cloned document ID
- `original_document_id`: Source document ID
- `title`: New document title
- `slug`: New document slug
- `sections_cloned`: Number of sections cloned

---

### cortex.document.merge

Merge multiple documents into a single new document with combined content and sections.

**Input:**
- `document_ids` (array, required): Array of document IDs to merge
- `new_title` (string, required): New merged document title
- `merge_sections` (boolean, optional): Include all sections (default: false)

**Output:**
- `merged_document_id`: New merged document ID
- `title`: Merged document title
- `slug`: Merged document slug
- `documents_merged`: Number of documents merged
- `sections_merged`: Number of sections merged

**Note:** Documents are merged with separator lines, tags and keywords are combined and deduplicated.

---

### cortex.document.stats

Get comprehensive statistics for a document.

**Input:**
- `document_id` (string, required): Document ID

**Output:**
- `document_id`: Document ID
- `content_length`: Total characters
- `word_count`: Total words
- `line_count`: Total lines
- `sections_count`: Number of sections
- `links_count`: Number of links
- `versions_count`: Number of versions
- `tags_count`: Number of tags
- `keywords_count`: Number of keywords

---

## Versioning

### cortex.document.version.create

Create a version snapshot of a document's current state.

**Input:**
- `document_id` (string, required): Document ID
- `version` (string, required): Version identifier (e.g., "1.0.0", "v2")
- `author` (string, required): Version author
- `message` (string, required): Version description/changelog

**Output:**
- `version_id`: Created version ID
- `document_id`: Document ID
- `version`: Version identifier
- `created_at`: Creation timestamp

---

### cortex.document.version.get

Get a specific document version by its ID to retrieve historical content.

**Input:**
- `version_id` (string, required): Version ID

**Output:**
- `version_id`: Version ID
- `document_id`: Parent document ID
- `version`: Version identifier
- `content`: Snapshot content
- `author`: Version author
- `message`: Version description
- `created_at`: Creation timestamp

---

### cortex.document.version.list

List all versions for a document in reverse chronological order.

**Input:**
- `document_id` (string, required): Document ID

**Output:**
- `versions`: Array of version summaries
- `total_count`: Total versions

---

## Document Types

- **Guide**: Step-by-step guides and how-tos
- **ApiReference**: API documentation and reference
- **Architecture**: Architecture design documents
- **Tutorial**: Learning tutorials
- **Explanation**: Conceptual explanations
- **Troubleshooting**: Problem-solving guides
- **Faq**: Frequently asked questions
- **ReleaseNotes**: Version release notes
- **Example**: Code examples and samples
- **General**: General documentation

## Document Status Lifecycle

1. **Draft**: Initial creation state
2. **Review**: Under review
3. **Published**: Published and visible
4. **Archived**: Archived but retained
5. **Deprecated**: Marked as deprecated

## Link Types

- **Reference**: General reference link
- **Related**: Related content
- **Prerequisite**: Required reading before this document
- **Next**: Next document in sequence
- **Previous**: Previous document in sequence
- **Parent**: Parent document
- **Child**: Child document
- **External**: External URL
- **ApiReference**: API reference link
- **Example**: Example code or usage

## Link Targets

- **Document**: Link to another document (with optional section_id)
- **CodeUnit**: Link to code entity (function, class, module)
- **External**: Link to external URL
- **File**: Link to file or directory in workspace

## Usage Patterns

### Creating Structured Documentation

```javascript
// 1. Create parent document
const parent = await mcp.call('cortex.document.create', {
  title: 'System Architecture',
  content: 'Overview of system architecture...',
  doc_type: 'architecture'
});

// 2. Create child documents
const child = await mcp.call('cortex.document.create', {
  title: 'API Gateway',
  content: 'API Gateway design...',
  parent_id: parent.document_id,
  doc_type: 'architecture'
});

// 3. Add sections
await mcp.call('cortex.document.section.create', {
  document_id: child.document_id,
  title: 'Overview',
  content: 'Gateway overview...',
  level: 2,
  order: 0
});
```

### Linking Documentation to Code

```javascript
// Link document to code unit
await mcp.call('cortex.document.link.create', {
  source_document_id: 'doc-123',
  link_type: 'api',
  target_type: 'codeunit',
  target_id: 'code-unit-456'
});

// Link to file
await mcp.call('cortex.document.link.create', {
  source_document_id: 'doc-123',
  link_type: 'reference',
  target_type: 'file',
  target_id: 'src/services/auth.rs'
});
```

### Merging Documentation

```javascript
// Merge multiple guides into one
const merged = await mcp.call('cortex.document.merge', {
  document_ids: ['doc-1', 'doc-2', 'doc-3'],
  new_title: 'Complete Guide',
  merge_sections: true
});
```

## Best Practices

1. **Use Hierarchies**: Organize documents with parent_id for logical structure
2. **Link to Code**: Always link documentation to relevant code units and files
3. **Version Important Changes**: Create versions before major updates
4. **Tag Appropriately**: Use consistent tags for discoverability
5. **Section Large Documents**: Break large documents into manageable sections
6. **Publish When Ready**: Keep documents as Draft until ready for consumption
7. **Use Related Links**: Connect related documentation for better navigation
8. **Regular Stats Review**: Use stats to identify documentation gaps

## Integration Points

- **Code Units**: Link documentation to functions, classes, modules via `link.create`
- **Workspaces**: Associate documents with workspaces for project-specific docs
- **VFS**: Link to files and directories in the virtual file system
- **Search**: Full-text search integrates with semantic search system
- **Versions**: Version control integrates with document history tracking

## Metadata Schema

Documents support custom metadata through the `metadata` field:

```json
{
  "metadata": {
    "audience": "developers",
    "complexity": "intermediate",
    "reading_time": 15,
    "last_reviewed": "2025-01-15",
    "reviewers": ["alice@example.com"],
    "custom_fields": {}
  }
}
```

## Performance Considerations

- **Pagination**: Use `limit` parameter for large result sets
- **Caching**: Document content is cached for 1 hour
- **Search**: Search is indexed for fast full-text queries
- **Sections**: Prefer sections over very large document content
- **Links**: Link counts are pre-aggregated for tree operations

## Error Handling

All tools return standard MCP error responses:

- **Invalid Input**: Validation errors for malformed input
- **Not Found**: When document/section/link doesn't exist
- **Permission Denied**: When user lacks required permissions
- **Internal Error**: For unexpected system errors

## Future Enhancements

- Template system for document creation
- Bulk operations for batch processing
- Advanced search with filters and facets
- Document comparison and diff
- Collaborative editing support
- Export to various formats (PDF, HTML, etc.)
