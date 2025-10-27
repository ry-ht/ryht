import useSWR from 'swr';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import LinearProgress from '@mui/material/LinearProgress';

import { cortexFetcher } from 'src/lib/cortex-client';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

export function MemoryPatternsView() {
  const { data: patterns = [], isLoading } = useSWR(
    '/api/v1/memory/patterns',
    cortexFetcher,
    { refreshInterval: 30000 }
  );

  return (
    <>
      <CustomBreadcrumbs
        heading="Learned Patterns"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Memory' },
          { name: 'Patterns' },
        ]}
        sx={{ mb: 3 }}
      />

      <Stack spacing={2}>
        <Card sx={{ p: 3 }}>
          <Typography variant="h6" sx={{ mb: 1 }}>
            Pattern Library
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Learned patterns from past experiences that can be applied to new situations.
          </Typography>
        </Card>

        {isLoading && <LinearProgress />}

        <Grid container spacing={2}>
          {patterns.map((pattern: any) => (
            <Grid size={{ xs: 12, md: 6, lg: 4 }} key={pattern.id}>
              <Card sx={{ p: 2.5, height: '100%' }}>
                <Stack spacing={2}>
                  <Stack direction="row" justifyContent="space-between" alignItems="flex-start">
                    <Box>
                      <Typography variant="h6">{pattern.name || 'Untitled Pattern'}</Typography>
                      <Typography variant="caption" color="text.disabled">
                        Pattern #{pattern.id}
                      </Typography>
                    </Box>
                    <Label variant="soft" color="success">
                      {pattern.confidence ? `${(pattern.confidence * 100).toFixed(0)}%` : 'N/A'}
                    </Label>
                  </Stack>

                  <Typography variant="body2" color="text.secondary">
                    {pattern.description || 'No description available'}
                  </Typography>

                  <Stack spacing={1}>
                    <Stack direction="row" spacing={1} alignItems="center">
                      <Iconify icon="mdi:chart-line" width={16} color="text.secondary" />
                      <Typography variant="caption" color="text.secondary">
                        Applied {pattern.usage_count || 0} times
                      </Typography>
                    </Stack>
                    <Stack direction="row" spacing={1} alignItems="center">
                      <Iconify icon="mdi:clock" width={16} color="text.secondary" />
                      <Typography variant="caption" color="text.secondary">
                        Last used: {pattern.last_used ? new Date(pattern.last_used).toLocaleDateString() : 'Never'}
                      </Typography>
                    </Stack>
                  </Stack>

                  {pattern.tags && pattern.tags.length > 0 && (
                    <Stack direction="row" spacing={0.5} flexWrap="wrap" gap={0.5}>
                      {pattern.tags.map((tag: string) => (
                        <Label key={tag} variant="soft">
                          {tag}
                        </Label>
                      ))}
                    </Stack>
                  )}
                </Stack>
              </Card>
            </Grid>
          ))}
        </Grid>

        {!isLoading && patterns.length === 0 && (
          <Card sx={{ p: 5, textAlign: 'center' }}>
            <Iconify icon="mdi:lightbulb-outline" width={64} sx={{ color: 'text.disabled', mb: 2 }} />
            <Typography variant="h6" color="text.secondary">
              No patterns learned yet
            </Typography>
            <Typography variant="body2" color="text.disabled" sx={{ mt: 1 }}>
              Patterns will be identified and stored as the system learns from experiences
            </Typography>
          </Card>
        )}
      </Stack>
    </>
  );
}
