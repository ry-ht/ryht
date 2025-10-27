import type { LearnedPattern } from 'src/types/cortex';

import useSWR from 'swr';
import { useState } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Chip from '@mui/material/Chip';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import Alert from '@mui/material/Alert';
import Paper from '@mui/material/Paper';
import Select from '@mui/material/Select';
import Button from '@mui/material/Button';
import Dialog from '@mui/material/Dialog';
import Divider from '@mui/material/Divider';
import MenuItem from '@mui/material/MenuItem';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import InputLabel from '@mui/material/InputLabel';
import CardContent from '@mui/material/CardContent';
import FormControl from '@mui/material/FormControl';
import DialogTitle from '@mui/material/DialogTitle';
import DialogContent from '@mui/material/DialogContent';
import DialogActions from '@mui/material/DialogActions';
import LinearProgress from '@mui/material/LinearProgress';
import CircularProgress from '@mui/material/CircularProgress';

import { cortexClient } from 'src/lib/cortex-client';

import { Iconify } from 'src/components/iconify';
import { Markdown } from 'src/components/markdown';

// ----------------------------------------------------------------------

export function MemoryPatternsView() {
  const [searchQuery, setSearchQuery] = useState('');
  const [typeFilter, setTypeFilter] = useState<string>('all');
  const [sortBy, setSortBy] = useState<'confidence' | 'occurrences' | 'recent'>('confidence');
  const [selectedPattern, setSelectedPattern] = useState<LearnedPattern | null>(null);

  // Fetch patterns
  const { data: patterns, error, isLoading, mutate } = useSWR<LearnedPattern[]>(
    '/memory/patterns',
    () => cortexClient.getLearnedPatterns()
  );

  // Get unique pattern types
  const patternTypes = Array.from(new Set(patterns?.map((p) => p.pattern_type) || []));

  // Filter patterns
  const filteredPatterns = patterns?.filter((pattern) => {
    // Type filter
    if (typeFilter !== 'all' && pattern.pattern_type !== typeFilter) {
      return false;
    }

    // Search filter
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      return (
        pattern.name.toLowerCase().includes(query) ||
        pattern.description.toLowerCase().includes(query) ||
        pattern.pattern_type.toLowerCase().includes(query)
      );
    }

    return true;
  }) || [];

  // Sort patterns
  const sortedPatterns = [...filteredPatterns].sort((a, b) => {
    if (sortBy === 'confidence') {
      return b.confidence - a.confidence;
    }
    if (sortBy === 'occurrences') {
      return b.occurrences - a.occurrences;
    }
    return new Date(b.last_seen).getTime() - new Date(a.last_seen).getTime();
  });

  const getConfidenceColor = (confidence: number) => {
    if (confidence >= 0.7) return 'success';
    if (confidence >= 0.4) return 'warning';
    return 'error';
  };

  const getOccurrencesColor = (occurrences: number) => {
    if (occurrences >= 10) return 'success';
    if (occurrences >= 5) return 'info';
    return 'default';
  };

  return (
    <Box sx={{ p: 3 }}>
      <Stack direction="row" justifyContent="space-between" alignItems="center" sx={{ mb: 3 }}>
        <Typography variant="h4">
          Learned Patterns
        </Typography>
        <Button
          variant="outlined"
          startIcon={<Iconify icon="eva:refresh-fill" />}
          onClick={() => mutate()}
        >
          Refresh
        </Button>
      </Stack>

      {/* Filters */}
      <Card sx={{ p: 3, mb: 3 }}>
        <Stack spacing={3}>
          <TextField
            fullWidth
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search patterns..."
            InputProps={{
              startAdornment: (
                <Iconify
                  icon="eva:search-fill"
                  sx={{ color: 'text.disabled', mr: 1 }}
                />
              ),
            }}
          />

          <Stack direction="row" spacing={2} flexWrap="wrap" useFlexGap>
            <FormControl sx={{ minWidth: 200 }}>
              <InputLabel>Pattern Type</InputLabel>
              <Select
                value={typeFilter}
                onChange={(e) => setTypeFilter(e.target.value)}
                label="Pattern Type"
              >
                <MenuItem value="all">All Types</MenuItem>
                {patternTypes.map((type) => (
                  <MenuItem key={type} value={type}>
                    {type}
                  </MenuItem>
                ))}
              </Select>
            </FormControl>

            <FormControl sx={{ minWidth: 200 }}>
              <InputLabel>Sort By</InputLabel>
              <Select
                value={sortBy}
                onChange={(e) => setSortBy(e.target.value as 'confidence' | 'occurrences' | 'recent')}
                label="Sort By"
              >
                <MenuItem value="confidence">Confidence (Highest First)</MenuItem>
                <MenuItem value="occurrences">Occurrences (Most First)</MenuItem>
                <MenuItem value="recent">Recently Seen</MenuItem>
              </Select>
            </FormControl>
          </Stack>
        </Stack>
      </Card>

      {/* Loading state */}
      {isLoading && (
        <Box sx={{ display: 'flex', justifyContent: 'center', p: 5 }}>
          <CircularProgress />
        </Box>
      )}

      {/* Error state */}
      {error && (
        <Alert severity="error" sx={{ mb: 3 }}>
          {error.message || 'Failed to load patterns'}
        </Alert>
      )}

      {/* Patterns grid */}
      {!isLoading && !error && sortedPatterns.length > 0 && (
        <>
          <Typography variant="subtitle2" color="text.secondary" sx={{ mb: 2 }}>
            {sortedPatterns.length} pattern{sortedPatterns.length !== 1 ? 's' : ''} found
          </Typography>

          <Grid container spacing={3}>
            {sortedPatterns.map((pattern) => (
              <Grid size={{ xs: 12, md: 6, lg: 4 }} key={pattern.id}>
                <Card
                  sx={{
                    height: '100%',
                    cursor: 'pointer',
                    transition: 'all 0.2s',
                    '&:hover': {
                      transform: 'translateY(-4px)',
                      boxShadow: 4,
                    },
                  }}
                  onClick={() => setSelectedPattern(pattern)}
                >
                  <CardContent>
                    <Stack spacing={2}>
                      {/* Header */}
                      <Stack direction="row" justifyContent="space-between" alignItems="flex-start">
                        <Typography variant="h6" sx={{ flexGrow: 1 }}>
                          {pattern.name}
                        </Typography>
                        <Chip
                          label={pattern.pattern_type}
                          size="small"
                          color="secondary"
                        />
                      </Stack>

                      {/* Description */}
                      <Typography
                        variant="body2"
                        color="text.secondary"
                        sx={{
                          minHeight: 60,
                          overflow: 'hidden',
                          textOverflow: 'ellipsis',
                          display: '-webkit-box',
                          WebkitLineClamp: 3,
                          WebkitBoxOrient: 'vertical',
                        }}
                      >
                        {pattern.description}
                      </Typography>

                      <Divider />

                      {/* Confidence score */}
                      <Box>
                        <Stack direction="row" justifyContent="space-between" sx={{ mb: 0.5 }}>
                          <Typography variant="caption" color="text.secondary">
                            Confidence
                          </Typography>
                          <Typography variant="caption" fontWeight="bold">
                            {(pattern.confidence * 100).toFixed(0)}%
                          </Typography>
                        </Stack>
                        <LinearProgress
                          variant="determinate"
                          value={pattern.confidence * 100}
                          color={getConfidenceColor(pattern.confidence)}
                          sx={{ height: 8, borderRadius: 1 }}
                        />
                      </Box>

                      {/* Metrics */}
                      <Stack direction="row" spacing={1} flexWrap="wrap">
                        <Chip
                          icon={<Iconify icon="eva:repeat-fill" />}
                          label={`${pattern.occurrences} occurrence${pattern.occurrences !== 1 ? 's' : ''}`}
                          size="small"
                          color={getOccurrencesColor(pattern.occurrences)}
                          variant="outlined"
                        />
                        <Chip
                          icon={<Iconify icon="eva:clock-outline" />}
                          label={new Date(pattern.last_seen).toLocaleDateString()}
                          size="small"
                          variant="outlined"
                        />
                      </Stack>

                      {/* Example count */}
                      {pattern.examples && pattern.examples.length > 0 && (
                        <Typography variant="caption" color="primary">
                          {pattern.examples.length} code example{pattern.examples.length !== 1 ? 's' : ''}
                        </Typography>
                      )}
                    </Stack>
                  </CardContent>
                </Card>
              </Grid>
            ))}
          </Grid>
        </>
      )}

      {/* Empty state */}
      {!isLoading && !error && sortedPatterns.length === 0 && (
        <Card sx={{ p: 5 }}>
          <Stack spacing={2} alignItems="center">
            <Iconify icon="eva:layers-outline" sx={{ width: 64, height: 64, color: 'text.disabled' }} />
            <Typography variant="h6" color="text.secondary">
              {searchQuery || typeFilter !== 'all' ? 'No patterns match your filters' : 'No patterns found'}
            </Typography>
            {(searchQuery || typeFilter !== 'all') && (
              <Button
                variant="outlined"
                onClick={() => {
                  setSearchQuery('');
                  setTypeFilter('all');
                }}
              >
                Clear Filters
              </Button>
            )}
          </Stack>
        </Card>
      )}

      {/* Pattern Detail Modal */}
      <Dialog
        open={!!selectedPattern}
        onClose={() => setSelectedPattern(null)}
        maxWidth="md"
        fullWidth
      >
        {selectedPattern && (
          <>
            <DialogTitle>
              <Stack spacing={2}>
                <Stack direction="row" justifyContent="space-between" alignItems="center">
                  <Typography variant="h5">{selectedPattern.name}</Typography>
                  <Chip
                    label={selectedPattern.pattern_type}
                    color="secondary"
                  />
                </Stack>
                <Typography variant="body2" color="text.secondary">
                  {selectedPattern.description}
                </Typography>
              </Stack>
            </DialogTitle>
            <DialogContent>
              <Stack spacing={3}>
                {/* Confidence */}
                <Box>
                  <Stack direction="row" justifyContent="space-between" sx={{ mb: 1 }}>
                    <Typography variant="subtitle2">Confidence Score</Typography>
                    <Typography variant="subtitle2" fontWeight="bold">
                      {(selectedPattern.confidence * 100).toFixed(0)}%
                    </Typography>
                  </Stack>
                  <LinearProgress
                    variant="determinate"
                    value={selectedPattern.confidence * 100}
                    color={getConfidenceColor(selectedPattern.confidence)}
                    sx={{ height: 10, borderRadius: 1 }}
                  />
                </Box>

                <Divider />

                {/* Metrics */}
                <Stack direction="row" spacing={2}>
                  <Paper variant="outlined" sx={{ p: 2, flexGrow: 1 }}>
                    <Typography variant="caption" color="text.secondary">
                      Occurrences
                    </Typography>
                    <Typography variant="h4">{selectedPattern.occurrences}</Typography>
                  </Paper>
                  <Paper variant="outlined" sx={{ p: 2, flexGrow: 1 }}>
                    <Typography variant="caption" color="text.secondary">
                      First Seen
                    </Typography>
                    <Typography variant="body1">
                      {new Date(selectedPattern.created_at).toLocaleDateString()}
                    </Typography>
                  </Paper>
                  <Paper variant="outlined" sx={{ p: 2, flexGrow: 1 }}>
                    <Typography variant="caption" color="text.secondary">
                      Last Seen
                    </Typography>
                    <Typography variant="body1">
                      {new Date(selectedPattern.last_seen).toLocaleDateString()}
                    </Typography>
                  </Paper>
                </Stack>

                {/* Code Examples */}
                {selectedPattern.examples && selectedPattern.examples.length > 0 && (
                  <>
                    <Divider />
                    <Box>
                      <Typography variant="subtitle2" sx={{ mb: 2 }}>
                        Code Examples ({selectedPattern.examples.length})
                      </Typography>
                      <Stack spacing={2}>
                        {selectedPattern.examples.map((example, index) => (
                          <Paper key={index} variant="outlined" sx={{ p: 2 }}>
                            <Stack spacing={1}>
                              {example.description && (
                                <Typography variant="body2" color="text.secondary">
                                  {example.description}
                                </Typography>
                              )}
                              {example.code && (
                                <Box
                                  sx={{
                                    '& pre': {
                                      borderRadius: 1,
                                      p: 2,
                                      bgcolor: 'background.neutral',
                                      overflow: 'auto',
                                    },
                                  }}
                                >
                                  <Markdown content={`\`\`\`${example.language || 'typescript'}\n${example.code}\n\`\`\``} />
                                </Box>
                              )}
                              {example.language && (
                                <Chip
                                  label={example.language}
                                  size="small"
                                  variant="outlined"
                                />
                              )}
                            </Stack>
                          </Paper>
                        ))}
                      </Stack>
                    </Box>
                  </>
                )}

                {/* Metadata */}
                {selectedPattern.metadata && Object.keys(selectedPattern.metadata).length > 0 && (
                  <>
                    <Divider />
                    <Box>
                      <Typography variant="subtitle2" sx={{ mb: 1 }}>
                        Metadata
                      </Typography>
                      <Stack direction="row" spacing={1} flexWrap="wrap">
                        {Object.entries(selectedPattern.metadata).map(([key, value]) => (
                          <Chip
                            key={key}
                            label={`${key}: ${value}`}
                            size="small"
                            variant="outlined"
                          />
                        ))}
                      </Stack>
                    </Box>
                  </>
                )}
              </Stack>
            </DialogContent>
            <DialogActions>
              <Button onClick={() => setSelectedPattern(null)}>Close</Button>
            </DialogActions>
          </>
        )}
      </Dialog>
    </Box>
  );
}
