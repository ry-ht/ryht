import { useState } from 'react';
import useSWR from 'swr';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import CardContent from '@mui/material/CardContent';
import Typography from '@mui/material/Typography';
import Stack from '@mui/material/Stack';
import Chip from '@mui/material/Chip';
import CircularProgress from '@mui/material/CircularProgress';
import Alert from '@mui/material/Alert';
import Divider from '@mui/material/Divider';
import Button from '@mui/material/Button';
import Paper from '@mui/material/Paper';
import Grid from '@mui/material/Grid';
import LinearProgress from '@mui/material/LinearProgress';

import { Iconify } from 'src/components/iconify';

import { cortexClient } from 'src/lib/cortex-client';
import type { ConsolidationResult } from 'src/types/cortex';

// ----------------------------------------------------------------------

export function MemoryConsolidationView() {
  const [isConsolidating, setIsConsolidating] = useState(false);
  const [consolidationError, setConsolidationError] = useState<string | null>(null);
  const [latestResult, setLatestResult] = useState<ConsolidationResult | null>(null);

  // In a real implementation, this would fetch consolidation history
  // For now, we'll use a mock endpoint that may not exist
  const { data: history, error: historyError, mutate } = useSWR<ConsolidationResult[]>(
    '/memory/consolidation/history',
    async () => {
      try {
        // This endpoint might not exist, so we'll handle the error gracefully
        const response = await cortexClient.consolidateMemory();
        // If the API returns a single result, wrap it in an array
        if (response && !Array.isArray(response)) {
          return [response];
        }
        return response || [];
      } catch (err) {
        // Return empty array if endpoint doesn't exist
        return [];
      }
    },
    {
      revalidateOnFocus: false,
      revalidateOnReconnect: false,
      shouldRetryOnError: false,
    }
  );

  const handleConsolidate = async () => {
    setIsConsolidating(true);
    setConsolidationError(null);

    try {
      const result = await cortexClient.consolidateMemory();
      setLatestResult(result);

      // Refresh history
      mutate();

      // Show success message
      setTimeout(() => {
        setLatestResult(null);
      }, 10000); // Clear after 10 seconds
    } catch (err) {
      setConsolidationError(err instanceof Error ? err.message : 'Consolidation failed');
    } finally {
      setIsConsolidating(false);
    }
  };

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${(ms / 60000).toFixed(1)}m`;
  };

  const getStatusColor = (status: ConsolidationResult['status']) => {
    switch (status) {
      case 'completed':
        return 'success';
      case 'failed':
        return 'error';
      case 'in_progress':
        return 'warning';
      default:
        return 'default';
    }
  };

  const consolidationHistory = history || [];

  return (
    <Box sx={{ p: 3 }}>
      <Typography variant="h4" sx={{ mb: 3 }}>
        Memory Consolidation
      </Typography>

      {/* Consolidation Control */}
      <Card sx={{ p: 3, mb: 3 }}>
        <Stack spacing={3}>
          <Box>
            <Typography variant="h6" sx={{ mb: 1 }}>
              Run Consolidation
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Consolidate memory episodes into patterns, decay old memories, and merge duplicates.
              This process helps optimize memory storage and improve pattern recognition.
            </Typography>
          </Box>

          <Divider />

          <Stack direction="row" spacing={2} alignItems="center">
            <Button
              variant="contained"
              size="large"
              onClick={handleConsolidate}
              disabled={isConsolidating}
              startIcon={
                isConsolidating ? (
                  <CircularProgress size={20} color="inherit" />
                ) : (
                  <Iconify icon="eva:flash-fill" />
                )
              }
            >
              {isConsolidating ? 'Consolidating...' : 'Start Consolidation'}
            </Button>

            {isConsolidating && (
              <Typography variant="body2" color="text.secondary">
                This may take a few moments...
              </Typography>
            )}
          </Stack>

          {isConsolidating && (
            <LinearProgress />
          )}
        </Stack>
      </Card>

      {/* Consolidation Error */}
      {consolidationError && (
        <Alert severity="error" sx={{ mb: 3 }} onClose={() => setConsolidationError(null)}>
          {consolidationError}
        </Alert>
      )}

      {/* Latest Result */}
      {latestResult && (
        <Alert
          severity={latestResult.status === 'completed' ? 'success' : 'error'}
          sx={{ mb: 3 }}
          onClose={() => setLatestResult(null)}
        >
          <Typography variant="subtitle2" sx={{ mb: 1 }}>
            Consolidation {latestResult.status === 'completed' ? 'Completed' : 'Failed'}
          </Typography>
          <Grid container spacing={2} sx={{ mt: 1 }}>
            <Grid size={{ xs: 6, sm: 3 }}>
              <Typography variant="caption" color="text.secondary">
                Episodes Processed
              </Typography>
              <Typography variant="h6">{latestResult.episodes_processed}</Typography>
            </Grid>
            <Grid size={{ xs: 6, sm: 3 }}>
              <Typography variant="caption" color="text.secondary">
                Patterns Extracted
              </Typography>
              <Typography variant="h6">{latestResult.patterns_extracted}</Typography>
            </Grid>
            <Grid size={{ xs: 6, sm: 3 }}>
              <Typography variant="caption" color="text.secondary">
                Memories Decayed
              </Typography>
              <Typography variant="h6">{latestResult.memories_decayed}</Typography>
            </Grid>
            <Grid size={{ xs: 6, sm: 3 }}>
              <Typography variant="caption" color="text.secondary">
                Duplicates Merged
              </Typography>
              <Typography variant="h6">{latestResult.duplicates_merged}</Typography>
            </Grid>
          </Grid>
          <Typography variant="caption" color="text.secondary" sx={{ mt: 2, display: 'block' }}>
            Duration: {formatDuration(latestResult.duration_ms)}
          </Typography>
        </Alert>
      )}

      {/* Consolidation History */}
      <Box>
        <Stack direction="row" justifyContent="space-between" alignItems="center" sx={{ mb: 2 }}>
          <Typography variant="h6">
            Consolidation History
          </Typography>
          <Button
            variant="outlined"
            startIcon={<Iconify icon="eva:refresh-fill" />}
            onClick={() => mutate()}
            size="small"
          >
            Refresh
          </Button>
        </Stack>

        {historyError && !consolidationHistory.length && (
          <Alert severity="info" sx={{ mb: 3 }}>
            Consolidation history is not available. Run a consolidation to see results here.
          </Alert>
        )}

        {consolidationHistory.length > 0 ? (
          <Stack spacing={2}>
            {consolidationHistory.map((result) => (
              <Card key={result.id}>
                <CardContent>
                  <Stack spacing={2}>
                    {/* Header */}
                    <Stack direction="row" justifyContent="space-between" alignItems="center">
                      <Stack direction="row" spacing={2} alignItems="center">
                        <Chip
                          label={result.status}
                          size="small"
                          color={getStatusColor(result.status)}
                        />
                        <Typography variant="caption" color="text.secondary">
                          {new Date(result.started_at).toLocaleString()}
                        </Typography>
                      </Stack>
                      <Typography variant="caption" color="text.secondary">
                        Duration: {formatDuration(result.duration_ms)}
                      </Typography>
                    </Stack>

                    <Divider />

                    {/* Metrics */}
                    <Grid container spacing={2}>
                      <Grid size={{ xs: 6, sm: 3 }}>
                        <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                          <Typography variant="h4" color="primary">
                            {result.episodes_processed}
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            Episodes Processed
                          </Typography>
                        </Paper>
                      </Grid>
                      <Grid size={{ xs: 6, sm: 3 }}>
                        <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                          <Typography variant="h4" color="secondary">
                            {result.patterns_extracted}
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            Patterns Extracted
                          </Typography>
                        </Paper>
                      </Grid>
                      <Grid size={{ xs: 6, sm: 3 }}>
                        <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                          <Typography variant="h4" color="warning.main">
                            {result.memories_decayed}
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            Memories Decayed
                          </Typography>
                        </Paper>
                      </Grid>
                      <Grid size={{ xs: 6, sm: 3 }}>
                        <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                          <Typography variant="h4" color="info.main">
                            {result.duplicates_merged}
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            Duplicates Merged
                          </Typography>
                        </Paper>
                      </Grid>
                    </Grid>

                    {/* Error message if failed */}
                    {result.status === 'failed' && result.error && (
                      <>
                        <Divider />
                        <Alert severity="error">
                          {result.error}
                        </Alert>
                      </>
                    )}

                    {/* Completion time */}
                    {result.completed_at && (
                      <Typography variant="caption" color="text.secondary">
                        Completed: {new Date(result.completed_at).toLocaleString()}
                      </Typography>
                    )}
                  </Stack>
                </CardContent>
              </Card>
            ))}
          </Stack>
        ) : (
          !historyError && (
            <Card sx={{ p: 5 }}>
              <Stack spacing={2} alignItems="center">
                <Iconify icon="eva:clock-outline" sx={{ width: 64, height: 64, color: 'text.disabled' }} />
                <Typography variant="h6" color="text.secondary">
                  No consolidation history yet
                </Typography>
                <Typography variant="body2" color="text.secondary" textAlign="center">
                  Run your first consolidation to see results here
                </Typography>
              </Stack>
            </Card>
          )
        )}
      </Box>

      {/* Info Card */}
      <Card sx={{ p: 3, mt: 3, bgcolor: 'background.neutral' }}>
        <Stack direction="row" spacing={2}>
          <Iconify icon="eva:info-outline" sx={{ width: 24, height: 24, color: 'info.main', flexShrink: 0 }} />
          <Box>
            <Typography variant="subtitle2" sx={{ mb: 1 }}>
              About Memory Consolidation
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Memory consolidation is a process that:
            </Typography>
            <Box component="ul" sx={{ mt: 1, pl: 2 }}>
              <Typography component="li" variant="body2" color="text.secondary">
                Processes memory episodes to extract recurring patterns
              </Typography>
              <Typography component="li" variant="body2" color="text.secondary">
                Decays old or less important memories to optimize storage
              </Typography>
              <Typography component="li" variant="body2" color="text.secondary">
                Merges duplicate memories to reduce redundancy
              </Typography>
              <Typography component="li" variant="body2" color="text.secondary">
                Improves pattern recognition and memory efficiency
              </Typography>
            </Box>
            <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
              Run consolidation periodically to keep your memory system optimized.
            </Typography>
          </Box>
        </Stack>
      </Card>
    </Box>
  );
}
