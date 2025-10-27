import type { HealthResponse as AxonHealthResponse } from 'src/types/axon';
import type { HealthResponse as CortexHealthResponse } from 'src/types/cortex';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import Divider from '@mui/material/Divider';
import CardHeader from '@mui/material/CardHeader';
import Typography from '@mui/material/Typography';
import { alpha, useTheme } from '@mui/material/styles';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';

import { StatusIndicator } from './status-indicator';

// ----------------------------------------------------------------------

interface SystemHealthSectionProps {
  axonHealth?: AxonHealthResponse;
  cortexHealth?: CortexHealthResponse;
  axonHealthy: boolean;
  cortexHealthy: boolean;
}

export function SystemHealthSection({
  axonHealth,
  cortexHealth,
  axonHealthy,
  cortexHealthy,
}: SystemHealthSectionProps) {
  const theme = useTheme();

  const overallStatus = axonHealthy && cortexHealthy ? 'healthy' : 'degraded';

  return (
    <Card>
      <CardHeader
        title="System Health"
        subheader="Overall system and service status"
        avatar={<StatusIndicator status={overallStatus} size={12} />}
      />
      <Divider />
      <Box sx={{ p: 3 }}>
        <Grid container spacing={3}>
          {/* Axon Service */}
          <Grid size={{ xs: 12, md: 6 }}>
            <Stack
              spacing={2}
              sx={{
                p: 2,
                borderRadius: 2,
                bgcolor: alpha(
                  axonHealthy ? theme.palette.success.main : theme.palette.error.main,
                  0.08
                ),
                border: `1px solid ${alpha(
                  axonHealthy ? theme.palette.success.main : theme.palette.error.main,
                  0.24
                )}`,
              }}
            >
              <Stack direction="row" alignItems="center" spacing={2}>
                <Box
                  sx={{
                    width: 48,
                    height: 48,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    borderRadius: 1.5,
                    bgcolor: alpha(theme.palette.info.main, 0.16),
                  }}
                >
                  <Iconify
                    icon="solar:cpu-bolt-bold"
                    width={28}
                    sx={{ color: theme.palette.info.main }}
                  />
                </Box>
                <Box sx={{ flexGrow: 1 }}>
                  <Typography variant="h6">Axon Multi-Agent System</Typography>
                  <Typography variant="caption" color="text.secondary">
                    Agent orchestration and workflow execution
                  </Typography>
                </Box>
                <Label color={axonHealthy ? 'success' : 'error'}>
                  {axonHealth?.status || 'Unknown'}
                </Label>
              </Stack>

              <Divider />

              <Grid container spacing={2}>
                <Grid size={{ xs: 6 }}>
                  <Stack spacing={0.5}>
                    <Typography variant="caption" color="text.secondary">
                      Version
                    </Typography>
                    <Typography variant="body2" fontWeight="bold">
                      {axonHealth?.version || 'N/A'}
                    </Typography>
                  </Stack>
                </Grid>
                <Grid size={{ xs: 6 }}>
                  <Stack spacing={0.5}>
                    <Typography variant="caption" color="text.secondary">
                      Active Agents
                    </Typography>
                    <Typography variant="body2" fontWeight="bold">
                      {axonHealth?.active_agents || 0}
                    </Typography>
                  </Stack>
                </Grid>
                <Grid size={{ xs: 6 }}>
                  <Stack spacing={0.5}>
                    <Typography variant="caption" color="text.secondary">
                      Running Workflows
                    </Typography>
                    <Typography variant="body2" fontWeight="bold">
                      {axonHealth?.running_workflows || 0}
                    </Typography>
                  </Stack>
                </Grid>
                <Grid size={{ xs: 6 }}>
                  <Stack spacing={0.5}>
                    <Typography variant="caption" color="text.secondary">
                      WebSocket Connections
                    </Typography>
                    <Typography variant="body2" fontWeight="bold">
                      {axonHealth?.websocket_connections || 0}
                    </Typography>
                  </Stack>
                </Grid>
              </Grid>
            </Stack>
          </Grid>

          {/* Cortex Service */}
          <Grid size={{ xs: 12, md: 6 }}>
            <Stack
              spacing={2}
              sx={{
                p: 2,
                borderRadius: 2,
                bgcolor: alpha(
                  cortexHealthy ? theme.palette.success.main : theme.palette.error.main,
                  0.08
                ),
                border: `1px solid ${alpha(
                  cortexHealthy ? theme.palette.success.main : theme.palette.error.main,
                  0.24
                )}`,
              }}
            >
              <Stack direction="row" alignItems="center" spacing={2}>
                <Box
                  sx={{
                    width: 48,
                    height: 48,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    borderRadius: 1.5,
                    bgcolor: alpha(theme.palette.success.main, 0.16),
                  }}
                >
                  <Iconify
                    icon="solar:database-bold"
                    width={28}
                    sx={{ color: theme.palette.success.main }}
                  />
                </Box>
                <Box sx={{ flexGrow: 1 }}>
                  <Typography variant="h6">Cortex Cognitive System</Typography>
                  <Typography variant="caption" color="text.secondary">
                    Knowledge base and memory management
                  </Typography>
                </Box>
                <Label color={cortexHealthy ? 'success' : 'error'}>
                  {cortexHealth?.status || 'Unknown'}
                </Label>
              </Stack>

              <Divider />

              <Grid container spacing={2}>
                <Grid size={{ xs: 6 }}>
                  <Stack spacing={0.5}>
                    <Typography variant="caption" color="text.secondary">
                      Version
                    </Typography>
                    <Typography variant="body2" fontWeight="bold">
                      {cortexHealth?.version || 'N/A'}
                    </Typography>
                  </Stack>
                </Grid>
                <Grid size={{ xs: 6 }}>
                  <Stack spacing={0.5}>
                    <Typography variant="caption" color="text.secondary">
                      Uptime
                    </Typography>
                    <Typography variant="body2" fontWeight="bold">
                      {formatUptime(cortexHealth?.uptime || 0)}
                    </Typography>
                  </Stack>
                </Grid>
                <Grid size={{ xs: 12 }}>
                  <Stack spacing={0.5}>
                    <Typography variant="caption" color="text.secondary">
                      Services
                    </Typography>
                    <Stack direction="row" spacing={1} flexWrap="wrap">
                      {cortexHealth?.services && (
                        <>
                          <Label
                            variant="soft"
                            color={cortexHealth.services.database === 'healthy' ? 'success' : 'error'}
                          >
                            Database
                          </Label>
                          <Label
                            variant="soft"
                            color={cortexHealth.services.vfs === 'healthy' ? 'success' : 'error'}
                          >
                            VFS
                          </Label>
                          <Label
                            variant="soft"
                            color={cortexHealth.services.mcp === 'healthy' ? 'success' : 'error'}
                          >
                            MCP
                          </Label>
                        </>
                      )}
                    </Stack>
                  </Stack>
                </Grid>
              </Grid>
            </Stack>
          </Grid>
        </Grid>
      </Box>
    </Card>
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
