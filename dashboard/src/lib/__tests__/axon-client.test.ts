import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import axios from 'axios';

// ----------------------------------------------------------------------
// Mock axios before importing AxonClient
// ----------------------------------------------------------------------

vi.mock('axios');

// Mock environment variables
const mockEnv = {
  VITE_AXON_API_URL: 'http://localhost:9090/api/v1',
  VITE_AXON_API_KEY: 'test-api-key',
};

vi.stubGlobal('import.meta', {
  env: mockEnv,
});

// Now import after mocking
const { AxonClient } = await import('src/lib/axon-client');

// ----------------------------------------------------------------------

describe('AxonClient', () => {
  let client: any;
  let mockAxiosInstance: any;

  beforeEach(() => {
    mockAxiosInstance = {
      get: vi.fn(),
      post: vi.fn(),
      put: vi.fn(),
      delete: vi.fn(),
      interceptors: {
        request: { use: vi.fn() },
        response: { use: vi.fn() },
      },
    };

    vi.mocked(axios.create).mockReturnValue(mockAxiosInstance as any);

    // Create new client instance
    client = new (AxonClient as any)();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  // ----------------------------------------------------------------------
  // Initialization
  // ----------------------------------------------------------------------

  describe('Initialization', () => {
    it('should create axios instance with correct config', () => {
      expect(axios.create).toHaveBeenCalledWith({
        baseURL: 'http://localhost:9090/api/v1',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': 'Bearer test-api-key',
        },
      });
    });

    it('should set up response interceptor', () => {
      expect(mockAxiosInstance.interceptors.response.use).toHaveBeenCalled();
    });
  });

  // ----------------------------------------------------------------------
  // Health & Status
  // ----------------------------------------------------------------------

  describe('Health & Status', () => {
    it('should get health status', async () => {
      const mockHealth = {
        status: 'healthy',
        version: '1.0.0',
        uptime_seconds: 3600,
        active_agents: 5,
        running_workflows: 2,
        websocket_connections: 3,
      };

      mockAxiosInstance.get.mockResolvedValueOnce({ data: mockHealth });

      const result = await client.getHealth();

      expect(mockAxiosInstance.get).toHaveBeenCalledWith('/health');
      expect(result).toEqual(mockHealth);
    });

    it('should get system status', async () => {
      const mockStatus = {
        active_agents: 5,
        running_workflows: 2,
        total_tasks_executed: 100,
        system_uptime_seconds: 7200,
        memory_usage_mb: 512,
        cpu_usage_percent: 45.5,
      };

      mockAxiosInstance.get.mockResolvedValueOnce({ data: mockStatus });

      const result = await client.getSystemStatus();

      expect(mockAxiosInstance.get).toHaveBeenCalledWith('/status');
      expect(result).toEqual(mockStatus);
    });
  });

  // ----------------------------------------------------------------------
  // Agent Management
  // ----------------------------------------------------------------------

  describe('Agent Management', () => {
    it('should list all agents', async () => {
      const mockAgents = [
        { id: 'agent-1', name: 'Agent 1', status: 'Idle' },
        { id: 'agent-2', name: 'Agent 2', status: 'Working' },
      ];

      mockAxiosInstance.get.mockResolvedValueOnce({ data: mockAgents });

      const result = await client.listAgents();

      expect(mockAxiosInstance.get).toHaveBeenCalledWith('/agents');
      expect(result).toEqual(mockAgents);
    });

    it('should get agent by id', async () => {
      const mockAgent = {
        id: 'agent-123',
        name: 'Test Agent',
        agent_type: 'Developer',
        status: 'Idle',
      };

      mockAxiosInstance.get.mockResolvedValueOnce({ data: mockAgent });

      const result = await client.getAgent('agent-123');

      expect(mockAxiosInstance.get).toHaveBeenCalledWith('/agents/agent-123');
      expect(result).toEqual(mockAgent);
    });

    it('should create agent with required fields', async () => {
      const newAgent = {
        name: 'New Agent',
        agent_type: 'Developer',
        capabilities: ['CodeGeneration', 'CodeReview'],
      };

      const mockResponse = { id: 'agent-new', name: 'New Agent' };

      mockAxiosInstance.post.mockResolvedValueOnce({ data: mockResponse });

      const result = await client.createAgent(newAgent);

      expect(mockAxiosInstance.post).toHaveBeenCalledWith('/agents', newAgent);
      expect(result).toEqual(mockResponse);
    });

    it('should create agent with optional max_concurrent_tasks', async () => {
      const newAgent = {
        name: 'New Agent',
        agent_type: 'Tester',
        capabilities: ['Testing'],
        max_concurrent_tasks: 10,
      };

      const mockResponse = { id: 'agent-new', name: 'New Agent' };

      mockAxiosInstance.post.mockResolvedValueOnce({ data: mockResponse });

      const result = await client.createAgent(newAgent);

      expect(mockAxiosInstance.post).toHaveBeenCalledWith('/agents', newAgent);
      expect(result).toEqual(mockResponse);
    });

    it('should delete agent', async () => {
      mockAxiosInstance.delete.mockResolvedValueOnce({});

      await client.deleteAgent('agent-123');

      expect(mockAxiosInstance.delete).toHaveBeenCalledWith('/agents/agent-123');
    });

    it('should pause agent', async () => {
      mockAxiosInstance.post.mockResolvedValueOnce({});

      await client.pauseAgent('agent-123');

      expect(mockAxiosInstance.post).toHaveBeenCalledWith('/agents/agent-123/pause');
    });

    it('should resume agent', async () => {
      mockAxiosInstance.post.mockResolvedValueOnce({});

      await client.resumeAgent('agent-123');

      expect(mockAxiosInstance.post).toHaveBeenCalledWith('/agents/agent-123/resume');
    });

    it('should restart agent', async () => {
      mockAxiosInstance.post.mockResolvedValueOnce({});

      await client.restartAgent('agent-123');

      expect(mockAxiosInstance.post).toHaveBeenCalledWith('/agents/agent-123/restart');
    });

    it('should get agent logs with default lines', async () => {
      const mockLogs = { logs: ['log line 1', 'log line 2'] };

      mockAxiosInstance.get.mockResolvedValueOnce({ data: mockLogs });

      const result = await client.getAgentLogs('agent-123');

      expect(mockAxiosInstance.get).toHaveBeenCalledWith('/agents/agent-123/logs', {
        params: { lines: 100 },
      });
      expect(result).toEqual(mockLogs);
    });

    it('should get agent logs with custom lines', async () => {
      const mockLogs = { logs: ['log line 1'] };

      mockAxiosInstance.get.mockResolvedValueOnce({ data: mockLogs });

      const result = await client.getAgentLogs('agent-123', 50);

      expect(mockAxiosInstance.get).toHaveBeenCalledWith('/agents/agent-123/logs', {
        params: { lines: 50 },
      });
      expect(result).toEqual(mockLogs);
    });
  });

  // ----------------------------------------------------------------------
  // Workflow Management
  // ----------------------------------------------------------------------

  describe('Workflow Management', () => {
    it('should list all workflows', async () => {
      const mockWorkflows = [
        { id: 'wf-1', name: 'Workflow 1', status: 'Running' },
        { id: 'wf-2', name: 'Workflow 2', status: 'Completed' },
      ];

      mockAxiosInstance.get.mockResolvedValueOnce({ data: mockWorkflows });

      const result = await client.listWorkflows();

      expect(mockAxiosInstance.get).toHaveBeenCalledWith('/workflows');
      expect(result).toEqual(mockWorkflows);
    });

    it('should get workflow by id', async () => {
      const mockWorkflow = {
        id: 'wf-123',
        name: 'Test Workflow',
        status: 'Running',
      };

      mockAxiosInstance.get.mockResolvedValueOnce({ data: mockWorkflow });

      const result = await client.getWorkflow('wf-123');

      expect(mockAxiosInstance.get).toHaveBeenCalledWith('/workflows/wf-123');
      expect(result).toEqual(mockWorkflow);
    });

    it('should run workflow', async () => {
      const workflowData = {
        workflow_def: 'test-workflow',
        input_params: { key: 'value' },
      };

      const mockResponse = { workflow_id: 'wf-new' };

      mockAxiosInstance.post.mockResolvedValueOnce({ data: mockResponse });

      const result = await client.runWorkflow(workflowData);

      expect(mockAxiosInstance.post).toHaveBeenCalledWith('/workflows', workflowData);
      expect(result).toEqual(mockResponse);
    });

    it('should cancel workflow', async () => {
      mockAxiosInstance.post.mockResolvedValueOnce({});

      await client.cancelWorkflow('wf-123');

      expect(mockAxiosInstance.post).toHaveBeenCalledWith('/workflows/wf-123/cancel');
    });

    it('should pause workflow', async () => {
      mockAxiosInstance.post.mockResolvedValueOnce({});

      await client.pauseWorkflow('wf-123');

      expect(mockAxiosInstance.post).toHaveBeenCalledWith('/workflows/wf-123/pause');
    });

    it('should resume workflow', async () => {
      mockAxiosInstance.post.mockResolvedValueOnce({});

      await client.resumeWorkflow('wf-123');

      expect(mockAxiosInstance.post).toHaveBeenCalledWith('/workflows/wf-123/resume');
    });
  });

  // ----------------------------------------------------------------------
  // Metrics & Telemetry
  // ----------------------------------------------------------------------

  describe('Metrics & Telemetry', () => {
    it('should get metrics', async () => {
      const mockMetrics = [
        { agent_id: 'agent-1', tasks_completed: 10, success_rate: 0.95 },
      ];

      mockAxiosInstance.get.mockResolvedValueOnce({ data: mockMetrics });

      const result = await client.getMetrics();

      expect(mockAxiosInstance.get).toHaveBeenCalledWith('/metrics');
      expect(result).toEqual(mockMetrics);
    });

    it('should get telemetry with default range', async () => {
      const mockTelemetry = { total_requests: 100, success_rate: 0.98 };

      mockAxiosInstance.get.mockResolvedValueOnce({ data: mockTelemetry });

      const result = await client.getTelemetry();

      expect(mockAxiosInstance.get).toHaveBeenCalledWith('/telemetry', {
        params: { range: 60 },
      });
      expect(result).toEqual(mockTelemetry);
    });

    it('should get telemetry with custom range', async () => {
      const mockTelemetry = { total_requests: 500, success_rate: 0.97 };

      mockAxiosInstance.get.mockResolvedValueOnce({ data: mockTelemetry });

      const result = await client.getTelemetry(120);

      expect(mockAxiosInstance.get).toHaveBeenCalledWith('/telemetry', {
        params: { range: 120 },
      });
      expect(result).toEqual(mockTelemetry);
    });

    it('should get telemetry summary', async () => {
      const mockSummary = {
        total_requests: 1000,
        success_rate: 0.96,
        avg_response_time_ms: 150,
        error_count: 40,
      };

      mockAxiosInstance.get.mockResolvedValueOnce({ data: mockSummary });

      const result = await client.getTelemetrySummary();

      expect(mockAxiosInstance.get).toHaveBeenCalledWith('/telemetry/summary');
      expect(result).toEqual(mockSummary);
    });

    it('should export metrics with default format', async () => {
      const mockResponse = { status: 'exported', file: 'metrics.json' };

      mockAxiosInstance.post.mockResolvedValueOnce({ data: mockResponse });

      const result = await client.exportMetrics();

      expect(mockAxiosInstance.post).toHaveBeenCalledWith('/metrics/export', {
        format: 'json',
        output_path: undefined,
      });
      expect(result).toEqual(mockResponse);
    });

    it('should export metrics with custom format and path', async () => {
      const mockResponse = { status: 'exported', file: 'metrics.csv' };

      mockAxiosInstance.post.mockResolvedValueOnce({ data: mockResponse });

      const result = await client.exportMetrics('csv', '/custom/path');

      expect(mockAxiosInstance.post).toHaveBeenCalledWith('/metrics/export', {
        format: 'csv',
        output_path: '/custom/path',
      });
      expect(result).toEqual(mockResponse);
    });
  });

  // ----------------------------------------------------------------------
  // Configuration
  // ----------------------------------------------------------------------

  describe('Configuration', () => {
    it('should get config', async () => {
      const mockConfig = {
        workspace_name: 'test-workspace',
        workspace_path: '/path/to/workspace',
      };

      mockAxiosInstance.get.mockResolvedValueOnce({ data: mockConfig });

      const result = await client.getConfig();

      expect(mockAxiosInstance.get).toHaveBeenCalledWith('/config');
      expect(result).toEqual(mockConfig);
    });

    it('should update config', async () => {
      const configData = { workspace_name: 'new-workspace' };

      mockAxiosInstance.put.mockResolvedValueOnce({});

      await client.updateConfig(configData);

      expect(mockAxiosInstance.put).toHaveBeenCalledWith('/config', configData);
    });

    it('should validate config', async () => {
      const config = { workspace_name: 'test' };
      const mockResponse = { valid: true, errors: [] };

      mockAxiosInstance.post.mockResolvedValueOnce({ data: mockResponse });

      const result = await client.validateConfig(config);

      expect(mockAxiosInstance.post).toHaveBeenCalledWith('/config/validate', { config });
      expect(result).toEqual(mockResponse);
    });
  });

  // ----------------------------------------------------------------------
  // Error Handling
  // ----------------------------------------------------------------------

  describe('Error Handling', () => {
    it('should handle network errors', async () => {
      const mockError = new Error('Network error');
      mockAxiosInstance.get.mockRejectedValueOnce(mockError);

      await expect(client.getHealth()).rejects.toThrow('Network error');
    });

    it('should handle API error responses', async () => {
      const mockError = {
        response: {
          data: { message: 'Agent not found' },
          status: 404,
        },
        message: 'Request failed with status code 404',
      };

      mockAxiosInstance.get.mockRejectedValueOnce(mockError);

      await expect(client.getAgent('invalid-id')).rejects.toBeDefined();
    });

    it('should handle timeout errors', async () => {
      const mockError = new Error('Timeout exceeded');
      mockAxiosInstance.post.mockRejectedValueOnce(mockError);

      await expect(client.createAgent({
        name: 'Test',
        agent_type: 'Developer',
        capabilities: []
      })).rejects.toThrow('Timeout exceeded');
    });
  });
});
