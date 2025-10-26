/**
 * Status color utility functions
 *
 * Provides centralized color mappings for agent and workflow statuses
 * to maintain consistency across the application.
 */

type AgentStatus = 'Idle' | 'Working' | 'Paused' | 'Failed' | 'ShuttingDown';
type WorkflowStatus = 'Pending' | 'Running' | 'Completed' | 'Failed' | 'Cancelled';

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
