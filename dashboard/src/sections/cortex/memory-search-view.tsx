import type { Dayjs } from 'dayjs';
import type { EpisodeType, MemorySearchResult } from 'src/types/cortex';

import { useState, useCallback } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Chip from '@mui/material/Chip';
import Stack from '@mui/material/Stack';
import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import Dialog from '@mui/material/Dialog';
import Select from '@mui/material/Select';
import Divider from '@mui/material/Divider';
import MenuItem from '@mui/material/MenuItem';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import InputLabel from '@mui/material/InputLabel';
import DialogTitle from '@mui/material/DialogTitle';
import FormControl from '@mui/material/FormControl';
import DialogContent from '@mui/material/DialogContent';
import DialogActions from '@mui/material/DialogActions';
import LinearProgress from '@mui/material/LinearProgress';
import { DatePicker } from '@mui/x-date-pickers/DatePicker';
import CircularProgress from '@mui/material/CircularProgress';
import { AdapterDayjs } from '@mui/x-date-pickers/AdapterDayjs';
import { LocalizationProvider } from '@mui/x-date-pickers/LocalizationProvider';

import { cortexClient } from 'src/lib/cortex-client';

import { Iconify } from 'src/components/iconify';
import { Markdown } from 'src/components/markdown';

// ----------------------------------------------------------------------

const EPISODE_TYPES: EpisodeType[] = ['Task', 'Pattern', 'Decision', 'Error', 'Success', 'General'];

const EPISODE_COLORS: Record<EpisodeType, 'default' | 'primary' | 'secondary' | 'error' | 'success' | 'warning' | 'info'> = {
  Task: 'primary',
  Pattern: 'secondary',
  Decision: 'info',
  Error: 'error',
  Success: 'success',
  General: 'default',
};

// ----------------------------------------------------------------------

export function MemorySearchView() {
  const [query, setQuery] = useState('');
  const [isSearching, setIsSearching] = useState(false);
  const [results, setResults] = useState<MemorySearchResult[]>([]);
  const [error, setError] = useState<string | null>(null);

  // Filters
  const [episodeTypeFilter, setEpisodeTypeFilter] = useState<EpisodeType | 'all'>('all');
  const [minImportance, setMinImportance] = useState<number>(0);
  const [startDate, setStartDate] = useState<Dayjs | null>(null);
  const [endDate, setEndDate] = useState<Dayjs | null>(null);

  // Detail modal
  const [selectedResult, setSelectedResult] = useState<MemorySearchResult | null>(null);

  const handleSearch = useCallback(async () => {
    if (!query.trim()) {
      return;
    }

    setIsSearching(true);
    setError(null);

    try {
      const filters: Record<string, unknown> = {};

      if (episodeTypeFilter !== 'all') {
        filters.episode_type = episodeTypeFilter;
      }

      if (minImportance > 0) {
        filters.min_importance = minImportance;
      }

      if (startDate) {
        filters.start_date = startDate.toDate().toISOString();
      }

      if (endDate) {
        filters.end_date = endDate.toDate().toISOString();
      }

      const data = await cortexClient.searchMemory({
        query: query.trim(),
        limit: 50,
        filters,
      });
      setResults(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Search failed');
    } finally {
      setIsSearching(false);
    }
  }, [query, episodeTypeFilter, minImportance, startDate, endDate]);

  const handleKeyPress = (event: React.KeyboardEvent) => {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      handleSearch();
    }
  };

  const handleClear = () => {
    setQuery('');
    setResults([]);
    setError(null);
    setEpisodeTypeFilter('all');
    setMinImportance(0);
    setStartDate(null);
    setEndDate(null);
  };

  const getImportanceColor = (importance: number) => {
    if (importance >= 0.7) return 'success';
    if (importance >= 0.4) return 'warning';
    return 'error';
  };

  const getEpisodeType = (metadata?: Record<string, unknown>): EpisodeType => {
    if (!metadata?.episode_type) return 'General';
    return metadata.episode_type as EpisodeType;
  };

  const getImportanceScore = (metadata?: Record<string, unknown>): number => {
    if (!metadata?.importance) return 0;
    return metadata.importance as number;
  };

  return (
    <LocalizationProvider dateAdapter={AdapterDayjs}>
      <Box sx={{ p: 3 }}>
        <Typography variant="h4" sx={{ mb: 3 }}>
          Memory Search
        </Typography>

        <Card sx={{ p: 3, mb: 3 }}>
          <Stack spacing={3}>
            <TextField
              fullWidth
              multiline
              rows={3}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyPress={handleKeyPress}
              placeholder="Search memory... (Press Enter to search)"
              InputProps={{
                startAdornment: (
                  <Iconify
                    icon="eva:search-fill"
                    sx={{ color: 'text.disabled', mr: 1, mt: 1, alignSelf: 'flex-start' }}
                  />
                ),
              }}
            />

            <Divider />

            <Typography variant="subtitle2" color="text.secondary">
              Filters
            </Typography>

            <Stack direction="row" spacing={2} flexWrap="wrap" useFlexGap>
              <FormControl sx={{ minWidth: 200 }}>
                <InputLabel>Episode Type</InputLabel>
                <Select
                  value={episodeTypeFilter}
                  onChange={(e) => setEpisodeTypeFilter(e.target.value as EpisodeType | 'all')}
                  label="Episode Type"
                >
                  <MenuItem value="all">All Types</MenuItem>
                  {EPISODE_TYPES.map((type) => (
                    <MenuItem key={type} value={type}>
                      {type}
                    </MenuItem>
                  ))}
                </Select>
              </FormControl>

              <FormControl sx={{ minWidth: 200 }}>
                <InputLabel>Min Importance</InputLabel>
                <Select
                  value={minImportance}
                  onChange={(e) => setMinImportance(e.target.value as number)}
                  label="Min Importance"
                >
                  <MenuItem value={0}>Any</MenuItem>
                  <MenuItem value={0.3}>Low (0.3+)</MenuItem>
                  <MenuItem value={0.5}>Medium (0.5+)</MenuItem>
                  <MenuItem value={0.7}>High (0.7+)</MenuItem>
                </Select>
              </FormControl>

              <DatePicker
                label="Start Date"
                value={startDate}
                onChange={setStartDate}
                slotProps={{ textField: { size: 'medium' } }}
              />

              <DatePicker
                label="End Date"
                value={endDate}
                onChange={setEndDate}
                slotProps={{ textField: { size: 'medium' } }}
              />
            </Stack>

            <Stack direction="row" spacing={2}>
              <Button
                variant="contained"
                onClick={handleSearch}
                disabled={isSearching || !query.trim()}
                startIcon={
                  isSearching ? (
                    <CircularProgress size={20} />
                  ) : (
                    <Iconify icon="eva:search-fill" />
                  )
                }
              >
                {isSearching ? 'Searching...' : 'Search'}
              </Button>

              <Button
                variant="outlined"
                onClick={handleClear}
                disabled={!query && !results.length}
              >
                Clear All
              </Button>
            </Stack>
          </Stack>
        </Card>

        {error && (
          <Alert severity="error" sx={{ mb: 3 }}>
            {error}
          </Alert>
        )}

        {results.length > 0 && (
          <Box>
            <Typography variant="h6" sx={{ mb: 2 }}>
              Results ({results.length})
            </Typography>

            <Stack spacing={2}>
              {results.map((result) => {
                const episodeType = getEpisodeType(result.metadata);
                const importance = getImportanceScore(result.metadata);

                return (
                  <Card key={result.id} sx={{ p: 3, cursor: 'pointer', '&:hover': { bgcolor: 'action.hover' } }} onClick={() => setSelectedResult(result)}>
                    <Stack direction="row" alignItems="center" spacing={2} sx={{ mb: 2 }} flexWrap="wrap">
                      <Chip
                        label={episodeType}
                        size="small"
                        color={EPISODE_COLORS[episodeType]}
                      />
                      <Chip
                        label={`Match: ${(result.score * 100).toFixed(1)}%`}
                        size="small"
                        color="primary"
                        variant="outlined"
                      />
                      {importance > 0 && (
                        <Chip
                          label={`Importance: ${(importance * 100).toFixed(0)}%`}
                          size="small"
                          color={getImportanceColor(importance)}
                          variant="outlined"
                        />
                      )}
                      <Typography variant="caption" color="text.secondary">
                        {new Date(result.created_at).toLocaleString()}
                      </Typography>
                    </Stack>

                    {importance > 0 && (
                      <Box sx={{ mb: 2 }}>
                        <Stack direction="row" spacing={1} alignItems="center">
                          <Typography variant="caption" color="text.secondary" sx={{ minWidth: 100 }}>
                            Importance:
                          </Typography>
                          <LinearProgress
                            variant="determinate"
                            value={importance * 100}
                            color={getImportanceColor(importance)}
                            sx={{ flexGrow: 1, height: 6, borderRadius: 1 }}
                          />
                        </Stack>
                      </Box>
                    )}

                    <Box
                      sx={{
                        maxHeight: 200,
                        overflow: 'hidden',
                        position: 'relative',
                        '&::after': {
                          content: '""',
                          position: 'absolute',
                          bottom: 0,
                          left: 0,
                          right: 0,
                          height: 40,
                          background: 'linear-gradient(transparent, white)',
                        },
                        '& pre': {
                          borderRadius: 1,
                          p: 2,
                          bgcolor: 'background.neutral',
                        },
                      }}
                    >
                      <Markdown content={result.content.slice(0, 500)} />
                    </Box>

                    <Box sx={{ mt: 2 }}>
                      <Button size="small" endIcon={<Iconify icon="eva:arrow-forward-fill" />}>
                        View Details
                      </Button>
                    </Box>
                  </Card>
                );
              })}
            </Stack>
          </Box>
        )}

        {!isSearching && !error && results.length === 0 && query && (
          <Card sx={{ p: 3 }}>
            <Typography variant="body2" color="text.secondary" align="center">
              No results found for &ldquo;{query}&rdquo;
            </Typography>
          </Card>
        )}

        {!query && !results.length && (
          <Card sx={{ p: 3 }}>
            <Typography variant="body2" color="text.secondary" align="center">
              Enter a search query to find related information in memory
            </Typography>
          </Card>
        )}

        {/* Detail Modal */}
        <Dialog
          open={!!selectedResult}
          onClose={() => setSelectedResult(null)}
          maxWidth="md"
          fullWidth
        >
          {selectedResult && (
            <>
              <DialogTitle>
                <Stack direction="row" spacing={2} alignItems="center">
                  <Chip
                    label={getEpisodeType(selectedResult.metadata)}
                    size="small"
                    color={EPISODE_COLORS[getEpisodeType(selectedResult.metadata)]}
                  />
                  <Typography variant="caption" color="text.secondary">
                    {new Date(selectedResult.created_at).toLocaleString()}
                  </Typography>
                </Stack>
              </DialogTitle>
              <DialogContent>
                <Stack spacing={2}>
                  {getImportanceScore(selectedResult.metadata) > 0 && (
                    <Box>
                      <Stack direction="row" spacing={1} alignItems="center">
                        <Typography variant="body2" color="text.secondary" sx={{ minWidth: 100 }}>
                          Importance:
                        </Typography>
                        <LinearProgress
                          variant="determinate"
                          value={getImportanceScore(selectedResult.metadata) * 100}
                          color={getImportanceColor(getImportanceScore(selectedResult.metadata))}
                          sx={{ flexGrow: 1, height: 8, borderRadius: 1 }}
                        />
                        <Typography variant="body2">
                          {(getImportanceScore(selectedResult.metadata) * 100).toFixed(0)}%
                        </Typography>
                      </Stack>
                    </Box>
                  )}

                  <Divider />

                  <Box
                    sx={{
                      '& pre': {
                        borderRadius: 1,
                        p: 2,
                        bgcolor: 'background.neutral',
                      },
                    }}
                  >
                    <Markdown content={selectedResult.content} />
                  </Box>

                  {selectedResult.metadata && Object.keys(selectedResult.metadata).length > 0 && (
                    <>
                      <Divider />
                      <Box>
                        <Typography variant="subtitle2" sx={{ mb: 1 }}>
                          Metadata
                        </Typography>
                        <Stack direction="row" spacing={1} flexWrap="wrap">
                          {Object.entries(selectedResult.metadata)
                            .filter(([key]) => key !== 'episode_type' && key !== 'importance')
                            .map(([key, value]) => (
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
                <Button onClick={() => setSelectedResult(null)}>Close</Button>
              </DialogActions>
            </>
          )}
        </Dialog>
      </Box>
    </LocalizationProvider>
  );
}
