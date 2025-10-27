import { useState } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

interface SemanticConcept {
  id: string;
  concept: string;
  category: string;
  related: string[];
  strength: number;
}

const MOCK_CONCEPTS: SemanticConcept[] = [
  {
    id: '1',
    concept: 'Authentication',
    category: 'Security',
    related: ['Authorization', 'JWT', 'OAuth', 'Session'],
    strength: 0.95,
  },
  {
    id: '2',
    concept: 'React Component',
    category: 'Frontend',
    related: ['JSX', 'Props', 'State', 'Hooks'],
    strength: 0.92,
  },
  {
    id: '3',
    concept: 'REST API',
    category: 'Backend',
    related: ['HTTP', 'Endpoint', 'Request', 'Response'],
    strength: 0.88,
  },
  {
    id: '4',
    concept: 'Database Schema',
    category: 'Data',
    related: ['Table', 'Column', 'Index', 'Migration'],
    strength: 0.85,
  },
];

export function MemorySemanticView() {
  const [searchQuery, setSearchQuery] = useState('');
  const [concepts] = useState<SemanticConcept[]>(MOCK_CONCEPTS);

  const filteredConcepts = concepts.filter(
    (c) =>
      c.concept.toLowerCase().includes(searchQuery.toLowerCase()) ||
      c.category.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <>
      <CustomBreadcrumbs
        heading="Semantic Memory"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Memory' },
          { name: 'Semantic' },
        ]}
        sx={{ mb: 3 }}
      />

      <Stack spacing={2}>
        <Card sx={{ p: 3 }}>
          <Typography variant="h6" sx={{ mb: 1 }}>
            Semantic Knowledge Network
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Structured knowledge about concepts, relationships, and their meanings.
          </Typography>
        </Card>

        <Card sx={{ p: 3 }}>
          <TextField
            fullWidth
            placeholder="Search concepts..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            slotProps={{
              input: {
                startAdornment: (
                  <Iconify icon="eva:search-fill" sx={{ color: 'text.disabled', mr: 1 }} />
                ),
              },
            }}
          />
        </Card>

        <Grid container spacing={2}>
          {filteredConcepts.map((concept) => (
            <Grid size={{ xs: 12, md: 6 }} key={concept.id}>
              <Card sx={{ p: 2.5, height: '100%' }}>
                <Stack spacing={2}>
                  <Stack direction="row" justifyContent="space-between" alignItems="flex-start">
                    <Box>
                      <Typography variant="h6">{concept.concept}</Typography>
                      <Label variant="soft" color="primary" sx={{ mt: 0.5 }}>
                        {concept.category}
                      </Label>
                    </Box>
                    <Box sx={{ textAlign: 'right' }}>
                      <Typography variant="h6" color="success.main">
                        {(concept.strength * 100).toFixed(0)}%
                      </Typography>
                      <Typography variant="caption" color="text.secondary">
                        Strength
                      </Typography>
                    </Box>
                  </Stack>

                  <Box>
                    <Typography variant="caption" color="text.secondary" sx={{ mb: 1, display: 'block' }}>
                      Related Concepts ({concept.related.length})
                    </Typography>
                    <Stack direction="row" spacing={0.5} flexWrap="wrap" gap={0.5}>
                      {concept.related.map((rel) => (
                        <Label key={rel} variant="soft">
                          {rel}
                        </Label>
                      ))}
                    </Stack>
                  </Box>

                  <Stack direction="row" spacing={1} alignItems="center">
                    <Iconify icon="mdi:network" width={16} color="text.secondary" />
                    <Typography variant="caption" color="text.secondary">
                      {concept.related.length} connections
                    </Typography>
                  </Stack>
                </Stack>
              </Card>
            </Grid>
          ))}
        </Grid>

        {filteredConcepts.length === 0 && (
          <Card sx={{ p: 5, textAlign: 'center' }}>
            <Iconify icon="mdi:brain-off" width={64} sx={{ color: 'text.disabled', mb: 2 }} />
            <Typography variant="h6" color="text.secondary">
              No concepts found
            </Typography>
          </Card>
        )}
      </Stack>
    </>
  );
}
