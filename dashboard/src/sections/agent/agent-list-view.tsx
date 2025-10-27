import type { AgentInfo } from 'src/types/axon';

import useSWR, { mutate } from 'swr';
import { useState, useCallback } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Table from '@mui/material/Table';
import Button from '@mui/material/Button';
import MenuItem from '@mui/material/MenuItem';
import TableBody from '@mui/material/TableBody';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import TableContainer from '@mui/material/TableContainer';
import TablePagination from '@mui/material/TablePagination';
import TextField from '@mui/material/TextField';
import Stack from '@mui/material/Stack';
import Chip from '@mui/material/Chip';
import Select from '@mui/material/Select';
import FormControl from '@mui/material/FormControl';
import InputLabel from '@mui/material/InputLabel';
import Checkbox from '@mui/material/Checkbox';
import Tooltip from '@mui/material/Tooltip';

import { getAgentStatusColor } from 'src/utils/status-colors';

import { axonClient, axonFetcher, axonEndpoints } from 'src/lib/axon-client';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { Scrollbar } from 'src/components/scrollbar';
import { useSnackbar } from 'src/components/snackbar';
import { usePopover, CustomPopover } from 'src/components/custom-popover';
import {
  useTable,
  emptyRows,
  TableNoData,
  TableEmptyRows,
  TableHeadCustom,
  TableSelectedAction,
} from 'src/components/table';
import TableRow from '@mui/material/TableRow';
import TableCell from '@mui/material/TableCell';

// ----------------------------------------------------------------------

const TABLE_HEAD = [
  { id: 'name', label: 'Name', width: 200 },
  { id: 'type', label: 'Type', width: 120 },
  { id: 'status', label: 'Status', width: 100 },
  { id: 'capabilities', label: 'Capabilities', width: 180 },
  { id: 'health', label: 'Health', width: 100 },
  { id: 'tasks', label: 'Tasks', width: 100 },
  { id: 'success_rate', label: 'Success Rate', width: 120 },
  { id: 'avg_duration', label: 'Avg Duration', width: 120 },
  { id: 'actions', label: 'Actions', width: 80 },
];

// ----------------------------------------------------------------------

export function AgentListView() {
  const table = useTable();
  const popover = usePopover();
  const { showSnackbar } = useSnackbar();

  const [selectedAgent, setSelectedAgent] = useState<AgentInfo | null>(null);
  const [filterStatus, setFilterStatus] = useState<string>('all');
  const [filterType, setFilterType] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [sortBy, setSortBy] = useState<'name' | 'tasks' | 'success_rate'>('name');
  const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('asc');

  // Fetch agents
  const { data: agents = [], isLoading } = useSWR<AgentInfo[]>(
    axonEndpoints.agents.list,
    axonFetcher,
    { refreshInterval: 5000 }
  );

  // Filter and sort agents
  const filteredAgents = agents
    .filter((agent) => {
      if (filterStatus !== 'all' && agent.status !== filterStatus) return false;
      if (filterType !== 'all' && agent.agent_type !== filterType) return false;
      if (searchQuery && !agent.name.toLowerCase().includes(searchQuery.toLowerCase())) return false;
      return true;
    })
    .sort((a, b) => {
      let comparison = 0;
      if (sortBy === 'name') {
        comparison = a.name.localeCompare(b.name);
      } else if (sortBy === 'tasks') {
        comparison = a.metadata.tasks_completed - b.metadata.tasks_completed;
      } else if (sortBy === 'success_rate') {
        const aRate = a.metadata.tasks_completed > 0
          ? (a.metadata.tasks_completed / (a.metadata.tasks_completed + a.metadata.tasks_failed)) * 100
          : 0;
        const bRate = b.metadata.tasks_completed > 0
          ? (b.metadata.tasks_completed / (b.metadata.tasks_completed + b.metadata.tasks_failed)) * 100
          : 0;
        comparison = aRate - bRate;
      }
      return sortOrder === 'asc' ? comparison : -comparison;
    });

  const notFound = !isLoading && !filteredAgents.length;

  const handleDeleteAgent = useCallback(async (id: string) => {
    try {
      await axonClient.deleteAgent(id);
      mutate(axonEndpoints.agents.list);
      popover.onClose();
      showSnackbar('Agent deleted successfully', 'success');
    } catch (err) {
      console.error('Failed to delete agent:', err);
      showSnackbar('Failed to delete agent. Please try again.', 'error');
    }
  }, [popover, showSnackbar]);

  const handlePauseAgent = useCallback(async (id: string) => {
    try {
      await axonClient.pauseAgent(id);
      mutate(axonEndpoints.agents.list);
      popover.onClose();
      showSnackbar('Agent paused successfully', 'success');
    } catch (err) {
      console.error('Failed to pause agent:', err);
      showSnackbar('Failed to pause agent. Please try again.', 'error');
    }
  }, [popover, showSnackbar]);

  const handleResumeAgent = useCallback(async (id: string) => {
    try {
      await axonClient.resumeAgent(id);
      mutate(axonEndpoints.agents.list);
      popover.onClose();
      showSnackbar('Agent resumed successfully', 'success');
    } catch (err) {
      console.error('Failed to resume agent:', err);
      showSnackbar('Failed to resume agent. Please try again.', 'error');
    }
  }, [popover, showSnackbar]);

  const handleRestartAgent = useCallback(async (id: string) => {
    try {
      await axonClient.restartAgent(id);
      mutate(axonEndpoints.agents.list);
      popover.onClose();
      showSnackbar('Agent restarted successfully', 'success');
    } catch (err) {
      console.error('Failed to restart agent:', err);
      showSnackbar('Failed to restart agent. Please try again.', 'error');
    }
  }, [popover, showSnackbar]);

  const handleBulkAction = useCallback(async (action: 'pause' | 'resume' | 'restart') => {
    try {
      await axonClient.bulkAgentAction(action, table.selected);
      mutate(axonEndpoints.agents.list);
      table.onSelectAllRows(false, []);
      showSnackbar(`Agents ${action}ed successfully`, 'success');
    } catch (err) {
      console.error(`Failed to ${action} agents:`, err);
      showSnackbar(`Failed to ${action} agents. Please try again.`, 'error');
    }
  }, [table, showSnackbar]);

  const renderStatus = (status: string) => (
    <Label variant="soft" color={getAgentStatusColor(status)}>
      {status}
    </Label>
  );

  const calculateHealth = (agent: AgentInfo) => {
    const totalTasks = agent.metadata.tasks_completed + agent.metadata.tasks_failed;
    if (totalTasks === 0) return 100;
    const successRate = (agent.metadata.tasks_completed / totalTasks) * 100;
    if (successRate >= 95) return 'Excellent';
    if (successRate >= 80) return 'Good';
    if (successRate >= 60) return 'Fair';
    return 'Poor';
  };

  const calculateSuccessRate = (agent: AgentInfo) => {
    const totalTasks = agent.metadata.tasks_completed + agent.metadata.tasks_failed;
    if (totalTasks === 0) return 0;
    return (agent.metadata.tasks_completed / totalTasks) * 100;
  };

  return (
    <>
      <Box sx={{ display: 'flex', alignItems: 'center', mb: 5 }}>
        <Typography variant="h4" sx={{ flexGrow: 1 }}>
          Agents
        </Typography>
        <Button
          variant="contained"
          startIcon={<Iconify icon="mingcute:add-line" />}
          href="/dashboard/agents/create"
        >
          Create Agent
        </Button>
      </Box>

      <Card>
        <Box sx={{ p: 3 }}>
          <Stack spacing={2}>
            <Stack direction={{ xs: 'column', md: 'row' }} spacing={2}>
              <TextField
                fullWidth
                placeholder="Search agents..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                InputProps={{
                  startAdornment: <Iconify icon="eva:search-fill" sx={{ color: 'text.disabled', mr: 1 }} />,
                }}
              />
              <FormControl sx={{ minWidth: 200 }}>
                <InputLabel>Status</InputLabel>
                <Select
                  value={filterStatus}
                  label="Status"
                  onChange={(e) => setFilterStatus(e.target.value)}
                >
                  <MenuItem value="all">All Status</MenuItem>
                  <MenuItem value="Idle">Idle</MenuItem>
                  <MenuItem value="Working">Working</MenuItem>
                  <MenuItem value="Paused">Paused</MenuItem>
                  <MenuItem value="Failed">Failed</MenuItem>
                </Select>
              </FormControl>
              <FormControl sx={{ minWidth: 200 }}>
                <InputLabel>Type</InputLabel>
                <Select
                  value={filterType}
                  label="Type"
                  onChange={(e) => setFilterType(e.target.value)}
                >
                  <MenuItem value="all">All Types</MenuItem>
                  <MenuItem value="Orchestrator">Orchestrator</MenuItem>
                  <MenuItem value="Developer">Developer</MenuItem>
                  <MenuItem value="Reviewer">Reviewer</MenuItem>
                  <MenuItem value="Tester">Tester</MenuItem>
                  <MenuItem value="Documenter">Documenter</MenuItem>
                </Select>
              </FormControl>
            </Stack>

            <Stack direction="row" spacing={2} alignItems="center">
              <FormControl sx={{ minWidth: 150 }}>
                <InputLabel>Sort By</InputLabel>
                <Select
                  value={sortBy}
                  label="Sort By"
                  onChange={(e) => setSortBy(e.target.value as any)}
                >
                  <MenuItem value="name">Name</MenuItem>
                  <MenuItem value="tasks">Tasks</MenuItem>
                  <MenuItem value="success_rate">Success Rate</MenuItem>
                </Select>
              </FormControl>
              <IconButton
                onClick={() => setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc')}
              >
                <Iconify icon={sortOrder === 'asc' ? 'eva:arrow-up-fill' : 'eva:arrow-down-fill'} />
              </IconButton>
              <Typography variant="body2" color="text.secondary">
                {filteredAgents.length} agents found
              </Typography>
            </Stack>
          </Stack>
        </Box>

        <Scrollbar>
          <TableContainer sx={{ minWidth: 960 }}>
            <Table>
              <TableHeadCustom
                order={sortOrder}
                orderBy={sortBy}
                headLabel={TABLE_HEAD}
                rowCount={filteredAgents.length}
                numSelected={table.selected.length}
                onSelectAllRows={(checked) =>
                  table.onSelectAllRows(
                    checked,
                    filteredAgents.map((row) => row.id)
                  )
                }
              />

              <TableBody>
                {filteredAgents
                  .slice(
                    table.page * table.rowsPerPage,
                    table.page * table.rowsPerPage + table.rowsPerPage
                  )
                  .map((agent: AgentInfo) => (
                    <AgentTableRow
                      key={agent.id}
                      row={agent}
                      selected={table.selected.includes(agent.id)}
                      onSelectRow={() => table.onSelectRow(agent.id)}
                      onOpenPopover={(event) => {
                        popover.onOpen(event);
                        setSelectedAgent(agent);
                      }}
                      renderStatus={renderStatus}
                      health={calculateHealth(agent)}
                      successRate={calculateSuccessRate(agent)}
                    />
                  ))}

                <TableEmptyRows
                  height={table.dense ? 56 : 76}
                  emptyRows={emptyRows(table.page, table.rowsPerPage, filteredAgents.length)}
                />

                <TableNoData notFound={notFound} />
              </TableBody>
            </Table>
          </TableContainer>
        </Scrollbar>

        <TablePagination
          component="div"
          page={table.page}
          count={filteredAgents.length}
          rowsPerPage={table.rowsPerPage}
          onPageChange={table.onChangePage}
          rowsPerPageOptions={[5, 10, 25]}
          onRowsPerPageChange={table.onChangeRowsPerPage}
        />
      </Card>

      <TableSelectedAction
        dense={table.dense}
        numSelected={table.selected.length}
        rowCount={filteredAgents.length}
        onSelectAllRows={(checked) =>
          table.onSelectAllRows(
            checked,
            filteredAgents.map((row) => row.id)
          )
        }
        action={
          <Stack direction="row" spacing={1}>
            <Tooltip title="Pause selected">
              <IconButton color="primary" onClick={() => handleBulkAction('pause')}>
                <Iconify icon="solar:pause-bold" />
              </IconButton>
            </Tooltip>
            <Tooltip title="Resume selected">
              <IconButton color="primary" onClick={() => handleBulkAction('resume')}>
                <Iconify icon="solar:play-bold" />
              </IconButton>
            </Tooltip>
            <Tooltip title="Restart selected">
              <IconButton color="primary" onClick={() => handleBulkAction('restart')}>
                <Iconify icon="solar:restart-bold" />
              </IconButton>
            </Tooltip>
          </Stack>
        }
      />

      <CustomPopover
        open={popover.open}
        anchorEl={popover.anchorEl}
        onClose={popover.onClose}
        slotProps={{ arrow: { placement: 'right-top' } }}
      >
        <MenuItem
          component="a"
          href={`/dashboard/agents/${selectedAgent?.id}`}
        >
          <Iconify icon="solar:eye-bold" />
          View Details
        </MenuItem>

        <MenuItem
          onClick={() => {
            if (selectedAgent?.status === 'Paused') {
              handleResumeAgent(selectedAgent.id);
            } else {
              handlePauseAgent(selectedAgent?.id || '');
            }
          }}
        >
          <Iconify
            icon={selectedAgent?.status === 'Paused' ? 'solar:play-bold' : 'solar:pause-bold'}
          />
          {selectedAgent?.status === 'Paused' ? 'Resume' : 'Pause'}
        </MenuItem>

        <MenuItem onClick={() => handleRestartAgent(selectedAgent?.id || '')}>
          <Iconify icon="solar:restart-bold" />
          Restart
        </MenuItem>

        <MenuItem
          onClick={() => handleDeleteAgent(selectedAgent?.id || '')}
          sx={{ color: 'error.main' }}
        >
          <Iconify icon="solar:trash-bin-trash-bold" />
          Delete
        </MenuItem>
      </CustomPopover>
    </>
  );
}

// ----------------------------------------------------------------------

type AgentTableRowProps = {
  row: AgentInfo;
  selected: boolean;
  onSelectRow: () => void;
  onOpenPopover: (event: React.MouseEvent<HTMLElement>) => void;
  renderStatus: (status: string) => React.ReactNode;
  health: string;
  successRate: number;
};

function AgentTableRow({ row, selected, onSelectRow, onOpenPopover, renderStatus, health, successRate }: AgentTableRowProps) {
  const getHealthColor = (healthStatus: string) => {
    if (healthStatus === 'Excellent') return 'success';
    if (healthStatus === 'Good') return 'info';
    if (healthStatus === 'Fair') return 'warning';
    return 'error';
  };

  return (
    <TableRow hover selected={selected}>
      <TableCell padding="checkbox">
        <Checkbox checked={selected} onClick={onSelectRow} />
      </TableCell>

      <TableCell>
        <Box>
          <Typography variant="subtitle2">{row.name}</Typography>
          <Typography variant="caption" sx={{ color: 'text.disabled' }}>
            {row.id.substring(0, 8)}
          </Typography>
        </Box>
      </TableCell>

      <TableCell>{row.agent_type}</TableCell>

      <TableCell>{renderStatus(row.status)}</TableCell>

      <TableCell>
        <Box sx={{ display: 'flex', gap: 0.5, flexWrap: 'wrap' }}>
          {row.capabilities.slice(0, 2).map((cap) => (
            <Chip key={cap} label={cap} size="small" variant="outlined" />
          ))}
          {row.capabilities.length > 2 && (
            <Chip label={`+${row.capabilities.length - 2}`} size="small" variant="outlined" />
          )}
        </Box>
      </TableCell>

      <TableCell>
        <Label variant="soft" color={getHealthColor(health) as any}>
          {health}
        </Label>
      </TableCell>

      <TableCell>
        <Typography variant="body2">
          {row.metadata.tasks_completed}
          {row.metadata.tasks_failed > 0 && (
            <Typography component="span" variant="caption" sx={{ color: 'error.main', ml: 0.5 }}>
              ({row.metadata.tasks_failed} failed)
            </Typography>
          )}
        </Typography>
      </TableCell>

      <TableCell>
        <Typography variant="body2" sx={{ color: successRate >= 80 ? 'success.main' : successRate >= 60 ? 'warning.main' : 'error.main' }}>
          {successRate.toFixed(1)}%
        </Typography>
      </TableCell>

      <TableCell>
        <Typography variant="body2">
          {row.metadata.avg_task_duration_ms > 0
            ? `${(row.metadata.avg_task_duration_ms / 1000).toFixed(2)}s`
            : '-'}
        </Typography>
      </TableCell>

      <TableCell align="right">
        <IconButton onClick={onOpenPopover}>
          <Iconify icon="eva:more-vertical-fill" />
        </IconButton>
      </TableCell>
    </TableRow>
  );
}
