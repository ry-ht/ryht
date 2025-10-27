import type { AgentInfo, AgentMetricsData, AgentMetricsTimeSeries, AgentLogEntry } from 'src/types/axon';

import useSWR, { mutate } from 'swr';
import { useState, useCallback, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router';
import { LineChart, Line, AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';

import Box from '@mui/material/Box';
import Grid from '@mui/material/Grid';
import Card from '@mui/material/Card';
import Chip from '@mui/material/Chip';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import CardHeader from '@mui/material/CardHeader';
import CardContent from '@mui/material/CardContent';
import TextField from '@mui/material/TextField';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import FormControl from '@mui/material/FormControl';
import InputLabel from '@mui/material/InputLabel';
import Paper from '@mui/material/Paper';
import Divider from '@mui/material/Divider';

import { getAgentStatusColor } from 'src/utils/status-colors';

import { axonClient, axonFetcher, axonEndpoints } from 'src/lib/axon-client';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { useSnackbar } from 'src/components/snackbar';
import { AnimateCountUp } from 'src/components/animate';

// ----------------------------------------------------------------------

const COLORS = {
  primary: '#1976d2',
  success: '#4caf50',
  warning: '#ff9800',
  error: '#f44336',
  info: '#2196f3',
};

// ----------------------------------------------------------------------

export function AgentDetailView() {
  const { id } = useParams();
  const navigate = useNavigate();
  const { showSnackbar } = useSnackbar();

  const [logLevel, setLogLevel] = useState<string>('all');
  const [logSearch, setLogSearch] = useState<string>('');
  const [autoRefresh, setAutoRefresh] = useState(true);

  // Fetch agent details
  const { data: agent, isLoading } = useSWR<AgentInfo>(
    id ? axonEndpoints.agents.details(id) : null,
    axonFetcher,
    { refreshInterval: autoRefresh ? 3000 : 0 }
  );

  // Fetch agent metrics (mock data - replace with real endpoint)
  const mockMetrics: AgentMetricsData = {
    agent_id: id || '',
    agent_name: agent?.name || '',
    tasks_completed: agent?.metadata.tasks_completed || 0,
    tasks_failed: agent?.metadata.tasks_failed || 0,
    success_rate: agent ? (agent.metadata.tasks_completed / (agent.metadata.tasks_completed + agent.metadata.tasks_failed || 1)) * 100 : 0,
    avg_response_time_ms: agent?.metadata.avg_task_duration_ms || 0,
    cpu_usage_percent: Math.random() * 100,
    memory_usage_mb: Math.random() * 512,
    tokens_used: Math.floor(Math.random() * 100000),
    estimated_cost: Math.random() * 5,
    uptime_seconds: Math.floor((new Date().getTime() - new Date(agent?.metadata.created_at || Date.now()).getTime()) / 1000),
    last_updated: new Date().toISOString(),
  };

  // Mock time series data
  const [metricsHistory, setMetricsHistory] = useState<AgentMetricsTimeSeries[]>([]);

  useEffect(() => {
    if (agent) {
      const newPoint: AgentMetricsTimeSeries = {
        timestamp: new Date().toISOString(),
        tasks_completed: agent.metadata.tasks_completed,
        response_time_ms: agent.metadata.avg_task_duration_ms,
        cpu_percent: Math.random() * 100,
        memory_mb: Math.random() * 512,
        error_count: agent.metadata.tasks_failed,
      };
      setMetricsHistory((prev) => [...prev, newPoint].slice(-20));
    }
  }, [agent]);

  // Mock logs
  const mockLogs: AgentLogEntry[] = [
    { timestamp: new Date().toISOString(), level: 'INFO', message: 'Agent started successfully', context: {} },
    { timestamp: new Date(Date.now() - 60000).toISOString(), level: 'INFO', message: 'Task completed: code-review-task-1', context: { task_id: 'task-1' } },
    { timestamp: new Date(Date.now() - 120000).toISOString(), level: 'WARNING', message: 'High memory usage detected', context: { memory_mb: 450 } },
    { timestamp: new Date(Date.now() - 180000).toISOString(), level: 'INFO', message: 'Processing task: code-generation-task-2', context: { task_id: 'task-2' } },
  ];

  const filteredLogs = mockLogs.filter((log) => {
    if (logLevel !== 'all' && log.level !== logLevel) return false;
    if (logSearch && !log.message.toLowerCase().includes(logSearch.toLowerCase())) return false;
    return true;
  });

  const handlePause = useCallback(async () => {
    try {
      await axonClient.pauseAgent(id!);
      mutate(axonEndpoints.agents.details(id!));
      showSnackbar('Agent paused successfully', 'success');
    } catch (err) {
      showSnackbar('Failed to pause agent', 'error');
    }
  }, [id, showSnackbar]);

  const handleResume = useCallback(async () => {
    try {
      await axonClient.resumeAgent(id!);
      mutate(axonEndpoints.agents.details(id!));
      showSnackbar('Agent resumed successfully', 'success');
    } catch (err) {
      showSnackbar('Failed to resume agent', 'error');
    }
  }, [id, showSnackbar]);

  const handleRestart = useCallback(async () => {
    try {
      await axonClient.restartAgent(id!);
      mutate(axonEndpoints.agents.details(id!));
      showSnackbar('Agent restarted successfully', 'success');
    } catch (err) {
      showSnackbar('Failed to restart agent', 'error');
    }
  }, [id, showSnackbar]);

  const handleDelete = useCallback(async () => {
    try {
      await axonClient.deleteAgent(id!);
      showSnackbar('Agent deleted successfully', 'success');
      navigate('/dashboard/agents');
    } catch (err) {
      showSnackbar('Failed to delete agent', 'error');
    }
  }, [id, navigate, showSnackbar]);

  if (isLoading) {
    return <Typography>Loading...</Typography>;
  }

  if (!agent) {
    return <Typography>Agent not found</Typography>;
  }

  return (
    <Box>
      {/* Header */}
      <Box sx={{ display: 'flex', alignItems: 'center', mb: 5 }}>
        <Box sx={{ flexGrow: 1 }}>
          <Stack direction="row" spacing={2} alignItems="center">
            <IconButton onClick={() => navigate('/dashboard/agents')}>
              <Iconify icon="eva:arrow-back-fill" />
            </IconButton>
            <div>
              <Typography variant="h4">{agent.name}</Typography>
              <Typography variant="body2" color="text.secondary">
                {agent.id}
              </Typography>
            </div>
            <Label variant="soft" color={getAgentStatusColor(agent.status)}>
              {agent.status}
            </Label>
          </Stack>
        </Box>
        <Stack direction="row" spacing={1}>
          {agent.status === 'Paused' ? (
            <Button variant="outlined" startIcon={<Iconify icon="solar:play-bold" />} onClick={handleResume}>
              Resume
            </Button>
          ) : (
            <Button variant="outlined" startIcon={<Iconify icon="solar:pause-bold" />} onClick={handlePause}>
              Pause
            </Button>
          )}
          <Button variant="outlined" startIcon={<Iconify icon="solar:restart-bold" />} onClick={handleRestart}>
            Restart
          </Button>
          <Button variant="outlined" color="error" startIcon={<Iconify icon="solar:trash-bin-trash-bold" />} onClick={handleDelete}>
            Delete
          </Button>
        </Stack>
      </Box>

      <Grid container spacing={3}>
        {/* Agent Info */}
        <Grid size={{ xs: 12, md: 4 }}>
          <Card>
            <CardHeader title="Agent Information" />
            <CardContent>
              <Stack spacing={2}>
                <Box>
                  <Typography variant="caption" color="text.secondary">Type</Typography>
                  <Typography variant="body1">{agent.agent_type}</Typography>
                </Box>
                <Divider />
                <Box>
                  <Typography variant="caption" color="text.secondary">Capabilities</Typography>
                  <Box sx={{ mt: 1, display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
                    {agent.capabilities.map((cap) => (
                      <Chip key={cap} label={cap} size="small" />
                    ))}
                  </Box>
                </Box>
                <Divider />
                <Box>
                  <Typography variant="caption" color="text.secondary">Current Task</Typography>
                  <Typography variant="body2">{agent.current_task || 'None'}</Typography>
                </Box>
                <Divider />
                <Box>
                  <Typography variant="caption" color="text.secondary">Created</Typography>
                  <Typography variant="body2">{new Date(agent.metadata.created_at).toLocaleString()}</Typography>
                </Box>
                <Box>
                  <Typography variant="caption" color="text.secondary">Last Active</Typography>
                  <Typography variant="body2">{new Date(agent.metadata.last_active_at).toLocaleString()}</Typography>
                </Box>
              </Stack>
            </CardContent>
          </Card>
        </Grid>

        {/* Metrics Cards */}
        <Grid size={{ xs: 12, md: 8 }}>
          <Grid container spacing={2}>
            <Grid size={{ xs: 6, sm: 3 }}>
              <Card sx={{ p: 2 }}>
                <Typography variant="subtitle2" color="text.secondary">Tasks Completed</Typography>
                <Typography variant="h4"><AnimateCountUp to={mockMetrics.tasks_completed} /></Typography>
              </Card>
            </Grid>
            <Grid size={{ xs: 6, sm: 3 }}>
              <Card sx={{ p: 2 }}>
                <Typography variant="subtitle2" color="text.secondary">Success Rate</Typography>
                <Typography variant="h4" color={mockMetrics.success_rate >= 80 ? 'success.main' : 'warning.main'}>
                  {mockMetrics.success_rate.toFixed(1)}%
                </Typography>
              </Card>
            </Grid>
            <Grid size={{ xs: 6, sm: 3 }}>
              <Card sx={{ p: 2 }}>
                <Typography variant="subtitle2" color="text.secondary">Avg Response</Typography>
                <Typography variant="h4">{(mockMetrics.avg_response_time_ms / 1000).toFixed(2)}s</Typography>
              </Card>
            </Grid>
            <Grid size={{ xs: 6, sm: 3 }}>
              <Card sx={{ p: 2 }}>
                <Typography variant="subtitle2" color="text.secondary">Uptime</Typography>
                <Typography variant="h4">{formatUptime(mockMetrics.uptime_seconds)}</Typography>
              </Card>
            </Grid>
          </Grid>
        </Grid>

        {/* Performance Charts */}
        <Grid size={{ xs: 12, md: 6 }}>
          <Card>
            <CardHeader title="Response Time Trend" />
            <CardContent>
              <ResponsiveContainer width="100%" height={250}>
                <AreaChart data={metricsHistory}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="timestamp" tickFormatter={(v) => new Date(v).toLocaleTimeString()} fontSize={12} />
                  <YAxis fontSize={12} />
                  <Tooltip labelFormatter={(v) => new Date(v).toLocaleString()} contentStyle={{ fontSize: 12 }} />
                  <Area type="monotone" dataKey="response_time_ms" stroke={COLORS.primary} fill={COLORS.primary} fillOpacity={0.3} name="Response (ms)" />
                </AreaChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>
        </Grid>

        <Grid size={{ xs: 12, md: 6 }}>
          <Card>
            <CardHeader title="Resource Usage" />
            <CardContent>
              <ResponsiveContainer width="100%" height={250}>
                <LineChart data={metricsHistory}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="timestamp" tickFormatter={(v) => new Date(v).toLocaleTimeString()} fontSize={12} />
                  <YAxis fontSize={12} />
                  <Tooltip labelFormatter={(v) => new Date(v).toLocaleString()} contentStyle={{ fontSize: 12 }} />
                  <Legend wrapperStyle={{ fontSize: 12 }} />
                  <Line type="monotone" dataKey="cpu_percent" stroke={COLORS.error} name="CPU %" strokeWidth={2} />
                  <Line type="monotone" dataKey="memory_mb" stroke={COLORS.success} name="Memory (MB)" strokeWidth={2} />
                </LineChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>
        </Grid>

        {/* Logs Viewer */}
        <Grid size={{ xs: 12 }}>
          <Card>
            <CardHeader
              title="Agent Logs"
              action={
                <Stack direction="row" spacing={1} alignItems="center">
                  <FormControl size="small" sx={{ minWidth: 120 }}>
                    <InputLabel>Level</InputLabel>
                    <Select value={logLevel} label="Level" onChange={(e) => setLogLevel(e.target.value)}>
                      <MenuItem value="all">All Levels</MenuItem>
                      <MenuItem value="DEBUG">Debug</MenuItem>
                      <MenuItem value="INFO">Info</MenuItem>
                      <MenuItem value="WARNING">Warning</MenuItem>
                      <MenuItem value="ERROR">Error</MenuItem>
                    </Select>
                  </FormControl>
                  <Button
                    size="small"
                    variant="outlined"
                    startIcon={<Iconify icon={autoRefresh ? 'eva:pause-circle-fill' : 'eva:play-circle-fill'} />}
                    onClick={() => setAutoRefresh(!autoRefresh)}
                  >
                    {autoRefresh ? 'Pause' : 'Resume'}
                  </Button>
                  <Button size="small" variant="outlined" startIcon={<Iconify icon="eva:download-fill" />}>
                    Download
                  </Button>
                </Stack>
              }
            />
            <Box sx={{ p: 2 }}>
              <TextField
                fullWidth
                size="small"
                placeholder="Search logs..."
                value={logSearch}
                onChange={(e) => setLogSearch(e.target.value)}
                InputProps={{
                  startAdornment: <Iconify icon="eva:search-fill" sx={{ mr: 1, color: 'text.disabled' }} />,
                }}
              />
            </Box>
            <Box sx={{ p: 2, maxHeight: 400, overflow: 'auto', bgcolor: 'background.neutral' }}>
              {filteredLogs.map((log, index) => (
                <Paper
                  key={index}
                  sx={{
                    p: 1.5,
                    mb: 1,
                    fontFamily: 'monospace',
                    fontSize: 12,
                    borderLeft: 4,
                    borderColor: log.level === 'ERROR' ? 'error.main' : log.level === 'WARNING' ? 'warning.main' : 'info.main',
                  }}
                >
                  <Stack direction="row" spacing={2}>
                    <Typography component="span" sx={{ color: 'text.disabled', minWidth: 160 }}>
                      {new Date(log.timestamp).toLocaleString()}
                    </Typography>
                    <Chip
                      label={log.level}
                      size="small"
                      color={log.level === 'ERROR' ? 'error' : log.level === 'WARNING' ? 'warning' : 'info'}
                      sx={{ minWidth: 80 }}
                    />
                    <Typography component="span">{log.message}</Typography>
                  </Stack>
                </Paper>
              ))}
            </Box>
          </Card>
        </Grid>

        {/* Configuration */}
        <Grid size={{ xs: 12, md: 6 }}>
          <Card>
            <CardHeader title="Configuration" />
            <CardContent>
              <Stack spacing={2}>
                <Box>
                  <Typography variant="caption" color="text.secondary">Max Concurrent Tasks</Typography>
                  <Typography variant="body1">{agent.metadata.max_concurrent_tasks}</Typography>
                </Box>
                <Divider />
                <Box>
                  <Typography variant="caption" color="text.secondary">Task Timeout</Typography>
                  <Typography variant="body1">{agent.metadata.task_timeout_seconds}s</Typography>
                </Box>
                <Divider />
                <Box>
                  <Typography variant="caption" color="text.secondary">Total Execution Time</Typography>
                  <Typography variant="body1">{(agent.metadata.total_execution_time_ms / 1000).toFixed(2)}s</Typography>
                </Box>
              </Stack>
            </CardContent>
          </Card>
        </Grid>

        {/* Resource Metrics */}
        <Grid size={{ xs: 12, md: 6 }}>
          <Card>
            <CardHeader title="Resource Metrics" />
            <CardContent>
              <Stack spacing={2}>
                <Box>
                  <Typography variant="caption" color="text.secondary">CPU Usage</Typography>
                  <Typography variant="h6">{mockMetrics.cpu_usage_percent.toFixed(1)}%</Typography>
                </Box>
                <Divider />
                <Box>
                  <Typography variant="caption" color="text.secondary">Memory Usage</Typography>
                  <Typography variant="h6">{mockMetrics.memory_usage_mb.toFixed(0)} MB</Typography>
                </Box>
                <Divider />
                <Box>
                  <Typography variant="caption" color="text.secondary">Tokens Used</Typography>
                  <Typography variant="h6">{mockMetrics.tokens_used.toLocaleString()}</Typography>
                </Box>
                <Divider />
                <Box>
                  <Typography variant="caption" color="text.secondary">Estimated Cost</Typography>
                  <Typography variant="h6">${mockMetrics.estimated_cost.toFixed(4)}</Typography>
                </Box>
              </Stack>
            </CardContent>
          </Card>
        </Grid>
      </Grid>
    </Box>
  );
}

// ----------------------------------------------------------------------

function formatUptime(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h`;
  const days = Math.floor(hours / 24);
  return `${days}d`;
}
