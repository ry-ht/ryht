// ----------------------------------------------------------------------
// Axon API Types
// ----------------------------------------------------------------------

export type AgentType =
  | 'Orchestrator'
  | 'Developer'
  | 'Reviewer'
  | 'Tester'
  | 'Documenter'
  | 'Architect'
  | 'Researcher'
  | 'Optimizer';

export type AgentStatus = 'Idle' | 'Working' | 'Paused' | 'Failed' | 'ShuttingDown';

export type WorkflowStatus = 'Pending' | 'Running' | 'Completed' | 'Failed' | 'Cancelled';

export type Capability =
  | 'CodeGeneration'
  | 'CodeReview'
  | 'Testing'
  | 'Documentation'
  | 'Debugging'
  | 'CodeAnalysis';

// ----------------------------------------------------------------------
// Health & Status
// ----------------------------------------------------------------------

export interface HealthResponse {
  status: string;
  version: string;
  uptime_seconds: number;
  active_agents: number;
  running_workflows: number;
  websocket_connections: number;
}

export interface SystemStatus {
  active_agents: number;
  running_workflows: number;
  total_tasks_executed: number;
  system_uptime_seconds: number;
  memory_usage_mb: number;
  cpu_usage_percent: number;
}

// ----------------------------------------------------------------------
// Agent Types
// ----------------------------------------------------------------------

export interface AgentMetadata {
  max_concurrent_tasks: number;
  task_timeout_seconds: number;
  tasks_completed: number;
  tasks_failed: number;
  total_execution_time_ms: number;
  avg_task_duration_ms: number;
  created_at: string;
  last_active_at: string;
}

export interface AgentInfo {
  id: string;
  name: string;
  agent_type: AgentType;
  status: AgentStatus;
  capabilities: Capability[];
  metadata: AgentMetadata;
  current_task: string | null;
}

export interface CreateAgentRequest {
  name: string;
  agent_type: AgentType;
  capabilities: string[];
  max_concurrent_tasks?: number;
}

export interface CreateAgentResponse {
  id: string;
  name: string;
}

export interface AgentLogsResponse {
  logs: string[];
}

// ----------------------------------------------------------------------
// Workflow Types
// ----------------------------------------------------------------------

export interface WorkflowTask {
  id: string;
  name: string;
  agent_type: AgentType;
  status: WorkflowStatus;
  dependencies: string[];
  started_at?: string;
  completed_at?: string;
  result?: any;
  error?: string;
}

export interface WorkflowInfo {
  id: string;
  name: string;
  status: WorkflowStatus;
  total_tasks: number;
  completed_tasks: number;
  failed_tasks: number;
  created_at: string;
  started_at?: string;
  completed_at?: string;
}

export interface WorkflowStatusDetail {
  id: string;
  name: string;
  status: WorkflowStatus;
  tasks: WorkflowTask[];
  created_at: string;
  started_at?: string;
  completed_at?: string;
  error?: string;
}

export interface RunWorkflowRequest {
  workflow_def: string;
  input_params: Record<string, any>;
}

export interface RunWorkflowResponse {
  workflow_id: string;
}

// ----------------------------------------------------------------------
// Metrics Types
// ----------------------------------------------------------------------

export interface MetricsData {
  agent_id: string;
  agent_name: string;
  tasks_completed: number;
  tasks_failed: number;
  total_execution_time_ms: number;
  avg_task_duration_ms: number;
  success_rate: number;
  last_updated: string;
}

export interface TelemetryData {
  total_requests: number;
  successful_requests: number;
  failed_requests: number;
  avg_response_time_ms: number;
  error_rate: number;
  timestamp: string;
}

export interface TelemetrySummary {
  total_requests: number;
  success_rate: number;
  avg_response_time_ms: number;
  error_count: number;
}

// ----------------------------------------------------------------------
// WebSocket Types
// ----------------------------------------------------------------------

export type WebSocketEventType =
  | 'agent_started'
  | 'agent_stopped'
  | 'agent_paused'
  | 'agent_resumed'
  | 'agent_status_changed'
  | 'workflow_started'
  | 'workflow_completed'
  | 'workflow_failed'
  | 'workflow_cancelled'
  | 'task_started'
  | 'task_completed'
  | 'task_failed'
  | 'metrics_updated'
  | 'system_status_updated';

export interface WebSocketEvent {
  type: WebSocketEventType;
  timestamp: string;
  data: any;
}

export interface AgentEvent extends WebSocketEvent {
  type:
    | 'agent_started'
    | 'agent_stopped'
    | 'agent_paused'
    | 'agent_resumed'
    | 'agent_status_changed';
  data: {
    agent_id: string;
    agent_name: string;
    status: AgentStatus;
  };
}

export interface WorkflowEvent extends WebSocketEvent {
  type: 'workflow_started' | 'workflow_completed' | 'workflow_failed' | 'workflow_cancelled';
  data: {
    workflow_id: string;
    workflow_name: string;
    status: WorkflowStatus;
  };
}

export interface TaskEvent extends WebSocketEvent {
  type: 'task_started' | 'task_completed' | 'task_failed';
  data: {
    task_id: string;
    workflow_id: string;
    agent_id: string;
    status: WorkflowStatus;
  };
}

// ----------------------------------------------------------------------
// Configuration Types
// ----------------------------------------------------------------------

export interface ConfigResponse {
  workspace_name: string;
  workspace_path: string;
}

export interface UpdateConfigRequest {
  workspace_name?: string;
}

export interface ValidateConfigResponse {
  valid: boolean;
  errors: string[];
}
