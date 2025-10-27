import useSWR from 'swr';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import LinearProgress from '@mui/material/LinearProgress';

import { cortexFetcher, cortexEndpoints } from 'src/lib/cortex-client';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

export function CoordinationSessionsView() {
  const { data: sessions = [], isLoading } = useSWR(
    '/api/v1/sessions',
    cortexFetcher,
    { refreshInterval: 5000 }
  );

  return (
    <>
      <CustomBreadcrumbs
        heading="Coordination Sessions"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Coordination' },
          { name: 'Sessions' },
        ]}
        sx={{ mb: 3 }}
      />

      <Stack spacing={2}>
        <Card sx={{ p: 3 }}>
          <Typography variant="h6" sx={{ mb: 1 }}>
            Active Coordination Sessions
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Monitor active collaboration sessions between agents and view session details.
          </Typography>
        </Card>

        {isLoading && <LinearProgress />}

        <Grid container spacing={2}>
          {sessions.map((session: any) => (
            <Grid size={{ xs: 12, md: 6 }} key={session.id}>
              <Card sx={{ p: 2.5 }}>
                <Stack spacing={2}>
                  <Stack direction="row" justifyContent="space-between" alignItems="flex-start">
                    <Box>
                      <Typography variant="h6">{session.name}</Typography>
                      <Typography variant="caption" color="text.disabled">
                        Session ID: {session.id}
                      </Typography>
                    </Box>
                    <Label variant="soft" color="success">
                      Active
                    </Label>
                  </Stack>

                  <Stack direction="row" spacing={2}>
                    <Stack spacing={0.5} flex={1}>
                      <Typography variant="caption" color="text.secondary">
                        Workspace
                      </Typography>
                      <Typography variant="body2">{session.workspace_id || 'N/A'}</Typography>
                    </Stack>
                    <Stack spacing={0.5} flex={1}>
                      <Typography variant="caption" color="text.secondary">
                        Created
                      </Typography>
                      <Typography variant="body2">
                        {session.created_at
                          ? new Date(session.created_at).toLocaleDateString()
                          : 'N/A'}
                      </Typography>
                    </Stack>
                  </Stack>

                  <Stack direction="row" spacing={1} alignItems="center">
                    <Iconify icon="mdi:account-group" width={20} color="text.secondary" />
                    <Typography variant="body2" color="text.secondary">
                      {session.participants?.length || 0} participants
                    </Typography>
                  </Stack>
                </Stack>
              </Card>
            </Grid>
          ))}
        </Grid>

        {!isLoading && sessions.length === 0 && (
          <Card sx={{ p: 5, textAlign: 'center' }}>
            <Iconify icon="mdi:account-group-outline" width={64} sx={{ color: 'text.disabled', mb: 2 }} />
            <Typography variant="h6" color="text.secondary">
              No active sessions
            </Typography>
            <Typography variant="body2" color="text.disabled" sx={{ mt: 1 }}>
              Coordination sessions will appear here when agents start collaborating
            </Typography>
          </Card>
        )}
      </Stack>
    </>
  );
}
