import useSWR from 'swr';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import LinearProgress from '@mui/material/LinearProgress';

import { cortexFetcher, cortexEndpoints } from 'src/lib/cortex-client';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

export function KnowledgeDocumentsView() {
  const { data: documents = [], isLoading } = useSWR(
    cortexEndpoints.documents.list,
    cortexFetcher,
    { refreshInterval: 10000 }
  );

  const getStatusColor = (status: string) => {
    if (status === 'published') return 'success';
    if (status === 'draft') return 'warning';
    if (status === 'archived') return 'default';
    return 'info';
  };

  return (
    <>
      <CustomBreadcrumbs
        heading="Knowledge Documents"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Knowledge' },
          { name: 'Documents' },
        ]}
        action={
          <Button
            variant="contained"
            startIcon={<Iconify icon="mdi:plus" />}
            href="/cortex/documents/create"
          >
            New Document
          </Button>
        }
        sx={{ mb: 3 }}
      />

      <Stack spacing={2}>
        <Card sx={{ p: 3 }}>
          <Typography variant="h6" sx={{ mb: 1 }}>
            Knowledge Base Documents
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Centralized documentation and knowledge repository for the multi-agent system.
          </Typography>
        </Card>

        {isLoading && <LinearProgress />}

        <Grid container spacing={2}>
          {documents.map((doc: any) => (
            <Grid size={{ xs: 12, md: 6, lg: 4 }} key={doc.id}>
              <Card sx={{ p: 2.5, height: '100%' }}>
                <Stack spacing={2}>
                  <Stack direction="row" justifyContent="space-between" alignItems="flex-start">
                    <Box
                      sx={{
                        width: 48,
                        height: 48,
                        borderRadius: 1.5,
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        bgcolor: 'primary.lighter',
                        color: 'primary.main',
                      }}
                    >
                      <Iconify icon="mdi:file-document" width={24} />
                    </Box>
                    <Label variant="soft" color={getStatusColor(doc.status)}>
                      {doc.status}
                    </Label>
                  </Stack>

                  <Box>
                    <Typography variant="h6" sx={{ mb: 0.5 }}>
                      {doc.title}
                    </Typography>
                    <Typography variant="body2" color="text.secondary" sx={{ mb: 1 }}>
                      {doc.summary || 'No summary available'}
                    </Typography>
                    <Typography variant="caption" color="text.disabled">
                      {doc.doc_type} • Updated {new Date(doc.updated_at).toLocaleDateString()}
                    </Typography>
                  </Box>

                  <Stack direction="row" spacing={2}>
                    <Stack direction="row" spacing={0.5} alignItems="center">
                      <Iconify icon="mdi:account" width={16} color="text.secondary" />
                      <Typography variant="caption" color="text.secondary">
                        {doc.author || 'Unknown'}
                      </Typography>
                    </Stack>
                    <Stack direction="row" spacing={0.5} alignItems="center">
                      <Iconify icon="mdi:eye" width={16} color="text.secondary" />
                      <Typography variant="caption" color="text.secondary">
                        {doc.views || 0} views
                      </Typography>
                    </Stack>
                  </Stack>

                  <Button
                    variant="outlined"
                    size="small"
                    href={`/cortex/documents/${doc.id}`}
                    fullWidth
                  >
                    View Document
                  </Button>
                </Stack>
              </Card>
            </Grid>
          ))}
        </Grid>

        {!isLoading && documents.length === 0 && (
          <Card sx={{ p: 5, textAlign: 'center' }}>
            <Iconify icon="mdi:book-open-blank-variant" width={64} sx={{ color: 'text.disabled', mb: 2 }} />
            <Typography variant="h6" color="text.secondary">
              No documents found
            </Typography>
            <Typography variant="body2" color="text.disabled" sx={{ mt: 1 }}>
              Create your first knowledge document to get started
            </Typography>
          </Card>
        )}
      </Stack>
    </>
  );
}
