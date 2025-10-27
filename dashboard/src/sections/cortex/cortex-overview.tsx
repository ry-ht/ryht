import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Stack from '@mui/material/Stack';

import { MetricCard } from 'src/sections/overview/components/metric-card';

import useSWR from 'swr';
import { cortexFetcher, cortexEndpoints } from 'src/lib/cortex-client';
import type { HealthResponse, SystemStats } from 'src/types/cortex';

export function CortexOverview() {
  const { data: health } = useSWR<HealthResponse>(
    cortexEndpoints.health,
    cortexFetcher,
    { refreshInterval: 5000 }
  );

  const { data: stats } = useSWR<SystemStats>(
    cortexEndpoints.systemStats,
    cortexFetcher,
    { refreshInterval: 10000 }
  );

  return (
    <>
      <Typography variant="h4" sx={{ mb: 3 }}>
        Cortex Cognitive System
      </Typography>

      <Box sx={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(250px, 1fr))', gap: 3 }}>
        <MetricCard
          title="Workspaces"
          value={stats?.workspaces_count || 0}
        />
        <MetricCard
          title="Documents"
          value={stats?.documents_count || 0}
        />
        <MetricCard
          title="Code Units"
          value={stats?.code_units_count || 0}
        />
        <MetricCard
          title="Status"
          value={health?.status === 'healthy' ? 'Healthy' : 'Unhealthy'}
        />
      </Box>
    </>
  );
}
