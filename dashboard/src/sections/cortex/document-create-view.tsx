import { mutate } from 'swr';
import { useCallback } from 'react';
import { useForm } from 'react-hook-form';
import { useNavigate } from 'react-router';

import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import MenuItem from '@mui/material/MenuItem';
import TextField from '@mui/material/TextField';

import { cortexClient , cortexEndpoints } from 'src/lib/cortex-client';

import { Iconify } from 'src/components/iconify';
import { useSnackbar } from 'src/components/snackbar';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

const DOC_TYPES = [
  'Guide',
  'ApiReference',
  'Architecture',
  'Tutorial',
  'Explanation',
  'Troubleshooting',
  'Faq',
  'ReleaseNotes',
  'Example',
  'General',
];

type FormValues = {
  title: string;
  content: string;
  doc_type: string;
  description: string;
  tags: string;
  author: string;
};

export function DocumentCreateView() {
  const navigate = useNavigate();
  const { enqueueSnackbar } = useSnackbar();

  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm<FormValues>({
    defaultValues: {
      title: '',
      content: '',
      doc_type: 'Guide',
      description: '',
      tags: '',
      author: '',
    },
  });

  const onSubmit = useCallback(
    async (data: FormValues) => {
      try {
        const tags = data.tags
          .split(',')
          .map((tag) => tag.trim())
          .filter(Boolean);

        await cortexClient.createDocument({
          title: data.title,
          content: data.content,
          doc_type: data.doc_type,
          description: data.description,
          tags,
          author: data.author,
        });

        mutate(cortexEndpoints.documents.list);
        enqueueSnackbar('Document created successfully', 'success');
        navigate('/cortex/documents');
      } catch (err) {
        enqueueSnackbar('Failed to create document', 'error');
      }
    },
    [navigate, enqueueSnackbar]
  );

  return (
    <Stack spacing={3}>
      <CustomBreadcrumbs
        heading="Create Document"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Cortex', href: '/cortex' },
          { name: 'Documents', href: '/cortex/documents' },
          { name: 'Create' },
        ]}
        sx={{ mb: 3 }}
      />

      <Card sx={{ p: 3, maxWidth: 960 }}>
        <form onSubmit={handleSubmit(onSubmit)}>
          <Stack spacing={3}>
            <TextField
              {...register('title', { required: 'Title is required' })}
              label="Title"
              error={!!errors.title}
              helperText={errors.title?.message}
              fullWidth
              required
            />

            <TextField
              {...register('doc_type')}
              label="Document Type"
              select
              fullWidth
            >
              {DOC_TYPES.map((type) => (
                <MenuItem key={type} value={type}>
                  {type}
                </MenuItem>
              ))}
            </TextField>

            <TextField
              {...register('description')}
              label="Description"
              multiline
              rows={2}
              fullWidth
            />

            <TextField
              {...register('content', { required: 'Content is required' })}
              label="Content (Markdown)"
              multiline
              rows={12}
              error={!!errors.content}
              helperText={errors.content?.message || 'Supports Markdown formatting'}
              fullWidth
              required
            />

            <TextField
              {...register('tags')}
              label="Tags"
              placeholder="tag1, tag2, tag3"
              helperText="Comma-separated tags"
              fullWidth
            />

            <TextField
              {...register('author')}
              label="Author"
              fullWidth
            />

            <Stack direction="row" spacing={2} justifyContent="flex-end">
              <Button
                variant="outlined"
                onClick={() => navigate('/cortex/documents')}
              >
                Cancel
              </Button>
              <Button
                type="submit"
                variant="contained"
                disabled={isSubmitting}
                startIcon={<Iconify icon="mingcute:add-line" />}
              >
                {isSubmitting ? 'Creating...' : 'Create Document'}
              </Button>
            </Stack>
          </Stack>
        </form>
      </Card>
    </Stack>
  );
}
