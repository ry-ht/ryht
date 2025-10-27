import { useState, useCallback } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import Stack from '@mui/material/Stack';
import Chip from '@mui/material/Chip';
import CircularProgress from '@mui/material/CircularProgress';
import Alert from '@mui/material/Alert';
import Divider from '@mui/material/Divider';

import { Iconify } from 'src/components/iconify';
import { Markdown } from 'src/components/markdown';

import { cortexClient } from 'src/lib/cortex-client';
import type { MemorySearchResult } from 'src/types/cortex';

// ----------------------------------------------------------------------

export function MemorySearchView() {
  const [query, setQuery] = useState('');
  const [isSearching, setIsSearching] = useState(false);
  const [results, setResults] = useState<MemorySearchResult[]>([]);
  const [error, setError] = useState<string | null>(null);

  const handleSearch = useCallback(async () => {
    if (!query.trim()) {
      return;
    }

    setIsSearching(true);
    setError(null);

    try {
      const data = await cortexClient.searchMemory({
        query: query.trim(),
        limit: 20,
      });
      setResults(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Search failed');
    } finally {
      setIsSearching(false);
    }
  }, [query]);

  const handleKeyPress = (event: React.KeyboardEvent) => {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      handleSearch();
    }
  };

  return (
    <Box sx={{ p: 3 }}>
      <Typography variant="h4" sx={{ mb: 3 }}>
        Memory Search
      </Typography>

      <Card sx={{ p: 3, mb: 3 }}>
        <Stack spacing={2}>
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
              onClick={() => {
                setQuery('');
                setResults([]);
                setError(null);
              }}
              disabled={!query && !results.length}
            >
              Clear
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
            {results.map((result, index) => (
              <Card key={result.id} sx={{ p: 3 }}>
                <Stack direction="row" alignItems="center" spacing={2} sx={{ mb: 2 }}>
                  <Chip
                    label={`Score: ${(result.score * 100).toFixed(1)}%`}
                    size="small"
                    color="primary"
                  />
                  <Typography variant="caption" color="text.secondary">
                    {new Date(result.created_at).toLocaleString()}
                  </Typography>
                </Stack>

                <Box
                  sx={{
                    '& pre': {
                      borderRadius: 1,
                      p: 2,
                      bgcolor: 'background.neutral',
                    },
                  }}
                >
                  <Markdown content={result.content} />
                </Box>

                {result.metadata && Object.keys(result.metadata).length > 0 && (
                  <>
                    <Divider sx={{ my: 2 }} />
                    <Box>
                      <Typography variant="caption" color="text.secondary" sx={{ mb: 1 }}>
                        Metadata:
                      </Typography>
                      <Stack direction="row" spacing={1} flexWrap="wrap">
                        {Object.entries(result.metadata).map(([key, value]) => (
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
              </Card>
            ))}
          </Stack>
        </Box>
      )}

      {!isSearching && !error && results.length === 0 && query && (
        <Card sx={{ p: 3 }}>
          <Typography variant="body2" color="text.secondary" align="center">
            No results found for "{query}"
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
    </Box>
  );
}
