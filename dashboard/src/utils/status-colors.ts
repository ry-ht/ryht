/**
 * Status color utility functions
 *
 * Provides centralized color mappings for agent and workflow statuses
 * to maintain consistency across the application.
 */

type AgentStatus = 'Idle' | 'Working' | 'Paused' | 'Failed' | 'ShuttingDown';
type WorkflowStatus = 'Pending' | 'Running' | 'Completed' | 'Failed' | 'Cancelled';
type TaskStatus = 'Pending' | 'InProgress' | 'Blocked' | 'Done' | 'Cancelled';
type TaskPriority = 'Critical' | 'High' | 'Medium' | 'Low';
type DocumentStatus = 'Draft' | 'Review' | 'Published' | 'Archived' | 'Deprecated';
type CortexTaskStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';

type StatusColor = 'default' | 'primary' | 'secondary' | 'info' | 'success' | 'warning' | 'error';

/**
 * Get the appropriate color for an agent status
 * @param status - The agent status
 * @returns The color variant for the Label component
 */
export function getAgentStatusColor(status: string): StatusColor {
  const statusColors: Record<AgentStatus, StatusColor> = {
    Idle: 'success',
    Working: 'info',
    Paused: 'warning',
    Failed: 'error',
    ShuttingDown: 'default',
  };

  return statusColors[status as AgentStatus] || 'default';
}

/**
 * Get the appropriate color for a workflow status
 * @param status - The workflow status
 * @returns The color variant for the Label component
 */
export function getWorkflowStatusColor(status: string): StatusColor {
  const statusColors: Record<WorkflowStatus, StatusColor> = {
    Pending: 'default',
    Running: 'info',
    Completed: 'success',
    Failed: 'error',
    Cancelled: 'warning',
  };

  return statusColors[status as WorkflowStatus] || 'default';
}

/**
 * Get the appropriate color for a task status
 * @param status - The task status
 * @returns The color variant for the Label component
 */
export function getTaskStatusColor(status: string): StatusColor {
  const statusColors: Record<TaskStatus, StatusColor> = {
    Pending: 'default',
    InProgress: 'info',
    Blocked: 'warning',
    Done: 'success',
    Cancelled: 'error',
  };

  return statusColors[status as TaskStatus] || 'default';
}

/**
 * Get the appropriate color for a task priority
 * @param priority - The task priority
 * @returns The color variant for the Label component
 */
export function getTaskPriorityColor(priority: string): StatusColor {
  const priorityColors: Record<TaskPriority, StatusColor> = {
    Critical: 'error',
    High: 'warning',
    Medium: 'info',
    Low: 'default',
  };

  return priorityColors[priority as TaskPriority] || 'default';
}

/**
 * Get the appropriate color for a document status
 * @param status - The document status
 * @returns The color variant for the Label component
 */
export function getDocumentStatusColor(status: string): StatusColor {
  const statusColors: Record<DocumentStatus, StatusColor> = {
    Draft: 'default',
    Review: 'warning',
    Published: 'success',
    Archived: 'secondary',
    Deprecated: 'error',
  };

  return statusColors[status as DocumentStatus] || 'default';
}

/**
 * Get the appropriate color for a Cortex task status
 * @param status - The Cortex task status
 * @returns The color variant for the Label component
 */
export function getCortexTaskStatusColor(status: string): StatusColor {
  const statusColors: Record<CortexTaskStatus, StatusColor> = {
    pending: 'default',
    running: 'info',
    completed: 'success',
    failed: 'error',
    cancelled: 'warning',
  };

  return statusColors[status as CortexTaskStatus] || 'default';
}
