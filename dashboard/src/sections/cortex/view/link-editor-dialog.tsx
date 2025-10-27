import type { LinkTargetType, DocumentLinkType } from 'src/types/cortex';

import { useState } from 'react';

import Stack from '@mui/material/Stack';
import Alert from '@mui/material/Alert';
import Dialog from '@mui/material/Dialog';
import Button from '@mui/material/Button';
import MenuItem from '@mui/material/MenuItem';
import TextField from '@mui/material/TextField';
import DialogTitle from '@mui/material/DialogTitle';
import DialogContent from '@mui/material/DialogContent';
import DialogActions from '@mui/material/DialogActions';

import { cortexClient } from 'src/lib/cortex-client';

import { useSnackbar } from 'src/components/snackbar';

// ----------------------------------------------------------------------

interface LinkEditorDialogProps {
  open: boolean;
  documentId: string;
  onClose: (saved: boolean) => void;
}

const LINK_TYPES: DocumentLinkType[] = [
  'Reference',
  'Related',
  'Prerequisite',
  'Next',
  'Previous',
  'Parent',
  'Child',
  'External',
  'ApiReference',
  'Example',
];

const TARGET_TYPES: LinkTargetType[] = ['Document', 'CodeUnit', 'ExternalUrl'];

// ----------------------------------------------------------------------

export function LinkEditorDialog({ open, documentId, onClose }: LinkEditorDialogProps) {
  const { enqueueSnackbar } = useSnackbar();
  const [loading, setLoading] = useState(false);

  const [formData, setFormData] = useState({
    link_type: 'Reference' as DocumentLinkType,
    target_type: 'Document' as LinkTargetType,
    target_id: '',
    description: '',
  });

  const handleChange = (field: string) => (event: React.ChangeEvent<HTMLInputElement>) => {
    setFormData((prev) => ({
      ...prev,
      [field]: event.target.value,
    }));
  };

  const handleSubmit = async () => {
    if (!formData.target_id.trim()) {
      enqueueSnackbar('Target ID/URL is required', 'error');
      return;
    }

    setLoading(true);
    try {
      await cortexClient.createDocumentLink(documentId, {
        link_type: formData.link_type,
        target_type: formData.target_type,
        target_id: formData.target_id,
      });
      enqueueSnackbar('Link created', 'success');
      onClose(true);
      // Reset form
      setFormData({
        link_type: 'Reference',
        target_type: 'Document',
        target_id: '',
        description: '',
      });
    } catch (error) {
      enqueueSnackbar('Failed to create link', 'error');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onClose={() => onClose(false)} maxWidth="sm" fullWidth>
      <DialogTitle>Create Link</DialogTitle>

      <DialogContent>
        <Stack spacing={3} sx={{ mt: 1 }}>
          <Alert severity="info">
            Create a link to another document, code unit, or external URL.
          </Alert>

          <TextField
            label="Link Type"
            select
            value={formData.link_type}
            onChange={handleChange('link_type')}
            fullWidth
            helperText="Type of relationship"
          >
            {LINK_TYPES.map((type) => (
              <MenuItem key={type} value={type}>
                {type}
              </MenuItem>
            ))}
          </TextField>

          <TextField
            label="Target Type"
            select
            value={formData.target_type}
            onChange={handleChange('target_type')}
            fullWidth
            helperText="What you're linking to"
          >
            {TARGET_TYPES.map((type) => (
              <MenuItem key={type} value={type}>
                {type}
              </MenuItem>
            ))}
          </TextField>

          <TextField
            label={
              formData.target_type === 'ExternalUrl'
                ? 'URL'
                : formData.target_type === 'Document'
                  ? 'Document ID or Slug'
                  : 'Code Unit ID'
            }
            value={formData.target_id}
            onChange={handleChange('target_id')}
            fullWidth
            required
            helperText={
              formData.target_type === 'ExternalUrl'
                ? 'Full URL (e.g., https://example.com)'
                : formData.target_type === 'Document'
                  ? 'Document ID or slug'
                  : 'Code unit identifier'
            }
          />

          <TextField
            label="Description"
            value={formData.description}
            onChange={handleChange('description')}
            multiline
            rows={3}
            fullWidth
            helperText="Optional description of the link"
          />
        </Stack>
      </DialogContent>

      <DialogActions>
        <Button onClick={() => onClose(false)}>Cancel</Button>
        <Button onClick={handleSubmit} variant="contained" disabled={loading}>
          {loading ? 'Creating...' : 'Create Link'}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
