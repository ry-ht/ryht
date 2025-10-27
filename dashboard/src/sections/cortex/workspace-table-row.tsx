import type { Workspace } from 'src/types/cortex';

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
  row: Workspace;
  selected: boolean;
  onSelectRow: () => void;
  onDeleteRow: () => void;
  onIndexRow: () => void;
  onBrowseFiles: () => void;
};

export function WorkspaceTableRow({ row, selected, onSelectRow, onDeleteRow, onIndexRow, onBrowseFiles }: Props) {
  const popover = usePopover();

  return (
    <>
      <TableRow hover selected={selected}>
        <TableCell padding="checkbox">
          <Checkbox checked={selected} onClick={onSelectRow} />
        </TableCell>

        <TableCell>{row.name}</TableCell>

        <TableCell>{row.path}</TableCell>

        <TableCell>
          {row.language && <Chip label={row.language} size="small" />}
        </TableCell>

        <TableCell>{fDateTime(row.created_at)}</TableCell>

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
        <MenuItem onClick={() => { onBrowseFiles(); popover.onClose(); }}>
          <Iconify icon="solar:folder-open-bold" />
          Browse Files
        </MenuItem>

        <MenuItem onClick={() => { onIndexRow(); popover.onClose(); }}>
          <Iconify icon="solar:refresh-bold" />
          Re-index
        </MenuItem>

        <MenuItem
          onClick={() => {
            onDeleteRow();
            popover.onClose();
          }}
          sx={{ color: 'error.main' }}
        >
          <Iconify icon="solar:trash-bin-trash-bold" />
          Delete
        </MenuItem>
      </CustomPopover>
    </>
  );
}
