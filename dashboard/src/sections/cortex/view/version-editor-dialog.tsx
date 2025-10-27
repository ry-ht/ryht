import { useState } from 'react';

import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogContent from '@mui/material/DialogContent';
import DialogActions from '@mui/material/DialogActions';
import Button from '@mui/material/Button';
import TextField from '@mui/material/TextField';
import Stack from '@mui/material/Stack';
import Alert from '@mui/material/Alert';

import { useSnackbar } from 'src/components/snackbar';

import { cortexClient } from 'src/lib/cortex-client';

// ----------------------------------------------------------------------

interface VersionEditorDialogProps {
  open: boolean;
  documentId: string;
  onClose: (saved: boolean) => void;
}

// ----------------------------------------------------------------------

export function VersionEditorDialog({ open, documentId, onClose }: VersionEditorDialogProps) {
  const { enqueueSnackbar } = useSnackbar();
  const [loading, setLoading] = useState(false);

  const [formData, setFormData] = useState({
    version: '',
    author: '',
    message: '',
  });

  const handleChange = (field: string) => (event: React.ChangeEvent<HTMLInputElement>) => {
    setFormData((prev) => ({
      ...prev,
      [field]: event.target.value,
    }));
  };

  const handleSubmit = async () => {
    if (!formData.version.trim()) {
      enqueueSnackbar('Version number is required', 'error');
      return;
    }

    if (!formData.message.trim()) {
      enqueueSnackbar('Message is required', 'error');
      return;
    }

    setLoading(true);
    try {
      await cortexClient.createDocumentVersion(documentId, {
        version: formData.version,
        author: formData.author || 'Unknown',
        message: formData.message,
      });
      enqueueSnackbar('Version created', 'success');
      onClose(true);
      // Reset form
      setFormData({
        version: '',
        author: '',
        message: '',
      });
    } catch (error) {
      enqueueSnackbar('Failed to create version', 'error');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onClose={() => onClose(false)} maxWidth="sm" fullWidth>
      <DialogTitle>Create Version</DialogTitle>

      <DialogContent>
        <Stack spacing={3} sx={{ mt: 1 }}>
          <Alert severity="info">
            Create a snapshot of the current document state. This allows you to track changes and
            restore previous versions if needed.
          </Alert>

          <TextField
            label="Version"
            value={formData.version}
            onChange={handleChange('version')}
            fullWidth
            required
            helperText="Version identifier (e.g., 1.0.0, v2, 2024-01-15)"
          />

          <TextField
            label="Author"
            value={formData.author}
            onChange={handleChange('author')}
            fullWidth
            helperText="Your name or identifier"
          />

          <TextField
            label="Change Message"
            value={formData.message}
            onChange={handleChange('message')}
            multiline
            rows={4}
            fullWidth
            required
            helperText="Describe what changed in this version"
          />
        </Stack>
      </DialogContent>

      <DialogActions>
        <Button onClick={() => onClose(false)}>Cancel</Button>
        <Button onClick={handleSubmit} variant="contained" disabled={loading}>
          {loading ? 'Creating...' : 'Create Version'}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
