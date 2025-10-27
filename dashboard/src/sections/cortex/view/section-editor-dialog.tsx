import type { DocumentSection } from 'src/types/cortex';

import { useState, useEffect } from 'react';

import Box from '@mui/material/Box';
import Tab from '@mui/material/Tab';
import Tabs from '@mui/material/Tabs';
import Stack from '@mui/material/Stack';
import Dialog from '@mui/material/Dialog';
import Button from '@mui/material/Button';
import MenuItem from '@mui/material/MenuItem';
import TextField from '@mui/material/TextField';
import DialogTitle from '@mui/material/DialogTitle';
import DialogContent from '@mui/material/DialogContent';
import DialogActions from '@mui/material/DialogActions';

import { cortexClient } from 'src/lib/cortex-client';

import { Markdown } from 'src/components/markdown';
import { useSnackbar } from 'src/components/snackbar';

// ----------------------------------------------------------------------

interface SectionEditorDialogProps {
  open: boolean;
  documentId: string;
  section: DocumentSection | null;
  sections: DocumentSection[];
  onClose: (saved: boolean) => void;
}

// ----------------------------------------------------------------------

export function SectionEditorDialog({
  open,
  documentId,
  section,
  sections,
  onClose,
}: SectionEditorDialogProps) {
  const { enqueueSnackbar } = useSnackbar();
  const [loading, setLoading] = useState(false);
  const [tab, setTab] = useState(0);

  const [formData, setFormData] = useState({
    title: '',
    content: '',
    level: 1,
    order: 0,
    parent_section_id: '',
  });

  useEffect(() => {
    if (section) {
      setFormData({
        title: section.title,
        content: section.content,
        level: section.level,
        order: section.order,
        parent_section_id: section.parent_section_id || '',
      });
    } else {
      setFormData({
        title: '',
        content: '',
        level: 1,
        order: sections.length,
        parent_section_id: '',
      });
    }
  }, [section, sections]);

  const handleChange = (field: string) => (event: React.ChangeEvent<HTMLInputElement>) => {
    setFormData((prev) => ({
      ...prev,
      [field]: event.target.value,
    }));
  };

  const handleSubmit = async () => {
    if (!formData.title.trim()) {
      enqueueSnackbar('Title is required', 'error');
      return;
    }

    setLoading(true);
    try {
      if (section) {
        await cortexClient.updateSection(section.id, {
          title: formData.title,
          content: formData.content,
          order: formData.order,
        });
        enqueueSnackbar('Section updated', 'success');
      } else {
        await cortexClient.createSection(documentId, {
          title: formData.title,
          content: formData.content,
          level: formData.level,
          order: formData.order,
        });
        enqueueSnackbar('Section created', 'success');
      }
      onClose(true);
    } catch (error) {
      enqueueSnackbar('Failed to save section', 'error');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onClose={() => onClose(false)} maxWidth="md" fullWidth>
      <DialogTitle>{section ? 'Edit Section' : 'Create Section'}</DialogTitle>

      <DialogContent>
        <Stack spacing={3} sx={{ mt: 1 }}>
          <TextField
            label="Title"
            value={formData.title}
            onChange={handleChange('title')}
            fullWidth
            required
          />

          <Stack direction="row" spacing={2}>
            <TextField
              label="Level"
              type="number"
              value={formData.level}
              onChange={handleChange('level')}
              inputProps={{ min: 1, max: 6 }}
              disabled={!!section}
              helperText="Heading level (1-6)"
              sx={{ width: 120 }}
            />

            <TextField
              label="Order"
              type="number"
              value={formData.order}
              onChange={handleChange('order')}
              inputProps={{ min: 0 }}
              helperText="Display order"
              sx={{ width: 120 }}
            />

            <TextField
              label="Parent Section"
              select
              value={formData.parent_section_id}
              onChange={handleChange('parent_section_id')}
              fullWidth
              helperText="Optional parent section"
            >
              <MenuItem value="">None</MenuItem>
              {sections
                .filter((s) => s.id !== section?.id)
                .map((s) => (
                  <MenuItem key={s.id} value={s.id}>
                    {s.title} (H{s.level})
                  </MenuItem>
                ))}
            </TextField>
          </Stack>

          <Box>
            <Tabs value={tab} onChange={(_, newValue) => setTab(newValue)} sx={{ mb: 2 }}>
              <Tab label="Edit" />
              <Tab label="Preview" />
            </Tabs>

            {tab === 0 && (
              <TextField
                label="Content"
                value={formData.content}
                onChange={handleChange('content')}
                multiline
                rows={12}
                fullWidth
                helperText="Markdown supported"
              />
            )}

            {tab === 1 && (
              <Box
                sx={{
                  p: 2,
                  border: 1,
                  borderColor: 'divider',
                  borderRadius: 1,
                  minHeight: 300,
                  bgcolor: 'background.neutral',
                }}
              >
                <Markdown content={formData.content || '*No content*'} />
              </Box>
            )}
          </Box>
        </Stack>
      </DialogContent>

      <DialogActions>
        <Button onClick={() => onClose(false)}>Cancel</Button>
        <Button onClick={handleSubmit} variant="contained" disabled={loading}>
          {loading ? 'Saving...' : 'Save'}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
