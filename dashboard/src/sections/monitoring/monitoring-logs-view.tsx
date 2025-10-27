import { useState } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Chip from '@mui/material/Chip';
import Stack from '@mui/material/Stack';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import InputLabel from '@mui/material/InputLabel';
import FormControl from '@mui/material/FormControl';

import { Iconify } from 'src/components/iconify';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

interface LogEntry {
  id: string;
  timestamp: string;
  level: 'DEBUG' | 'INFO' | 'WARNING' | 'ERROR' | 'CRITICAL';
  source: string;
  message: string;
}

const MOCK_LOGS: LogEntry[] = [
  {
    id: '1',
    timestamp: new Date().toISOString(),
    level: 'INFO',
    source: 'orchestrator-001',
    message: 'Workflow execution started for workflow_abc123',
  },
  {
    id: '2',
    timestamp: new Date(Date.now() - 60000).toISOString(),
    level: 'WARNING',
    source: 'developer-001',
    message: 'Task execution taking longer than expected',
  },
  {
    id: '3',
    timestamp: new Date(Date.now() - 120000).toISOString(),
    level: 'ERROR',
    source: 'tester-001',
    message: 'Test suite failed with 3 failures',
  },
];

export function MonitoringLogsView() {
  const [filterLevel, setFilterLevel] = useState('all');
  const [filterSource, setFilterSource] = useState('all');
  const [searchQuery, setSearchQuery] = useState('');

  const filteredLogs = MOCK_LOGS.filter((log) => {
    if (filterLevel !== 'all' && log.level !== filterLevel) return false;
    if (filterSource !== 'all' && log.source !== filterSource) return false;
    if (searchQuery && !log.message.toLowerCase().includes(searchQuery.toLowerCase()))
      return false;
    return true;
  });

  const getLevelColor = (level: string) => {
    if (level === 'CRITICAL' || level === 'ERROR') return 'error';
    if (level === 'WARNING') return 'warning';
    if (level === 'INFO') return 'info';
    return 'default';
  };

  return (
    <>
      <CustomBreadcrumbs
        heading="Event Logs"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Monitoring' },
          { name: 'Logs' },
        ]}
        sx={{ mb: 3 }}
      />

      <Card>
        <Box sx={{ p: 3 }}>
          <Stack spacing={2}>
            <Typography variant="h6" sx={{ mb: 1 }}>
              System Event Logs
            </Typography>
            <Stack direction={{ xs: 'column', md: 'row' }} spacing={2}>
              <TextField
                fullWidth
                placeholder="Search logs..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                slotProps={{
                  input: {
                    startAdornment: (
                      <Iconify icon="eva:search-fill" sx={{ color: 'text.disabled', mr: 1 }} />
                    ),
                  },
                }}
              />
              <FormControl sx={{ minWidth: 150 }}>
                <InputLabel>Level</InputLabel>
                <Select
                  value={filterLevel}
                  label="Level"
                  onChange={(e) => setFilterLevel(e.target.value)}
                >
                  <MenuItem value="all">All Levels</MenuItem>
                  <MenuItem value="DEBUG">Debug</MenuItem>
                  <MenuItem value="INFO">Info</MenuItem>
                  <MenuItem value="WARNING">Warning</MenuItem>
                  <MenuItem value="ERROR">Error</MenuItem>
                  <MenuItem value="CRITICAL">Critical</MenuItem>
                </Select>
              </FormControl>
              <FormControl sx={{ minWidth: 150 }}>
                <InputLabel>Source</InputLabel>
                <Select
                  value={filterSource}
                  label="Source"
                  onChange={(e) => setFilterSource(e.target.value)}
                >
                  <MenuItem value="all">All Sources</MenuItem>
                  <MenuItem value="orchestrator-001">Orchestrator</MenuItem>
                  <MenuItem value="developer-001">Developer</MenuItem>
                  <MenuItem value="tester-001">Tester</MenuItem>
                </Select>
              </FormControl>
            </Stack>
          </Stack>
        </Box>

        <Stack spacing={0} divider={<Box sx={{ borderBottom: 1, borderColor: 'divider' }} />}>
          {filteredLogs.map((log) => (
            <Box key={log.id} sx={{ p: 2.5 }}>
              <Stack spacing={1}>
                <Stack direction="row" spacing={2} alignItems="center" justifyContent="space-between">
                  <Stack direction="row" spacing={1} alignItems="center">
                    <Chip
                      label={log.level}
                      size="small"
                      color={getLevelColor(log.level) as any}
                      sx={{ minWidth: 80 }}
                    />
                    <Typography variant="caption" color="text.secondary">
                      {new Date(log.timestamp).toLocaleString()}
                    </Typography>
                  </Stack>
                  <Chip label={log.source} size="small" variant="outlined" />
                </Stack>
                <Typography variant="body2">{log.message}</Typography>
              </Stack>
            </Box>
          ))}
        </Stack>

        {filteredLogs.length === 0 && (
          <Box sx={{ p: 5, textAlign: 'center' }}>
            <Iconify icon="mdi:file-document-outline" width={64} sx={{ color: 'text.disabled', mb: 2 }} />
            <Typography variant="h6" color="text.secondary">
              No logs found
            </Typography>
          </Box>
        )}
      </Card>
    </>
  );
}
