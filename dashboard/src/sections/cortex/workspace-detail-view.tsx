import type { Workspace, WorkspaceStats } from 'src/types/cortex';

import useSWR, { mutate } from 'swr';
import { useState , useCallback } from 'react';
import { useParams, useNavigate } from 'react-router';

import Box from '@mui/material/Box';
import Tab from '@mui/material/Tab';
import Card from '@mui/material/Card';
import Chip from '@mui/material/Chip';
import List from '@mui/material/List';
import TabList from '@mui/lab/TabList';
import Stack from '@mui/material/Stack';
import Alert from '@mui/material/Alert';
import TabPanel from '@mui/lab/TabPanel';
import Button from '@mui/material/Button';
import TabContext from '@mui/lab/TabContext';
import { alpha } from '@mui/material/styles';
import ListItem from '@mui/material/ListItem';
import Typography from '@mui/material/Typography';
import ListItemText from '@mui/material/ListItemText';
import ListItemIcon from '@mui/material/ListItemIcon';
import LinearProgress from '@mui/material/LinearProgress';

import { fData } from 'src/utils/format-number';
import { fDateTime } from 'src/utils/format-time';

import { cortexClient, cortexFetcher, cortexEndpoints } from 'src/lib/cortex-client';

import { Iconify } from 'src/components/iconify';
import { useSnackbar } from 'src/components/snackbar';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

export function WorkspaceDetailView() {
  const params = useParams();
  const navigate = useNavigate();
  const workspaceId = params.id as string;
  const { enqueueSnackbar } = useSnackbar();
  const [currentTab, setCurrentTab] = useState('overview');

  // Fetch workspace
  const { data: workspace, isLoading: loadingWorkspace } = useSWR<Workspace>(
    workspaceId ? cortexEndpoints.workspaces.get(workspaceId) : null,
    cortexFetcher,
    { refreshInterval: 30000 }
  );

  // Fetch workspace stats
  const { data: stats, isLoading: loadingStats } = useSWR<WorkspaceStats>(
    workspaceId ? cortexEndpoints.workspaces.stats(workspaceId) : null,
    cortexFetcher,
    { refreshInterval: 30000 }
  );

  // Fetch active sessions
  const { data: sessions = [] } = useSWR<any[]>(
    '/api/v1/sessions',
    cortexFetcher,
    { refreshInterval: 10000 }
  );

  const workspaceSessions = sessions.filter((s) => s.workspace_id === workspaceId);

  // ----------------------------------------------------------------------
  // Handlers
  // ----------------------------------------------------------------------

  const handleIndexWorkspace = useCallback(async () => {
    try {
      await cortexClient.indexWorkspace(workspaceId);
      enqueueSnackbar('Workspace indexing started', 'success');
      setTimeout(() => {
        mutate(cortexEndpoints.workspaces.stats(workspaceId));
      }, 2000);
    } catch (err) {
      enqueueSnackbar('Failed to start indexing', 'error');
    }
  }, [workspaceId, enqueueSnackbar]);

  const handleDeleteWorkspace = useCallback(async () => {
    if (!window.confirm('Are you sure you want to delete this workspace?')) return;

    try {
      await cortexClient.deleteWorkspace(workspaceId);
      enqueueSnackbar('Workspace deleted successfully', 'success');
      navigate('/cortex/workspaces');
    } catch (err) {
      enqueueSnackbar('Failed to delete workspace', 'error');
    }
  }, [workspaceId, navigate, enqueueSnackbar]);

  const handleTabChange = useCallback((_event: React.SyntheticEvent, newValue: string) => {
    setCurrentTab(newValue);
  }, []);

  // ----------------------------------------------------------------------
  // Loading State
  // ----------------------------------------------------------------------

  if (loadingWorkspace || loadingStats) {
    return (
      <Box sx={{ p: 3 }}>
        <Typography variant="h4" sx={{ mb: 3 }}>
          Loading workspace...
        </Typography>
        <LinearProgress />
      </Box>
    );
  }

  if (!workspace) {
    return (
      <Box sx={{ p: 3 }}>
        <Alert severity="error">Workspace not found</Alert>
      </Box>
    );
  }

  // ----------------------------------------------------------------------
  // Render
  // ----------------------------------------------------------------------

  return (
    <Stack spacing={3}>
      {/* Header */}
      <Stack direction="row" alignItems="center" spacing={2}>
        <Button
          startIcon={<Iconify icon="eva:arrow-back-fill" />}
          onClick={() => navigate('/cortex/workspaces')}
        >
          Back
        </Button>
        <Typography variant="h4" sx={{ flexGrow: 1 }}>
          {workspace.name}
        </Typography>
        <Button
          variant="outlined"
          startIcon={<Iconify icon="solar:refresh-bold-duotone" />}
          onClick={handleIndexWorkspace}
        >
          Re-index
        </Button>
        <Button
          variant="outlined"
          color="error"
          startIcon={<Iconify icon="solar:trash-bin-trash-bold-duotone" />}
          onClick={handleDeleteWorkspace}
        >
          Delete
        </Button>
      </Stack>

      {/* Workspace Info Card */}
      <Card sx={{ p: 3, mb: 3 }}>
        <Box sx={{ display: "flex", gap: 3, flexWrap: "wrap" }}>
          <Box sx={{ flex: "1 1 auto", minWidth: { xs: "100%", md: "calc(8/12*100% - 16px)" } }}>
            <Stack spacing={2}>
              <Box>
                <Typography variant="overline" color="text.secondary">
                  Description
                </Typography>
                <Typography variant="body1">
                  {workspace.description || 'No description provided'}
                </Typography>
              </Box>
              <Box>
                <Typography variant="overline" color="text.secondary">
                  Path
                </Typography>
                <Typography
                  variant="body2"
                  sx={{
                    fontFamily: 'monospace',
                    bgcolor: 'background.neutral',
                    p: 1,
                    borderRadius: 1,
                  }}
                >
                  {workspace.path}
                </Typography>
              </Box>
              {workspace.root_directory && (
                <Box>
                  <Typography variant="overline" color="text.secondary">
                    Root Directory
                  </Typography>
                  <Typography
                    variant="body2"
                    sx={{
                      fontFamily: 'monospace',
                      bgcolor: 'background.neutral',
                      p: 1,
                      borderRadius: 1,
                    }}
                  >
                    {workspace.root_directory}
                  </Typography>
                </Box>
              )}
            </Stack>
          </Box>
          <Box sx={{ flex: "1 1 auto", minWidth: { xs: "100%", md: "calc(4/12*100% - 16px)" } }}>
            <Stack spacing={2}>
              {workspace.language && (
                <Box>
                  <Typography variant="overline" color="text.secondary">
                    Language
                  </Typography>
                  <Box sx={{ mt: 0.5 }}>
                    <Chip label={workspace.language} color="primary" />
                  </Box>
                </Box>
              )}
              <Box>
                <Typography variant="overline" color="text.secondary">
                  Created
                </Typography>
                <Typography variant="body2">{fDateTime(workspace.created_at)}</Typography>
              </Box>
              <Box>
                <Typography variant="overline" color="text.secondary">
                  Last Updated
                </Typography>
                <Typography variant="body2">{fDateTime(workspace.updated_at)}</Typography>
              </Box>
            </Stack>
          </Box>
        </Box>
      </Card>

      {/* Stats Cards */}
      {stats && (
        <Box sx={{ display: "flex", gap: 3, flexWrap: "wrap", mb: 3 }}>
          <Box sx={{ flex: "1 1 auto", minWidth: { xs: "100%", sm: "calc(6/12*100% - 16px)", md: "calc(3/12*100% - 16px)" } }}>
            <Card sx={{ p: 3 }}>
              <Stack spacing={1}>
                <Stack direction="row" alignItems="center" spacing={1}>
                  <Box
                    sx={{
                      width: 48,
                      height: 48,
                      borderRadius: 1.5,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      bgcolor: (theme) => alpha(theme.palette.primary.main, 0.08),
                    }}
                  >
                    <Iconify
                      icon="solar:file-bold-duotone"
                      width={24}
                      sx={{ color: 'primary.main' }}
                    />
                  </Box>
                  <Typography variant="h3">{stats.file_count.toLocaleString()}</Typography>
                </Stack>
                <Typography variant="subtitle2" color="text.secondary">
                  Total Files
                </Typography>
              </Stack>
            </Card>
          </Box>

          <Box sx={{ flex: "1 1 auto", minWidth: { xs: "100%", sm: "calc(6/12*100% - 16px)", md: "calc(3/12*100% - 16px)" } }}>
            <Card sx={{ p: 3 }}>
              <Stack spacing={1}>
                <Stack direction="row" alignItems="center" spacing={1}>
                  <Box
                    sx={{
                      width: 48,
                      height: 48,
                      borderRadius: 1.5,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      bgcolor: (theme) => alpha(theme.palette.info.main, 0.08),
                    }}
                  >
                    <Iconify
                      icon="solar:code-bold-duotone"
                      width={24}
                      sx={{ color: 'info.main' }}
                    />
                  </Box>
                  <Typography variant="h3">{stats.code_units_count.toLocaleString()}</Typography>
                </Stack>
                <Typography variant="subtitle2" color="text.secondary">
                  Code Units
                </Typography>
              </Stack>
            </Card>
          </Box>

          <Box sx={{ flex: "1 1 auto", minWidth: { xs: "100%", sm: "calc(6/12*100% - 16px)", md: "calc(3/12*100% - 16px)" } }}>
            <Card sx={{ p: 3 }}>
              <Stack spacing={1}>
                <Stack direction="row" alignItems="center" spacing={1}>
                  <Box
                    sx={{
                      width: 48,
                      height: 48,
                      borderRadius: 1.5,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      bgcolor: (theme) => alpha(theme.palette.success.main, 0.08),
                    }}
                  >
                    <Iconify
                      icon="solar:database-bold-duotone"
                      width={24}
                      sx={{ color: 'success.main' }}
                    />
                  </Box>
                  <Typography variant="h3">{fData(stats.total_size)}</Typography>
                </Stack>
                <Typography variant="subtitle2" color="text.secondary">
                  Total Size
                </Typography>
              </Stack>
            </Card>
          </Box>

          <Box sx={{ flex: "1 1 auto", minWidth: { xs: "100%", sm: "calc(6/12*100% - 16px)", md: "calc(3/12*100% - 16px)" } }}>
            <Card sx={{ p: 3 }}>
              <Stack spacing={1}>
                <Stack direction="row" alignItems="center" spacing={1}>
                  <Box
                    sx={{
                      width: 48,
                      height: 48,
                      borderRadius: 1.5,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      bgcolor: (theme) => alpha(theme.palette.warning.main, 0.08),
                    }}
                  >
                    <Iconify
                      icon="solar:clock-circle-bold-duotone"
                      width={24}
                      sx={{ color: 'warning.main' }}
                    />
                  </Box>
                  <Typography variant="caption" sx={{ flexGrow: 1 }}>
                    {stats.last_indexed ? fDateTime(stats.last_indexed) : 'Never'}
                  </Typography>
                </Stack>
                <Typography variant="subtitle2" color="text.secondary">
                  Last Indexed
                </Typography>
              </Stack>
            </Card>
          </Box>
        </Box>
      )}

      {/* Tabs */}
      <Card>
        <TabContext value={currentTab}>
          <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
            <TabList onChange={handleTabChange}>
              <Tab label="Overview" value="overview" />
              <Tab label="Files" value="files" />
              <Tab label="Code Units" value="code-units" />
              <Tab label="Dependencies" value="dependencies" />
              <Tab label="Sessions" value="sessions" />
            </TabList>
          </Box>

          <TabPanel value="overview">
            <Box sx={{ display: "flex", gap: 3, flexWrap: "wrap" }}>
              <Box sx={{ flex: "1 1 auto", minWidth: { xs: "100%", md: "calc(6/12*100% - 16px)" } }}>
                <Typography variant="h6" sx={{ mb: 2 }}>
                  Quick Actions
                </Typography>
                <Stack spacing={2}>
                  <Button
                    variant="outlined"
                    fullWidth
                    startIcon={<Iconify icon="solar:folder-open-bold-duotone" />}
                    onClick={() => navigate(`/cortex/workspaces/${workspaceId}/browse`)}
                    sx={{ justifyContent: 'flex-start' }}
                  >
                    Browse Files
                  </Button>
                  <Button
                    variant="outlined"
                    fullWidth
                    startIcon={<Iconify icon="solar:code-bold-duotone" />}
                    onClick={() => navigate(`/cortex/workspaces/${workspaceId}/code-units`)}
                    sx={{ justifyContent: 'flex-start' }}
                  >
                    View Code Units
                  </Button>
                  <Button
                    variant="outlined"
                    fullWidth
                    startIcon={<Iconify icon="solar:diagram-bold-duotone" />}
                    onClick={() => navigate(`/cortex/workspaces/${workspaceId}/dependencies`)}
                    sx={{ justifyContent: 'flex-start' }}
                  >
                    View Dependencies
                  </Button>
                  <Button
                    variant="outlined"
                    fullWidth
                    startIcon={<Iconify icon="solar:refresh-bold-duotone" />}
                    onClick={handleIndexWorkspace}
                    sx={{ justifyContent: 'flex-start' }}
                  >
                    Re-index Workspace
                  </Button>
                </Stack>
              </Box>

              <Box sx={{ flex: "1 1 auto", minWidth: { xs: "100%", md: "calc(6/12*100% - 16px)" } }}>
                <Typography variant="h6" sx={{ mb: 2 }}>
                  Workspace Metadata
                </Typography>
                {workspace.metadata && Object.keys(workspace.metadata).length > 0 ? (
                  <List>
                    {Object.entries(workspace.metadata).map(([key, value]) => (
                      <ListItem key={key}>
                        <ListItemIcon>
                          <Iconify icon="solar:tag-bold-duotone" />
                        </ListItemIcon>
                        <ListItemText
                          primary={key}
                          secondary={typeof value === 'object' ? JSON.stringify(value) : String(value)}
                        />
                      </ListItem>
                    ))}
                  </List>
                ) : (
                  <Typography variant="body2" color="text.secondary">
                    No metadata available
                  </Typography>
                )}
              </Box>
            </Box>
          </TabPanel>

          <TabPanel value="files">
            <Box sx={{ textAlign: 'center', py: 5 }}>
              <Iconify icon="solar:folder-open-bold-duotone" width={64} sx={{ mb: 2, opacity: 0.5 }} />
              <Typography variant="h6" sx={{ mb: 2 }}>
                File Browser
              </Typography>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
                Browse and manage files in this workspace
              </Typography>
              <Button
                variant="contained"
                startIcon={<Iconify icon="solar:folder-open-bold-duotone" />}
                onClick={() => navigate(`/cortex/workspaces/${workspaceId}/browse`)}
              >
                Open File Browser
              </Button>
            </Box>
          </TabPanel>

          <TabPanel value="code-units">
            <Box sx={{ textAlign: 'center', py: 5 }}>
              <Iconify icon="solar:code-bold-duotone" width={64} sx={{ mb: 2, opacity: 0.5 }} />
              <Typography variant="h6" sx={{ mb: 2 }}>
                Code Units
              </Typography>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
                Explore functions, classes, and other code structures
              </Typography>
              <Button
                variant="contained"
                startIcon={<Iconify icon="solar:code-bold-duotone" />}
                onClick={() => navigate(`/cortex/workspaces/${workspaceId}/code-units`)}
              >
                View Code Units
              </Button>
            </Box>
          </TabPanel>

          <TabPanel value="dependencies">
            <Box sx={{ textAlign: 'center', py: 5 }}>
              <Iconify icon="solar:diagram-bold-duotone" width={64} sx={{ mb: 2, opacity: 0.5 }} />
              <Typography variant="h6" sx={{ mb: 2 }}>
                Dependency Graph
              </Typography>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
                Visualize relationships between code units
              </Typography>
              <Button
                variant="contained"
                startIcon={<Iconify icon="solar:diagram-bold-duotone" />}
                onClick={() => navigate(`/cortex/workspaces/${workspaceId}/dependencies`)}
              >
                View Dependencies
              </Button>
            </Box>
          </TabPanel>

          <TabPanel value="sessions">
            {workspaceSessions.length > 0 ? (
              <List>
                {workspaceSessions.map((session) => (
                  <ListItem key={session.id}>
                    <ListItemIcon>
                      <Iconify icon="solar:users-group-rounded-bold-duotone" />
                    </ListItemIcon>
                    <ListItemText
                      primary={session.name}
                      secondary={`Created ${fDateTime(session.created_at)}`}
                    />
                  </ListItem>
                ))}
              </List>
            ) : (
              <Box sx={{ textAlign: 'center', py: 5 }}>
                <Typography variant="body2" color="text.secondary">
                  No active sessions in this workspace
                </Typography>
              </Box>
            )}
          </TabPanel>
        </TabContext>
      </Card>
    </Stack>
  );
}
