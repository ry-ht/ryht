import type { EpisodeType, MemoryEpisode } from 'src/types/cortex';

import useSWR from 'swr';
import { useState, useEffect } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Chip from '@mui/material/Chip';
import Stack from '@mui/material/Stack';
import Alert from '@mui/material/Alert';
import Select from '@mui/material/Select';
import Button from '@mui/material/Button';
import Divider from '@mui/material/Divider';
import Collapse from '@mui/material/Collapse';
import MenuItem from '@mui/material/MenuItem';
import TextField from '@mui/material/TextField';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import InputLabel from '@mui/material/InputLabel';
import Pagination from '@mui/material/Pagination';
import CardContent from '@mui/material/CardContent';
import FormControl from '@mui/material/FormControl';
import LinearProgress from '@mui/material/LinearProgress';
import CircularProgress from '@mui/material/CircularProgress';

import { cortexClient } from 'src/lib/cortex-client';

import { Iconify } from 'src/components/iconify';
import { Markdown } from 'src/components/markdown';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

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

const ITEMS_PER_PAGE = 10;

// ----------------------------------------------------------------------

export function MemoryEpisodesView() {
  const [searchQuery, setSearchQuery] = useState('');
  const [typeFilter, setTypeFilter] = useState<EpisodeType | 'all'>('all');
  const [sortBy, setSortBy] = useState<'importance' | 'date'>('date');
  const [page, setPage] = useState(1);
  const [expandedId, setExpandedId] = useState<string | null>(null);

  // Fetch episodes
  const { data: episodes, error, isLoading, mutate } = useSWR<MemoryEpisode[]>(
    '/memory/episodes',
    () => cortexClient.listMemoryEpisodes()
  );

  // Filter and sort episodes
  const filteredEpisodes = episodes?.filter((episode) => {
    // Type filter
    if (typeFilter !== 'all' && episode.episode_type !== typeFilter) {
      return false;
    }

    // Search filter
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      return (
        episode.content.toLowerCase().includes(query) ||
        episode.episode_type.toLowerCase().includes(query)
      );
    }

    return true;
  }) || [];

  // Sort episodes
  const sortedEpisodes = [...filteredEpisodes].sort((a, b) => {
    if (sortBy === 'importance') {
      return b.importance - a.importance;
    }
    return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
  });

  // Paginate episodes
  const totalPages = Math.ceil(sortedEpisodes.length / ITEMS_PER_PAGE);
  const paginatedEpisodes = sortedEpisodes.slice(
    (page - 1) * ITEMS_PER_PAGE,
    page * ITEMS_PER_PAGE
  );

  const handleToggleExpand = (id: string) => {
    setExpandedId(expandedId === id ? null : id);
  };

  const handlePageChange = (_event: React.ChangeEvent<unknown>, value: number) => {
    setPage(value);
    setExpandedId(null);
  };

  const getImportanceColor = (importance: number) => {
    if (importance >= 0.7) return 'success';
    if (importance >= 0.4) return 'warning';
    return 'error';
  };

  // Reset page when filters change
  useEffect(() => {
    setPage(1);
  }, [typeFilter, searchQuery, sortBy]);

  return (
    <Box sx={{ p: 3 }}>
      <Stack direction="row" justifyContent="space-between" alignItems="center" sx={{ mb: 3 }}>
        <Typography variant="h4">
          Memory Episodes
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
            placeholder="Search episodes..."
            slotProps={{
              input: {
                startAdornment: (
                  <Iconify
                    icon="eva:search-fill"
                    sx={{ color: 'text.disabled', mr: 1 }}
                  />
                ),
              },
            }}
          />

          <Stack direction="row" spacing={2} flexWrap="wrap" useFlexGap>
            <FormControl sx={{ minWidth: 200 }}>
              <InputLabel>Episode Type</InputLabel>
              <Select
                value={typeFilter}
                onChange={(e) => setTypeFilter(e.target.value as EpisodeType | 'all')}
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
              <InputLabel>Sort By</InputLabel>
              <Select
                value={sortBy}
                onChange={(e) => setSortBy(e.target.value as 'importance' | 'date')}
                label="Sort By"
              >
                <MenuItem value="date">Date (Newest First)</MenuItem>
                <MenuItem value="importance">Importance (Highest First)</MenuItem>
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
          {error.message || 'Failed to load episodes'}
        </Alert>
      )}

      {/* Episodes list */}
      {!isLoading && !error && paginatedEpisodes.length > 0 && (
        <>
          <Typography variant="subtitle2" color="text.secondary" sx={{ mb: 2 }}>
            Showing {(page - 1) * ITEMS_PER_PAGE + 1}-{Math.min(page * ITEMS_PER_PAGE, sortedEpisodes.length)} of {sortedEpisodes.length} episodes
          </Typography>

          <Stack spacing={2}>
            {paginatedEpisodes.map((episode) => (
              <Card key={episode.id}>
                <CardContent>
                  <Stack spacing={2}>
                    {/* Header */}
                    <Stack direction="row" justifyContent="space-between" alignItems="flex-start">
                      <Stack direction="row" spacing={2} alignItems="center" flexWrap="wrap">
                        <Chip
                          label={episode.episode_type}
                          size="small"
                          color={EPISODE_COLORS[episode.episode_type]}
                        />
                        <Chip
                          label={`Importance: ${(episode.importance * 100).toFixed(0)}%`}
                          size="small"
                          color={getImportanceColor(episode.importance)}
                          variant="outlined"
                        />
                        <Typography variant="caption" color="text.secondary">
                          {new Date(episode.created_at).toLocaleString()}
                        </Typography>
                      </Stack>

                      <IconButton
                        onClick={() => handleToggleExpand(episode.id)}
                        size="small"
                      >
                        <Iconify
                          icon={expandedId === episode.id ? 'eva:arrow-up-fill' : 'eva:arrow-down-fill'}
                        />
                      </IconButton>
                    </Stack>

                    {/* Importance bar */}
                    <Box>
                      <Stack direction="row" spacing={1} alignItems="center">
                        <Typography variant="caption" color="text.secondary" sx={{ minWidth: 80 }}>
                          Importance:
                        </Typography>
                        <LinearProgress
                          variant="determinate"
                          value={episode.importance * 100}
                          color={getImportanceColor(episode.importance)}
                          sx={{ flexGrow: 1, height: 6, borderRadius: 1 }}
                        />
                      </Stack>
                    </Box>

                    {/* Preview */}
                    {expandedId !== episode.id && (
                      <Box
                        sx={{
                          maxHeight: 100,
                          overflow: 'hidden',
                          position: 'relative',
                          '&::after': {
                            content: '""',
                            position: 'absolute',
                            bottom: 0,
                            left: 0,
                            right: 0,
                            height: 30,
                            background: 'linear-gradient(transparent, white)',
                          },
                        }}
                      >
                        <Typography variant="body2" color="text.secondary">
                          {episode.content.slice(0, 200)}...
                        </Typography>
                      </Box>
                    )}

                    {/* Expanded content */}
                    <Collapse in={expandedId === episode.id}>
                      <Stack spacing={2}>
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
                          <Markdown content={episode.content} />
                        </Box>

                        {/* Related patterns */}
                        {episode.patterns && episode.patterns.length > 0 && (
                          <>
                            <Divider />
                            <Box>
                              <Typography variant="subtitle2" sx={{ mb: 1 }}>
                                Related Patterns ({episode.patterns.length})
                              </Typography>
                              <Stack direction="row" spacing={1} flexWrap="wrap">
                                {episode.patterns.map((patternId) => (
                                  <Chip
                                    key={patternId}
                                    label={patternId}
                                    size="small"
                                    variant="outlined"
                                    color="secondary"
                                  />
                                ))}
                              </Stack>
                            </Box>
                          </>
                        )}

                        {/* Context metadata */}
                        {episode.context && Object.keys(episode.context).length > 0 && (
                          <>
                            <Divider />
                            <Box>
                              <Typography variant="subtitle2" sx={{ mb: 1 }}>
                                Context
                              </Typography>
                              <Stack direction="row" spacing={1} flexWrap="wrap">
                                {Object.entries(episode.context).map(([key, value]) => (
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

                        {/* Additional metadata */}
                        {episode.metadata && Object.keys(episode.metadata).length > 0 && (
                          <>
                            <Divider />
                            <Box>
                              <Typography variant="subtitle2" sx={{ mb: 1 }}>
                                Metadata
                              </Typography>
                              <Stack direction="row" spacing={1} flexWrap="wrap">
                                {Object.entries(episode.metadata).map(([key, value]) => (
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
                    </Collapse>
                  </Stack>
                </CardContent>
              </Card>
            ))}
          </Stack>

          {/* Pagination */}
          {totalPages > 1 && (
            <Box sx={{ display: 'flex', justifyContent: 'center', mt: 4 }}>
              <Pagination
                count={totalPages}
                page={page}
                onChange={handlePageChange}
                color="primary"
                showFirstButton
                showLastButton
              />
            </Box>
          )}
        </>
      )}

      {/* Empty state */}
      {!isLoading && !error && paginatedEpisodes.length === 0 && (
        <Card sx={{ p: 5 }}>
          <Stack spacing={2} alignItems="center">
            <Iconify icon="eva:inbox-outline" sx={{ width: 64, height: 64, color: 'text.disabled' }} />
            <Typography variant="h6" color="text.secondary">
              {searchQuery || typeFilter !== 'all' ? 'No episodes match your filters' : 'No episodes found'}
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
    </Box>
  );
}
