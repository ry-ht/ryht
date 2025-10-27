import useSWR from 'swr';
import { useState } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import Typography from '@mui/material/Typography';
import InputLabel from '@mui/material/InputLabel';
import FormControl from '@mui/material/FormControl';
import Breadcrumbs from '@mui/material/Breadcrumbs';
import LinearProgress from '@mui/material/LinearProgress';

import { cortexFetcher, cortexEndpoints } from 'src/lib/cortex-client';

import { Iconify } from 'src/components/iconify';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

export function CodeVfsView() {
  const [selectedWorkspace, setSelectedWorkspace] = useState('default');
  const [currentPath, setCurrentPath] = useState('/');

  const { data: workspaces = [] } = useSWR('/api/v1/workspaces', cortexFetcher);

  const { data: files, isLoading } = useSWR(
    selectedWorkspace ? cortexEndpoints.vfs.list(selectedWorkspace, currentPath) : null,
    cortexFetcher,
    { refreshInterval: 10000 }
  );

  const handleNavigate = (path: string) => {
    setCurrentPath(path);
  };

  const pathParts = currentPath.split('/').filter(Boolean);

  return (
    <>
      <CustomBreadcrumbs
        heading="Virtual File System"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Code' },
          { name: 'VFS' },
        ]}
        sx={{ mb: 3 }}
      />

      <Stack spacing={2}>
        <Card sx={{ p: 3 }}>
          <Stack direction="row" justifyContent="space-between" alignItems="center" spacing={2}>
            <Box flex={1}>
              <Typography variant="h6" sx={{ mb: 0.5 }}>
                File System Browser
              </Typography>
              <Typography variant="body2" color="text.secondary">
                Browse and manage files in the virtual file system.
              </Typography>
            </Box>
            <FormControl sx={{ minWidth: 250 }}>
              <InputLabel>Workspace</InputLabel>
              <Select
                value={selectedWorkspace}
                label="Workspace"
                onChange={(e) => setSelectedWorkspace(e.target.value)}
              >
                {workspaces.map((ws: any) => (
                  <MenuItem key={ws.id} value={ws.id}>
                    {ws.name}
                  </MenuItem>
                ))}
                <MenuItem value="default">Default Workspace</MenuItem>
              </Select>
            </FormControl>
          </Stack>
        </Card>

        <Card sx={{ p: 2 }}>
          <Stack direction="row" spacing={2} alignItems="center">
            <Button
              size="small"
              startIcon={<Iconify icon="mdi:home" />}
              onClick={() => handleNavigate('/')}
              disabled={currentPath === '/'}
            >
              Root
            </Button>
            <Breadcrumbs separator={<Iconify icon="mdi:chevron-right" width={16} />}>
              {pathParts.map((part, index) => {
                const path = '/' + pathParts.slice(0, index + 1).join('/');
                return (
                  <Button
                    key={path}
                    size="small"
                    onClick={() => handleNavigate(path)}
                    sx={{ textTransform: 'none' }}
                  >
                    {part}
                  </Button>
                );
              })}
            </Breadcrumbs>
          </Stack>
        </Card>

        {isLoading && <LinearProgress />}

        <Card>
          <Stack spacing={0} divider={<Box sx={{ borderBottom: 1, borderColor: 'divider' }} />}>
            {files?.entries?.map((entry: any) => (
              <Box
                key={entry.path}
                sx={{
                  p: 2,
                  cursor: entry.type === 'directory' ? 'pointer' : 'default',
                  '&:hover': { bgcolor: 'action.hover' },
                }}
                onClick={() => entry.type === 'directory' && handleNavigate(entry.path)}
              >
                <Stack direction="row" spacing={2} alignItems="center">
                  <Box
                    sx={{
                      width: 40,
                      height: 40,
                      borderRadius: 1,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      bgcolor: entry.type === 'directory' ? 'warning.lighter' : 'info.lighter',
                      color: entry.type === 'directory' ? 'warning.main' : 'info.main',
                    }}
                  >
                    <Iconify
                      icon={entry.type === 'directory' ? 'mdi:folder' : 'mdi:file-document'}
                      width={24}
                    />
                  </Box>
                  <Box flex={1}>
                    <Typography variant="subtitle2">{entry.name}</Typography>
                    <Typography variant="caption" color="text.secondary">
                      {entry.type === 'file' && entry.size && `${Math.round(entry.size / 1024)} KB`}
                      {entry.modified_at && ` • Modified: ${new Date(entry.modified_at).toLocaleDateString()}`}
                    </Typography>
                  </Box>
                  {entry.type === 'directory' && (
                    <Iconify icon="mdi:chevron-right" width={20} color="text.disabled" />
                  )}
                </Stack>
              </Box>
            ))}
          </Stack>

          {!isLoading && (!files?.entries || files.entries.length === 0) && (
            <Box sx={{ p: 5, textAlign: 'center' }}>
              <Iconify icon="mdi:folder-open" width={64} sx={{ color: 'text.disabled', mb: 2 }} />
              <Typography variant="h6" color="text.secondary">
                No files or directories
              </Typography>
              <Typography variant="body2" color="text.disabled" sx={{ mt: 1 }}>
                This directory is empty
              </Typography>
            </Box>
          )}
        </Card>
      </Stack>
    </>
  );
}
