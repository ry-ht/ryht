import type { SystemStats, HealthResponse } from 'src/types/cortex';

import useSWR from 'swr';

import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';

import { cortexFetcher, cortexEndpoints } from 'src/lib/cortex-client';

import { MetricCard } from 'src/sections/overview/components/metric-card';

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
          icon="solar:folder-with-files-bold"
          color="primary"
        />
        <MetricCard
          title="Documents"
          value={stats?.documents_count || 0}
          icon="solar:document-text-bold"
          color="info"
        />
        <MetricCard
          title="Code Units"
          value={stats?.code_units_count || 0}
          icon="solar:code-bold"
          color="warning"
        />
        <MetricCard
          title="Services"
          value={health?.services ? Object.keys(health.services).length : 0}
          icon="solar:database-bold"
          color={health?.status === 'healthy' ? 'success' : 'error'}
        />
      </Box>
    </>
  );
}
