import useSWR from 'swr';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import LinearProgress from '@mui/material/LinearProgress';

import { cortexFetcher } from 'src/lib/cortex-client';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

export function MemoryEpisodicView() {
  const { data: episodes = [], isLoading } = useSWR(
    '/api/v1/memory/episodes',
    cortexFetcher,
    { refreshInterval: 10000 }
  );

  return (
    <>
      <CustomBreadcrumbs
        heading="Episodic Memory"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Memory' },
          { name: 'Episodic' },
        ]}
        sx={{ mb: 3 }}
      />

      <Stack spacing={2}>
        <Card sx={{ p: 3 }}>
          <Typography variant="h6" sx={{ mb: 1 }}>
            Episodic Memory Episodes
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Temporal records of past experiences and events with contextual information.
          </Typography>
        </Card>

        {isLoading && <LinearProgress />}

        {episodes.map((episode: any) => (
          <Card key={episode.id} sx={{ p: 2.5 }}>
            <Stack spacing={2}>
              <Stack direction="row" justifyContent="space-between" alignItems="flex-start">
                <Box>
                  <Typography variant="h6">{episode.title || 'Untitled Episode'}</Typography>
                  <Typography variant="caption" color="text.disabled">
                    Episode ID: {episode.id}
                  </Typography>
                </Box>
                <Label variant="soft" color="info">
                  {episode.type || 'general'}
                </Label>
              </Stack>

              <Typography variant="body2">{episode.description || 'No description'}</Typography>

              <Stack direction="row" spacing={2}>
                <Stack direction="row" spacing={1} alignItems="center">
                  <Iconify icon="mdi:calendar" width={16} color="text.secondary" />
                  <Typography variant="caption" color="text.secondary">
                    {episode.timestamp
                      ? new Date(episode.timestamp).toLocaleDateString()
                      : 'N/A'}
                  </Typography>
                </Stack>
                <Stack direction="row" spacing={1} alignItems="center">
                  <Iconify icon="mdi:tag" width={16} color="text.secondary" />
                  <Typography variant="caption" color="text.secondary">
                    {episode.tags?.length || 0} tags
                  </Typography>
                </Stack>
              </Stack>
            </Stack>
          </Card>
        ))}

        {!isLoading && episodes.length === 0 && (
          <Card sx={{ p: 5, textAlign: 'center' }}>
            <Iconify icon="mdi:brain" width={64} sx={{ color: 'text.disabled', mb: 2 }} />
            <Typography variant="h6" color="text.secondary">
              No episodic memories found
            </Typography>
            <Typography variant="body2" color="text.disabled" sx={{ mt: 1 }}>
              Episodes will be recorded as agents complete tasks and interact
            </Typography>
          </Card>
        )}
      </Stack>
    </>
  );
}
