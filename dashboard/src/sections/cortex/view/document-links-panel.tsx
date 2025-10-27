import { useState } from 'react';

import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import Card from '@mui/material/Card';
import Chip from '@mui/material/Chip';
import Alert from '@mui/material/Alert';
import Grid from '@mui/material/Grid';

import { Iconify } from 'src/components/iconify';
import { useSnackbar } from 'src/components/snackbar';

import { cortexClient } from 'src/lib/cortex-client';
import type { DocumentLink, DocumentLinkType } from 'src/types/cortex';

import { LinkEditorDialog } from './link-editor-dialog';

// ----------------------------------------------------------------------

interface DocumentLinksPanelProps {
  documentId: string;
  links: DocumentLink[];
  onRefresh: () => void;
  editMode: boolean;
}

// ----------------------------------------------------------------------

const LINK_TYPE_CONFIG: Record<
  DocumentLinkType,
  { color: string; icon: string; description: string }
> = {
  Reference: {
    color: 'primary',
    icon: 'eva:link-2-fill',
    description: 'Referenced document or resource',
  },
  Related: { color: 'info', icon: 'eva:shuffle-2-fill', description: 'Related content' },
  Prerequisite: {
    color: 'warning',
    icon: 'eva:arrow-left-fill',
    description: 'Required reading',
  },
  Next: { color: 'success', icon: 'eva:arrow-right-fill', description: 'Next in sequence' },
  Previous: { color: 'secondary', icon: 'eva:arrow-left-fill', description: 'Previous in sequence' },
  Parent: { color: 'default', icon: 'eva:arrow-up-fill', description: 'Parent document' },
  Child: { color: 'default', icon: 'eva:arrow-down-fill', description: 'Child document' },
  External: { color: 'default', icon: 'eva:external-link-fill', description: 'External link' },
  ApiReference: { color: 'primary', icon: 'eva:code-fill', description: 'API Reference' },
  Example: { color: 'success', icon: 'eva:file-text-fill', description: 'Example' },
};

// ----------------------------------------------------------------------

export function DocumentLinksPanel({
  documentId,
  links,
  onRefresh,
  editMode,
}: DocumentLinksPanelProps) {
  const { enqueueSnackbar } = useSnackbar();
  const [editorOpen, setEditorOpen] = useState(false);

  // Group links by type
  const groupedLinks = links.reduce(
    (acc, link) => {
      const type = link.link_type;
      if (!acc[type]) {
        acc[type] = [];
      }
      acc[type].push(link);
      return acc;
    },
    {} as Record<DocumentLinkType, DocumentLink[]>
  );

  const handleCreateLink = () => {
    setEditorOpen(true);
  };

  const handleDeleteLink = async (linkId: string) => {
    if (!window.confirm('Are you sure you want to delete this link?')) {
      return;
    }

    try {
      await cortexClient.deleteDocumentLink(linkId);
      enqueueSnackbar('Link deleted', 'success');
      onRefresh();
    } catch (error) {
      enqueueSnackbar('Failed to delete link', 'error');
    }
  };

  const handleEditorClose = (saved: boolean) => {
    setEditorOpen(false);
    if (saved) {
      onRefresh();
    }
  };

  const renderLink = (link: DocumentLink) => {
    const config = LINK_TYPE_CONFIG[link.link_type];

    return (
      <Card
        key={link.id}
        sx={{
          p: 2,
          mb: 1,
          borderLeft: 3,
          borderColor: `${config.color}.main`,
          '&:hover': {
            bgcolor: 'action.hover',
          },
        }}
      >
        <Stack direction="row" alignItems="center" spacing={2}>
          <Iconify icon={config.icon} width={24} sx={{ color: `${config.color}.main` }} />

          <Box sx={{ flexGrow: 1 }}>
            <Stack direction="row" spacing={1} alignItems="center" sx={{ mb: 0.5 }}>
              <Chip label={link.target_type} size="small" variant="outlined" />
              <Typography variant="subtitle2">
                {link.target_title || link.target_id}
              </Typography>
            </Stack>

            {link.description && (
              <Typography variant="body2" color="text.secondary">
                {link.description}
              </Typography>
            )}

            {link.target_url && (
              <Typography variant="caption" color="text.secondary" sx={{ display: 'block' }}>
                {link.target_url}
              </Typography>
            )}
          </Box>

          <Stack direction="row" spacing={0.5}>
            {link.target_type === 'ExternalUrl' && link.target_url && (
              <IconButton
                size="small"
                onClick={() => window.open(link.target_url, '_blank')}
                color="primary"
              >
                <Iconify icon="eva:external-link-fill" />
              </IconButton>
            )}

            {editMode && (
              <IconButton
                size="small"
                onClick={() => handleDeleteLink(link.id)}
                color="error"
              >
                <Iconify icon="solar:trash-bin-trash-bold" />
              </IconButton>
            )}
          </Stack>
        </Stack>
      </Card>
    );
  };

  const renderLinkGroup = (type: DocumentLinkType, linkList: DocumentLink[]) => {
    const config = LINK_TYPE_CONFIG[type];

    return (
      <Box key={type} sx={{ mb: 3 }}>
        <Stack direction="row" alignItems="center" spacing={1} sx={{ mb: 2 }}>
          <Iconify icon={config.icon} width={20} sx={{ color: `${config.color}.main` }} />
          <Typography variant="h6">{type}</Typography>
          <Chip label={linkList.length} size="small" />
          <Typography variant="caption" color="text.secondary">
            {config.description}
          </Typography>
        </Stack>

        <Box>{linkList.map(renderLink)}</Box>
      </Box>
    );
  };

  return (
    <Box>
      {editMode && (
        <Box sx={{ mb: 3 }}>
          <Button
            variant="contained"
            startIcon={<Iconify icon="eva:plus-fill" />}
            onClick={handleCreateLink}
          >
            Add Link
          </Button>
        </Box>
      )}

      {links.length === 0 ? (
        <Alert severity="info">
          No links found. {editMode && 'Click "Add Link" to create one.'}
        </Alert>
      ) : (
        <Grid container spacing={3}>
          <Grid xs={12} md={6}>
            {/* Navigation Links */}
            {groupedLinks.Previous && renderLinkGroup('Previous', groupedLinks.Previous)}
            {groupedLinks.Next && renderLinkGroup('Next', groupedLinks.Next)}
            {groupedLinks.Parent && renderLinkGroup('Parent', groupedLinks.Parent)}
            {groupedLinks.Child && renderLinkGroup('Child', groupedLinks.Child)}
          </Grid>

          <Grid xs={12} md={6}>
            {/* Content Links */}
            {groupedLinks.Reference && renderLinkGroup('Reference', groupedLinks.Reference)}
            {groupedLinks.Related && renderLinkGroup('Related', groupedLinks.Related)}
            {groupedLinks.Prerequisite && renderLinkGroup('Prerequisite', groupedLinks.Prerequisite)}
            {groupedLinks.ApiReference && renderLinkGroup('ApiReference', groupedLinks.ApiReference)}
            {groupedLinks.Example && renderLinkGroup('Example', groupedLinks.Example)}
            {groupedLinks.External && renderLinkGroup('External', groupedLinks.External)}
          </Grid>
        </Grid>
      )}

      <LinkEditorDialog
        open={editorOpen}
        documentId={documentId}
        onClose={handleEditorClose}
      />
    </Box>
  );
}
