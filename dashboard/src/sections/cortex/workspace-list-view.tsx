import type { Workspace } from 'src/types/cortex';

import useSWR, { mutate } from 'swr';
import { useNavigate } from 'react-router';
import { useState, useCallback } from 'react';

import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Table from '@mui/material/Table';
import Button from '@mui/material/Button';
import TableBody from '@mui/material/TableBody';
import { useTheme } from '@mui/material/styles';
import TableContainer from '@mui/material/TableContainer';
import TablePagination from '@mui/material/TablePagination';

import { cortexClient, cortexFetcher, cortexEndpoints } from 'src/lib/cortex-client';

import { Iconify } from 'src/components/iconify';
import { Scrollbar } from 'src/components/scrollbar';
import { useSnackbar } from 'src/components/snackbar';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';
import {
  useTable,
  emptyRows,
  TableNoData,
  TableEmptyRows,
  TableHeadCustom,
  TableSelectedAction,
} from 'src/components/table';

import { WorkspaceTableRow } from './workspace-table-row';
import { WorkspaceTableToolbar } from './workspace-table-toolbar';

// ----------------------------------------------------------------------

const TABLE_HEAD = [
  { id: 'name', label: 'Name' },
  { id: 'path', label: 'Path' },
  { id: 'language', label: 'Language' },
  { id: 'created_at', label: 'Created' },
  { id: 'updated_at', label: 'Updated' },
  { id: '', width: 88 },
];

// Helper function to get rows for current page
function rowInRange<T>(array: T[], page: number, rowsPerPage: number): T[] {
  return array.slice(page * rowsPerPage, page * rowsPerPage + rowsPerPage);
}

// ----------------------------------------------------------------------

export function WorkspaceListView() {
  const theme = useTheme();
  const navigate = useNavigate();
  const { enqueueSnackbar } = useSnackbar();

  const table = useTable();

  const [filters, setFilters] = useState({ name: '' });

  // Fetch workspaces
  const { data: workspaces = [], isLoading, error } = useSWR<Workspace[]>(
    cortexEndpoints.workspaces.list,
    cortexFetcher,
    { refreshInterval: 10000 } // Refresh every 10 seconds
  );

  // Filter workspaces
  const dataFiltered = applyFilter({
    inputData: workspaces,
    filters,
  });

  const dataInPage = rowInRange(dataFiltered, table.page, table.rowsPerPage);

  const canReset = !!filters.name;

  const notFound = (!dataFiltered.length && canReset) || !dataFiltered.length;

  // ----------------------------------------------------------------------
  // Handlers
  // ----------------------------------------------------------------------

  const handleDeleteRow = useCallback(
    async (id: string) => {
      try {
        await cortexClient.deleteWorkspace(id);
        mutate(cortexEndpoints.workspaces.list);
        enqueueSnackbar('Workspace deleted successfully', 'success');
      } catch (err) {
        enqueueSnackbar('Failed to delete workspace', 'error');
      }
    },
    [enqueueSnackbar]
  );

  const handleDeleteRows = useCallback(
    async () => {
      try {
        await Promise.all(table.selected.map((id) => cortexClient.deleteWorkspace(id)));
        mutate(cortexEndpoints.workspaces.list);
        table.setSelected([]);
        enqueueSnackbar(`Deleted ${table.selected.length} workspaces`, 'success');
      } catch (err) {
        enqueueSnackbar('Failed to delete workspaces', 'error');
      }
    },
    [table, enqueueSnackbar]
  );

  const handleIndexWorkspace = useCallback(
    async (id: string) => {
      try {
        await cortexClient.indexWorkspace(id);
        enqueueSnackbar('Workspace indexing started', 'success');
      } catch (err) {
        enqueueSnackbar('Failed to start indexing', 'error');
      }
    },
    [enqueueSnackbar]
  );

  const handleBrowseFiles = useCallback(
    (id: string) => {
      navigate(`/cortex/workspaces/${id}/browse`);
    },
    [navigate]
  );

  const handleViewDetails = useCallback(
    (id: string) => {
      navigate(`/cortex/workspaces/${id}`);
    },
    [navigate]
  );

  const handleFilters = useCallback(
    (name: string, value: string) => {
      table.onResetPage();
      setFilters((prev) => ({ ...prev, [name]: value }));
    },
    [table]
  );

  const handleResetFilters = useCallback(() => {
    setFilters({ name: '' });
  }, []);

  // ----------------------------------------------------------------------

  return (
    <Stack spacing={2}>
      <CustomBreadcrumbs
        heading="Workspaces"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Cortex', href: '/cortex' },
          { name: 'Workspaces' },
        ]}
        action={
          <Button
            variant="contained"
            startIcon={<Iconify icon="mingcute:add-line" />}
            href="/cortex/workspaces/create"
          >
            New Workspace
          </Button>
        }
        sx={{ mb: 3 }}
      />

      <Card>
        <WorkspaceTableToolbar
          filters={filters}
          onFilters={handleFilters}
          canReset={canReset}
          onResetFilters={handleResetFilters}
        />

        {notFound && <TableNoData searchQuery={filters.name} sx={{ py: 10 }} />}

        <TableContainer sx={{ position: 'relative', overflow: 'unset' }}>
          <TableSelectedAction
            dense={table.dense}
            numSelected={table.selected.length}
            rowCount={dataFiltered.length}
            onSelectAllRows={(checked) =>
              table.onSelectAllRows(checked, dataFiltered.map((row) => row.id))
            }
            action={
              <Button
                color="error"
                variant="contained"
                startIcon={<Iconify icon="solar:trash-bin-trash-bold" />}
                onClick={handleDeleteRows}
              >
                Delete
              </Button>
            }
          />

          <Scrollbar sx={{ minHeight: 444 }}>
            <Table size={table.dense ? 'small' : 'medium'} sx={{ minWidth: 960 }}>
              <TableHeadCustom
                order={table.order}
                orderBy={table.orderBy}
                headLabel={TABLE_HEAD}
                rowCount={dataFiltered.length}
                numSelected={table.selected.length}
                onSort={table.onSort}
                onSelectAllRows={(checked) =>
                  table.onSelectAllRows(checked, dataFiltered.map((row) => row.id))
                }
              />

              <TableBody>
                {dataInPage.map((row) => (
                  <WorkspaceTableRow
                    key={row.id}
                    row={row}
                    selected={table.selected.includes(row.id)}
                    onSelectRow={() => table.onSelectRow(row.id)}
                    onDeleteRow={() => handleDeleteRow(row.id)}
                    onIndexRow={() => handleIndexWorkspace(row.id)}
                    onBrowseFiles={() => handleBrowseFiles(row.id)}
                    onViewDetails={() => handleViewDetails(row.id)}
                  />
                ))}

                <TableEmptyRows
                  height={table.dense ? 56 : 76}
                  emptyRows={emptyRows(table.page, table.rowsPerPage, dataFiltered.length)}
                />

                {isLoading && (
                  <TableNoData searchQuery="" sx={{ py: 10 }}>
                    Loading...
                  </TableNoData>
                )}
              </TableBody>
            </Table>
          </Scrollbar>
        </TableContainer>

        <TablePagination
          component="div"
          page={table.page}
          count={dataFiltered.length}
          rowsPerPage={table.rowsPerPage}
          onPageChange={table.onChangePage}
          rowsPerPageOptions={[5, 10, 25]}
          onRowsPerPageChange={table.onChangeRowsPerPage}
        />
      </Card>
    </Stack>
  );
}

// ----------------------------------------------------------------------

type ApplyFilterProps = {
  inputData: Workspace[];
  filters: { name: string };
};

function applyFilter({ inputData, filters }: ApplyFilterProps): Workspace[] {
  const { name } = filters;

  if (name) {
    return inputData.filter(
      (workspace) =>
        workspace.name.toLowerCase().includes(name.toLowerCase()) ||
        workspace.path.toLowerCase().includes(name.toLowerCase())
    );
  }

  return inputData;
}
