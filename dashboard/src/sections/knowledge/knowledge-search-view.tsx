import { useState } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import LinearProgress from '@mui/material/LinearProgress';

import { cortexClient } from 'src/lib/cortex-client';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

export function KnowledgeSearchView() {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<any[]>([]);
  const [isSearching, setIsSearching] = useState(false);

  const handleSearch = async () => {
    if (!query.trim()) return;

    try {
      setIsSearching(true);
      const searchResults = await cortexClient.search({
        query,
        limit: 20,
        types: ['document', 'code_unit', 'memory'],
      });
      setResults(searchResults);
    } catch (error) {
      console.error('Search failed:', error);
    } finally {
      setIsSearching(false);
    }
  };

  const getTypeColor = (type: string) => {
    if (type === 'document') return 'primary';
    if (type === 'code_unit') return 'info';
    if (type === 'memory') return 'success';
    return 'default';
  };

  const getTypeIcon = (type: string) => {
    if (type === 'document') return 'mdi:file-document';
    if (type === 'code_unit') return 'mdi:code-braces';
    if (type === 'memory') return 'mdi:brain';
    return 'mdi:circle';
  };

  return (
    <>
      <CustomBreadcrumbs
        heading="Semantic Search"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Knowledge' },
          { name: 'Search' },
        ]}
        sx={{ mb: 3 }}
      />

      <Stack spacing={2}>
        <Card sx={{ p: 3 }}>
          <Typography variant="h6" sx={{ mb: 1 }}>
            Semantic Search Interface
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
            Search across documents, code, and memory using natural language queries.
          </Typography>

          <Stack direction="row" spacing={2}>
            <TextField
              fullWidth
              placeholder="Enter your search query..."
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyPress={(e) => e.key === 'Enter' && handleSearch()}
              slotProps={{
                input: {
                  startAdornment: (
                    <Iconify icon="mdi:magnify" width={24} sx={{ color: 'text.disabled', mr: 1 }} />
                  ),
                },
              }}
            />
            <Button
              variant="contained"
              size="large"
              onClick={handleSearch}
              disabled={isSearching || !query.trim()}
              sx={{ minWidth: 120 }}
            >
              {isSearching ? 'Searching...' : 'Search'}
            </Button>
          </Stack>
        </Card>

        {isSearching && <LinearProgress />}

        {results.length > 0 && (
          <Card>
            <Box sx={{ p: 2, borderBottom: 1, borderColor: 'divider' }}>
              <Typography variant="subtitle1">
                {results.length} results found
              </Typography>
            </Box>
            <Stack spacing={0} divider={<Box sx={{ borderBottom: 1, borderColor: 'divider' }} />}>
              {results.map((result: any, index) => (
                <Box key={index} sx={{ p: 2.5 }}>
                  <Stack spacing={1.5}>
                    <Stack direction="row" spacing={1} alignItems="center">
                      <Box
                        sx={{
                          width: 36,
                          height: 36,
                          borderRadius: 1,
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'center',
                          bgcolor: `${getTypeColor(result.type)}.lighter`,
                          color: `${getTypeColor(result.type)}.main`,
                        }}
                      >
                        <Iconify icon={getTypeIcon(result.type)} width={20} />
                      </Box>
                      <Box flex={1}>
                        <Stack direction="row" spacing={1} alignItems="center">
                          <Typography variant="subtitle2">
                            {result.title || result.name || 'Untitled'}
                          </Typography>
                          <Label variant="soft" color={getTypeColor(result.type) as any}>
                            {result.type}
                          </Label>
                          {result.relevance && (
                            <Label variant="soft" color="success">
                              {(result.relevance * 100).toFixed(0)}% match
                            </Label>
                          )}
                        </Stack>
                      </Box>
                    </Stack>

                    <Typography variant="body2" color="text.secondary">
                      {result.excerpt || result.content || result.description || 'No preview available'}
                    </Typography>

                    <Stack direction="row" spacing={2}>
                      {result.path && (
                        <Typography variant="caption" color="text.disabled">
                          {result.path}
                        </Typography>
                      )}
                      {result.updated_at && (
                        <Typography variant="caption" color="text.disabled">
                          Updated: {new Date(result.updated_at).toLocaleDateString()}
                        </Typography>
                      )}
                    </Stack>
                  </Stack>
                </Box>
              ))}
            </Stack>
          </Card>
        )}

        {!isSearching && query && results.length === 0 && (
          <Card sx={{ p: 5, textAlign: 'center' }}>
            <Iconify icon="mdi:magnify" width={64} sx={{ color: 'text.disabled', mb: 2 }} />
            <Typography variant="h6" color="text.secondary">
              No results found
            </Typography>
            <Typography variant="body2" color="text.disabled" sx={{ mt: 1 }}>
              Try adjusting your search query or using different keywords
            </Typography>
          </Card>
        )}
      </Stack>
    </>
  );
}
