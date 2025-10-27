import useSWR from 'swr';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import LinearProgress from '@mui/material/LinearProgress';

import { axonFetcher, axonEndpoints } from 'src/lib/axon-client';

import { Iconify } from 'src/components/iconify';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

interface AxonMetrics {
  avg_response_time_ms?: number;
  request_rate?: number;
  error_rate?: number;
}

interface AxonStatus {
  active_agents?: number;
  running_workflows?: number;
  total_tasks_executed?: number;
  system_uptime_seconds?: number;
  memory_usage_mb?: number;
  cpu_usage_percent?: number;
}

// ----------------------------------------------------------------------

export function MonitoringMetricsView() {
  const { data: metrics, isLoading: metricsLoading } = useSWR<AxonMetrics>(
    axonEndpoints.metrics,
    axonFetcher,
    { refreshInterval: 5000 }
  );

  const { data: status, isLoading: statusLoading } = useSWR<AxonStatus>(
    axonEndpoints.status,
    axonFetcher,
    { refreshInterval: 5000 }
  );

  const isLoading = metricsLoading || statusLoading;

  return (
    <>
      <CustomBreadcrumbs
        heading="System Metrics"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Monitoring' },
          { name: 'Metrics' },
        ]}
        sx={{ mb: 3 }}
      />

      <Stack spacing={2}>
        <Card sx={{ p: 3 }}>
          <Typography variant="h6" sx={{ mb: 1 }}>
            Real-time System Metrics
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Monitor system performance, resource usage, and operational metrics in real-time.
          </Typography>
        </Card>

        {isLoading && <LinearProgress />}

        <Grid container spacing={2}>
          <Grid size={{ xs: 6, md: 3 }}>
            <Card sx={{ p: 2.5 }}>
              <Stack spacing={1}>
                <Iconify icon="mdi:robot" width={32} color="primary.main" />
                <Typography variant="h4">{status?.active_agents || 0}</Typography>
                <Typography variant="body2" color="text.secondary">
                  Active Agents
                </Typography>
              </Stack>
            </Card>
          </Grid>
          <Grid size={{ xs: 6, md: 3 }}>
            <Card sx={{ p: 2.5 }}>
              <Stack spacing={1}>
                <Iconify icon="mdi:workflow" width={32} color="info.main" />
                <Typography variant="h4">{status?.running_workflows || 0}</Typography>
                <Typography variant="body2" color="text.secondary">
                  Running Workflows
                </Typography>
              </Stack>
            </Card>
          </Grid>
          <Grid size={{ xs: 6, md: 3 }}>
            <Card sx={{ p: 2.5 }}>
              <Stack spacing={1}>
                <Iconify icon="mdi:check-circle" width={32} color="success.main" />
                <Typography variant="h4">{status?.total_tasks_executed || 0}</Typography>
                <Typography variant="body2" color="text.secondary">
                  Tasks Executed
                </Typography>
              </Stack>
            </Card>
          </Grid>
          <Grid size={{ xs: 6, md: 3 }}>
            <Card sx={{ p: 2.5 }}>
              <Stack spacing={1}>
                <Iconify icon="mdi:clock" width={32} color="warning.main" />
                <Typography variant="h4">
                  {status?.system_uptime_seconds
                    ? `${Math.floor(status.system_uptime_seconds / 3600)}h`
                    : '0h'}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  System Uptime
                </Typography>
              </Stack>
            </Card>
          </Grid>
        </Grid>

        <Grid container spacing={2}>
          <Grid size={{ xs: 12, md: 6 }}>
            <Card sx={{ p: 3 }}>
              <Typography variant="h6" sx={{ mb: 2 }}>
                Resource Usage
              </Typography>
              <Stack spacing={2}>
                <Box>
                  <Stack direction="row" justifyContent="space-between" sx={{ mb: 0.5 }}>
                    <Typography variant="body2">Memory Usage</Typography>
                    <Typography variant="body2" fontWeight="bold">
                      {status?.memory_usage_mb
                        ? `${Math.round(status.memory_usage_mb)} MB`
                        : 'N/A'}
                    </Typography>
                  </Stack>
                  <LinearProgress
                    variant="determinate"
                    value={status?.memory_usage_mb ? Math.min((status.memory_usage_mb / 1024) * 100, 100) : 0}
                    sx={{ height: 8, borderRadius: 1 }}
                  />
                </Box>
                <Box>
                  <Stack direction="row" justifyContent="space-between" sx={{ mb: 0.5 }}>
                    <Typography variant="body2">CPU Usage</Typography>
                    <Typography variant="body2" fontWeight="bold">
                      {status?.cpu_usage_percent
                        ? `${status.cpu_usage_percent.toFixed(1)}%`
                        : 'N/A'}
                    </Typography>
                  </Stack>
                  <LinearProgress
                    variant="determinate"
                    value={status?.cpu_usage_percent || 0}
                    color="warning"
                    sx={{ height: 8, borderRadius: 1 }}
                  />
                </Box>
              </Stack>
            </Card>
          </Grid>

          <Grid size={{ xs: 12, md: 6 }}>
            <Card sx={{ p: 3 }}>
              <Typography variant="h6" sx={{ mb: 2 }}>
                Performance Metrics
              </Typography>
              <Stack spacing={2}>
                <Stack direction="row" justifyContent="space-between">
                  <Typography variant="body2" color="text.secondary">
                    Avg Response Time
                  </Typography>
                  <Typography variant="subtitle2">
                    {metrics?.avg_response_time_ms
                      ? `${metrics.avg_response_time_ms.toFixed(0)}ms`
                      : 'N/A'}
                  </Typography>
                </Stack>
                <Stack direction="row" justifyContent="space-between">
                  <Typography variant="body2" color="text.secondary">
                    Request Rate
                  </Typography>
                  <Typography variant="subtitle2">
                    {metrics?.request_rate ? `${metrics.request_rate}/min` : 'N/A'}
                  </Typography>
                </Stack>
                <Stack direction="row" justifyContent="space-between">
                  <Typography variant="body2" color="text.secondary">
                    Error Rate
                  </Typography>
                  <Typography
                    variant="subtitle2"
                    color={
                      metrics?.error_rate !== undefined && metrics.error_rate > 5
                        ? 'error.main'
                        : metrics?.error_rate !== undefined && metrics.error_rate > 1
                        ? 'warning.main'
                        : 'success.main'
                    }
                  >
                    {metrics?.error_rate !== undefined ? `${metrics.error_rate.toFixed(2)}%` : 'N/A'}
                  </Typography>
                </Stack>
              </Stack>
            </Card>
          </Grid>
        </Grid>
      </Stack>
    </>
  );
}
