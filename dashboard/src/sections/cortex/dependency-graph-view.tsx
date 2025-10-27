import { useState, useCallback, useEffect, useRef } from 'react';
import { useParams, useNavigate } from 'react-router';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Typography from '@mui/material/Typography';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import ToggleButton from '@mui/material/ToggleButton';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import Alert from '@mui/material/Alert';
import LinearProgress from '@mui/material/LinearProgress';
import Chip from '@mui/material/Chip';
import IconButton from '@mui/material/IconButton';
import Tooltip from '@mui/material/Tooltip';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import Divider from '@mui/material/Divider';
import { alpha } from '@mui/material/styles';

import { Iconify } from 'src/components/iconify';
import { useSnackbar } from 'src/components/snackbar';

import useSWR from 'swr';
import { cortexClient, cortexFetcher } from 'src/lib/cortex-client';

// Import mermaid for graph rendering
import mermaid from 'mermaid';

// ----------------------------------------------------------------------

type GraphFormat = 'mermaid' | 'json' | 'dot';
type LayoutDirection = 'TB' | 'LR' | 'BT' | 'RL';

interface DependencyNode {
  id: string;
  name: string;
  type: string;
  file_path?: string;
}

interface DependencyEdge {
  source: string;
  target: string;
  type: string;
}

interface DependencyGraph {
  nodes: DependencyNode[];
  edges: DependencyEdge[];
  cycles?: string[][];
}

// ----------------------------------------------------------------------

export function DependencyGraphView() {
  const params = useParams();
  const navigate = useNavigate();
  const workspaceId = params.id as string;
  const { enqueueSnackbar } = useSnackbar();

  const [graphFormat, setGraphFormat] = useState<GraphFormat>('mermaid');
  const [layoutDirection, setLayoutDirection] = useState<LayoutDirection>('TB');
  const [showCycles, setShowCycles] = useState(true);
  const [anchorEl, setAnchorEl] = useState<null | HTMLElement>(null);

  const mermaidRef = useRef<HTMLDivElement>(null);

  // Initialize mermaid
  useEffect(() => {
    mermaid.initialize({
      startOnLoad: false,
      theme: 'default',
      securityLevel: 'loose',
      flowchart: {
        useMaxWidth: true,
        htmlLabels: true,
        curve: 'basis',
      },
    });
  }, []);

  // Fetch dependencies
  const { data: dependencies, isLoading, error } = useSWR<any>(
    workspaceId ? `/api/v1/workspaces/${workspaceId}/dependencies` : null,
    cortexFetcher,
    { refreshInterval: 60000 }
  );

  // Fetch cycles
  const { data: cycles } = useSWR<any>(
    showCycles ? '/api/v1/analysis/cycles' : null,
    cortexFetcher
  );

  // ----------------------------------------------------------------------
  // Generate Graph Formats
  // ----------------------------------------------------------------------

  const generateMermaidGraph = useCallback((data: DependencyGraph, direction: LayoutDirection = 'TB') => {
    if (!data?.nodes || !data?.edges) return '';

    let mermaidCode = `graph ${direction}\n`;

    // Add nodes with styling
    data.nodes.forEach((node) => {
      const nodeId = node.id.replace(/[^a-zA-Z0-9]/g, '_');
      const shape = node.type === 'module' ? '[' : '(';
      const endShape = node.type === 'module' ? ']' : ')';
      mermaidCode += `    ${nodeId}${shape}"${node.name}"${endShape}\n`;
    });

    // Add edges
    data.edges.forEach((edge) => {
      const sourceId = edge.source.replace(/[^a-zA-Z0-9]/g, '_');
      const targetId = edge.target.replace(/[^a-zA-Z0-9]/g, '_');
      const edgeStyle = edge.type === 'import' ? '-->' : '-.->';
      mermaidCode += `    ${sourceId} ${edgeStyle} ${targetId}\n`;
    });

    // Highlight cycles
    if (data.cycles && data.cycles.length > 0) {
      mermaidCode += '\n    classDef cycleNode fill:#ffebee,stroke:#f44336,stroke-width:2px\n';
      const cycleNodeIds = new Set<string>();
      data.cycles.forEach((cycle) => {
        cycle.forEach((nodeId) => {
          cycleNodeIds.add(nodeId.replace(/[^a-zA-Z0-9]/g, '_'));
        });
      });
      mermaidCode += `    class ${Array.from(cycleNodeIds).join(',')} cycleNode\n`;
    }

    return mermaidCode;
  }, []);

  const generateDotGraph = useCallback((data: DependencyGraph) => {
    if (!data?.nodes || !data?.edges) return '';

    let dotCode = 'digraph Dependencies {\n';
    dotCode += '    rankdir=TB;\n';
    dotCode += '    node [shape=box, style=rounded];\n\n';

    // Add nodes
    data.nodes.forEach((node) => {
      const nodeId = node.id.replace(/[^a-zA-Z0-9]/g, '_');
      dotCode += `    ${nodeId} [label="${node.name}"];\n`;
    });

    dotCode += '\n';

    // Add edges
    data.edges.forEach((edge) => {
      const sourceId = edge.source.replace(/[^a-zA-Z0-9]/g, '_');
      const targetId = edge.target.replace(/[^a-zA-Z0-9]/g, '_');
      dotCode += `    ${sourceId} -> ${targetId};\n`;
    });

    dotCode += '}\n';

    return dotCode;
  }, []);

  const generateJsonGraph = useCallback((data: DependencyGraph) => {
    return JSON.stringify(data, null, 2);
  }, []);

  // ----------------------------------------------------------------------
  // Render Mermaid Graph
  // ----------------------------------------------------------------------

  useEffect(() => {
    const renderMermaid = async () => {
      if (!dependencies || graphFormat !== 'mermaid' || !mermaidRef.current) return;

      const mermaidCode = generateMermaidGraph(dependencies, layoutDirection);

      if (!mermaidCode) return;

      try {
        const { svg } = await mermaid.render('mermaid-graph', mermaidCode);
        mermaidRef.current.innerHTML = svg;
      } catch (err) {
        console.error('Mermaid render error:', err);
        mermaidRef.current.innerHTML = '<p>Error rendering graph</p>';
      }
    };

    renderMermaid();
  }, [dependencies, graphFormat, layoutDirection, generateMermaidGraph]);

  // ----------------------------------------------------------------------
  // Handlers
  // ----------------------------------------------------------------------

  const handleFormatChange = useCallback(
    (_event: React.MouseEvent<HTMLElement>, newFormat: GraphFormat | null) => {
      if (newFormat !== null) {
        setGraphFormat(newFormat);
      }
    },
    []
  );

  const handleDirectionChange = useCallback(
    (_event: React.MouseEvent<HTMLElement>, newDirection: LayoutDirection | null) => {
      if (newDirection !== null) {
        setLayoutDirection(newDirection);
      }
    },
    []
  );

  const handleExport = useCallback(
    (format: 'svg' | 'png' | 'json') => {
      if (!dependencies) return;

      let content = '';
      let filename = `dependencies-${workspaceId}`;
      let mimeType = 'application/json';

      switch (format) {
        case 'json':
          content = generateJsonGraph(dependencies);
          filename += '.json';
          mimeType = 'application/json';
          break;
        case 'svg':
          if (mermaidRef.current) {
            content = mermaidRef.current.innerHTML;
            filename += '.svg';
            mimeType = 'image/svg+xml';
          }
          break;
        case 'png':
          // For PNG, we need to convert SVG to canvas
          enqueueSnackbar('PNG export not yet implemented', 'info');
          return;
        default:
          return;
      }

      const blob = new Blob([content], { type: mimeType });
      const url = window.URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = filename;
      a.click();
      window.URL.revokeObjectURL(url);

      enqueueSnackbar(`Exported as ${format.toUpperCase()}`, 'success');
      setAnchorEl(null);
    },
    [dependencies, workspaceId, generateJsonGraph, enqueueSnackbar]
  );

  const handleCopyToClipboard = useCallback(() => {
    if (!dependencies) return;

    let content = '';

    switch (graphFormat) {
      case 'mermaid':
        content = generateMermaidGraph(dependencies, layoutDirection);
        break;
      case 'dot':
        content = generateDotGraph(dependencies);
        break;
      case 'json':
        content = generateJsonGraph(dependencies);
        break;
      default:
        return;
    }

    navigator.clipboard.writeText(content);
    enqueueSnackbar('Copied to clipboard', 'success');
  }, [dependencies, graphFormat, layoutDirection, generateMermaidGraph, generateDotGraph, generateJsonGraph, enqueueSnackbar]);

  // ----------------------------------------------------------------------
  // Stats
  // ----------------------------------------------------------------------

  const stats = dependencies
    ? {
        nodes: dependencies.nodes?.length || 0,
        edges: dependencies.edges?.length || 0,
        cycles: dependencies.cycles?.length || 0,
      }
    : null;

  // ----------------------------------------------------------------------

  return (
    <Box sx={{ p: 3 }}>
      {/* Header */}
      <Stack direction="row" alignItems="center" spacing={2} sx={{ mb: 3 }}>
        <Button
          startIcon={<Iconify icon="eva:arrow-back-fill" />}
          onClick={() => navigate(`/dashboard/cortex/workspaces/${workspaceId}`)}
        >
          Back
        </Button>
        <Typography variant="h4" sx={{ flexGrow: 1 }}>
          Dependency Graph
        </Typography>

        <Tooltip title="Copy to clipboard">
          <IconButton onClick={handleCopyToClipboard}>
            <Iconify icon="solar:copy-bold-duotone" />
          </IconButton>
        </Tooltip>

        <Tooltip title="Export">
          <IconButton onClick={(e) => setAnchorEl(e.currentTarget)}>
            <Iconify icon="solar:export-bold-duotone" />
          </IconButton>
        </Tooltip>

        <Menu
          anchorEl={anchorEl}
          open={Boolean(anchorEl)}
          onClose={() => setAnchorEl(null)}
        >
          <MenuItem onClick={() => handleExport('json')}>
            <Iconify icon="vscode-icons:file-type-json" sx={{ mr: 1 }} />
            Export as JSON
          </MenuItem>
          <MenuItem onClick={() => handleExport('svg')}>
            <Iconify icon="vscode-icons:file-type-svg" sx={{ mr: 1 }} />
            Export as SVG
          </MenuItem>
          <MenuItem onClick={() => handleExport('png')} disabled>
            <Iconify icon="solar:image-bold-duotone" sx={{ mr: 1 }} />
            Export as PNG (Coming Soon)
          </MenuItem>
        </Menu>
      </Stack>

      {/* Stats */}
      {stats && (
        <Stack direction="row" spacing={2} sx={{ mb: 3 }}>
          <Card sx={{ p: 2, minWidth: 150 }}>
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
                <Iconify icon="solar:box-bold-duotone" width={20} sx={{ color: 'primary.main' }} />
              </Box>
              <Box>
                <Typography variant="h4">{stats.nodes}</Typography>
                <Typography variant="caption" color="text.secondary">
                  Nodes
                </Typography>
              </Box>
            </Stack>
          </Card>

          <Card sx={{ p: 2, minWidth: 150 }}>
            <Stack direction="row" alignItems="center" spacing={2}>
              <Box
                sx={{
                  width: 40,
                  height: 40,
                  borderRadius: 1,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  bgcolor: (theme) => alpha(theme.palette.info.main, 0.08),
                }}
              >
                <Iconify icon="solar:link-bold-duotone" width={20} sx={{ color: 'info.main' }} />
              </Box>
              <Box>
                <Typography variant="h4">{stats.edges}</Typography>
                <Typography variant="caption" color="text.secondary">
                  Dependencies
                </Typography>
              </Box>
            </Stack>
          </Card>

          {stats.cycles > 0 && (
            <Card sx={{ p: 2, minWidth: 150, bgcolor: (theme) => alpha(theme.palette.error.main, 0.08) }}>
              <Stack direction="row" alignItems="center" spacing={2}>
                <Box
                  sx={{
                    width: 40,
                    height: 40,
                    borderRadius: 1,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    bgcolor: (theme) => alpha(theme.palette.error.main, 0.16),
                  }}
                >
                  <Iconify icon="solar:danger-bold-duotone" width={20} sx={{ color: 'error.main' }} />
                </Box>
                <Box>
                  <Typography variant="h4" color="error.main">{stats.cycles}</Typography>
                  <Typography variant="caption" color="error.main">
                    Cycles Detected
                  </Typography>
                </Box>
              </Stack>
            </Card>
          )}
        </Stack>
      )}

      {/* Cycle Warning */}
      {dependencies?.cycles && dependencies.cycles.length > 0 && (
        <Alert severity="warning" sx={{ mb: 3 }}>
          <Typography variant="subtitle2">
            {dependencies.cycles.length} circular {dependencies.cycles.length === 1 ? 'dependency' : 'dependencies'} detected
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Circular dependencies can lead to initialization issues and make code harder to maintain.
            Highlighted nodes are part of dependency cycles.
          </Typography>
        </Alert>
      )}

      {/* Controls */}
      <Card sx={{ p: 3, mb: 3 }}>
        <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2} alignItems="center">
          <Typography variant="subtitle2" sx={{ minWidth: 100 }}>
            Format:
          </Typography>
          <ToggleButtonGroup
            value={graphFormat}
            exclusive
            onChange={handleFormatChange}
            size="small"
          >
            <ToggleButton value="mermaid">
              <Iconify icon="solar:diagram-bold-duotone" sx={{ mr: 0.5 }} />
              Mermaid
            </ToggleButton>
            <ToggleButton value="json">
              <Iconify icon="vscode-icons:file-type-json" sx={{ mr: 0.5 }} />
              JSON
            </ToggleButton>
            <ToggleButton value="dot">
              <Iconify icon="solar:code-bold-duotone" sx={{ mr: 0.5 }} />
              DOT
            </ToggleButton>
          </ToggleButtonGroup>

          {graphFormat === 'mermaid' && (
            <>
              <Divider orientation="vertical" flexItem />
              <Typography variant="subtitle2" sx={{ minWidth: 100 }}>
                Layout:
              </Typography>
              <ToggleButtonGroup
                value={layoutDirection}
                exclusive
                onChange={handleDirectionChange}
                size="small"
              >
                <ToggleButton value="TB">
                  <Tooltip title="Top to Bottom">
                    <Iconify icon="solar:arrow-down-bold-duotone" />
                  </Tooltip>
                </ToggleButton>
                <ToggleButton value="LR">
                  <Tooltip title="Left to Right">
                    <Iconify icon="solar:arrow-right-bold-duotone" />
                  </Tooltip>
                </ToggleButton>
                <ToggleButton value="BT">
                  <Tooltip title="Bottom to Top">
                    <Iconify icon="solar:arrow-up-bold-duotone" />
                  </Tooltip>
                </ToggleButton>
                <ToggleButton value="RL">
                  <Tooltip title="Right to Left">
                    <Iconify icon="solar:arrow-left-bold-duotone" />
                  </Tooltip>
                </ToggleButton>
              </ToggleButtonGroup>
            </>
          )}
        </Stack>
      </Card>

      {/* Graph Display */}
      <Card sx={{ p: 3 }}>
        {isLoading ? (
          <Box sx={{ py: 10 }}>
            <LinearProgress />
            <Typography align="center" sx={{ mt: 2 }}>
              Loading dependencies...
            </Typography>
          </Box>
        ) : error ? (
          <Alert severity="error">
            Failed to load dependencies: {error.message}
          </Alert>
        ) : !dependencies ? (
          <Box sx={{ py: 10, textAlign: 'center' }}>
            <Iconify icon="solar:diagram-bold-duotone" width={64} sx={{ mb: 2, opacity: 0.5 }} />
            <Typography variant="body2" color="text.secondary">
              No dependency data available
            </Typography>
          </Box>
        ) : graphFormat === 'mermaid' ? (
          <Box
            ref={mermaidRef}
            sx={{
              '& svg': {
                maxWidth: '100%',
                height: 'auto',
              },
            }}
          />
        ) : (
          <Box
            component="pre"
            sx={{
              p: 2,
              bgcolor: 'background.neutral',
              borderRadius: 1,
              overflow: 'auto',
              fontFamily: 'monospace',
              fontSize: '0.875rem',
              maxHeight: '70vh',
            }}
          >
            {graphFormat === 'json'
              ? generateJsonGraph(dependencies)
              : generateDotGraph(dependencies)}
          </Box>
        )}
      </Card>
    </Box>
  );
}
