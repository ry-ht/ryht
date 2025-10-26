import TableRow from '@mui/material/TableRow';
import TableCell from '@mui/material/TableCell';
import Typography from '@mui/material/Typography';

// ----------------------------------------------------------------------

type TableNoDataProps = {
  notFound: boolean;
  sx?: object;
};

export function TableNoData({ notFound, sx }: TableNoDataProps) {
  if (!notFound) {
    return null;
  }

  return (
    <TableRow>
      <TableCell colSpan={12} sx={{ py: 3, textAlign: 'center', ...sx }}>
        <Typography variant="body2" sx={{ color: 'text.secondary' }}>
          No data found
        </Typography>
      </TableCell>
    </TableRow>
  );
}
