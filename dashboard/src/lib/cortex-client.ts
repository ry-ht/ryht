import axios, { type AxiosInstance } from 'axios';

import type {
  ApiResponse,
  CodeUnit,
  CreateDocumentRequest,
  CreateWorkspaceRequest,
  DirectoryListing,
  Document,
  DocumentLink,
  DocumentSection,
  DocumentStats,
  DocumentTree,
  DocumentVersion,
  HealthResponse,
  MemoryQuery,
  MemorySearchResult,
  SearchRequest,
  SearchResult,
  SystemStats,
  Task,
  UpdateDocumentRequest,
  UpdateWorkspaceRequest,
  VfsEntry,
  Workspace,
  WorkspaceStats,
} from 'src/types/cortex';

// ----------------------------------------------------------------------

const CORTEX_API_URL = import.meta.env.VITE_CORTEX_API_URL || 'http://localhost:9090';
const CORTEX_API_KEY = import.meta.env.VITE_CORTEX_API_KEY || '';

console.log('Cortex API URL:', CORTEX_API_URL);

// ----------------------------------------------------------------------

/**
 * Cortex API Client
 * Handles all communication with the Cortex cognitive system
 */
class CortexClient {
  private client: AxiosInstance;

  constructor() {
    this.client = axios.create({
      baseURL: `${CORTEX_API_URL}/api/v1`,
      headers: {
        'Content-Type': 'application/json',
        ...(CORTEX_API_KEY && { Authorization: `Bearer ${CORTEX_API_KEY}` }),
      },
    });

    // Response interceptor
    this.client.interceptors.response.use(
      (response) => {
        // Unwrap ApiResponse<T> to just T
        if (response.data?.success && response.data?.data !== undefined) {
          return { ...response, data: response.data.data };
        }
        return response;
      },
      (error) => {
        const message =
          error?.response?.data?.error?.message ||
          error?.response?.data?.message ||
          error?.message ||
          'API request failed';
        console.error('Cortex API error:', message);
        return Promise.reject(new Error(message));
      }
    );
  }

  // ----------------------------------------------------------------------
  // Health & System
  // ----------------------------------------------------------------------

  async getHealth(): Promise<HealthResponse> {
    const response = await this.client.get('/health');
    return response.data;
  }

  async getSystemStats(): Promise<SystemStats> {
    const response = await this.client.get('/system/stats');
    return response.data;
  }

  // ----------------------------------------------------------------------
  // Workspace Management
  // ----------------------------------------------------------------------

  async listWorkspaces(): Promise<Workspace[]> {
    const response = await this.client.get('/workspaces');
    return response.data;
  }

  async getWorkspace(id: string): Promise<Workspace> {
    const response = await this.client.get(`/workspaces/${id}`);
    return response.data;
  }

  async createWorkspace(data: CreateWorkspaceRequest): Promise<Workspace> {
    const response = await this.client.post('/workspaces', data);
    return response.data;
  }

  async updateWorkspace(id: string, data: UpdateWorkspaceRequest): Promise<Workspace> {
    const response = await this.client.put(`/workspaces/${id}`, data);
    return response.data;
  }

  async deleteWorkspace(id: string): Promise<void> {
    await this.client.delete(`/workspaces/${id}`);
  }

  async getWorkspaceStats(id: string): Promise<WorkspaceStats> {
    const response = await this.client.get(`/workspaces/${id}/stats`);
    return response.data;
  }

  async indexWorkspace(id: string): Promise<Task> {
    const response = await this.client.post(`/workspaces/${id}/index`);
    return response.data;
  }

  // ----------------------------------------------------------------------
  // Document Management
  // ----------------------------------------------------------------------

  async listDocuments(filters?: {
    status?: string;
    doc_type?: string;
    workspace_id?: string;
    limit?: number;
  }): Promise<Document[]> {
    const response = await this.client.get('/documents', { params: filters });
    return response.data;
  }

  async getDocument(id: string): Promise<Document> {
    const response = await this.client.get(`/documents/${id}`);
    return response.data;
  }

  async getDocumentBySlug(slug: string): Promise<Document> {
    const response = await this.client.get(`/documents/slug/${slug}`);
    return response.data;
  }

  async createDocument(data: CreateDocumentRequest): Promise<Document> {
    const response = await this.client.post('/documents', data);
    return response.data;
  }

  async updateDocument(id: string, data: UpdateDocumentRequest): Promise<Document> {
    const response = await this.client.put(`/documents/${id}`, data);
    return response.data;
  }

  async deleteDocument(id: string): Promise<void> {
    await this.client.delete(`/documents/${id}`);
  }

  async publishDocument(id: string): Promise<Document> {
    const response = await this.client.post(`/documents/${id}/publish`);
    return response.data;
  }

  async archiveDocument(id: string): Promise<Document> {
    const response = await this.client.post(`/documents/${id}/archive`);
    return response.data;
  }

  async getDocumentTree(id: string): Promise<DocumentTree> {
    const response = await this.client.get(`/documents/${id}/tree`);
    return response.data;
  }

  async getDocumentStats(id: string): Promise<DocumentStats> {
    const response = await this.client.get(`/documents/${id}/stats`);
    return response.data;
  }

  async searchDocuments(query: string, limit = 20): Promise<Document[]> {
    const response = await this.client.get('/documents/search', {
      params: { q: query, limit },
    });
    return response.data;
  }

  // ----------------------------------------------------------------------
  // Document Sections
  // ----------------------------------------------------------------------

  async listSections(documentId: string): Promise<DocumentSection[]> {
    const response = await this.client.get(`/documents/${documentId}/sections`);
    return response.data;
  }

  async createSection(
    documentId: string,
    data: { title: string; content: string; level: number; order?: number }
  ): Promise<DocumentSection> {
    const response = await this.client.post(`/documents/${documentId}/sections`, data);
    return response.data;
  }

  async updateSection(
    sectionId: string,
    data: { title?: string; content?: string; order?: number }
  ): Promise<DocumentSection> {
    const response = await this.client.put(`/sections/${sectionId}`, data);
    return response.data;
  }

  async deleteSection(sectionId: string): Promise<void> {
    await this.client.delete(`/sections/${sectionId}`);
  }

  // ----------------------------------------------------------------------
  // Document Links
  // ----------------------------------------------------------------------

  async listDocumentLinks(documentId: string): Promise<DocumentLink[]> {
    const response = await this.client.get(`/documents/${documentId}/links`);
    return response.data;
  }

  async createDocumentLink(
    documentId: string,
    data: { link_type: string; target_type: string; target_id: string }
  ): Promise<DocumentLink> {
    const response = await this.client.post(`/documents/${documentId}/links`, data);
    return response.data;
  }

  async deleteDocumentLink(linkId: string): Promise<void> {
    await this.client.delete(`/links/${linkId}`);
  }

  // ----------------------------------------------------------------------
  // Document Versions
  // ----------------------------------------------------------------------

  async listDocumentVersions(documentId: string): Promise<DocumentVersion[]> {
    const response = await this.client.get(`/documents/${documentId}/versions`);
    return response.data;
  }

  async createDocumentVersion(
    documentId: string,
    data: { version: string; author: string; message: string }
  ): Promise<DocumentVersion> {
    const response = await this.client.post(`/documents/${documentId}/versions`, data);
    return response.data;
  }

  // ----------------------------------------------------------------------
  // Virtual File System (VFS)
  // ----------------------------------------------------------------------

  async listFiles(workspaceId: string, path = '/'): Promise<DirectoryListing> {
    const response = await this.client.get(`/vfs/${workspaceId}/list`, {
      params: { path },
    });
    return response.data;
  }

  async getFile(workspaceId: string, path: string): Promise<VfsEntry> {
    const response = await this.client.get(`/vfs/${workspaceId}/file`, {
      params: { path },
    });
    return response.data;
  }

  async readFileContent(workspaceId: string, path: string): Promise<string> {
    const response = await this.client.get(`/vfs/${workspaceId}/read`, {
      params: { path },
    });
    return response.data;
  }

  async writeFile(workspaceId: string, path: string, content: string): Promise<VfsEntry> {
    const response = await this.client.post(`/vfs/${workspaceId}/write`, {
      path,
      content,
    });
    return response.data;
  }

  async deleteFile(workspaceId: string, path: string): Promise<void> {
    await this.client.delete(`/vfs/${workspaceId}/delete`, {
      params: { path },
    });
  }

  // ----------------------------------------------------------------------
  // Memory/Cognitive Search
  // ----------------------------------------------------------------------

  async searchMemory(query: MemoryQuery): Promise<MemorySearchResult[]> {
    const response = await this.client.post('/memory/search', query);
    return response.data;
  }

  async getCodeUnit(id: string): Promise<CodeUnit> {
    const response = await this.client.get(`/code-units/${id}`);
    return response.data;
  }

  async searchCodeUnits(workspaceId: string, query: string): Promise<CodeUnit[]> {
    const response = await this.client.get('/code-units/search', {
      params: { workspace_id: workspaceId, q: query },
    });
    return response.data;
  }

  // ----------------------------------------------------------------------
  // Universal Search
  // ----------------------------------------------------------------------

  async search(request: SearchRequest): Promise<SearchResult[]> {
    const response = await this.client.post('/search', request);
    return response.data;
  }

  // ----------------------------------------------------------------------
  // Tasks
  // ----------------------------------------------------------------------

  async getTask(id: string): Promise<Task> {
    const response = await this.client.get(`/tasks/${id}`);
    return response.data;
  }

  async listTasks(filters?: { status?: string; limit?: number }): Promise<Task[]> {
    const response = await this.client.get('/tasks', { params: filters });
    return response.data;
  }

  // ----------------------------------------------------------------------
  // Memory Episodes & Patterns
  // ----------------------------------------------------------------------

  async listMemoryEpisodes(): Promise<any[]> {
    const response = await this.client.get('/memory/episodes');
    return response.data;
  }

  async getMemoryEpisode(episodeId: string): Promise<any> {
    const response = await this.client.get(`/memory/episodes/${episodeId}`);
    return response.data;
  }

  async consolidateMemory(): Promise<any> {
    const response = await this.client.post('/memory/consolidate');
    return response.data;
  }

  async getLearnedPatterns(): Promise<any[]> {
    const response = await this.client.get('/memory/patterns');
    return response.data;
  }

  // ----------------------------------------------------------------------
  // Dependencies & Analysis
  // ----------------------------------------------------------------------

  async getWorkspaceDependencies(workspaceId: string): Promise<any> {
    const response = await this.client.get(`/workspaces/${workspaceId}/dependencies`);
    return response.data;
  }

  async analyzeImpact(data: { unit_id: string; change_type: string }): Promise<any> {
    const response = await this.client.post('/analysis/impact', data);
    return response.data;
  }

  async detectCycles(): Promise<any> {
    const response = await this.client.get('/analysis/cycles');
    return response.data;
  }

  // ----------------------------------------------------------------------
  // Sessions & Collaboration
  // ----------------------------------------------------------------------

  async listSessions(): Promise<any[]> {
    const response = await this.client.get('/sessions');
    return response.data;
  }

  async createSession(data: { name: string; workspace_id: string }): Promise<any> {
    const response = await this.client.post('/sessions', data);
    return response.data;
  }

  async getSession(sessionId: string): Promise<any> {
    const response = await this.client.get(`/sessions/${sessionId}`);
    return response.data;
  }

  async deleteSession(sessionId: string): Promise<void> {
    await this.client.delete(`/sessions/${sessionId}`);
  }

  async listLocks(): Promise<any[]> {
    const response = await this.client.get('/locks');
    return response.data;
  }
}

// ----------------------------------------------------------------------
// Export singleton instance
// ----------------------------------------------------------------------

export const cortexClient = new CortexClient();

// ----------------------------------------------------------------------
// Export endpoints for SWR usage
// ----------------------------------------------------------------------

export const cortexEndpoints = {
  health: '/api/v1/health',
  systemStats: '/api/v1/system/stats',
  workspaces: {
    list: '/api/v1/workspaces',
    get: (id: string) => `/api/v1/workspaces/${id}`,
    stats: (id: string) => `/api/v1/workspaces/${id}/stats`,
  },
  documents: {
    list: '/api/v1/documents',
    get: (id: string) => `/api/v1/documents/${id}`,
    tree: (id: string) => `/api/v1/documents/${id}/tree`,
    stats: (id: string) => `/api/v1/documents/${id}/stats`,
    sections: (id: string) => `/api/v1/documents/${id}/sections`,
    links: (id: string) => `/api/v1/documents/${id}/links`,
    versions: (id: string) => `/api/v1/documents/${id}/versions`,
  },
  vfs: {
    list: (workspaceId: string, path: string) =>
      `/api/v1/vfs/${workspaceId}/list?path=${encodeURIComponent(path)}`,
  },
  tasks: {
    list: '/api/v1/tasks',
    get: (id: string) => `/api/v1/tasks/${id}`,
  },
};

// ----------------------------------------------------------------------
// SWR Fetcher
// ----------------------------------------------------------------------

export const cortexFetcher = async (url: string) => {
  const fullUrl = url.startsWith('http') ? url : `${CORTEX_API_URL}${url}`;
  const response = await fetch(fullUrl, {
    headers: {
      'Content-Type': 'application/json',
      ...(CORTEX_API_KEY && { Authorization: `Bearer ${CORTEX_API_KEY}` }),
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: response.statusText }));
    throw new Error(error.message || 'API request failed');
  }

  const data = await response.json();

  // Unwrap ApiResponse<T> to just T
  if (data?.success && data?.data !== undefined) {
    return data.data;
  }

  return data;
};
