import type { Document } from 'src/types/cortex';

import MenuItem from '@mui/material/MenuItem';
import TableRow from '@mui/material/TableRow';
import Checkbox from '@mui/material/Checkbox';
import TableCell from '@mui/material/TableCell';
import IconButton from '@mui/material/IconButton';
import Chip from '@mui/material/Chip';

import { fDateTime } from 'src/utils/format-time';

import { Iconify } from 'src/components/iconify';
import { usePopover, CustomPopover } from 'src/components/custom-popover';

type Props = {
  row: Document;
  selected: boolean;
  onSelectRow: () => void;
  onViewRow: () => void;
  onDeleteRow: () => void;
  onPublishRow: () => void;
};

export function DocumentTableRow({
  row,
  selected,
  onSelectRow,
  onViewRow,
  onDeleteRow,
  onPublishRow,
}: Props) {
  const popover = usePopover();

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'Published':
        return 'success';
      case 'Draft':
        return 'default';
      case 'Review':
        return 'warning';
      case 'Archived':
        return 'error';
      default:
        return 'default';
    }
  };

  return (
    <>
      <TableRow hover selected={selected}>
        <TableCell padding="checkbox">
          <Checkbox checked={selected} onClick={onSelectRow} />
        </TableCell>

        <TableCell sx={{ cursor: 'pointer' }} onClick={onViewRow}>
          {row.title}
        </TableCell>

        <TableCell>{row.doc_type}</TableCell>

        <TableCell>
          <Chip label={row.status} size="small" color={getStatusColor(row.status)} />
        </TableCell>

        <TableCell>{row.author || '-'}</TableCell>

        <TableCell>{fDateTime(row.updated_at)}</TableCell>

        <TableCell align="right" sx={{ px: 1, whiteSpace: 'nowrap' }}>
          <IconButton color={popover.open ? 'inherit' : 'default'} onClick={popover.onOpen}>
            <Iconify icon="eva:more-vertical-fill" />
          </IconButton>
        </TableCell>
      </TableRow>

      <CustomPopover
        open={popover.open}
        anchorEl={popover.anchorEl}
        onClose={popover.onClose}
        slotProps={{ arrow: { placement: 'right-top' } }}
      >
        <MenuItem onClick={() => { onViewRow(); popover.onClose(); }}>
          <Iconify icon="solar:eye-bold" />
          View
        </MenuItem>

        {row.status !== 'Published' && (
          <MenuItem onClick={() => { onPublishRow(); popover.onClose(); }}>
            <Iconify icon="solar:upload-bold" />
            Publish
          </MenuItem>
        )}

        <MenuItem
          onClick={() => { onDeleteRow(); popover.onClose(); }}
          sx={{ color: 'error.main' }}
        >
          <Iconify icon="solar:trash-bin-trash-bold" />
          Delete
        </MenuItem>
      </CustomPopover>
    </>
  );
}
