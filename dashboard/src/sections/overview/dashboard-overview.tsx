import type {
  AgentInfo,
  SystemStatus,
  WorkflowInfo,
  WebSocketEvent,
  HealthResponse as AxonHealthResponse,
} from 'src/types/axon';
import type {
  Task,
  Document,
  Workspace,
  SystemStats as CortexSystemStats,
  HealthResponse as CortexHealthResponse,
} from 'src/types/cortex';

import useSWR from 'swr';
import { useState, useEffect } from 'react';

import Box from '@mui/material/Box';
import Grid from '@mui/material/Grid';
import Card from '@mui/material/Card';
import Chip from '@mui/material/Chip';
import Stack from '@mui/material/Stack';
import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import Divider from '@mui/material/Divider';
import { useTheme } from '@mui/material/styles';
import CardHeader from '@mui/material/CardHeader';
import Typography from '@mui/material/Typography';

import { axonWebSocket } from 'src/lib/axon-websocket';
import { axonFetcher, axonEndpoints } from 'src/lib/axon-client';
import { cortexFetcher, cortexEndpoints } from 'src/lib/cortex-client';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { AnimateCountUp } from 'src/components/animate';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { MetricCard } from './components/metric-card';
import { ActivityFeed } from './components/activity-feed';
import { StatusIndicator } from './components/status-indicator';
import { QuickActionsPanel } from './components/quick-actions-panel';
import { SystemHealthSection } from './components/system-health-section';

// ----------------------------------------------------------------------

export function DashboardOverview() {
  const theme = useTheme();
  const [wsConnected, setWsConnected] = useState(false);
  const [recentActivity, setRecentActivity] = useState<ActivityEvent[]>([]);

  // Axon Data Fetching
  const { data: axonHealth, error: axonHealthError } = useSWR<AxonHealthResponse>(
    axonEndpoints.health,
    axonFetcher,
    { refreshInterval: 5000 }
  );

  const { data: axonSystemStatus } = useSWR<SystemStatus>(
    axonEndpoints.status,
    axonFetcher,
    { refreshInterval: 5000 }
  );

  const { data: agents = [] } = useSWR<AgentInfo[]>(
    axonEndpoints.agents.list,
    axonFetcher,
    { refreshInterval: 5000 }
  );

  const { data: workflows = [] } = useSWR<WorkflowInfo[]>(
    axonEndpoints.workflows.list,
    axonFetcher,
    { refreshInterval: 3000 }
  );

  // Cortex Data Fetching
  const { data: cortexHealth, error: cortexHealthError } = useSWR<CortexHealthResponse>(
    cortexEndpoints.health,
    cortexFetcher,
    { refreshInterval: 5000 }
  );

  const { data: cortexSystemStats } = useSWR<CortexSystemStats>(
    cortexEndpoints.systemStats,
    cortexFetcher,
    { refreshInterval: 10000 }
  );

  const { data: workspaces = [] } = useSWR<Workspace[]>(
    cortexEndpoints.workspaces.list,
    cortexFetcher,
    { refreshInterval: 10000 }
  );

  const { data: documents = [] } = useSWR<Document[]>(
    cortexEndpoints.documents.list,
    cortexFetcher,
    { refreshInterval: 10000 }
  );

  const { data: tasks = [] } = useSWR<Task[]>(
    cortexEndpoints.tasks.list,
    cortexFetcher,
    { refreshInterval: 5000 }
  );

  // WebSocket connection for real-time updates
  useEffect(() => {
    axonWebSocket.connect();

    const checkConnection = setInterval(() => {
      setWsConnected(axonWebSocket.isConnected());
    }, 1000);

    const unsubscribe = axonWebSocket.subscribe((event: WebSocketEvent) => {
      // Add to activity feed
      setRecentActivity((prev) => [
        {
          id: `${event.type}-${Date.now()}`,
          type: event.type,
          service: 'axon',
          timestamp: event.timestamp,
          data: event.data,
        },
        ...prev.slice(0, 49), // Keep last 50 events
      ]);
    });

    return () => {
      clearInterval(checkConnection);
      unsubscribe();
      axonWebSocket.disconnect();
    };
  }, []);

  // Calculate metrics
  const activeAgents = agents.filter((a) => a.status === 'Working' || a.status === 'Idle').length;
  const runningWorkflows = workflows.filter((w) => w.status === 'Running').length;
  const tasksInProgress = tasks.filter((t) => t.status === 'running').length;
  const recentDocuments = documents.filter((d) => {
    const dayAgo = Date.now() - 24 * 60 * 60 * 1000;
    return new Date(d.updated_at).getTime() > dayAgo;
  }).length;

  // System health status
  const axonHealthy = !axonHealthError && axonHealth?.status === 'healthy';
  const cortexHealthy = !cortexHealthError && cortexHealth?.status === 'healthy';
  const overallHealth = axonHealthy && cortexHealthy ? 'healthy' : 'degraded';

  return (
    <>
      <CustomBreadcrumbs
        heading="Dashboard"
        links={[
          { name: 'Dashboard' },
        ]}
        action={
          <Stack direction="row" spacing={1} alignItems="center">
            <Chip
              label={wsConnected ? 'WebSocket Connected' : 'WebSocket Disconnected'}
              color={wsConnected ? 'success' : 'default'}
              size="small"
              icon={<Iconify icon={wsConnected ? 'eva:wifi-fill' : 'eva:wifi-off-fill'} />}
            />
            <Label color={overallHealth === 'healthy' ? 'success' : 'warning'}>
              {overallHealth}
            </Label>
          </Stack>
        }
        sx={{ mb: 3 }}
      />

      <Grid container spacing={3}>
        {/* System Health Section */}
        <Grid size={{ xs: 12 }}>
          <SystemHealthSection
            axonHealth={axonHealth}
            cortexHealth={cortexHealth}
            axonHealthy={axonHealthy}
            cortexHealthy={cortexHealthy}
          />
        </Grid>

        {/* Quick Metrics Row */}
        <Grid size={{ xs: 12, sm: 6, md: 3 }}>
          <MetricCard
            title="Active Agents"
            value={activeAgents}
            total={agents.length}
            icon="solar:users-group-rounded-bold"
            color="info"
            href="/agents"
          />
        </Grid>

        <Grid size={{ xs: 12, sm: 6, md: 3 }}>
          <MetricCard
            title="Running Workflows"
            value={runningWorkflows}
            total={workflows.length}
            icon="solar:routing-2-bold"
            color="primary"
            href="/workflows"
          />
        </Grid>

        <Grid size={{ xs: 12, sm: 6, md: 3 }}>
          <MetricCard
            title="Total Workspaces"
            value={workspaces.length}
            icon="solar:folder-with-files-bold"
            color="success"
            href="/cortex/workspaces"
          />
        </Grid>

        <Grid size={{ xs: 12, sm: 6, md: 3 }}>
          <MetricCard
            title="Total Documents"
            value={documents.length}
            icon="solar:document-text-bold"
            color="warning"
            href="/cortex/documents"
          />
        </Grid>

        {/* Axon Status Section */}
        <Grid size={{ xs: 12, md: 6 }}>
          <Card sx={{ height: '100%' }}>
            <CardHeader
              title="Axon Multi-Agent System"
              subheader="Real-time agent and workflow status"
              avatar={
                <StatusIndicator
                  status={axonHealthy ? 'healthy' : 'unhealthy'}
                  size={12}
                />
              }
              action={
                <Button
                  size="small"
                  href="/agents"
                  endIcon={<Iconify icon="eva:arrow-ios-forward-fill" />}
                >
                  View All
                </Button>
              }
            />
            <Divider />
            <Stack spacing={3} sx={{ p: 3 }}>
              <Grid container spacing={2}>
                <Grid size={{ xs: 6 }}>
                  <Stack spacing={1}>
                    <Typography variant="caption" color="text.secondary">
                      Active Agents
                    </Typography>
                    <Typography variant="h4">
                      <AnimateCountUp to={activeAgents} />
                    </Typography>
                  </Stack>
                </Grid>
                <Grid size={{ xs: 6 }}>
                  <Stack spacing={1}>
                    <Typography variant="caption" color="text.secondary">
                      Running Workflows
                    </Typography>
                    <Typography variant="h4">
                      <AnimateCountUp to={runningWorkflows} />
                    </Typography>
                  </Stack>
                </Grid>
                <Grid size={{ xs: 6 }}>
                  <Stack spacing={1}>
                    <Typography variant="caption" color="text.secondary">
                      Tasks Executed
                    </Typography>
                    <Typography variant="h4">
                      <AnimateCountUp to={axonSystemStatus?.total_tasks_executed || 0} />
                    </Typography>
                  </Stack>
                </Grid>
                <Grid size={{ xs: 6 }}>
                  <Stack spacing={1}>
                    <Typography variant="caption" color="text.secondary">
                      System Uptime
                    </Typography>
                    <Typography variant="h4">
                      {formatUptime(axonHealth?.uptime_seconds || 0)}
                    </Typography>
                  </Stack>
                </Grid>
              </Grid>

              {axonSystemStatus && (
                <Box>
                  <Typography variant="caption" color="text.secondary" sx={{ mb: 1, display: 'block' }}>
                    System Resources
                  </Typography>
                  <Stack spacing={1}>
                    <Stack direction="row" justifyContent="space-between" alignItems="center">
                      <Typography variant="body2">CPU Usage</Typography>
                      <Typography variant="body2" fontWeight="bold">
                        {(axonSystemStatus.cpu_usage_percent || 0).toFixed(1)}%
                      </Typography>
                    </Stack>
                    <Stack direction="row" justifyContent="space-between" alignItems="center">
                      <Typography variant="body2">Memory Usage</Typography>
                      <Typography variant="body2" fontWeight="bold">
                        {(axonSystemStatus.memory_usage_mb || 0).toFixed(0)} MB
                      </Typography>
                    </Stack>
                  </Stack>
                </Box>
              )}
            </Stack>
          </Card>
        </Grid>

        {/* Cortex Status Section */}
        <Grid size={{ xs: 12, md: 6 }}>
          <Card sx={{ height: '100%' }}>
            <CardHeader
              title="Cortex Cognitive System"
              subheader="Knowledge base and memory status"
              avatar={
                <StatusIndicator
                  status={cortexHealthy ? 'healthy' : 'unhealthy'}
                  size={12}
                />
              }
              action={
                <Button
                  size="small"
                  href="/cortex"
                  endIcon={<Iconify icon="eva:arrow-ios-forward-fill" />}
                >
                  View All
                </Button>
              }
            />
            <Divider />
            <Stack spacing={3} sx={{ p: 3 }}>
              <Grid container spacing={2}>
                <Grid size={{ xs: 6 }}>
                  <Stack spacing={1}>
                    <Typography variant="caption" color="text.secondary">
                      Workspaces
                    </Typography>
                    <Typography variant="h4">
                      <AnimateCountUp to={cortexSystemStats?.workspaces_count || 0} />
                    </Typography>
                  </Stack>
                </Grid>
                <Grid size={{ xs: 6 }}>
                  <Stack spacing={1}>
                    <Typography variant="caption" color="text.secondary">
                      Documents
                    </Typography>
                    <Typography variant="h4">
                      <AnimateCountUp to={cortexSystemStats?.documents_count || 0} />
                    </Typography>
                  </Stack>
                </Grid>
                <Grid size={{ xs: 6 }}>
                  <Stack spacing={1}>
                    <Typography variant="caption" color="text.secondary">
                      Code Units
                    </Typography>
                    <Typography variant="h4">
                      <AnimateCountUp to={cortexSystemStats?.code_units_count || 0} />
                    </Typography>
                  </Stack>
                </Grid>
                <Grid size={{ xs: 6 }}>
                  <Stack spacing={1}>
                    <Typography variant="caption" color="text.secondary">
                      Tasks Active
                    </Typography>
                    <Typography variant="h4">
                      <AnimateCountUp to={tasksInProgress} />
                    </Typography>
                  </Stack>
                </Grid>
              </Grid>

              {cortexSystemStats && (
                <Box>
                  <Typography variant="caption" color="text.secondary" sx={{ mb: 1, display: 'block' }}>
                    Storage & Files
                  </Typography>
                  <Stack spacing={1}>
                    <Stack direction="row" justifyContent="space-between" alignItems="center">
                      <Typography variant="body2">Total Files</Typography>
                      <Typography variant="body2" fontWeight="bold">
                        {cortexSystemStats.files_count.toLocaleString()}
                      </Typography>
                    </Stack>
                    <Stack direction="row" justifyContent="space-between" alignItems="center">
                      <Typography variant="body2">Storage Used</Typography>
                      <Typography variant="body2" fontWeight="bold">
                        {formatBytes(cortexSystemStats.total_storage)}
                      </Typography>
                    </Stack>
                    <Stack direction="row" justifyContent="space-between" alignItems="center">
                      <Typography variant="body2">Updated Today</Typography>
                      <Typography variant="body2" fontWeight="bold" color="success.main">
                        {recentDocuments} documents
                      </Typography>
                    </Stack>
                  </Stack>
                </Box>
              )}
            </Stack>
          </Card>
        </Grid>

        {/* Activity Feed */}
        <Grid size={{ xs: 12, lg: 8 }}>
          <ActivityFeed
            events={recentActivity}
            agents={agents}
            workflows={workflows}
            documents={documents}
            workspaces={workspaces}
          />
        </Grid>

        {/* Quick Actions */}
        <Grid size={{ xs: 12, lg: 4 }}>
          <QuickActionsPanel />
        </Grid>

        {/* Additional Info */}
        {!axonHealthy && (
          <Grid size={{ xs: 12 }}>
            <Alert severity="error">
              Axon system is not responding. Please check the service status.
            </Alert>
          </Grid>
        )}

        {!cortexHealthy && (
          <Grid size={{ xs: 12 }}>
            <Alert severity="error">
              Cortex system is not responding. Please check the service status.
            </Alert>
          </Grid>
        )}
      </Grid>
    </>
  );
}

// ----------------------------------------------------------------------
// Helper Functions
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

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';

  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));

  return `${parseFloat((bytes / k ** i).toFixed(2))} ${sizes[i]}`;
}

// ----------------------------------------------------------------------
// Types
// ----------------------------------------------------------------------

interface ActivityEvent {
  id: string;
  type: string;
  service: 'axon' | 'cortex';
  timestamp: string;
  data: any;
}
