import { useCallback } from 'react';
import { useNavigate } from 'react-router';
import { useForm } from 'react-hook-form';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';

import { Iconify } from 'src/components/iconify';
import { useSnackbar } from 'src/components/snackbar';

import { cortexClient } from 'src/lib/cortex-client';
import { mutate } from 'swr';
import { cortexEndpoints } from 'src/lib/cortex-client';

type FormValues = {
  name: string;
  path: string;
  description: string;
  language: string;
};

export function WorkspaceCreateView() {
  const navigate = useNavigate();
  const { enqueueSnackbar } = useSnackbar();

  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm<FormValues>({
    defaultValues: {
      name: '',
      path: '',
      description: '',
      language: '',
    },
  });

  const onSubmit = useCallback(
    async (data: FormValues) => {
      try {
        await cortexClient.createWorkspace(data);
        mutate(cortexEndpoints.workspaces.list);
        enqueueSnackbar('Workspace created successfully', 'success');
        navigate('/dashboard/cortex/workspaces');
      } catch (err) {
        enqueueSnackbar('Failed to create workspace', 'error');
      }
    },
    [navigate, enqueueSnackbar]
  );

  return (
    <Box sx={{ p: 3 }}>
      <Button
        startIcon={<Iconify icon="eva:arrow-back-fill" />}
        onClick={() => navigate('/dashboard/cortex/workspaces')}
        sx={{ mb: 3 }}
      >
        Back
      </Button>

      <Typography variant="h4" sx={{ mb: 3 }}>
        Create New Workspace
      </Typography>

      <Card sx={{ p: 3, maxWidth: 720 }}>
        <form onSubmit={handleSubmit(onSubmit)}>
          <Stack spacing={3}>
            <TextField
              {...register('name', { required: 'Name is required' })}
              label="Workspace Name"
              error={!!errors.name}
              helperText={errors.name?.message}
              fullWidth
              required
            />

            <TextField
              {...register('path', { required: 'Path is required' })}
              label="Workspace Path"
              placeholder="/path/to/workspace"
              error={!!errors.path}
              helperText={errors.path?.message || 'Absolute path to workspace directory'}
              fullWidth
              required
            />

            <TextField
              {...register('description')}
              label="Description"
              multiline
              rows={3}
              fullWidth
            />

            <TextField
              {...register('language')}
              label="Primary Language"
              placeholder="typescript, rust, python, etc."
              fullWidth
            />

            <Stack direction="row" spacing={2} justifyContent="flex-end">
              <Button
                variant="outlined"
                onClick={() => navigate('/dashboard/cortex/workspaces')}
              >
                Cancel
              </Button>
              <Button
                type="submit"
                variant="contained"
                disabled={isSubmitting}
                startIcon={<Iconify icon="mingcute:add-line" />}
              >
                {isSubmitting ? 'Creating...' : 'Create Workspace'}
              </Button>
            </Stack>
          </Stack>
        </form>
      </Card>
    </Box>
  );
}
