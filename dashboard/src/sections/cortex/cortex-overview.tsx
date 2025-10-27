import Grid from '@mui/material/Grid2';
import Typography from '@mui/material/Typography';

import { AnalyticsWidgetSummary } from 'src/sections/overview/analytics-widget-summary';
import { AnalyticsCurrentVisits } from 'src/sections/overview/analytics-current-visits';

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

      <Grid container spacing={3}>
        <Grid size={{ xs: 12, sm: 6, md: 3 }}>
          <AnalyticsWidgetSummary
            title="Workspaces"
            total={stats?.workspaces_count || 0}
            icon={<img alt="icon" src="/assets/icons/glass/ic-glass-bag.svg" />}
          />
        </Grid>

        <Grid size={{ xs: 12, sm: 6, md: 3 }}>
          <AnalyticsWidgetSummary
            title="Documents"
            total={stats?.documents_count || 0}
            color="info"
            icon={<img alt="icon" src="/assets/icons/glass/ic-glass-users.svg" />}
          />
        </Grid>

        <Grid size={{ xs: 12, sm: 6, md: 3 }}>
          <AnalyticsWidgetSummary
            title="Code Units"
            total={stats?.code_units_count || 0}
            color="warning"
            icon={<img alt="icon" src="/assets/icons/glass/ic-glass-buy.svg" />}
          />
        </Grid>

        <Grid size={{ xs: 12, sm: 6, md: 3 }}>
          <AnalyticsWidgetSummary
            title="Status"
            total={0}
            color={health?.status === 'healthy' ? 'success' : 'error'}
            icon={<img alt="icon" src="/assets/icons/glass/ic-glass-message.svg" />}
          />
        </Grid>
      </Grid>
    </>
  );
}
