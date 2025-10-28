import { mutate } from 'swr';
import { useCallback } from 'react';
import { useForm } from 'react-hook-form';
import { useNavigate } from 'react-router';

import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import TextField from '@mui/material/TextField';

import { cortexClient , cortexEndpoints } from 'src/lib/cortex-client';

import { Iconify } from 'src/components/iconify';
import { useSnackbar } from 'src/components/snackbar';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

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
        navigate('/cortex/workspaces');
      } catch (err) {
        enqueueSnackbar('Failed to create workspace', 'error');
      }
    },
    [navigate, enqueueSnackbar]
  );

  return (
    <Stack spacing={3}>
      <CustomBreadcrumbs
        heading="Create Workspace"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Cortex', href: '/cortex' },
          { name: 'Workspaces', href: '/cortex/workspaces' },
          { name: 'Create' },
        ]}
      />

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
                onClick={() => navigate('/cortex/workspaces')}
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
    </Stack>
  );
}
