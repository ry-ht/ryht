import type { ReactElement } from 'react';
import type { RenderOptions } from '@testing-library/react';

import { BrowserRouter } from 'react-router';
import { render } from '@testing-library/react';

// ----------------------------------------------------------------------
// Custom render function with providers
// ----------------------------------------------------------------------

interface AllTheProvidersProps {
  children: React.ReactNode;
}

function AllTheProviders({ children }: AllTheProvidersProps) {
  return <BrowserRouter>{children}</BrowserRouter>;
}

const customRender = (ui: ReactElement, options?: Omit<RenderOptions, 'wrapper'>) =>
  render(ui, { wrapper: AllTheProviders, ...options });

export * from '@testing-library/react';
export { customRender as render };

// ----------------------------------------------------------------------
// Mock data generators
// ----------------------------------------------------------------------

export const mockAgentInfo = (overrides?: Partial<any>) => ({
  id: 'agent-123',
  name: 'Test Agent',
  agent_type: 'Developer',
  status: 'Idle',
  capabilities: ['CodeGeneration', 'CodeReview'],
  metadata: {
    max_concurrent_tasks: 5,
    task_timeout_seconds: 300,
    tasks_completed: 10,
    tasks_failed: 2,
    total_execution_time_ms: 5000,
    avg_task_duration_ms: 500,
    created_at: '2024-01-01T00:00:00Z',
    last_active_at: '2024-01-02T00:00:00Z',
  },
  current_task: null,
  ...overrides,
});

export const mockWorkflowInfo = (overrides?: Partial<any>) => ({
  id: 'workflow-123',
  name: 'Test Workflow',
  status: 'Running',
  total_tasks: 5,
  completed_tasks: 2,
  failed_tasks: 0,
  created_at: '2024-01-01T00:00:00Z',
  started_at: '2024-01-01T00:01:00Z',
  ...overrides,
});

export const mockWebSocketEvent = (overrides?: Partial<any>) => ({
  type: 'agent_status_changed',
  timestamp: '2024-01-01T00:00:00Z',
  data: {
    agent_id: 'agent-123',
    agent_name: 'Test Agent',
    status: 'Working',
  },
  ...overrides,
});

// ----------------------------------------------------------------------
// Wait utilities
// ----------------------------------------------------------------------

export const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

// ----------------------------------------------------------------------
// Mock axios responses
// ----------------------------------------------------------------------

export const mockAxiosSuccess = (data: any) => ({
  data,
  status: 200,
  statusText: 'OK',
  headers: {},
  config: {} as any,
});

export const mockAxiosError = (message: string, status = 400) => ({
  response: {
    data: { message },
    status,
    statusText: 'Error',
    headers: {},
    config: {} as any,
  },
  message,
  config: {} as any,
  isAxiosError: true,
  toJSON: () => ({}),
});
