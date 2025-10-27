import { useParams, useNavigate } from 'react-router';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import Stack from '@mui/material/Stack';
import Chip from '@mui/material/Chip';
import Divider from '@mui/material/Divider';
import IconButton from '@mui/material/IconButton';

import { fDateTime } from 'src/utils/format-time';

import { Iconify } from 'src/components/iconify';
import { Markdown } from 'src/components/markdown';
import { useSnackbar } from 'src/components/snackbar';

import useSWR from 'swr';
import { cortexClient, cortexFetcher, cortexEndpoints } from 'src/lib/cortex-client';
import type { Document } from 'src/types/cortex';

// ----------------------------------------------------------------------

export function DocumentView() {
  const params = useParams();
  const navigate = useNavigate();
  const { enqueueSnackbar } = useSnackbar();

  const documentId = params.id as string;

  // Fetch document
  const { data: document, isLoading, error } = useSWR<Document>(
    documentId ? cortexEndpoints.documents.get(documentId) : null,
    cortexFetcher
  );

  const handleBack = () => {
    navigate('/dashboard/cortex/documents');
  };

  const handleEdit = () => {
    navigate(`/dashboard/cortex/documents/${documentId}/edit`);
  };

  const handlePublish = async () => {
    try {
      await cortexClient.publishDocument(documentId);
      enqueueSnackbar('Document published', { variant: 'success' });
      // Refresh the document
      window.location.reload();
    } catch (err) {
      enqueueSnackbar('Failed to publish document', { variant: 'error' });
    }
  };

  const handleDelete = async () => {
    if (!window.confirm('Are you sure you want to delete this document?')) {
      return;
    }

    try {
      await cortexClient.deleteDocument(documentId);
      enqueueSnackbar('Document deleted', { variant: 'success' });
      navigate('/dashboard/cortex/documents');
    } catch (err) {
      enqueueSnackbar('Failed to delete document', { variant: 'error' });
    }
  };

  if (isLoading) {
    return (
      <Box sx={{ p: 3 }}>
        <Typography>Loading...</Typography>
      </Box>
    );
  }

  if (error || !document) {
    return (
      <Box sx={{ p: 3 }}>
        <Typography color="error">Document not found</Typography>
        <Button onClick={handleBack} sx={{ mt: 2 }}>
          Back to Documents
        </Button>
      </Box>
    );
  }

  return (
    <Box sx={{ p: 3 }}>
      <Button
        startIcon={<Iconify icon="eva:arrow-back-fill" />}
        onClick={handleBack}
        sx={{ mb: 3 }}
      >
        Back
      </Button>

      <Card sx={{ p: 3 }}>
        {/* Header */}
        <Stack direction="row" alignItems="flex-start" spacing={2} sx={{ mb: 3 }}>
          <Box sx={{ flexGrow: 1 }}>
            <Typography variant="h3" sx={{ mb: 1 }}>
              {document.title}
            </Typography>

            <Stack direction="row" spacing={1} alignItems="center" flexWrap="wrap">
              <Chip label={document.status} size="small" color="primary" />
              <Chip label={document.doc_type} size="small" variant="outlined" />
              {document.tags.map((tag) => (
                <Chip key={tag} label={tag} size="small" variant="outlined" />
              ))}
            </Stack>
          </Box>

          <Stack direction="row" spacing={1}>
            <IconButton onClick={handleEdit}>
              <Iconify icon="solar:pen-bold" />
            </IconButton>

            {document.status !== 'Published' && (
              <IconButton onClick={handlePublish} color="primary">
                <Iconify icon="solar:upload-bold" />
              </IconButton>
            )}

            <IconButton onClick={handleDelete} color="error">
              <Iconify icon="solar:trash-bin-trash-bold" />
            </IconButton>
          </Stack>
        </Stack>

        {/* Metadata */}
        <Stack spacing={1} sx={{ mb: 3, color: 'text.secondary' }}>
          {document.description && (
            <Typography variant="body2">{document.description}</Typography>
          )}

          <Stack direction="row" spacing={2} flexWrap="wrap">
            {document.author && (
              <Typography variant="caption">
                <strong>Author:</strong> {document.author}
              </Typography>
            )}
            <Typography variant="caption">
              <strong>Created:</strong> {fDateTime(document.created_at)}
            </Typography>
            <Typography variant="caption">
              <strong>Updated:</strong> {fDateTime(document.updated_at)}
            </Typography>
            {document.published_at && (
              <Typography variant="caption">
                <strong>Published:</strong> {fDateTime(document.published_at)}
              </Typography>
            )}
            <Typography variant="caption">
              <strong>Version:</strong> {document.version}
            </Typography>
          </Stack>
        </Stack>

        <Divider sx={{ mb: 3 }} />

        {/* Content */}
        <Box sx={{
          '& pre': {
            borderRadius: 1,
            p: 2,
            bgcolor: 'background.neutral'
          }
        }}>
          <Markdown content={document.content} />
        </Box>
      </Card>
    </Box>
  );
}
