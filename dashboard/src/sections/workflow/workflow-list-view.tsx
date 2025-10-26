import type { WorkflowInfo } from 'src/types/axon';

import useSWR, { mutate } from 'swr';
import { useState, useCallback } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Table from '@mui/material/Table';
import Button from '@mui/material/Button';
import MenuItem from '@mui/material/MenuItem';
import TableRow from '@mui/material/TableRow';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import TableContainer from '@mui/material/TableContainer';
import LinearProgress from '@mui/material/LinearProgress';
import TablePagination from '@mui/material/TablePagination';

import { getWorkflowStatusColor } from 'src/utils/status-colors';

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
  { id: 'status', label: 'Status', width: 120 },
  { id: 'progress', label: 'Progress', width: 200 },
  { id: 'tasks', label: 'Tasks', width: 120 },
  { id: 'created', label: 'Created', width: 180 },
  { id: 'duration', label: 'Duration', width: 120 },
  { id: 'actions', label: 'Actions', width: 80 },
];

// ----------------------------------------------------------------------

export function WorkflowListView() {
  const table = useTable();
  const popover = usePopover();

  const [selectedWorkflow, setSelectedWorkflow] = useState<WorkflowInfo | null>(null);

  // Fetch workflows
  const { data: workflows = [], isLoading } = useSWR<WorkflowInfo[]>(
    axonEndpoints.workflows.list,
    axonFetcher,
    { refreshInterval: 3000 } // Refresh every 3 seconds for real-time updates
  );

  const notFound = !isLoading && !workflows.length;

  const handleCancelWorkflow = useCallback(
    async (id: string) => {
      try {
        await axonClient.cancelWorkflow(id);
        mutate(axonEndpoints.workflows.list);
        popover.onClose();
      } catch (err) {
        console.error('Failed to cancel workflow:', err);
      }
    },
    [popover]
  );

  const renderStatus = (status: string) => (
    <Label variant="soft" color={getWorkflowStatusColor(status)}>
      {status}
    </Label>
  );

  const calculateDuration = (workflow: WorkflowInfo) => {
    if (!workflow.started_at) return '-';

    const start = new Date(workflow.started_at).getTime();
    const end = workflow.completed_at ? new Date(workflow.completed_at).getTime() : Date.now();
    const durationMs = end - start;
    const seconds = Math.floor(durationMs / 1000);

    if (seconds < 60) return `${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m ${seconds % 60}s`;
    const hours = Math.floor(minutes / 60);
    return `${hours}h ${minutes % 60}m`;
  };

  return (
    <>
      <Box sx={{ display: 'flex', alignItems: 'center', mb: 5 }}>
        <Typography variant="h4" sx={{ flexGrow: 1 }}>
          Workflows
        </Typography>
        <Button
          variant="contained"
          startIcon={<Iconify icon="mingcute:add-line" />}
          href="/dashboard/workflows/create"
        >
          Run Workflow
        </Button>
      </Box>

      <Card>
        <Scrollbar>
          <TableContainer sx={{ minWidth: 960 }}>
            <Table>
              <TableHeadCustom headLabel={TABLE_HEAD} />

              <TableBody>
                {workflows
                  .slice(
                    table.page * table.rowsPerPage,
                    table.page * table.rowsPerPage + table.rowsPerPage
                  )
                  .map((workflow: WorkflowInfo) => (
                    <WorkflowTableRow
                      key={workflow.id}
                      row={workflow}
                      onOpenPopover={(event) => {
                        popover.onOpen(event);
                        setSelectedWorkflow(workflow);
                      }}
                      renderStatus={renderStatus}
                      calculateDuration={calculateDuration}
                    />
                  ))}

                <TableEmptyRows
                  height={table.dense ? 56 : 76}
                  emptyRows={emptyRows(table.page, table.rowsPerPage, workflows.length)}
                />

                <TableNoData notFound={notFound} />
              </TableBody>
            </Table>
          </TableContainer>
        </Scrollbar>

        <TablePagination
          component="div"
          page={table.page}
          count={workflows.length}
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
        <MenuItem onClick={() => window.open(`/dashboard/workflows/${selectedWorkflow?.id}`, '_blank')}>
          <Iconify icon="solar:eye-bold" />
          View Details
        </MenuItem>

        {selectedWorkflow?.status === 'Running' && (
          <MenuItem
            onClick={() => handleCancelWorkflow(selectedWorkflow.id)}
            sx={{ color: 'error.main' }}
          >
            <Iconify icon="solar:close-circle-bold" />
            Cancel
          </MenuItem>
        )}
      </CustomPopover>
    </>
  );
}

// ----------------------------------------------------------------------

type WorkflowTableRowProps = {
  row: WorkflowInfo;
  onOpenPopover: (event: React.MouseEvent<HTMLElement>) => void;
  renderStatus: (status: string) => React.ReactNode;
  calculateDuration: (workflow: WorkflowInfo) => string;
};

function WorkflowTableRow({
  row,
  onOpenPopover,
  renderStatus,
  calculateDuration,
}: WorkflowTableRowProps) {
  const progress = row.total_tasks > 0 ? (row.completed_tasks / row.total_tasks) * 100 : 0;

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

      <TableCell>{renderStatus(row.status)}</TableCell>

      <TableCell>
        <Box sx={{ width: '100%' }}>
          <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 0.5 }}>
            <Typography variant="caption" sx={{ color: 'text.secondary' }}>
              {row.completed_tasks} / {row.total_tasks}
            </Typography>
            <Typography variant="caption" sx={{ color: 'text.secondary' }}>
              {progress.toFixed(0)}%
            </Typography>
          </Box>
          <LinearProgress variant="determinate" value={progress} />
        </Box>
      </TableCell>

      <TableCell>
        <Box>
          <Typography variant="body2">
            {row.completed_tasks} completed
          </Typography>
          {row.failed_tasks > 0 && (
            <Typography variant="caption" sx={{ color: 'error.main' }}>
              {row.failed_tasks} failed
            </Typography>
          )}
        </Box>
      </TableCell>

      <TableCell>
        <Typography variant="body2">
          {new Date(row.created_at).toLocaleString()}
        </Typography>
      </TableCell>

      <TableCell>
        <Typography variant="body2">{calculateDuration(row)}</Typography>
      </TableCell>

      <TableCell align="right">
        <IconButton onClick={onOpenPopover}>
          <Iconify icon="eva:more-vertical-fill" />
        </IconButton>
      </TableCell>
    </TableRow>
  );
}
