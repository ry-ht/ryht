import type { Breakpoint } from '@mui/material/styles';
import type { HealthResponse as AxonHealthResponse } from 'src/types/axon';
import type { HealthResponse as CortexHealthResponse } from 'src/types/cortex';

import useSWR from 'swr';
import { useState, useEffect } from 'react';

import Box from '@mui/material/Box';
import Link from '@mui/material/Link';
import Stack from '@mui/material/Stack';
import Divider from '@mui/material/Divider';
import Container from '@mui/material/Container';
import Typography from '@mui/material/Typography';
import { alpha, useTheme } from '@mui/material/styles';

import { fTime } from 'src/utils/format-time';

import { axonClient } from 'src/lib/axon-client';
import { cortexClient } from 'src/lib/cortex-client';
import { axonWebSocket } from 'src/lib/axon-websocket';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { useSettingsContext } from 'src/components/settings';

// ----------------------------------------------------------------------

export type FooterProps = {
  layoutQuery?: Breakpoint;
};

export function Footer({ layoutQuery = 'lg' }: FooterProps) {
  const theme = useTheme();
  const settings = useSettingsContext();

  const isNavMini = settings.state.navLayout === 'mini';
  const isNavVertical = isNavMini || settings.state.navLayout === 'vertical';
  const [currentTime, setCurrentTime] = useState(new Date());
  const [wsConnected, setWsConnected] = useState(false);

  // Fetch health data
  const { data: axonHealth, error: axonError } = useSWR<AxonHealthResponse | null>(
    '/health',
    async () => {
      try {
        return await axonClient.getHealth();
      } catch (error) {
        console.error('Failed to fetch Axon health:', error);
        return null;
      }
    },
    { refreshInterval: 10000, revalidateOnFocus: false }
  );

  const { data: cortexHealth, error: cortexError } = useSWR<CortexHealthResponse | null>(
    '/cortex/health',
    async () => {
      try {
        return await cortexClient.getHealth();
      } catch (error) {
        console.error('Failed to fetch Cortex health:', error);
        return null;
      }
    },
    { refreshInterval: 10000, revalidateOnFocus: false }
  );

  // Update current time every second
  useEffect(() => {
    const timer = setInterval(() => {
      setCurrentTime(new Date());
    }, 1000);

    return () => clearInterval(timer);
  }, []);

  // Monitor WebSocket connection
  useEffect(() => {
    const checkConnection = setInterval(() => {
      setWsConnected(axonWebSocket.isConnected());
    }, 1000);

    return () => clearInterval(checkConnection);
  }, []);

  // Calculate system health status - more robust logic
  const axonHealthy = !axonError && (axonHealth?.status === 'healthy' || axonHealth?.status === 'ok');
  const cortexHealthy = !cortexError && cortexHealth?.status === 'healthy';

  // System is healthy only if both services are explicitly healthy
  // If data is still loading or there's an error, show degraded
  const systemStatus = axonHealthy && cortexHealthy ? 'healthy' : 'degraded';

  const getStatusColor = (status: string) => {
    if (status === 'healthy') return theme.palette.success.main;
    if (status === 'degraded') return theme.palette.warning.main;
    return theme.palette.error.main;
  };

  const formatUptime = (seconds: number): string => {
    if (!seconds) return 'N/A';
    if (seconds < 60) return `${seconds}s`;

    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m`;

    const hours = Math.floor(minutes / 60);
    const remainingMinutes = minutes % 60;
    if (hours < 24) return `${hours}h ${remainingMinutes}m`;

    const days = Math.floor(hours / 24);
    const remainingHours = hours % 24;
    return `${days}d ${remainingHours}h`;
  };

  return (
    <Box
      component="footer"
      sx={{
        position: 'fixed',
        bottom: 0,
        left: 0,
        right: 0,
        height: 48,
        bgcolor: alpha(theme.palette.background.default, 0.8),
        backdropFilter: 'blur(8px)',
        borderTop: `1px solid ${alpha(theme.palette.divider, 0.1)}`,
        zIndex: theme.zIndex.appBar - 1,
        transition: theme.transitions.create(
          ['background-color', 'border-color', 'padding-left'],
          {
            easing: 'var(--layout-transition-easing)',
            duration: 'var(--layout-transition-duration)',
          }
        ),
        // Adapt to sidebar layout - same logic as the sidebarContainer
        ...(isNavVertical && {
          [theme.breakpoints.up(layoutQuery)]: {
            pl: isNavMini ? 'var(--layout-nav-mini-width)' : 'var(--layout-nav-vertical-width)',
          },
        }),
      }}
    >
      <Container maxWidth={false} sx={{ height: '100%' }}>
        <Stack
          direction="row"
          alignItems="center"
          justifyContent="space-between"
          spacing={2}
          sx={{ height: '100%' }}
        >
          {/* Left section - System Status */}
          <Stack direction="row" alignItems="center" spacing={2} divider={<Divider orientation="vertical" flexItem />}>
            {/* WebSocket Status */}
            <Stack direction="row" alignItems="center" spacing={1}>
              <Box
                sx={{
                  width: 8,
                  height: 8,
                  borderRadius: '50%',
                  bgcolor: wsConnected ? theme.palette.success.main : theme.palette.error.main,
                  boxShadow: `0 0 8px ${wsConnected ? theme.palette.success.main : theme.palette.error.main}`,
                  animation: wsConnected ? 'pulse 2s ease-in-out infinite' : 'none',
                  '@keyframes pulse': {
                    '0%, 100%': { opacity: 1 },
                    '50%': { opacity: 0.5 },
                  },
                }}
              />
              <Typography variant="caption" color="text.secondary">
                {wsConnected ? 'Connected' : 'Disconnected'}
              </Typography>
            </Stack>

            {/* System Health */}
            <Stack direction="row" alignItems="center" spacing={1}>
              <Iconify
                icon="solar:shield-check-bold"
                width={16}
                sx={{ color: getStatusColor(systemStatus) }}
              />
              <Typography variant="caption" color="text.secondary">
                System:{' '}
                <Typography
                  component="span"
                  variant="caption"
                  fontWeight="bold"
                  sx={{ color: getStatusColor(systemStatus) }}
                >
                  {systemStatus === 'healthy' ? 'Healthy' : 'Degraded'}
                </Typography>
              </Typography>
            </Stack>
          </Stack>

          {/* Center section - Service Pills */}
          <Stack direction="row" alignItems="center" spacing={1.5}>
            {/* Axon Service */}
            <Label
              variant="soft"
              color={axonHealthy ? 'success' : 'error'}
              sx={{
                px: 1,
                py: 0.5,
                height: 24,
                fontSize: '0.6875rem',
              }}
            >
              <Stack direction="row" alignItems="center" spacing={0.5}>
                <Iconify icon="solar:cpu-bolt-bold" width={12} />
                <span>Axon</span>
                {axonHealth?.version && (
                  <>
                    <Divider orientation="vertical" flexItem sx={{ mx: 0.5, height: 12, alignSelf: 'center' }} />
                    <span style={{ opacity: 0.7 }}>{axonHealth.version}</span>
                  </>
                )}
                {axonHealth?.uptime_seconds !== undefined && (
                  <>
                    <Divider orientation="vertical" flexItem sx={{ mx: 0.5, height: 12, alignSelf: 'center' }} />
                    <span style={{ opacity: 0.7 }}>{formatUptime(axonHealth.uptime_seconds)}</span>
                  </>
                )}
              </Stack>
            </Label>

            {/* Cortex Service */}
            <Label
              variant="soft"
              color={cortexHealthy ? 'success' : 'error'}
              sx={{
                px: 1,
                py: 0.5,
                height: 24,
                fontSize: '0.6875rem',
              }}
            >
              <Stack direction="row" alignItems="center" spacing={0.5}>
                <Iconify icon="solar:database-bold" width={12} />
                <span>Cortex</span>
                {cortexHealth?.version && (
                  <>
                    <Divider orientation="vertical" flexItem sx={{ mx: 0.5, height: 12, alignSelf: 'center' }} />
                    <span style={{ opacity: 0.7 }}>{cortexHealth.version}</span>
                  </>
                )}
                {cortexHealth?.uptime !== undefined && (
                  <>
                    <Divider orientation="vertical" flexItem sx={{ mx: 0.5, height: 12, alignSelf: 'center' }} />
                    <span style={{ opacity: 0.7 }}>{formatUptime(cortexHealth.uptime)}</span>
                  </>
                )}
              </Stack>
            </Label>

            {/* Active Agents */}
            {axonHealth?.active_agents !== undefined && (
              <Label
                variant="soft"
                color="info"
                sx={{
                  px: 1,
                  py: 0.5,
                  height: 24,
                  fontSize: '0.6875rem',
                }}
              >
                <Stack direction="row" alignItems="center" spacing={0.5}>
                  <Iconify icon="solar:users-group-rounded-bold" width={12} />
                  <span>{axonHealth.active_agents} agents</span>
                </Stack>
              </Label>
            )}

            {/* Running Workflows */}
            {axonHealth?.running_workflows !== undefined && (
              <Label
                variant="soft"
                color="warning"
                sx={{
                  px: 1,
                  py: 0.5,
                  height: 24,
                  fontSize: '0.6875rem',
                }}
              >
                <Stack direction="row" alignItems="center" spacing={0.5}>
                  <Iconify icon="solar:widget-5-bold" width={12} />
                  <span>{axonHealth.running_workflows} workflows</span>
                </Stack>
              </Label>
            )}
          </Stack>

          {/* Right section - Time and Links */}
          <Stack direction="row" alignItems="center" spacing={2} divider={<Divider orientation="vertical" flexItem />}>
            {/* Current Time */}
            <Stack direction="row" alignItems="center" spacing={1}>
              <Iconify icon="solar:clock-circle-bold" width={14} sx={{ color: 'text.secondary' }} />
              <Typography variant="caption" color="text.secondary" fontFamily="monospace">
                {fTime(currentTime, 'HH:mm:ss')}
              </Typography>
            </Stack>

            {/* Links */}
            <Stack direction="row" alignItems="center" spacing={1.5}>
              <Link
                href="https://github.com/yourusername/ryht"
                target="_blank"
                rel="noopener"
                underline="none"
                sx={{
                  color: 'text.secondary',
                  display: 'flex',
                  alignItems: 'center',
                  transition: theme.transitions.create(['color']),
                  '&:hover': {
                    color: 'primary.main',
                  },
                }}
              >
                <Typography variant="caption">Docs</Typography>
              </Link>
              <Link
                href="https://github.com/yourusername/ryht/issues"
                target="_blank"
                rel="noopener"
                underline="none"
                sx={{
                  color: 'text.secondary',
                  display: 'flex',
                  alignItems: 'center',
                  transition: theme.transitions.create(['color']),
                  '&:hover': {
                    color: 'primary.main',
                  },
                }}
              >
                <Typography variant="caption">Support</Typography>
              </Link>
            </Stack>
          </Stack>
        </Stack>
      </Container>
    </Box>
  );
}
