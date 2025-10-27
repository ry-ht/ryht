import useSWR from 'swr';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Table from '@mui/material/Table';
import Stack from '@mui/material/Stack';
import TableRow from '@mui/material/TableRow';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import Typography from '@mui/material/Typography';
import TableContainer from '@mui/material/TableContainer';
import LinearProgress from '@mui/material/LinearProgress';

import { cortexFetcher } from 'src/lib/cortex-client';

import { Label } from 'src/components/label';
import { Scrollbar } from 'src/components/scrollbar';
import { TableHeadCustom, TableNoData } from 'src/components/table';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

const TABLE_HEAD = [
  { id: 'resource', label: 'Resource', width: 200 },
  { id: 'holder', label: 'Lock Holder', width: 150 },
  { id: 'type', label: 'Type', width: 100 },
  { id: 'acquired', label: 'Acquired At', width: 180 },
  { id: 'expires', label: 'Expires', width: 180 },
  { id: 'status', label: 'Status', width: 100 },
];

export function CoordinationLocksView() {
  const { data: locks = [], isLoading } = useSWR(
    '/api/v1/locks',
    cortexFetcher,
    { refreshInterval: 3000 }
  );

  return (
    <>
      <CustomBreadcrumbs
        heading="Distributed Locks"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Coordination' },
          { name: 'Locks' },
        ]}
        sx={{ mb: 3 }}
      />

      <Card>
        <Box sx={{ p: 3 }}>
          <Typography variant="h6" sx={{ mb: 1 }}>
            Resource Locks
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Monitor distributed locks to prevent race conditions and ensure safe resource access.
          </Typography>
        </Box>

        {isLoading && <LinearProgress />}

        <Scrollbar>
          <TableContainer sx={{ minWidth: 800 }}>
            <Table>
              <TableHeadCustom headLabel={TABLE_HEAD} />
              <TableBody>
                {locks.map((lock: any) => (
                  <TableRow key={lock.id} hover>
                    <TableCell>
                      <Stack spacing={0.5}>
                        <Typography variant="subtitle2">{lock.resource_id}</Typography>
                        <Typography variant="caption" color="text.disabled">
                          {lock.resource_type}
                        </Typography>
                      </Stack>
                    </TableCell>
                    <TableCell>
                      <Typography variant="body2">{lock.holder_id || 'N/A'}</Typography>
                    </TableCell>
                    <TableCell>
                      <Label variant="soft" color={lock.lock_type === 'exclusive' ? 'error' : 'info'}>
                        {lock.lock_type || 'shared'}
                      </Label>
                    </TableCell>
                    <TableCell>
                      <Typography variant="body2">
                        {lock.acquired_at
                          ? new Date(lock.acquired_at).toLocaleString()
                          : 'N/A'}
                      </Typography>
                    </TableCell>
                    <TableCell>
                      <Typography variant="body2">
                        {lock.expires_at
                          ? new Date(lock.expires_at).toLocaleString()
                          : 'Never'}
                      </Typography>
                    </TableCell>
                    <TableCell>
                      <Label variant="soft" color="success">
                        Active
                      </Label>
                    </TableCell>
                  </TableRow>
                ))}
                <TableNoData notFound={!isLoading && locks.length === 0} />
              </TableBody>
            </Table>
          </TableContainer>
        </Scrollbar>
      </Card>
    </>
  );
}
