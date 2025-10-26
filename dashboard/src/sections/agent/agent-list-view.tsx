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

import { axonClient, axonFetcher, axonEndpoints } from 'src/lib/axon-client';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { Scrollbar } from 'src/components/scrollbar';
import { usePopover, CustomPopover } from 'src/components/custom-popover';
import {
  useTable,
  emptyRows,
  TableNoData,
  TableEmptyRows,
  TableHeadCustom,
} from 'src/components/table';

// ----------------------------------------------------------------------

const TABLE_HEAD = [
  { id: 'name', label: 'Name' },
  { id: 'type', label: 'Type', width: 140 },
  { id: 'status', label: 'Status', width: 120 },
  { id: 'capabilities', label: 'Capabilities', width: 200 },
  { id: 'tasks', label: 'Tasks', width: 100 },
  { id: 'avg_duration', label: 'Avg Duration', width: 120 },
  { id: 'actions', label: 'Actions', width: 80 },
];

// ----------------------------------------------------------------------

export function AgentListView() {
  const table = useTable();
  const popover = usePopover();

  const [selectedAgent, setSelectedAgent] = useState<AgentInfo | null>(null);

  // Fetch agents
  const { data: agents = [], isLoading } = useSWR<AgentInfo[]>(
    axonEndpoints.agents.list,
    axonFetcher,
    { refreshInterval: 5000 } // Refresh every 5 seconds
  );

  const notFound = !isLoading && !agents.length;

  const handleDeleteAgent = useCallback(async (id: string) => {
    try {
      await axonClient.deleteAgent(id);
      mutate(axonEndpoints.agents.list);
      popover.onClose();
    } catch (err) {
      console.error('Failed to delete agent:', err);
    }
  }, [popover]);

  const handlePauseAgent = useCallback(async (id: string) => {
    try {
      await axonClient.pauseAgent(id);
      mutate(axonEndpoints.agents.list);
      popover.onClose();
    } catch (err) {
      console.error('Failed to pause agent:', err);
    }
  }, [popover]);

  const handleResumeAgent = useCallback(async (id: string) => {
    try {
      await axonClient.resumeAgent(id);
      mutate(axonEndpoints.agents.list);
      popover.onClose();
    } catch (err) {
      console.error('Failed to resume agent:', err);
    }
  }, [popover]);

  const handleRestartAgent = useCallback(async (id: string) => {
    try {
      await axonClient.restartAgent(id);
      mutate(axonEndpoints.agents.list);
      popover.onClose();
    } catch (err) {
      console.error('Failed to restart agent:', err);
    }
  }, [popover]);

  const renderStatus = (status: string) => {
    const statusColors = {
      Idle: 'success',
      Working: 'info',
      Paused: 'warning',
      Failed: 'error',
      ShuttingDown: 'default',
    } as const;

    return (
      <Label variant="soft" color={statusColors[status as keyof typeof statusColors] || 'default'}>
        {status}
      </Label>
    );
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
        <Scrollbar>
          <TableContainer sx={{ minWidth: 960 }}>
            <Table>
              <TableHeadCustom headLabel={TABLE_HEAD} />

              <TableBody>
                {agents
                  .slice(
                    table.page * table.rowsPerPage,
                    table.page * table.rowsPerPage + table.rowsPerPage
                  )
                  .map((agent: AgentInfo) => (
                    <AgentTableRow
                      key={agent.id}
                      row={agent}
                      onOpenPopover={(event) => {
                        popover.onOpen(event);
                        setSelectedAgent(agent);
                      }}
                      renderStatus={renderStatus}
                    />
                  ))}

                <TableEmptyRows
                  height={table.dense ? 56 : 76}
                  emptyRows={emptyRows(table.page, table.rowsPerPage, agents.length)}
                />

                <TableNoData notFound={notFound} />
              </TableBody>
            </Table>
          </TableContainer>
        </Scrollbar>

        <TablePagination
          component="div"
          page={table.page}
          count={agents.length}
          rowsPerPage={table.rowsPerPage}
          onPageChange={table.onChangePage}
          rowsPerPageOptions={[5, 10, 25]}
          onRowsPerPageChange={table.onChangeRowsPerPage}
        />
      </Card>

      <CustomPopover
        open={popover.open}
        anchorEl={popover.anchorEl}
        onClose={popover.onClose}
        slotProps={{ arrow: { placement: 'right-top' } }}
      >
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

import Chip from '@mui/material/Chip';
import TableRow from '@mui/material/TableRow';
import TableCell from '@mui/material/TableCell';

type AgentTableRowProps = {
  row: AgentInfo;
  onOpenPopover: (event: React.MouseEvent<HTMLElement>) => void;
  renderStatus: (status: string) => React.ReactNode;
};

function AgentTableRow({ row, onOpenPopover, renderStatus }: AgentTableRowProps) {
  return (
    <TableRow hover>
      <TableCell>
        <Box>
          <Typography variant="subtitle2">{row.name}</Typography>
          <Typography variant="caption" sx={{ color: 'text.disabled' }}>
            {row.id}
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
