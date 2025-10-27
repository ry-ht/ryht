// Cortex API Types
// Auto-generated from Cortex REST API schema

// =============================================================================
// Common Types
// =============================================================================

export interface ApiResponse<T> {
  success: boolean;
  data: T;
  request_id: string;
  duration_ms: number;
  error?: {
    code: string;
    message: string;
    details?: Record<string, unknown>;
  };
}

export interface PaginationInfo {
  page: number;
  limit: number;
  offset: number;
  total: number;
  total_pages: number;
}

// =============================================================================
// Workspace Types
// =============================================================================

export interface Workspace {
  id: string;
  name: string;
  path: string;
  root_directory: string;
  description?: string;
  language?: string;
  created_at: string;
  updated_at: string;
  metadata?: Record<string, unknown>;
}

export interface WorkspaceStats {
  workspace_id: string;
  file_count: number;
  total_size: number;
  code_units_count: number;
  last_indexed: string;
}

export interface CreateWorkspaceRequest {
  name: string;
  path: string;
  description?: string;
  language?: string;
  metadata?: Record<string, unknown>;
}

export interface UpdateWorkspaceRequest {
  name?: string;
  description?: string;
  language?: string;
  metadata?: Record<string, unknown>;
}

// =============================================================================
// Document Types
// =============================================================================

export type DocumentType =
  | 'Guide'
  | 'ApiReference'
  | 'Architecture'
  | 'Tutorial'
  | 'Explanation'
  | 'Troubleshooting'
  | 'Faq'
  | 'ReleaseNotes'
  | 'Example'
  | 'General';

export type DocumentStatus = 'Draft' | 'Review' | 'Published' | 'Archived' | 'Deprecated';

export interface Document {
  id: string;
  title: string;
  content: string;
  slug: string;
  doc_type: DocumentType;
  status: DocumentStatus;
  description?: string;
  parent_id?: string;
  tags: string[];
  keywords: string[];
  author?: string;
  language: string;
  workspace_id?: string;
  version: string;
  created_at: string;
  updated_at: string;
  published_at?: string;
  metadata: Record<string, unknown>;
}

export interface DocumentSection {
  id: string;
  document_id: string;
  title: string;
  content: string;
  level: number;
  order: number;
  parent_section_id?: string;
  created_at: string;
  updated_at: string;
}

export type DocumentLinkType =
  | 'Reference'
  | 'Related'
  | 'Prerequisite'
  | 'Next'
  | 'Previous'
  | 'Parent'
  | 'Child'
  | 'External'
  | 'ApiReference'
  | 'Example';

export type LinkTargetType = 'Document' | 'CodeUnit' | 'ExternalUrl';

export interface DocumentLink {
  id: string;
  source_document_id: string;
  link_type: DocumentLinkType;
  target_type: LinkTargetType;
  target_id: string;
  target_title?: string;
  target_url?: string;
  description?: string;
  created_at: string;
  updated_at: string;
}

export interface DocumentVersion {
  id: string;
  document_id: string;
  version: string;
  author: string;
  message: string;
  content_snapshot?: string;
  metadata?: Record<string, unknown>;
  created_at: string;
}

export interface DocumentStats {
  document_id: string;
  content_length: number;
  word_count: number;
  line_count: number;
  sections_count: number;
  links_count: number;
  versions_count: number;
  tags_count: number;
  keywords_count: number;
}

export interface DocumentTree {
  document: Document;
  children: Document[];
  sections_count: number;
  links_count: number;
}

export interface CreateDocumentRequest {
  title: string;
  content: string;
  doc_type?: string;
  description?: string;
  parent_id?: string;
  tags?: string[];
  keywords?: string[];
  author?: string;
  language?: string;
  workspace_id?: string;
  metadata?: Record<string, unknown>;
}

export interface UpdateDocumentRequest {
  title?: string;
  content?: string;
  description?: string;
  doc_type?: string;
  tags?: string[];
  keywords?: string[];
  metadata?: Record<string, unknown>;
}

// =============================================================================
// VFS (Virtual File System) Types
// =============================================================================

export type FileType = 'file' | 'directory' | 'symlink';

export interface VfsEntry {
  id: string;
  workspace_id: string;
  path: string;
  name: string;
  file_type: FileType;
  size: number;
  mime_type?: string;
  hash?: string;
  created_at: string;
  updated_at: string;
  metadata?: Record<string, unknown>;
}

export interface DirectoryListing {
  path: string;
  entries: VfsEntry[];
  total_count: number;
}

// =============================================================================
// Memory/Cognitive Types
// =============================================================================

export interface MemoryQuery {
  query: string;
  limit?: number;
  filters?: Record<string, unknown>;
}

export interface MemorySearchResult {
  id: string;
  content: string;
  score: number;
  metadata?: Record<string, unknown>;
  created_at: string;
}

export interface CodeUnit {
  id: string;
  workspace_id: string;
  unit_type: string;
  name: string;
  qualified_name: string;
  file_path: string;
  start_line: number;
  end_line: number;
  signature?: string;
  docstring?: string;
  created_at: string;
  updated_at: string;
}

// =============================================================================
// Search Types
// =============================================================================

export interface SearchRequest {
  query: string;
  filters?: {
    workspace_id?: string;
    file_type?: string;
    tags?: string[];
  };
  limit?: number;
}

export interface SearchResult {
  id: string;
  type: 'document' | 'code_unit' | 'file';
  title: string;
  content: string;
  score: number;
  metadata?: Record<string, unknown>;
}

// =============================================================================
// Task Types
// =============================================================================

export type TaskStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';

export interface Task {
  id: string;
  task_type: string;
  status: TaskStatus;
  progress: number;
  result?: unknown;
  error?: string;
  created_at: string;
  updated_at: string;
  completed_at?: string;
}

// =============================================================================
// Health & System Types
// =============================================================================

export interface HealthResponse {
  status: 'healthy' | 'degraded' | 'unhealthy';
  version: string;
  uptime: number;
  timestamp: string;
  services: {
    database: 'healthy' | 'unhealthy';
    vfs: 'healthy' | 'unhealthy';
    mcp: 'healthy' | 'unhealthy';
  };
}

export interface SystemStats {
  workspaces_count: number;
  documents_count: number;
  code_units_count: number;
  files_count: number;
  total_storage: number;
}

// =============================================================================
// Memory Episodes & Patterns Types
// =============================================================================

export type EpisodeType = 'Task' | 'Pattern' | 'Decision' | 'Error' | 'Success' | 'General';

export interface MemoryEpisode {
  id: string;
  episode_type: EpisodeType;
  content: string;
  importance: number; // 0-1 score
  context?: Record<string, unknown>;
  metadata?: Record<string, unknown>;
  created_at: string;
  updated_at: string;
  patterns?: string[]; // Related pattern IDs
}

export interface LearnedPattern {
  id: string;
  name: string;
  description: string;
  pattern_type: string;
  occurrences: number;
  confidence: number; // 0-1 score
  examples?: Array<{
    code?: string;
    language?: string;
    description?: string;
  }>;
  metadata?: Record<string, unknown>;
  created_at: string;
  last_seen: string;
}

export interface ConsolidationResult {
  id: string;
  episodes_processed: number;
  patterns_extracted: number;
  memories_decayed: number;
  duplicates_merged: number;
  duration_ms: number;
  started_at: string;
  completed_at: string;
  status: 'completed' | 'failed' | 'in_progress';
  error?: string;
}
