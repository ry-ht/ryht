import type { Document, DocumentLink, DocumentSection, DocumentVersion } from 'src/types/cortex';

import useSWR from 'swr';
import { useState } from 'react';
import { useParams, useNavigate } from 'react-router';

import Box from '@mui/material/Box';
import Tab from '@mui/material/Tab';
import Card from '@mui/material/Card';
import Chip from '@mui/material/Chip';
import Tabs from '@mui/material/Tabs';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Divider from '@mui/material/Divider';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';

import { fDateTime } from 'src/utils/format-time';

import { cortexClient, cortexFetcher, cortexEndpoints } from 'src/lib/cortex-client';

import { Iconify } from 'src/components/iconify';
import { Markdown } from 'src/components/markdown';
import { useSnackbar } from 'src/components/snackbar';

import { DocumentLinksPanel } from './view/document-links-panel';
import { DocumentSectionsPanel } from './view/document-sections-panel';
import { DocumentVersionsPanel } from './view/document-versions-panel';

// ----------------------------------------------------------------------

export function DocumentView() {
  const params = useParams();
  const navigate = useNavigate();
  const { enqueueSnackbar } = useSnackbar();

  const documentId = params.id as string;
  const [activeTab, setActiveTab] = useState(0);
  const [editMode, setEditMode] = useState(false);

  // Fetch document
  const { data: document, isLoading, error, mutate } = useSWR<Document>(
    documentId ? cortexEndpoints.documents.get(documentId) : null,
    cortexFetcher
  );

  // Fetch sections
  const { data: sections = [], mutate: mutateSections } = useSWR<DocumentSection[]>(
    documentId ? cortexEndpoints.documents.sections(documentId) : null,
    cortexFetcher
  );

  // Fetch links
  const { data: links = [], mutate: mutateLinks } = useSWR<DocumentLink[]>(
    documentId ? cortexEndpoints.documents.links(documentId) : null,
    cortexFetcher
  );

  // Fetch versions
  const { data: versions = [], mutate: mutateVersions } = useSWR<DocumentVersion[]>(
    documentId ? cortexEndpoints.documents.versions(documentId) : null,
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
      enqueueSnackbar('Document published', 'success');
      // Refresh the document
      window.location.reload();
    } catch (err) {
      enqueueSnackbar('Failed to publish document', 'error');
    }
  };

  const handleDelete = async () => {
    if (!window.confirm('Are you sure you want to delete this document?')) {
      return;
    }

    try {
      await cortexClient.deleteDocument(documentId);
      enqueueSnackbar('Document deleted', 'success');
      navigate('/dashboard/cortex/documents');
    } catch (err) {
      enqueueSnackbar('Failed to delete document', 'error');
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

  const handleTabChange = (_event: React.SyntheticEvent, newValue: number) => {
    setActiveTab(newValue);
  };

  const handleToggleEditMode = () => {
    setEditMode(!editMode);
  };

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
            <IconButton onClick={handleToggleEditMode} color={editMode ? 'primary' : 'default'}>
              <Iconify icon={editMode ? 'solar:eye-bold' : 'solar:pen-bold'} />
            </IconButton>

            <IconButton onClick={handleEdit}>
              <Iconify icon="solar:settings-bold" />
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

        {/* Tabs */}
        <Tabs value={activeTab} onChange={handleTabChange} sx={{ mb: 3 }}>
          <Tab label="Content" />
          <Tab label={`Sections (${sections.length})`} />
          <Tab label={`Links (${links.length})`} />
          <Tab label={`Versions (${versions.length})`} />
        </Tabs>

        {/* Tab Panels */}
        {activeTab === 0 && (
          <Box sx={{
            '& pre': {
              borderRadius: 1,
              p: 2,
              bgcolor: 'background.neutral'
            }
          }}>
            <Markdown content={document.content} />
          </Box>
        )}

        {activeTab === 1 && (
          <DocumentSectionsPanel
            documentId={documentId}
            sections={sections}
            onRefresh={mutateSections}
            editMode={editMode}
          />
        )}

        {activeTab === 2 && (
          <DocumentLinksPanel
            documentId={documentId}
            links={links}
            onRefresh={mutateLinks}
            editMode={editMode}
          />
        )}

        {activeTab === 3 && (
          <DocumentVersionsPanel
            documentId={documentId}
            versions={versions}
            onRefresh={mutateVersions}
            currentVersion={document.version}
          />
        )}
      </Card>
    </Box>
  );
}
