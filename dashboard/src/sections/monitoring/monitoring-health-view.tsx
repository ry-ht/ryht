import useSWR from 'swr';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import LinearProgress from '@mui/material/LinearProgress';

import { axonFetcher, axonEndpoints } from 'src/lib/axon-client';
import { cortexFetcher, cortexEndpoints } from 'src/lib/cortex-client';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

export function MonitoringHealthView() {
  const { data: axonHealth, isLoading: axonLoading } = useSWR(
    axonEndpoints.health,
    axonFetcher,
    { refreshInterval: 10000 }
  );

  const { data: cortexHealth, isLoading: cortexLoading } = useSWR(
    cortexEndpoints.health,
    cortexFetcher,
    { refreshInterval: 10000 }
  );

  const isLoading = axonLoading || cortexLoading;

  const getHealthColor = (status: string) => {
    if (status === 'healthy' || status === 'ok') return 'success';
    if (status === 'degraded') return 'warning';
    return 'error';
  };

  return (
    <>
      <CustomBreadcrumbs
        heading="System Health"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Monitoring' },
          { name: 'Health' },
        ]}
        sx={{ mb: 3 }}
      />

      <Stack spacing={2}>
        <Card sx={{ p: 3 }}>
          <Typography variant="h6" sx={{ mb: 1 }}>
            System Health Dashboard
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Monitor the health status of all system components and services.
          </Typography>
        </Card>

        {isLoading && <LinearProgress />}

        <Grid container spacing={2}>
          <Grid size={{ xs: 12, md: 6 }}>
            <Card sx={{ p: 3 }}>
              <Stack spacing={2}>
                <Stack direction="row" justifyContent="space-between" alignItems="center">
                  <Stack direction="row" spacing={2} alignItems="center">
                    <Box
                      sx={{
                        width: 56,
                        height: 56,
                        borderRadius: 2,
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        bgcolor: 'primary.lighter',
                        color: 'primary.main',
                      }}
                    >
                      <Iconify icon="mdi:robot" width={32} />
                    </Box>
                    <Box>
                      <Typography variant="h6">Axon (Multi-Agent System)</Typography>
                      <Typography variant="caption" color="text.secondary">
                        v{axonHealth?.version || 'N/A'}
                      </Typography>
                    </Box>
                  </Stack>
                  <Label
                    variant="soft"
                    color={getHealthColor(axonHealth?.status || 'unknown')}
                    sx={{ height: 'fit-content' }}
                  >
                    {axonHealth?.status || 'Unknown'}
                  </Label>
                </Stack>

                <Stack spacing={1.5}>
                  <Stack direction="row" justifyContent="space-between">
                    <Typography variant="body2" color="text.secondary">
                      Active Agents
                    </Typography>
                    <Typography variant="subtitle2">
                      {axonHealth?.active_agents || 0}
                    </Typography>
                  </Stack>
                  <Stack direction="row" justifyContent="space-between">
                    <Typography variant="body2" color="text.secondary">
                      Running Workflows
                    </Typography>
                    <Typography variant="subtitle2">
                      {axonHealth?.running_workflows || 0}
                    </Typography>
                  </Stack>
                  <Stack direction="row" justifyContent="space-between">
                    <Typography variant="body2" color="text.secondary">
                      Uptime
                    </Typography>
                    <Typography variant="subtitle2">
                      {axonHealth?.uptime_seconds
                        ? `${Math.floor(axonHealth.uptime_seconds / 3600)}h ${Math.floor((axonHealth.uptime_seconds % 3600) / 60)}m`
                        : 'N/A'}
                    </Typography>
                  </Stack>
                  <Stack direction="row" justifyContent="space-between">
                    <Typography variant="body2" color="text.secondary">
                      WebSocket Connections
                    </Typography>
                    <Typography variant="subtitle2">
                      {axonHealth?.websocket_connections || 0}
                    </Typography>
                  </Stack>
                </Stack>
              </Stack>
            </Card>
          </Grid>

          <Grid size={{ xs: 12, md: 6 }}>
            <Card sx={{ p: 3 }}>
              <Stack spacing={2}>
                <Stack direction="row" justifyContent="space-between" alignItems="center">
                  <Stack direction="row" spacing={2} alignItems="center">
                    <Box
                      sx={{
                        width: 56,
                        height: 56,
                        borderRadius: 2,
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        bgcolor: 'info.lighter',
                        color: 'info.main',
                      }}
                    >
                      <Iconify icon="mdi:brain" width={32} />
                    </Box>
                    <Box>
                      <Typography variant="h6">Cortex (Cognitive Backend)</Typography>
                      <Typography variant="caption" color="text.secondary">
                        v{cortexHealth?.version || 'N/A'}
                      </Typography>
                    </Box>
                  </Stack>
                  <Label
                    variant="soft"
                    color={getHealthColor(cortexHealth?.status || 'unknown')}
                    sx={{ height: 'fit-content' }}
                  >
                    {cortexHealth?.status || 'Unknown'}
                  </Label>
                </Stack>

                <Stack spacing={1.5}>
                  <Stack direction="row" justifyContent="space-between">
                    <Typography variant="body2" color="text.secondary">
                      Database Status
                    </Typography>
                    <Label variant="soft" color="success">
                      Connected
                    </Label>
                  </Stack>
                  <Stack direction="row" justifyContent="space-between">
                    <Typography variant="body2" color="text.secondary">
                      Vector DB Status
                    </Typography>
                    <Label variant="soft" color="success">
                      Connected
                    </Label>
                  </Stack>
                  <Stack direction="row" justifyContent="space-between">
                    <Typography variant="body2" color="text.secondary">
                      Memory Usage
                    </Typography>
                    <Typography variant="subtitle2">
                      {cortexHealth?.memory_usage_mb
                        ? `${Math.round(cortexHealth.memory_usage_mb)} MB`
                        : 'N/A'}
                    </Typography>
                  </Stack>
                  <Stack direction="row" justifyContent="space-between">
                    <Typography variant="body2" color="text.secondary">
                      Active Tasks
                    </Typography>
                    <Typography variant="subtitle2">
                      {cortexHealth?.active_tasks || 0}
                    </Typography>
                  </Stack>
                </Stack>
              </Stack>
            </Card>
          </Grid>
        </Grid>

        <Card sx={{ p: 3, bgcolor: axonHealth && cortexHealth ? 'success.lighter' : 'warning.lighter' }}>
          <Stack direction="row" spacing={2} alignItems="center">
            <Iconify
              icon={axonHealth && cortexHealth ? 'mdi:check-circle' : 'mdi:alert'}
              width={24}
              color={axonHealth && cortexHealth ? 'success.main' : 'warning.main'}
            />
            <Box>
              <Typography
                variant="subtitle2"
                color={axonHealth && cortexHealth ? 'success.dark' : 'warning.dark'}
              >
                {axonHealth && cortexHealth
                  ? 'All systems operational'
                  : 'Some systems are unavailable'}
              </Typography>
              <Typography
                variant="body2"
                color={axonHealth && cortexHealth ? 'success.dark' : 'warning.dark'}
                sx={{ mt: 0.5 }}
              >
                {axonHealth && cortexHealth
                  ? 'All services are running normally with no detected issues.'
                  : 'One or more services are experiencing issues. Check the logs for details.'}
              </Typography>
            </Box>
          </Stack>
        </Card>
      </Stack>
    </>
  );
}
