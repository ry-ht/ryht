import type { ChangeEvent } from 'react';
import { useCallback } from 'react';

import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import InputAdornment from '@mui/material/InputAdornment';
import Button from '@mui/material/Button';
import MenuItem from '@mui/material/MenuItem';

import { Iconify } from 'src/components/iconify';

type Props = {
  filters: { search: string; status: string; docType: string };
  onFilters: (name: string, value: string) => void;
  canReset: boolean;
  onResetFilters: () => void;
};

const STATUS_OPTIONS = ['Draft', 'Review', 'Published', 'Archived'];
const TYPE_OPTIONS = ['Guide', 'ApiReference', 'Architecture', 'Tutorial', 'Explanation'];

export function DocumentTableToolbar({ filters, onFilters, canReset, onResetFilters }: Props) {
  const handleFilterSearch = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      onFilters('search', event.target.value);
    },
    [onFilters]
  );

  const handleFilterStatus = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      onFilters('status', event.target.value);
    },
    [onFilters]
  );

  const handleFilterType = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      onFilters('docType', event.target.value);
    },
    [onFilters]
  );

  return (
    <Stack
      spacing={2}
      alignItems={{ xs: 'flex-end', md: 'center' }}
      direction={{ xs: 'column', md: 'row' }}
      sx={{ p: 2.5, pr: { xs: 2.5, md: 1 } }}
    >
      <TextField
        fullWidth
        value={filters.search}
        onChange={handleFilterSearch}
        placeholder="Search documents..."
        InputProps={{
          startAdornment: (
            <InputAdornment position="start">
              <Iconify icon="eva:search-fill" sx={{ color: 'text.disabled' }} />
            </InputAdornment>
          ),
        }}
      />

      <TextField
        select
        label="Status"
        value={filters.status}
        onChange={handleFilterStatus}
        sx={{ minWidth: 160 }}
      >
        <MenuItem value="">All</MenuItem>
        {STATUS_OPTIONS.map((option) => (
          <MenuItem key={option} value={option}>
            {option}
          </MenuItem>
        ))}
      </TextField>

      <TextField
        select
        label="Type"
        value={filters.docType}
        onChange={handleFilterType}
        sx={{ minWidth: 160 }}
      >
        <MenuItem value="">All</MenuItem>
        {TYPE_OPTIONS.map((option) => (
          <MenuItem key={option} value={option}>
            {option}
          </MenuItem>
        ))}
      </TextField>

      {canReset && (
        <Button
          color="error"
          sx={{ flexShrink: 0 }}
          onClick={onResetFilters}
          startIcon={<Iconify icon="solar:trash-bin-trash-bold" />}
        >
          Clear
        </Button>
      )}
    </Stack>
  );
}
