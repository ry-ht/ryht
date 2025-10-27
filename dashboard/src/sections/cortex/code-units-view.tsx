import type { CodeUnit } from 'src/types/cortex';

import useSWR from 'swr';
import { useParams, useNavigate } from 'react-router';
import { useMemo, useState, useCallback } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Chip from '@mui/material/Chip';
import List from '@mui/material/List';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Select from '@mui/material/Select';
import Dialog from '@mui/material/Dialog';
import Divider from '@mui/material/Divider';
import { alpha } from '@mui/material/styles';
import MenuItem from '@mui/material/MenuItem';
import ListItem from '@mui/material/ListItem';
import TextField from '@mui/material/TextField';
import Accordion from '@mui/material/Accordion';
import Typography from '@mui/material/Typography';
import InputLabel from '@mui/material/InputLabel';
import IconButton from '@mui/material/IconButton';
import FormControl from '@mui/material/FormControl';
import DialogTitle from '@mui/material/DialogTitle';
import ListItemText from '@mui/material/ListItemText';
import ListItemIcon from '@mui/material/ListItemIcon';
import DialogContent from '@mui/material/DialogContent';
import DialogActions from '@mui/material/DialogActions';
import InputAdornment from '@mui/material/InputAdornment';
import ListItemButton from '@mui/material/ListItemButton';
import LinearProgress from '@mui/material/LinearProgress';
import AccordionSummary from '@mui/material/AccordionSummary';
import AccordionDetails from '@mui/material/AccordionDetails';

import { cortexFetcher } from 'src/lib/cortex-client';

import { Iconify } from 'src/components/iconify';
import { Markdown } from 'src/components/markdown';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

type UnitTypeFilter = 'all' | 'function' | 'class' | 'interface' | 'module' | 'method';

const UNIT_TYPE_FILTERS: { value: UnitTypeFilter; label: string; icon: string }[] = [
  { value: 'all', label: 'All Types', icon: 'solar:code-bold-duotone' },
  { value: 'function', label: 'Functions', icon: 'solar:function-bold-duotone' },
  { value: 'class', label: 'Classes', icon: 'solar:box-bold-duotone' },
  { value: 'interface', label: 'Interfaces', icon: 'solar:layers-bold-duotone' },
  { value: 'module', label: 'Modules', icon: 'solar:module-bold-duotone' },
  { value: 'method', label: 'Methods', icon: 'solar:cog-bold-duotone' },
];

const COMPLEXITY_LEVELS = [
  { min: 0, max: 5, label: 'Low', color: 'success' },
  { min: 6, max: 10, label: 'Medium', color: 'warning' },
  { min: 11, max: 999, label: 'High', color: 'error' },
] as const;

// ----------------------------------------------------------------------

export function CodeUnitsView() {
  const params = useParams();
  const navigate = useNavigate();
  const workspaceId = params.id as string;

  const [searchQuery, setSearchQuery] = useState('');
  const [unitTypeFilter, setUnitTypeFilter] = useState<UnitTypeFilter>('all');
  const [selectedUnit, setSelectedUnit] = useState<CodeUnit | null>(null);
  const [isDetailOpen, setIsDetailOpen] = useState(false);

  // Fetch code units
  const { data: codeUnits = [], isLoading } = useSWR<CodeUnit[]>(
    workspaceId ? `/api/v1/code-units/search?workspace_id=${workspaceId}&q=` : null,
    cortexFetcher,
    { refreshInterval: 30000 }
  );

  // ----------------------------------------------------------------------
  // Filtering & Sorting
  // ----------------------------------------------------------------------

  const filteredUnits = useMemo(() => {
    let filtered = codeUnits;

    // Apply type filter
    if (unitTypeFilter !== 'all') {
      filtered = filtered.filter(
        (unit) => unit.unit_type.toLowerCase() === unitTypeFilter.toLowerCase()
      );
    }

    // Apply search query
    if (searchQuery) {
      filtered = filtered.filter(
        (unit) =>
          unit.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          unit.qualified_name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          unit.file_path.toLowerCase().includes(searchQuery.toLowerCase())
      );
    }

    // Sort by name
    filtered.sort((a, b) => a.name.localeCompare(b.name));

    return filtered;
  }, [codeUnits, unitTypeFilter, searchQuery]);

  // Group units by file
  const unitsByFile = useMemo(() => {
    const grouped = new Map<string, CodeUnit[]>();

    filteredUnits.forEach((unit) => {
      const existing = grouped.get(unit.file_path) || [];
      grouped.set(unit.file_path, [...existing, unit]);
    });

    return Array.from(grouped.entries()).sort((a, b) => a[0].localeCompare(b[0]));
  }, [filteredUnits]);

  // Unit type stats
  const unitTypeStats = useMemo(() => {
    const stats: Record<string, number> = {};

    codeUnits.forEach((unit) => {
      const type = unit.unit_type.toLowerCase();
      stats[type] = (stats[type] || 0) + 1;
    });

    return stats;
  }, [codeUnits]);

  // ----------------------------------------------------------------------
  // Handlers
  // ----------------------------------------------------------------------

  const handleUnitClick = useCallback(async (unit: CodeUnit) => {
    setSelectedUnit(unit);
    setIsDetailOpen(true);
  }, []);

  const handleCloseDetail = useCallback(() => {
    setIsDetailOpen(false);
    setSelectedUnit(null);
  }, []);

  // ----------------------------------------------------------------------
  // Helpers
  // ----------------------------------------------------------------------

  const getUnitIcon = (unitType: string) => {
    const type = unitType.toLowerCase();
    const iconMap: Record<string, string> = {
      function: 'solar:function-bold-duotone',
      class: 'solar:box-bold-duotone',
      interface: 'solar:layers-bold-duotone',
      module: 'solar:module-bold-duotone',
      method: 'solar:cog-bold-duotone',
      const: 'solar:square-bold-duotone',
      variable: 'solar:square-bold-duotone',
    };

    return iconMap[type] || 'solar:code-bold-duotone';
  };

  const getUnitColor = (unitType: string) => {
    const type = unitType.toLowerCase();
    const colorMap: Record<string, string> = {
      function: 'primary',
      class: 'success',
      interface: 'info',
      module: 'warning',
      method: 'secondary',
    };

    return colorMap[type] || 'default';
  };

  const getComplexityLevel = (complexity: number) => COMPLEXITY_LEVELS.find((level) => complexity >= level.min && complexity <= level.max);

  const calculateComplexity = (unit: CodeUnit): number => {
    // Simple heuristic based on line count and signature
    const lineCount = unit.end_line - unit.start_line;
    const signatureComplexity = unit.signature?.length || 0;

    // Mock complexity calculation
    return Math.min(Math.floor((lineCount + signatureComplexity / 10) / 5), 20);
  };

  // ----------------------------------------------------------------------

  return (
    <Box sx={{ p: 3 }}>
      {/* Header */}
      <Stack direction="row" alignItems="center" spacing={2} sx={{ mb: 3 }}>
        <Button
          startIcon={<Iconify icon="eva:arrow-back-fill" />}
          onClick={() => navigate(`/cortex/workspaces/${workspaceId}`)}
        >
          Back
        </Button>
        <Typography variant="h4" sx={{ flexGrow: 1 }}>
          Code Units
        </Typography>
      </Stack>

      {/* Stats Cards */}
      <Stack direction="row" spacing={2} sx={{ mb: 3, overflowX: 'auto' }}>
        <Card sx={{ p: 2, minWidth: 180 }}>
          <Stack direction="row" alignItems="center" spacing={2}>
            <Box
              sx={{
                width: 40,
                height: 40,
                borderRadius: 1,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                bgcolor: (theme) => alpha(theme.palette.primary.main, 0.08),
              }}
            >
              <Iconify icon="solar:code-bold-duotone" width={20} sx={{ color: 'primary.main' }} />
            </Box>
            <Box>
              <Typography variant="h4">{codeUnits.length}</Typography>
              <Typography variant="caption" color="text.secondary">
                Total Units
              </Typography>
            </Box>
          </Stack>
        </Card>

        {Object.entries(unitTypeStats).map(([type, count]) => (
          <Card key={type} sx={{ p: 2, minWidth: 150 }}>
            <Stack direction="row" alignItems="center" spacing={2}>
              <Iconify icon={getUnitIcon(type)} width={24} />
              <Box>
                <Typography variant="h5">{count}</Typography>
                <Typography variant="caption" color="text.secondary">
                  {type.charAt(0).toUpperCase() + type.slice(1)}s
                </Typography>
              </Box>
            </Stack>
          </Card>
        ))}
      </Stack>

      {/* Filters */}
      <Card sx={{ p: 3, mb: 3 }}>
        <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2}>
          <TextField
            fullWidth
            placeholder="Search by name, path, or qualified name..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            InputProps={{
              startAdornment: (
                <InputAdornment position="start">
                  <Iconify icon="eva:search-fill" />
                </InputAdornment>
              ),
              endAdornment: searchQuery && (
                <InputAdornment position="end">
                  <IconButton onClick={() => setSearchQuery('')} edge="end" size="small">
                    <Iconify icon="eva:close-fill" />
                  </IconButton>
                </InputAdornment>
              ),
            }}
          />
          <FormControl sx={{ minWidth: 200 }}>
            <InputLabel>Unit Type</InputLabel>
            <Select
              value={unitTypeFilter}
              label="Unit Type"
              onChange={(e) => setUnitTypeFilter(e.target.value as UnitTypeFilter)}
            >
              {UNIT_TYPE_FILTERS.map((filter) => (
                <MenuItem key={filter.value} value={filter.value}>
                  <Stack direction="row" spacing={1} alignItems="center">
                    <Iconify icon={filter.icon} width={20} />
                    <span>{filter.label}</span>
                  </Stack>
                </MenuItem>
              ))}
            </Select>
          </FormControl>
        </Stack>

        <Box sx={{ mt: 2 }}>
          <Typography variant="caption" color="text.secondary">
            {filteredUnits.length} of {codeUnits.length} units
            {unitTypeFilter !== 'all' && ` (filtered by ${unitTypeFilter})`}
          </Typography>
        </Box>
      </Card>

      {/* Code Units List */}
      <Card>
        {isLoading ? (
          <Box sx={{ p: 3 }}>
            <LinearProgress />
            <Typography align="center" sx={{ mt: 2 }}>
              Loading code units...
            </Typography>
          </Box>
        ) : unitsByFile.length > 0 ? (
          <Box>
            {unitsByFile.map(([filePath, units]) => (
              <Accordion key={filePath} defaultExpanded={unitsByFile.length <= 5}>
                <AccordionSummary expandIcon={<Iconify icon="eva:arrow-ios-downward-fill" />}>
                  <Stack direction="row" spacing={2} alignItems="center" sx={{ width: '100%' }}>
                    <Iconify icon="solar:file-bold-duotone" width={20} />
                    <Typography variant="body2" sx={{ flexGrow: 1, fontFamily: 'monospace' }}>
                      {filePath}
                    </Typography>
                    <Chip label={`${units.length} units`} size="small" />
                  </Stack>
                </AccordionSummary>
                <AccordionDetails>
                  <List disablePadding>
                    {units.map((unit) => {
                      const complexity = calculateComplexity(unit);
                      const complexityLevel = getComplexityLevel(complexity);

                      return (
                        <ListItem
                          key={unit.id}
                          disablePadding
                          secondaryAction={
                            <Stack direction="row" spacing={1} alignItems="center">
                              {complexityLevel && (
                                <Chip
                                  label={`Complexity: ${complexity}`}
                                  size="small"
                                  color={complexityLevel.color as any}
                                  variant="outlined"
                                />
                              )}
                              <Chip
                                label={`L${unit.start_line}-${unit.end_line}`}
                                size="small"
                                variant="outlined"
                              />
                            </Stack>
                          }
                        >
                          <ListItemButton onClick={() => handleUnitClick(unit)}>
                            <ListItemIcon>
                              <Iconify icon={getUnitIcon(unit.unit_type)} width={24} />
                            </ListItemIcon>
                            <ListItemText
                              primary={
                                <Stack direction="row" spacing={1} alignItems="center">
                                  <Typography variant="body2">{unit.name}</Typography>
                                  <Chip
                                    label={unit.unit_type}
                                    size="small"
                                    color={getUnitColor(unit.unit_type) as any}
                                    variant="outlined"
                                    sx={{ height: 20 }}
                                  />
                                </Stack>
                              }
                              secondary={
                                <Typography
                                  variant="caption"
                                  color="text.secondary"
                                  sx={{ fontFamily: 'monospace' }}
                                >
                                  {unit.qualified_name}
                                </Typography>
                              }
                            />
                          </ListItemButton>
                        </ListItem>
                      );
                    })}
                  </List>
                </AccordionDetails>
              </Accordion>
            ))}
          </Box>
        ) : (
          <Box sx={{ py: 10, textAlign: 'center' }}>
            <Iconify icon="solar:code-bold-duotone" width={64} sx={{ mb: 2, opacity: 0.5 }} />
            <Typography variant="body2" color="text.secondary">
              {searchQuery || unitTypeFilter !== 'all'
                ? 'No code units match your filters'
                : 'No code units found in this workspace'}
            </Typography>
          </Box>
        )}
      </Card>

      {/* Detail Dialog */}
      <Dialog
        open={isDetailOpen}
        onClose={handleCloseDetail}
        maxWidth="md"
        fullWidth
        PaperProps={{ sx: { height: '80vh' } }}
      >
        {selectedUnit && (
          <>
            <DialogTitle>
              <Stack direction="row" alignItems="center" spacing={2}>
                <Iconify icon={getUnitIcon(selectedUnit.unit_type)} width={24} />
                <Box sx={{ flexGrow: 1 }}>
                  <Typography variant="h6">{selectedUnit.name}</Typography>
                  <Typography variant="caption" color="text.secondary" sx={{ fontFamily: 'monospace' }}>
                    {selectedUnit.qualified_name}
                  </Typography>
                </Box>
                <Chip label={selectedUnit.unit_type} color={getUnitColor(selectedUnit.unit_type) as any} />
                <IconButton onClick={handleCloseDetail}>
                  <Iconify icon="eva:close-fill" />
                </IconButton>
              </Stack>
            </DialogTitle>

            <DialogContent dividers>
              <Stack spacing={3}>
                {/* File Location */}
                <Box>
                  <Typography variant="overline" color="text.secondary">
                    File Location
                  </Typography>
                  <Typography
                    variant="body2"
                    sx={{ fontFamily: 'monospace', bgcolor: 'background.neutral', p: 1, borderRadius: 1 }}
                  >
                    {selectedUnit.file_path}
                  </Typography>
                  <Typography variant="caption" color="text.secondary" sx={{ mt: 0.5, display: 'block' }}>
                    Lines {selectedUnit.start_line} - {selectedUnit.end_line} ({selectedUnit.end_line - selectedUnit.start_line + 1} lines)
                  </Typography>
                </Box>

                {/* Signature */}
                {selectedUnit.signature && (
                  <Box>
                    <Typography variant="overline" color="text.secondary">
                      Signature
                    </Typography>
                    <Box
                      component="pre"
                      sx={{
                        p: 2,
                        bgcolor: 'background.neutral',
                        borderRadius: 1,
                        overflow: 'auto',
                        fontFamily: 'monospace',
                        fontSize: '0.875rem',
                      }}
                    >
                      {selectedUnit.signature}
                    </Box>
                  </Box>
                )}

                {/* Documentation */}
                {selectedUnit.docstring && (
                  <Box>
                    <Typography variant="overline" color="text.secondary">
                      Documentation
                    </Typography>
                    <Card sx={{ p: 2, bgcolor: 'background.neutral' }}>
                      <Markdown content={selectedUnit.docstring} />
                    </Card>
                  </Box>
                )}

                {/* Complexity Metrics */}
                <Box>
                  <Typography variant="overline" color="text.secondary">
                    Complexity Metrics
                  </Typography>
                  <Stack direction="row" spacing={2} sx={{ mt: 1 }}>
                    {(() => {
                      const complexity = calculateComplexity(selectedUnit);
                      const level = getComplexityLevel(complexity);
                      return (
                        <>
                          <Chip
                            icon={<Iconify icon="solar:chart-bold-duotone" />}
                            label={`Complexity: ${complexity}`}
                            color={level?.color as any}
                          />
                          <Chip
                            icon={<Iconify icon="solar:text-bold-duotone" />}
                            label={`${selectedUnit.end_line - selectedUnit.start_line + 1} lines`}
                            variant="outlined"
                          />
                        </>
                      );
                    })()}
                  </Stack>
                </Box>

                <Divider />

                {/* Metadata */}
                <Box>
                  <Typography variant="overline" color="text.secondary">
                    Metadata
                  </Typography>
                  <Stack spacing={1} sx={{ mt: 1 }}>
                    <Typography variant="body2">
                      <strong>ID:</strong> {selectedUnit.id}
                    </Typography>
                    <Typography variant="body2">
                      <strong>Created:</strong> {new Date(selectedUnit.created_at).toLocaleString()}
                    </Typography>
                    <Typography variant="body2">
                      <strong>Updated:</strong> {new Date(selectedUnit.updated_at).toLocaleString()}
                    </Typography>
                  </Stack>
                </Box>
              </Stack>
            </DialogContent>

            <DialogActions>
              <Button onClick={handleCloseDetail}>Close</Button>
            </DialogActions>
          </>
        )}
      </Dialog>
    </Box>
  );
}
