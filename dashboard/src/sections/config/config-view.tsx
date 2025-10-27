import useSWR, { mutate } from 'swr';
import { useState, useCallback } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import LinearProgress from '@mui/material/LinearProgress';

import { axonClient, axonFetcher, axonEndpoints } from 'src/lib/axon-client';

import { Iconify } from 'src/components/iconify';
import { useSnackbar } from 'src/components/snackbar';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

export function ConfigView() {
  const { showSnackbar } = useSnackbar();
  const { data: config, isLoading } = useSWR(axonEndpoints.config, axonFetcher);

  const [axonSettings, setAxonSettings] = useState({
    workspace_name: '',
  });

  const [cortexSettings, setCortexSettings] = useState({
    api_url: import.meta.env.VITE_CORTEX_API_URL || '',
    api_key: '',
  });

  const [isSaving, setIsSaving] = useState(false);

  const handleSaveAxonConfig = useCallback(async () => {
    try {
      setIsSaving(true);
      await axonClient.updateConfig(axonSettings);
      mutate(axonEndpoints.config);
      showSnackbar('Axon configuration updated successfully', 'success');
    } catch (error) {
      console.error('Failed to save config:', error);
      showSnackbar('Failed to save configuration', 'error');
    } finally {
      setIsSaving(false);
    }
  }, [axonSettings, showSnackbar]);

  return (
    <>
      <CustomBreadcrumbs
        heading="System Configuration"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Configuration' },
        ]}
        sx={{ mb: 3 }}
      />

      <Stack spacing={2}>
        <Card sx={{ p: 3 }}>
          <Typography variant="h6" sx={{ mb: 1 }}>
            System Configuration
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Configure Axon multi-agent system and Cortex cognitive backend settings.
          </Typography>
        </Card>

        {isLoading && <LinearProgress />}

        <Grid container spacing={2}>
          <Grid size={{ xs: 12, md: 6 }}>
            <Card sx={{ p: 3 }}>
              <Stack spacing={2}>
                <Stack direction="row" spacing={2} alignItems="center" sx={{ mb: 2 }}>
                  <Box
                    sx={{
                      width: 48,
                      height: 48,
                      borderRadius: 2,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      bgcolor: 'primary.lighter',
                      color: 'primary.main',
                    }}
                  >
                    <Iconify icon="mdi:robot" width={28} />
                  </Box>
                  <Box>
                    <Typography variant="h6">Axon Settings</Typography>
                    <Typography variant="caption" color="text.secondary">
                      Multi-agent system configuration
                    </Typography>
                  </Box>
                </Stack>

                <TextField
                  fullWidth
                  label="Workspace Name"
                  value={axonSettings.workspace_name || config?.workspace_name || ''}
                  onChange={(e) =>
                    setAxonSettings({ ...axonSettings, workspace_name: e.target.value })
                  }
                  helperText="Name of the current workspace"
                />

                <TextField
                  fullWidth
                  label="Workspace Path"
                  value={config?.workspace_path || ''}
                  disabled
                  helperText="Path to workspace directory (read-only)"
                />

                <TextField
                  fullWidth
                  label="API URL"
                  value={import.meta.env.VITE_AXON_API_URL || ''}
                  disabled
                  helperText="Axon API endpoint (configured via environment)"
                />

                <Button
                  variant="contained"
                  size="large"
                  startIcon={<Iconify icon="mdi:content-save" />}
                  onClick={handleSaveAxonConfig}
                  disabled={isSaving}
                  fullWidth
                >
                  {isSaving ? 'Saving...' : 'Save Axon Settings'}
                </Button>
              </Stack>
            </Card>
          </Grid>

          <Grid size={{ xs: 12, md: 6 }}>
            <Card sx={{ p: 3 }}>
              <Stack spacing={2}>
                <Stack direction="row" spacing={2} alignItems="center" sx={{ mb: 2 }}>
                  <Box
                    sx={{
                      width: 48,
                      height: 48,
                      borderRadius: 2,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      bgcolor: 'info.lighter',
                      color: 'info.main',
                    }}
                  >
                    <Iconify icon="mdi:brain" width={28} />
                  </Box>
                  <Box>
                    <Typography variant="h6">Cortex Settings</Typography>
                    <Typography variant="caption" color="text.secondary">
                      Cognitive backend configuration
                    </Typography>
                  </Box>
                </Stack>

                <TextField
                  fullWidth
                  label="API URL"
                  value={cortexSettings.api_url}
                  onChange={(e) => setCortexSettings({ ...cortexSettings, api_url: e.target.value })}
                  disabled
                  helperText="Cortex API endpoint (configured via environment)"
                />

                <TextField
                  fullWidth
                  label="API Key"
                  type="password"
                  value={cortexSettings.api_key}
                  onChange={(e) => setCortexSettings({ ...cortexSettings, api_key: e.target.value })}
                  disabled
                  helperText="Authentication key (configured via environment)"
                />

                <Box sx={{ p: 2, bgcolor: 'background.neutral', borderRadius: 1 }}>
                  <Typography variant="caption" color="text.secondary">
                    Environment Variables:
                  </Typography>
                  <Typography variant="caption" display="block" sx={{ mt: 0.5 }}>
                    VITE_CORTEX_API_URL
                  </Typography>
                  <Typography variant="caption" display="block">
                    VITE_CORTEX_API_KEY
                  </Typography>
                </Box>

                <Button
                  variant="outlined"
                  size="large"
                  disabled
                  fullWidth
                >
                  Cortex Settings (Environment Only)
                </Button>
              </Stack>
            </Card>
          </Grid>
        </Grid>

        <Card sx={{ p: 3, bgcolor: 'info.lighter' }}>
          <Stack direction="row" spacing={2} alignItems="flex-start">
            <Iconify icon="mdi:information" width={24} color="info.main" />
            <Box>
              <Typography variant="subtitle2" color="info.dark">
                Configuration Notes
              </Typography>
              <Typography variant="body2" color="info.dark" sx={{ mt: 0.5 }}>
                Most settings are configured via environment variables for security. Only workspace-specific
                settings can be modified from this interface. To change API endpoints or authentication
                keys, update your .env file and restart the application.
              </Typography>
            </Box>
          </Stack>
        </Card>
      </Stack>
    </>
  );
}
