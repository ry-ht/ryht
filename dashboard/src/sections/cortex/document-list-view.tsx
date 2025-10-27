import { useState, useCallback } from 'react';
import { useNavigate } from 'react-router';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Table from '@mui/material/Table';
import Button from '@mui/material/Button';
import TableBody from '@mui/material/TableBody';
import Typography from '@mui/material/Typography';
import TableContainer from '@mui/material/TableContainer';
import Chip from '@mui/material/Chip';

import { fDateTime } from 'src/utils/format-time';

import { Iconify } from 'src/components/iconify';
import { Scrollbar } from 'src/components/scrollbar';
import { useSnackbar } from 'src/components/snackbar';
import {
  useTable,
  emptyRows,
  rowInRange,
  TableNoData,
  TableEmptyRows,
  TableHeadCustom,
  TableSelectedAction,
  TablePaginationCustom,
} from 'src/components/table';

import useSWR, { mutate } from 'swr';
import { cortexClient, cortexFetcher, cortexEndpoints } from 'src/lib/cortex-client';
import type { Document } from 'src/types/cortex';
import { DocumentTableRow } from './document-table-row';
import { DocumentTableToolbar } from './document-table-toolbar';

// ----------------------------------------------------------------------

const TABLE_HEAD = [
  { id: 'title', label: 'Title' },
  { id: 'doc_type', label: 'Type' },
  { id: 'status', label: 'Status' },
  { id: 'author', label: 'Author' },
  { id: 'updated_at', label: 'Updated' },
  { id: '', width: 88 },
];

// ----------------------------------------------------------------------

export function DocumentListView() {
  const navigate = useNavigate();
  const { enqueueSnackbar } = useSnackbar();

  const table = useTable();

  const [filters, setFilters] = useState({
    search: '',
    status: '',
    docType: '',
  });

  // Fetch documents
  const { data: documents = [], isLoading } = useSWR<Document[]>(
    cortexEndpoints.documents.list,
    cortexFetcher,
    { refreshInterval: 10000 }
  );

  // Filter documents
  const dataFiltered = applyFilter({
    inputData: documents,
    filters,
  });

  const dataInPage = rowInRange(dataFiltered, table.page, table.rowsPerPage);

  const canReset = !!filters.search || !!filters.status || !!filters.docType;

  const notFound = (!dataFiltered.length && canReset) || !dataFiltered.length;

  // ----------------------------------------------------------------------
  // Handlers
  // ----------------------------------------------------------------------

  const handleViewRow = useCallback(
    (id: string) => {
      navigate(`/dashboard/cortex/documents/${id}`);
    },
    [navigate]
  );

  const handleDeleteRow = useCallback(
    async (id: string) => {
      try {
        await cortexClient.deleteDocument(id);
        mutate(cortexEndpoints.documents.list);
        enqueueSnackbar('Document deleted', { variant: 'success' });
      } catch (err) {
        enqueueSnackbar('Failed to delete document', { variant: 'error' });
      }
    },
    [enqueueSnackbar]
  );

  const handleDeleteRows = useCallback(
    async () => {
      try {
        await Promise.all(table.selected.map((id) => cortexClient.deleteDocument(id)));
        mutate(cortexEndpoints.documents.list);
        table.setSelected([]);
        enqueueSnackbar(`Deleted ${table.selected.length} documents`, { variant: 'success' });
      } catch (err) {
        enqueueSnackbar('Failed to delete documents', { variant: 'error' });
      }
    },
    [table, enqueueSnackbar]
  );

  const handlePublishDocument = useCallback(
    async (id: string) => {
      try {
        await cortexClient.publishDocument(id);
        mutate(cortexEndpoints.documents.list);
        enqueueSnackbar('Document published', { variant: 'success' });
      } catch (err) {
        enqueueSnackbar('Failed to publish document', { variant: 'error' });
      }
    },
    [enqueueSnackbar]
  );

  const handleFilters = useCallback(
    (name: string, value: string) => {
      table.onResetPage();
      setFilters((prev) => ({ ...prev, [name]: value }));
    },
    [table]
  );

  const handleResetFilters = useCallback(() => {
    setFilters({ search: '', status: '', docType: '' });
  }, []);

  // ----------------------------------------------------------------------

  return (
    <Box sx={{ p: 3 }}>
      <Box sx={{ display: 'flex', alignItems: 'center', mb: 3 }}>
        <Typography variant="h4" sx={{ flexGrow: 1 }}>
          Documents
        </Typography>

        <Button
          variant="contained"
          startIcon={<Iconify icon="mingcute:add-line" />}
          href="/dashboard/cortex/documents/create"
        >
          New Document
        </Button>
      </Box>

      <Card>
        <DocumentTableToolbar
          filters={filters}
          onFilters={handleFilters}
          canReset={canReset}
          onResetFilters={handleResetFilters}
        />

        {notFound && <TableNoData searchQuery={filters.search} sx={{ py: 10 }} />}

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
                  <DocumentTableRow
                    key={row.id}
                    row={row}
                    selected={table.selected.includes(row.id)}
                    onSelectRow={() => table.onSelectRow(row.id)}
                    onViewRow={() => handleViewRow(row.id)}
                    onDeleteRow={() => handleDeleteRow(row.id)}
                    onPublishRow={() => handlePublishDocument(row.id)}
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

        <TablePaginationCustom
          page={table.page}
          dense={table.dense}
          count={dataFiltered.length}
          rowsPerPage={table.rowsPerPage}
          onPageChange={table.onChangePage}
          onChangeDense={table.onChangeDense}
          onRowsPerPageChange={table.onChangeRowsPerPage}
        />
      </Card>
    </Box>
  );
}

// ----------------------------------------------------------------------

type ApplyFilterProps = {
  inputData: Document[];
  filters: { search: string; status: string; docType: string };
};

function applyFilter({ inputData, filters }: ApplyFilterProps): Document[] {
  const { search, status, docType } = filters;

  let data = inputData;

  if (search) {
    data = data.filter(
      (doc) =>
        doc.title.toLowerCase().includes(search.toLowerCase()) ||
        doc.content.toLowerCase().includes(search.toLowerCase()) ||
        doc.tags.some((tag) => tag.toLowerCase().includes(search.toLowerCase()))
    );
  }

  if (status) {
    data = data.filter((doc) => doc.status === status);
  }

  if (docType) {
    data = data.filter((doc) => doc.doc_type === docType);
  }

  return data;
}
