import { useState } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import LinearProgress from '@mui/material/LinearProgress';

import { cortexClient } from 'src/lib/cortex-client';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { useSnackbar } from 'src/components/snackbar';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

export function MemoryConsolidationView() {
  const [isConsolidating, setIsConsolidating] = useState(false);
  const [lastConsolidation, setLastConsolidation] = useState<Date | null>(null);
  const { showSnackbar } = useSnackbar();

  const handleConsolidate = async () => {
    try {
      setIsConsolidating(true);
      await cortexClient.consolidateMemory();
      setLastConsolidation(new Date());
      showSnackbar('Memory consolidation completed successfully', 'success');
    } catch (error) {
      console.error('Consolidation failed:', error);
      showSnackbar('Memory consolidation failed', 'error');
    } finally {
      setIsConsolidating(false);
    }
  };

  return (
    <>
      <CustomBreadcrumbs
        heading="Memory Consolidation"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Memory' },
          { name: 'Consolidation' },
        ]}
        sx={{ mb: 3 }}
      />

      <Stack spacing={2}>
        <Card sx={{ p: 3 }}>
          <Typography variant="h6" sx={{ mb: 1 }}>
            Memory Consolidation Process
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Transfer information from working memory to long-term storage, strengthen important
            connections, and prune less relevant data.
          </Typography>
        </Card>

        <Grid container spacing={2}>
          <Grid size={{ xs: 12, md: 6 }}>
            <Card sx={{ p: 3 }}>
              <Stack spacing={2}>
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
                  <Iconify icon="mdi:database-sync" width={32} />
                </Box>

                <Box>
                  <Typography variant="h6">Consolidation Status</Typography>
                  <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5 }}>
                    {lastConsolidation
                      ? `Last run: ${lastConsolidation.toLocaleString()}`
                      : 'Never run'}
                  </Typography>
                </Box>

                {isConsolidating && (
                  <Box>
                    <Typography variant="body2" sx={{ mb: 1 }}>
                      Consolidating memories...
                    </Typography>
                    <LinearProgress />
                  </Box>
                )}

                <Button
                  variant="contained"
                  size="large"
                  startIcon={<Iconify icon="mdi:play" />}
                  onClick={handleConsolidate}
                  disabled={isConsolidating}
                  fullWidth
                >
                  {isConsolidating ? 'Consolidating...' : 'Start Consolidation'}
                </Button>
              </Stack>
            </Card>
          </Grid>

          <Grid size={{ xs: 12, md: 6 }}>
            <Card sx={{ p: 3 }}>
              <Stack spacing={2}>
                <Typography variant="h6">Consolidation Process</Typography>

                <Stack spacing={1.5}>
                  <Stack direction="row" spacing={1.5} alignItems="flex-start">
                    <Box
                      sx={{
                        width: 32,
                        height: 32,
                        borderRadius: 1,
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        bgcolor: 'success.lighter',
                        color: 'success.main',
                        flexShrink: 0,
                      }}
                    >
                      <Typography variant="subtitle2">1</Typography>
                    </Box>
                    <Box>
                      <Typography variant="subtitle2">Transfer to Long-term</Typography>
                      <Typography variant="body2" color="text.secondary">
                        Move important items from working memory to long-term storage
                      </Typography>
                    </Box>
                  </Stack>

                  <Stack direction="row" spacing={1.5} alignItems="flex-start">
                    <Box
                      sx={{
                        width: 32,
                        height: 32,
                        borderRadius: 1,
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        bgcolor: 'info.lighter',
                        color: 'info.main',
                        flexShrink: 0,
                      }}
                    >
                      <Typography variant="subtitle2">2</Typography>
                    </Box>
                    <Box>
                      <Typography variant="subtitle2">Strengthen Connections</Typography>
                      <Typography variant="body2" color="text.secondary">
                        Reinforce frequently accessed patterns and relationships
                      </Typography>
                    </Box>
                  </Stack>

                  <Stack direction="row" spacing={1.5} alignItems="flex-start">
                    <Box
                      sx={{
                        width: 32,
                        height: 32,
                        borderRadius: 1,
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        bgcolor: 'warning.lighter',
                        color: 'warning.main',
                        flexShrink: 0,
                      }}
                    >
                      <Typography variant="subtitle2">3</Typography>
                    </Box>
                    <Box>
                      <Typography variant="subtitle2">Prune Unused Data</Typography>
                      <Typography variant="body2" color="text.secondary">
                        Remove or archive rarely accessed information
                      </Typography>
                    </Box>
                  </Stack>
                </Stack>
              </Stack>
            </Card>
          </Grid>
        </Grid>

        <Card sx={{ p: 3, bgcolor: 'info.lighter' }}>
          <Stack direction="row" spacing={2} alignItems="flex-start">
            <Iconify icon="mdi:information" width={24} color="info.main" />
            <Box>
              <Typography variant="subtitle2" color="info.dark">
                About Memory Consolidation
              </Typography>
              <Typography variant="body2" color="info.dark" sx={{ mt: 0.5 }}>
                Memory consolidation is typically run periodically (e.g., during idle times or at
                the end of work sessions) to optimize memory usage and improve recall performance.
              </Typography>
            </Box>
          </Stack>
        </Card>
      </Stack>
    </>
  );
}
