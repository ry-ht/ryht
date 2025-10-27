import { useState } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import Typography from '@mui/material/Typography';
import InputLabel from '@mui/material/InputLabel';
import FormControl from '@mui/material/FormControl';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

const MOCK_DEPENDENCIES = [
  {
    name: 'react',
    version: '18.2.0',
    type: 'production',
    dependents: 45,
    size: '125 KB',
    status: 'up-to-date',
  },
  {
    name: '@mui/material',
    version: '5.14.0',
    type: 'production',
    dependents: 32,
    size: '2.1 MB',
    status: 'up-to-date',
  },
  {
    name: 'typescript',
    version: '5.0.4',
    type: 'development',
    dependents: 0,
    size: '34 MB',
    status: 'outdated',
  },
  {
    name: 'axios',
    version: '1.4.0',
    type: 'production',
    dependents: 12,
    size: '89 KB',
    status: 'up-to-date',
  },
];

export function CodeDependenciesView() {
  const [selectedWorkspace, setSelectedWorkspace] = useState('default');
  const [filterType, setFilterType] = useState('all');

  const filteredDeps = MOCK_DEPENDENCIES.filter(
    (dep) => filterType === 'all' || dep.type === filterType
  );

  const getStatusColor = (status: string) => {
    if (status === 'up-to-date') return 'success';
    if (status === 'outdated') return 'warning';
    return 'error';
  };

  return (
    <>
      <CustomBreadcrumbs
        heading="Dependency Graph"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Code' },
          { name: 'Dependencies' },
        ]}
        sx={{ mb: 3 }}
      />

      <Stack spacing={2}>
        <Card sx={{ p: 3 }}>
          <Stack direction="row" justifyContent="space-between" alignItems="center" spacing={2}>
            <Box>
              <Typography variant="h6" sx={{ mb: 0.5 }}>
                Dependency Graph Viewer
              </Typography>
              <Typography variant="body2" color="text.secondary">
                Visualize and manage project dependencies and their relationships.
              </Typography>
            </Box>
            <Stack direction="row" spacing={2}>
              <FormControl sx={{ minWidth: 150 }}>
                <InputLabel>Type</InputLabel>
                <Select value={filterType} label="Type" onChange={(e) => setFilterType(e.target.value)}>
                  <MenuItem value="all">All Types</MenuItem>
                  <MenuItem value="production">Production</MenuItem>
                  <MenuItem value="development">Development</MenuItem>
                </Select>
              </FormControl>
              <FormControl sx={{ minWidth: 200 }}>
                <InputLabel>Workspace</InputLabel>
                <Select
                  value={selectedWorkspace}
                  label="Workspace"
                  onChange={(e) => setSelectedWorkspace(e.target.value)}
                >
                  <MenuItem value="default">Default Workspace</MenuItem>
                </Select>
              </FormControl>
            </Stack>
          </Stack>
        </Card>

        <Stack spacing={1}>
          {filteredDeps.map((dep) => (
            <Card key={dep.name} sx={{ p: 2.5 }}>
              <Stack direction="row" justifyContent="space-between" alignItems="center" spacing={2}>
                <Stack direction="row" spacing={2} alignItems="center" flex={1}>
                  <Box
                    sx={{
                      width: 48,
                      height: 48,
                      borderRadius: 1.5,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      bgcolor: 'primary.lighter',
                      color: 'primary.main',
                    }}
                  >
                    <Iconify icon="mdi:package" width={24} />
                  </Box>
                  <Box flex={1}>
                    <Stack direction="row" spacing={1} alignItems="center">
                      <Typography variant="h6">{dep.name}</Typography>
                      <Label variant="soft" color={dep.type === 'production' ? 'primary' : 'default'}>
                        {dep.type}
                      </Label>
                      <Label variant="soft" color={getStatusColor(dep.status)}>
                        {dep.status}
                      </Label>
                    </Stack>
                    <Stack direction="row" spacing={2} sx={{ mt: 0.5 }}>
                      <Typography variant="caption" color="text.secondary">
                        v{dep.version}
                      </Typography>
                      <Typography variant="caption" color="text.secondary">
                        • {dep.size}
                      </Typography>
                      <Typography variant="caption" color="text.secondary">
                        • {dep.dependents} dependents
                      </Typography>
                    </Stack>
                  </Box>
                </Stack>
                <Stack direction="row" spacing={1}>
                  <Iconify
                    icon="mdi:network"
                    width={20}
                    color="text.secondary"
                    sx={{ cursor: 'pointer', '&:hover': { color: 'primary.main' } }}
                  />
                  <Iconify
                    icon="mdi:information"
                    width={20}
                    color="text.secondary"
                    sx={{ cursor: 'pointer', '&:hover': { color: 'primary.main' } }}
                  />
                </Stack>
              </Stack>
            </Card>
          ))}
        </Stack>

        <Card sx={{ p: 3, bgcolor: 'info.lighter' }}>
          <Stack direction="row" spacing={2} alignItems="flex-start">
            <Iconify icon="mdi:information" width={24} color="info.main" />
            <Box>
              <Typography variant="subtitle2" color="info.dark">
                Dependency Analysis
              </Typography>
              <Typography variant="body2" color="info.dark" sx={{ mt: 0.5 }}>
                Total dependencies: {MOCK_DEPENDENCIES.length} •
                Production: {MOCK_DEPENDENCIES.filter(d => d.type === 'production').length} •
                Development: {MOCK_DEPENDENCIES.filter(d => d.type === 'development').length}
              </Typography>
            </Box>
          </Stack>
        </Card>
      </Stack>
    </>
  );
}
