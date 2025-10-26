import type { AgentInfo, SystemStatus, WorkflowInfo, HealthResponse } from 'src/types/axon';

import useSWR from 'swr';
import { useState, useEffect } from 'react';

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

import { getAgentStatusColor, getWorkflowStatusColor } from 'src/utils/status-colors';

import { axonWebSocket } from 'src/lib/axon-websocket';
import { axonFetcher, axonEndpoints } from 'src/lib/axon-client';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { AnimateCountUp } from 'src/components/animate';

// ----------------------------------------------------------------------

export function AxonOverview() {
  const [wsConnected, setWsConnected] = useState(false);

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

  const runningWorkflows = workflows.filter((w: WorkflowInfo) => w.status === 'Running').length;
  const activeAgents = agents.filter((a: AgentInfo) => a.status === 'Working' || a.status === 'Idle').length;

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
      }}
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
