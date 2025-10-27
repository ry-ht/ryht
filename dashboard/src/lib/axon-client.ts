import axios, { type AxiosInstance } from 'axios';

// ----------------------------------------------------------------------

const AXON_API_URL = import.meta.env.VITE_AXON_API_URL;
const AXON_API_KEY = import.meta.env.VITE_AXON_API_KEY;

// Validate required environment variables
if (!AXON_API_URL) {
  throw new Error('VITE_AXON_API_URL environment variable is required');
}

// API key is optional
// if (!AXON_API_KEY) {
//   console.warn('VITE_AXON_API_KEY not set - API calls may be restricted');
// }

// ----------------------------------------------------------------------

/**
 * Axon API Client
 * Handles all communication with the Axon multi-agent system
 */
class AxonClient {
  private client: AxiosInstance;

  constructor() {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };

    // Add Authorization header only if API key is provided
    if (AXON_API_KEY) {
      headers['Authorization'] = `Bearer ${AXON_API_KEY}`;
    }

    this.client = axios.create({
      baseURL: AXON_API_URL,
      headers,
    });

    // Response interceptor
    this.client.interceptors.response.use(
      (response) => response,
      (error) => {
        const message = error?.response?.data?.message || error?.message || 'API request failed';
        console.error('Axon API error:', message);
        return Promise.reject(new Error(message));
      }
    );
  }

  // ----------------------------------------------------------------------
  // Health & Status
  // ----------------------------------------------------------------------

  async getHealth() {
    const response = await this.client.get('/health');
    return response.data;
  }

  async getSystemStatus() {
    const response = await this.client.get('/status');
    return response.data;
  }

  // ----------------------------------------------------------------------
  // Agent Management
  // ----------------------------------------------------------------------

  async listAgents() {
    const response = await this.client.get('/agents');
    return response.data;
  }

  async getAgent(id: string) {
    const response = await this.client.get(`/agents/${id}`);
    return response.data;
  }

  async createAgent(data: {
    name: string;
    agent_type: string;
    capabilities: string[];
    max_concurrent_tasks?: number;
  }) {
    const response = await this.client.post('/agents', data);
    return response.data;
  }

  async deleteAgent(id: string) {
    await this.client.delete(`/agents/${id}`);
  }

  async pauseAgent(id: string) {
    await this.client.post(`/agents/${id}/pause`);
  }

  async resumeAgent(id: string) {
    await this.client.post(`/agents/${id}/resume`);
  }

  async restartAgent(id: string) {
    await this.client.post(`/agents/${id}/restart`);
  }

  async getAgentLogs(id: string, lines = 100) {
    const response = await this.client.get(`/agents/${id}/logs`, {
      params: { lines },
    });
    return response.data;
  }

  async getAgentMetrics(id: string) {
    const response = await this.client.get(`/agents/${id}/metrics`);
    return response.data;
  }

  async getAgentMetricsTimeSeries(id: string, range = 3600) {
    const response = await this.client.get(`/agents/${id}/metrics/timeseries`, {
      params: { range },
    });
    return response.data;
  }

  async bulkAgentAction(action: 'pause' | 'resume' | 'restart', agentIds: string[]) {
    const response = await this.client.post(`/agents/bulk/${action}`, {
      agent_ids: agentIds,
    });
    return response.data;
  }

  // ----------------------------------------------------------------------
  // Workflow Management
  // ----------------------------------------------------------------------

  async listWorkflows() {
    const response = await this.client.get('/workflows');
    return response.data;
  }

  async getWorkflow(id: string) {
    const response = await this.client.get(`/workflows/${id}`);
    return response.data;
  }

  async runWorkflow(data: {
    workflow_def: string;
    input_params: Record<string, any>;
  }) {
    const response = await this.client.post('/workflows', data);
    return response.data;
  }

  async cancelWorkflow(id: string) {
    await this.client.post(`/workflows/${id}/cancel`);
  }

  async pauseWorkflow(id: string) {
    await this.client.post(`/workflows/${id}/pause`);
  }

  async resumeWorkflow(id: string) {
    await this.client.post(`/workflows/${id}/resume`);
  }

  // ----------------------------------------------------------------------
  // Task Management
  // ----------------------------------------------------------------------

  async listTasks() {
    const response = await this.client.get('/tasks');
    return response.data;
  }

  async getTask(id: string) {
    const response = await this.client.get(`/tasks/${id}`);
    return response.data;
  }

  async createTask(data: {
    title: string;
    description?: string;
    priority: string;
    assigned_agents?: string[];
    estimated_hours?: number;
    tags?: string[];
    dependencies?: string[];
    spec_reference?: string;
  }) {
    const response = await this.client.post('/tasks', data);
    return response.data;
  }

  async updateTask(id: string, data: {
    title?: string;
    description?: string;
    status?: string;
    priority?: string;
    assigned_agents?: string[];
    progress?: number;
    estimated_hours?: number;
    actual_hours?: number;
    tags?: string[];
    dependencies?: string[];
    spec_reference?: string;
    completion_notes?: string;
  }) {
    const response = await this.client.put(`/tasks/${id}`, data);
    return response.data;
  }

  async deleteTask(id: string) {
    await this.client.delete(`/tasks/${id}`);
  }

  async getTaskActivities(id: string) {
    const response = await this.client.get(`/tasks/${id}/activities`);
    return response.data;
  }

  // ----------------------------------------------------------------------
  // Metrics & Telemetry
  // ----------------------------------------------------------------------

  async getMetrics() {
    const response = await this.client.get('/metrics');
    return response.data;
  }

  async getTelemetry(range = 60) {
    const response = await this.client.get('/telemetry', {
      params: { range },
    });
    return response.data;
  }

  async getTelemetrySummary() {
    const response = await this.client.get('/telemetry/summary');
    return response.data;
  }

  async exportMetrics(format = 'json', output_path?: string) {
    const response = await this.client.post('/metrics/export', {
      format,
      output_path,
    });
    return response.data;
  }

  // ----------------------------------------------------------------------
  // Configuration
  // ----------------------------------------------------------------------

  async getConfig() {
    const response = await this.client.get('/config');
    return response.data;
  }

  async updateConfig(data: { workspace_name?: string }) {
    await this.client.put('/config', data);
  }

  async validateConfig(config: Record<string, any>) {
    const response = await this.client.post('/config/validate', { config });
    return response.data;
  }
}

// ----------------------------------------------------------------------

export const axonClient = new AxonClient();

// ----------------------------------------------------------------------
// Endpoints for SWR
// ----------------------------------------------------------------------

export const axonEndpoints = {
  health: '/health',
  status: '/status',
  agents: {
    list: '/agents',
    details: (id: string) => `/agents/${id}`,
    logs: (id: string) => `/agents/${id}/logs`,
    metrics: (id: string) => `/agents/${id}/metrics`,
    metricsTimeSeries: (id: string) => `/agents/${id}/metrics/timeseries`,
  },
  workflows: {
    list: '/workflows',
    details: (id: string) => `/workflows/${id}`,
  },
  tasks: {
    list: '/tasks',
    details: (id: string) => `/tasks/${id}`,
    activities: (id: string) => `/tasks/${id}/activities`,
  },
  metrics: '/metrics',
  telemetry: '/telemetry',
  telemetrySummary: '/telemetry/summary',
  config: '/config',
} as const;

// ----------------------------------------------------------------------
// Fetchers for SWR
// ----------------------------------------------------------------------

export const axonFetcher = async <T = unknown>(url: string): Promise<T> => {
  const response = await axonClient['client'].get<T>(url);
  return response.data;
};
