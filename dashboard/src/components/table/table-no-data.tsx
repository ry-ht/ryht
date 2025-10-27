import type { ReactNode } from 'react';

import TableRow from '@mui/material/TableRow';
import TableCell from '@mui/material/TableCell';
import Typography from '@mui/material/Typography';

// ----------------------------------------------------------------------

type TableNoDataProps = {
  notFound?: boolean;
  searchQuery?: string;
  sx?: object;
  children?: ReactNode;
};

export function TableNoData({ notFound = true, searchQuery, sx, children }: TableNoDataProps) {
  if (!notFound) {
    return null;
  }

  const message = children || (searchQuery ? `No results found for "${searchQuery}"` : 'No data found');

  return (
    <TableRow>
      <TableCell colSpan={12} sx={{ py: 3, textAlign: 'center', ...sx }}>
        <Typography variant="body2" sx={{ color: 'text.secondary' }}>
          {message}
        </Typography>
      </TableCell>
    </TableRow>
  );
}
