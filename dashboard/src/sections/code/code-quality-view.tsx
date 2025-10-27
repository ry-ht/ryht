import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import LinearProgress from '@mui/material/LinearProgress';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

const QUALITY_METRICS = [
  { name: 'Code Coverage', value: 78, target: 80, unit: '%', icon: 'mdi:shield-check', color: 'warning' },
  { name: 'Test Pass Rate', value: 95, target: 95, unit: '%', icon: 'mdi:test-tube', color: 'success' },
  { name: 'Technical Debt', value: 23, target: 20, unit: 'days', icon: 'mdi:alert', color: 'error' },
  { name: 'Code Duplication', value: 5, target: 10, unit: '%', icon: 'mdi:content-copy', color: 'success' },
  { name: 'Maintainability', value: 85, target: 70, unit: '/100', icon: 'mdi:wrench', color: 'success' },
  { name: 'Security Issues', value: 3, target: 0, unit: '', icon: 'mdi:security', color: 'warning' },
  { name: 'Documentation', value: 72, target: 80, unit: '%', icon: 'mdi:file-document', color: 'warning' },
  { name: 'Code Smells', value: 45, target: 30, unit: '', icon: 'mdi:nose', color: 'error' },
  { name: 'Cyclomatic Complexity', value: 8.5, target: 10, unit: '', icon: 'mdi:chart-line', color: 'success' },
  { name: 'Lines per Function', value: 15, target: 20, unit: '', icon: 'mdi:function', color: 'success' },
  { name: 'Comment Density', value: 12, target: 15, unit: '%', icon: 'mdi:comment-text', color: 'warning' },
  { name: 'Type Coverage', value: 89, target: 90, unit: '%', icon: 'mdi:code-tags', color: 'warning' },
  { name: 'Build Success Rate', value: 98, target: 95, unit: '%', icon: 'mdi:check-circle', color: 'success' },
  { name: 'Linting Errors', value: 12, target: 0, unit: '', icon: 'mdi:alert-circle', color: 'error' },
  { name: 'Code Churn', value: 18, target: 25, unit: '%', icon: 'mdi:refresh', color: 'success' },
  { name: 'Module Coupling', value: 0.45, target: 0.5, unit: '', icon: 'mdi:link', color: 'success' },
  { name: 'Accessibility Score', value: 88, target: 90, unit: '/100', icon: 'mdi:human', color: 'warning' },
  { name: 'Performance Score', value: 92, target: 85, unit: '/100', icon: 'mdi:speedometer', color: 'success' },
  { name: 'Bundle Size', value: 245, target: 300, unit: 'KB', icon: 'mdi:package', color: 'success' },
  { name: 'API Response Time', value: 120, target: 200, unit: 'ms', icon: 'mdi:clock-fast', color: 'success' },
];

export function CodeQualityView() {
  const getMetricColor = (metric: typeof QUALITY_METRICS[0]) => metric.color as any;

  const overallScore = Math.round(
    QUALITY_METRICS.reduce((sum, m) => {
      const normalized = m.unit === '%' ? m.value : (m.value / m.target) * 100;
      return sum + normalized;
    }, 0) / QUALITY_METRICS.length
  );

  return (
    <>
      <CustomBreadcrumbs
        heading="Code Quality Metrics"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Code' },
          { name: 'Quality' },
        ]}
        sx={{ mb: 3 }}
      />

      <Stack spacing={2}>
        <Card sx={{ p: 3 }}>
          <Stack direction="row" justifyContent="space-between" alignItems="center">
            <Box>
              <Typography variant="h6" sx={{ mb: 0.5 }}>
                Code Quality Dashboard
              </Typography>
              <Typography variant="body2" color="text.secondary">
                20+ quality metrics to ensure high code standards and maintainability.
              </Typography>
            </Box>
            <Stack alignItems="center">
              <Typography variant="h3" color="primary.main">
                {overallScore}
              </Typography>
              <Typography variant="caption" color="text.secondary">
                Overall Score
              </Typography>
            </Stack>
          </Stack>
        </Card>

        <Grid container spacing={2}>
          {QUALITY_METRICS.map((metric) => (
            <Grid size={{ xs: 12, sm: 6, md: 4, lg: 3 }} key={metric.name}>
              <Card sx={{ p: 2.5, height: '100%' }}>
                <Stack spacing={1.5}>
                  <Stack direction="row" justifyContent="space-between" alignItems="flex-start">
                    <Box
                      sx={{
                        width: 40,
                        height: 40,
                        borderRadius: 1.5,
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        bgcolor: `${metric.color}.lighter`,
                        color: `${metric.color}.main`,
                      }}
                    >
                      <Iconify icon={metric.icon} width={20} />
                    </Box>
                    <Label variant="soft" color={getMetricColor(metric)}>
                      {metric.value}
                      {metric.unit}
                    </Label>
                  </Stack>

                  <Box>
                    <Typography variant="subtitle2" sx={{ mb: 0.5 }}>
                      {metric.name}
                    </Typography>
                    <Typography variant="caption" color="text.secondary">
                      Target: {metric.target}
                      {metric.unit}
                    </Typography>
                  </Box>

                  <Box>
                    <LinearProgress
                      variant="determinate"
                      value={Math.min((metric.value / metric.target) * 100, 100)}
                      color={getMetricColor(metric)}
                      sx={{ height: 6, borderRadius: 1 }}
                    />
                  </Box>
                </Stack>
              </Card>
            </Grid>
          ))}
        </Grid>

        <Card sx={{ p: 3 }}>
          <Typography variant="h6" sx={{ mb: 2 }}>
            Quality Trends
          </Typography>
          <Grid container spacing={2}>
            <Grid size={{ xs: 4 }}>
              <Stack spacing={0.5}>
                <Stack direction="row" spacing={1} alignItems="center">
                  <Iconify icon="mdi:arrow-up" width={20} color="success.main" />
                  <Typography variant="h6" color="success.main">
                    12
                  </Typography>
                </Stack>
                <Typography variant="body2" color="text.secondary">
                  Metrics Improved
                </Typography>
              </Stack>
            </Grid>
            <Grid size={{ xs: 4 }}>
              <Stack spacing={0.5}>
                <Stack direction="row" spacing={1} alignItems="center">
                  <Iconify icon="mdi:arrow-down" width={20} color="error.main" />
                  <Typography variant="h6" color="error.main">
                    3
                  </Typography>
                </Stack>
                <Typography variant="body2" color="text.secondary">
                  Metrics Declined
                </Typography>
              </Stack>
            </Grid>
            <Grid size={{ xs: 4 }}>
              <Stack spacing={0.5}>
                <Stack direction="row" spacing={1} alignItems="center">
                  <Iconify icon="mdi:minus" width={20} color="text.secondary" />
                  <Typography variant="h6">5</Typography>
                </Stack>
                <Typography variant="body2" color="text.secondary">
                  Unchanged
                </Typography>
              </Stack>
            </Grid>
          </Grid>
        </Card>
      </Stack>
    </>
  );
}
