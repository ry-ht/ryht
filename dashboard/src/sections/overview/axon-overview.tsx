import type { AgentInfo, SystemStatus, WorkflowInfo, HealthResponse, SystemMetrics } from 'src/types/axon';

import useSWR from 'swr';
import { useState, useEffect } from 'react';
import { LineChart, Line, AreaChart, Area, BarChart, Bar, PieChart, Pie, Cell, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';

import Box from '@mui/material/Box';
import Grid from '@mui/material/Grid';
import Card from '@mui/material/Card';
import Chip from '@mui/material/Chip';
import Stack from '@mui/material/Stack';
import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import CardHeader from '@mui/material/CardHeader';
import Typography from '@mui/material/Typography';
import LinearProgress from '@mui/material/LinearProgress';
import CardContent from '@mui/material/CardContent';

import { getAgentStatusColor, getWorkflowStatusColor } from 'src/utils/status-colors';

import { axonWebSocket } from 'src/lib/axon-websocket';
import { axonFetcher, axonEndpoints } from 'src/lib/axon-client';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { AnimateCountUp } from 'src/components/animate';

// ----------------------------------------------------------------------

const COLORS = {
  primary: '#1976d2',
  success: '#4caf50',
  warning: '#ff9800',
  error: '#f44336',
  info: '#2196f3',
};

const PIE_COLORS = ['#1976d2', '#4caf50', '#ff9800', '#f44336', '#9c27b0'];

// ----------------------------------------------------------------------

export function AxonOverview() {
  const [wsConnected, setWsConnected] = useState(false);
  const [metricsHistory, setMetricsHistory] = useState<SystemMetrics[]>([]);

  // Fetch health status
  const { data: health } = useSWR<HealthResponse>(
    axonEndpoints.health,
    axonFetcher,
    { refreshInterval: 5000 }
  );

  // Fetch system status
  const { data: systemStatus } = useSWR<SystemStatus>(
    axonEndpoints.status,
    axonFetcher,
    { refreshInterval: 5000 }
  );

  // Fetch agents
  const { data: agents = [] } = useSWR<AgentInfo[]>(
    axonEndpoints.agents.list,
    axonFetcher,
    { refreshInterval: 5000 }
  );

  // Fetch workflows
  const { data: workflows = [] } = useSWR<WorkflowInfo[]>(
    axonEndpoints.workflows.list,
    axonFetcher,
    { refreshInterval: 3000 }
  );

  // WebSocket connection
  useEffect(() => {
    axonWebSocket.connect();

    const interval = setInterval(() => {
      setWsConnected(axonWebSocket.isConnected());
    }, 1000);

    return () => {
      clearInterval(interval);
      axonWebSocket.disconnect();
    };
  }, []);

  // Update metrics history
  useEffect(() => {
    if (systemStatus && agents.length > 0) {
      const idleAgents = agents.filter((a: AgentInfo) => a.status === 'Idle').length;
      const busyAgents = agents.filter((a: AgentInfo) => a.status === 'Working').length;
      const pausedAgents = agents.filter((a: AgentInfo) => a.status === 'Paused').length;
      const failedAgents = agents.filter((a: AgentInfo) => a.status === 'Failed').length;

      const pendingWorkflows = workflows.filter((w: WorkflowInfo) => w.status === 'Pending').length;
      const runningWorkflows = workflows.filter((w: WorkflowInfo) => w.status === 'Running').length;
      const completedWorkflows = workflows.filter((w: WorkflowInfo) => w.status === 'Completed').length;
      const failedWorkflows = workflows.filter((w: WorkflowInfo) => w.status === 'Failed').length;

      const newMetric: SystemMetrics = {
        timestamp: new Date().toISOString(),
        total_agents: agents.length,
        idle_agents: idleAgents,
        busy_agents: busyAgents,
        paused_agents: pausedAgents,
        failed_agents: failedAgents,
        total_workflows: workflows.length,
        pending_workflows: pendingWorkflows,
        running_workflows: runningWorkflows,
        completed_workflows: completedWorkflows,
        failed_workflows: failedWorkflows,
        cpu_usage_percent: systemStatus.cpu_usage_percent || 0,
        memory_usage_mb: systemStatus.memory_usage_mb || 0,
        total_memory_mb: 8192, // Mock value
        request_rate: 0, // Mock value
        error_rate: 0, // Mock value
        avg_latency_ms: 0, // Mock value
      };

      setMetricsHistory((prev) => {
        const updated = [...prev, newMetric];
        // Keep only last 20 data points
        return updated.slice(-20);
      });
    }
  }, [systemStatus, agents, workflows]);

  const runningWorkflows = workflows.filter((w: WorkflowInfo) => w.status === 'Running').length;
  const activeAgents = agents.filter((a: AgentInfo) => a.status === 'Working' || a.status === 'Idle').length;

  // Calculate agent status distribution
  const agentStatusData = [
    { name: 'Idle', value: agents.filter((a) => a.status === 'Idle').length },
    { name: 'Working', value: agents.filter((a) => a.status === 'Working').length },
    { name: 'Paused', value: agents.filter((a) => a.status === 'Paused').length },
    { name: 'Failed', value: agents.filter((a) => a.status === 'Failed').length },
  ].filter((item) => item.value > 0);

  // Calculate workflow status distribution
  const workflowStatusData = [
    { name: 'Pending', value: workflows.filter((w) => w.status === 'Pending').length },
    { name: 'Running', value: workflows.filter((w) => w.status === 'Running').length },
    { name: 'Completed', value: workflows.filter((w) => w.status === 'Completed').length },
    { name: 'Failed', value: workflows.filter((w) => w.status === 'Failed').length },
  ].filter((item) => item.value > 0);

  const hasAlerts = agents.filter((a) => a.status === 'Failed').length > 0 ||
                    workflows.filter((w) => w.status === 'Failed').length > 0;

  return (
    <Box>
      <Box sx={{ display: 'flex', alignItems: 'center', mb: 5 }}>
        <Typography variant="h4" sx={{ flexGrow: 1 }}>
          Axon Multi-Agent System
        </Typography>
        <Stack direction="row" spacing={1} alignItems="center">
          <Chip
            label={wsConnected ? 'WebSocket Connected' : 'WebSocket Disconnected'}
            color={wsConnected ? 'success' : 'default'}
            size="small"
            icon={<Iconify icon={wsConnected ? 'eva:wifi-fill' : 'eva:wifi-off-fill'} />}
          />
          <Label color={health?.status === 'healthy' ? 'success' : 'error'}>
            {health?.status || 'Unknown'}
          </Label>
        </Stack>
      </Box>

      {hasAlerts && (
        <Alert severity="error" sx={{ mb: 3 }}>
          <Typography variant="subtitle2">System Alerts</Typography>
          <Typography variant="body2">
            {agents.filter((a) => a.status === 'Failed').length > 0 &&
              `${agents.filter((a) => a.status === 'Failed').length} agent(s) failed. `}
            {workflows.filter((w) => w.status === 'Failed').length > 0 &&
              `${workflows.filter((w) => w.status === 'Failed').length} workflow(s) failed.`}
          </Typography>
        </Alert>
      )}

      <Grid container spacing={3}>
        {/* System Overview Cards */}
        <Grid size={{ xs: 12, sm: 6, md: 3 }}>
          <Card sx={{ p: 3 }}>
            <Stack spacing={1}>
              <Stack direction="row" alignItems="center" spacing={1}>
                <Iconify icon="solar:users-group-rounded-bold" width={24} />
                <Typography variant="subtitle2">Active Agents</Typography>
              </Stack>
              <Typography variant="h3">
                <AnimateCountUp to={activeAgents} />
              </Typography>
              <Typography variant="caption" sx={{ color: 'text.secondary' }}>
                {agents.length} total
              </Typography>
            </Stack>
          </Card>
        </Grid>

        <Grid size={{ xs: 12, sm: 6, md: 3 }}>
          <Card sx={{ p: 3 }}>
            <Stack spacing={1}>
              <Stack direction="row" alignItems="center" spacing={1}>
                <Iconify icon="solar:routing-2-bold" width={24} />
                <Typography variant="subtitle2">Running Workflows</Typography>
              </Stack>
              <Typography variant="h3">
                <AnimateCountUp to={runningWorkflows} />
              </Typography>
              <Typography variant="caption" sx={{ color: 'text.secondary' }}>
                {workflows.length} total
              </Typography>
            </Stack>
          </Card>
        </Grid>

        <Grid size={{ xs: 12, sm: 6, md: 3 }}>
          <Card sx={{ p: 3 }}>
            <Stack spacing={1}>
              <Stack direction="row" alignItems="center" spacing={1}>
                <Iconify icon="solar:widget-5-bold" width={24} />
                <Typography variant="subtitle2">Tasks Executed</Typography>
              </Stack>
              <Typography variant="h3">
                <AnimateCountUp to={systemStatus?.total_tasks_executed || 0} />
              </Typography>
              <Typography variant="caption" sx={{ color: 'text.secondary' }}>
                All time
              </Typography>
            </Stack>
          </Card>
        </Grid>

        <Grid size={{ xs: 12, sm: 6, md: 3 }}>
          <Card sx={{ p: 3 }}>
            <Stack spacing={1}>
              <Stack direction="row" alignItems="center" spacing={1}>
                <Iconify icon="solar:clock-circle-bold" width={24} />
                <Typography variant="subtitle2">Uptime</Typography>
              </Stack>
              <Typography variant="h3">
                {formatUptime(health?.uptime_seconds || 0)}
              </Typography>
              <Typography variant="caption" sx={{ color: 'text.secondary' }}>
                Server uptime
              </Typography>
            </Stack>
          </Card>
        </Grid>

        {/* System Resource Metrics */}
        <Grid size={{ xs: 12, md: 6 }}>
          <Card>
            <CardHeader title="System Resources" />
            <CardContent>
              <Stack spacing={3}>
                <Box>
                  <Stack direction="row" justifyContent="space-between" sx={{ mb: 1 }}>
                    <Typography variant="body2">CPU Usage</Typography>
                    <Typography variant="body2" color="text.secondary">
                      {systemStatus?.cpu_usage_percent?.toFixed(1) || 0}%
                    </Typography>
                  </Stack>
                  <LinearProgress
                    variant="determinate"
                    value={systemStatus?.cpu_usage_percent || 0}
                    color={systemStatus?.cpu_usage_percent! > 80 ? 'error' : systemStatus?.cpu_usage_percent! > 60 ? 'warning' : 'primary'}
                  />
                </Box>
                <Box>
                  <Stack direction="row" justifyContent="space-between" sx={{ mb: 1 }}>
                    <Typography variant="body2">Memory Usage</Typography>
                    <Typography variant="body2" color="text.secondary">
                      {systemStatus?.memory_usage_mb?.toFixed(0) || 0} MB
                    </Typography>
                  </Stack>
                  <LinearProgress
                    variant="determinate"
                    value={((systemStatus?.memory_usage_mb || 0) / 8192) * 100}
                    color={systemStatus?.memory_usage_mb! > 6500 ? 'error' : systemStatus?.memory_usage_mb! > 5000 ? 'warning' : 'primary'}
                  />
                </Box>
              </Stack>
            </CardContent>
          </Card>
        </Grid>

        {/* Performance Metrics Over Time */}
        <Grid size={{ xs: 12, md: 6 }}>
          <Card>
            <CardHeader title="CPU & Memory Trends" />
            <CardContent>
              <ResponsiveContainer width="100%" height={200}>
                <LineChart data={metricsHistory}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis
                    dataKey="timestamp"
                    tickFormatter={(value) => new Date(value).toLocaleTimeString()}
                    fontSize={12}
                  />
                  <YAxis fontSize={12} />
                  <Tooltip
                    labelFormatter={(value) => new Date(value).toLocaleString()}
                    contentStyle={{ fontSize: 12 }}
                  />
                  <Legend wrapperStyle={{ fontSize: 12 }} />
                  <Line
                    type="monotone"
                    dataKey="cpu_usage_percent"
                    stroke={COLORS.primary}
                    name="CPU %"
                    strokeWidth={2}
                  />
                  <Line
                    type="monotone"
                    dataKey="memory_usage_mb"
                    stroke={COLORS.success}
                    name="Memory (MB)"
                    strokeWidth={2}
                  />
                </LineChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>
        </Grid>

        {/* Agent Status Distribution */}
        <Grid size={{ xs: 12, md: 6 }}>
          <Card>
            <CardHeader title="Agent Status Distribution" />
            <CardContent>
              <ResponsiveContainer width="100%" height={250}>
                <PieChart>
                  <Pie
                    data={agentStatusData}
                    cx="50%"
                    cy="50%"
                    labelLine={false}
                    label={(entry) => `${entry.name}: ${entry.value}`}
                    outerRadius={80}
                    fill="#8884d8"
                    dataKey="value"
                  >
                    {agentStatusData.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={PIE_COLORS[index % PIE_COLORS.length]} />
                    ))}
                  </Pie>
                  <Tooltip />
                </PieChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>
        </Grid>

        {/* Workflow Status Distribution */}
        <Grid size={{ xs: 12, md: 6 }}>
          <Card>
            <CardHeader title="Workflow Status Distribution" />
            <CardContent>
              <ResponsiveContainer width="100%" height={250}>
                <BarChart data={workflowStatusData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="name" fontSize={12} />
                  <YAxis fontSize={12} />
                  <Tooltip contentStyle={{ fontSize: 12 }} />
                  <Bar dataKey="value" fill={COLORS.primary}>
                    {workflowStatusData.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={PIE_COLORS[index % PIE_COLORS.length]} />
                    ))}
                  </Bar>
                </BarChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>
        </Grid>

        {/* Recent Agents */}
        <Grid size={{ xs: 12, md: 6 }}>
          <Card>
            <CardHeader
              title="Recent Agents"
              action={
                <Button size="small" href="/dashboard/agents">
                  View All
                </Button>
              }
            />
            <Stack spacing={2} sx={{ p: 3 }}>
              {agents.slice(0, 5).map((agent) => (
                <AgentCard key={agent.id} agent={agent} />
              ))}
              {agents.length === 0 && (
                <Alert severity="info">No agents running. Create your first agent to get started.</Alert>
              )}
            </Stack>
          </Card>
        </Grid>

        {/* Recent Workflows */}
        <Grid size={{ xs: 12, md: 6 }}>
          <Card>
            <CardHeader
              title="Recent Workflows"
              action={
                <Button size="small" href="/dashboard/workflows">
                  View All
                </Button>
              }
            />
            <Stack spacing={2} sx={{ p: 3 }}>
              {workflows.slice(0, 5).map((workflow) => (
                <WorkflowCard key={workflow.id} workflow={workflow} />
              ))}
              {workflows.length === 0 && (
                <Alert severity="info">No workflows yet. Run your first workflow to get started.</Alert>
              )}
            </Stack>
          </Card>
        </Grid>
      </Grid>
    </Box>
  );
}

// ----------------------------------------------------------------------

function AgentCard({ agent }: { agent: AgentInfo }) {
  return (
    <Stack
      direction="row"
      alignItems="center"
      spacing={2}
      sx={{
        p: 2,
        borderRadius: 1,
        border: (theme) => `1px solid ${theme.palette.divider}`,
        '&:hover': {
          bgcolor: 'action.hover',
          cursor: 'pointer',
        },
      }}
      component="a"
      href={`/dashboard/agents/${agent.id}`}
    >
      <Box sx={{ flexGrow: 1, minWidth: 0 }}>
        <Typography variant="subtitle2" noWrap>
          {agent.name}
        </Typography>
        <Typography variant="caption" sx={{ color: 'text.secondary' }}>
          {agent.agent_type}
        </Typography>
      </Box>
      <Label variant="soft" color={getAgentStatusColor(agent.status)}>
        {agent.status}
      </Label>
    </Stack>
  );
}

function WorkflowCard({ workflow }: { workflow: WorkflowInfo }) {
  const progress =
    workflow.total_tasks > 0 ? (workflow.completed_tasks / workflow.total_tasks) * 100 : 0;

  return (
    <Stack
      spacing={1}
      sx={{
        p: 2,
        borderRadius: 1,
        border: (theme) => `1px solid ${theme.palette.divider}`,
      }}
    >
      <Stack direction="row" alignItems="center" spacing={2}>
        <Box sx={{ flexGrow: 1, minWidth: 0 }}>
          <Typography variant="subtitle2" noWrap>
            {workflow.name}
          </Typography>
          <Typography variant="caption" sx={{ color: 'text.secondary' }}>
            {workflow.completed_tasks} / {workflow.total_tasks} tasks
          </Typography>
        </Box>
        <Label variant="soft" color={getWorkflowStatusColor(workflow.status)}>
          {workflow.status}
        </Label>
      </Stack>
      {workflow.status === 'Running' && (
        <LinearProgress variant="determinate" value={progress} />
      )}
    </Stack>
  );
}

// ----------------------------------------------------------------------

function formatUptime(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;

  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m`;

  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h`;

  const days = Math.floor(hours / 24);
  return `${days}d`;
}
