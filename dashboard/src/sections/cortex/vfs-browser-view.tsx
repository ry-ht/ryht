import type { VfsEntry, DirectoryListing } from 'src/types/cortex';

import useSWR, { mutate } from 'swr';
import { useParams } from 'react-router';
import { useMemo, useState, useCallback } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Link from '@mui/material/Link';
import List from '@mui/material/List';
import Chip from '@mui/material/Chip';
import Menu from '@mui/material/Menu';
import Stack from '@mui/material/Stack';
import Dialog from '@mui/material/Dialog';
import Select from '@mui/material/Select';
import Button from '@mui/material/Button';
import Divider from '@mui/material/Divider';
import Tooltip from '@mui/material/Tooltip';
import ListItem from '@mui/material/ListItem';
import MenuItem from '@mui/material/MenuItem';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import InputLabel from '@mui/material/InputLabel';
import Breadcrumbs from '@mui/material/Breadcrumbs';
import DialogTitle from '@mui/material/DialogTitle';
import FormControl from '@mui/material/FormControl';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import ToggleButton from '@mui/material/ToggleButton';
import DialogContent from '@mui/material/DialogContent';
import DialogActions from '@mui/material/DialogActions';
import ListItemButton from '@mui/material/ListItemButton';
import InputAdornment from '@mui/material/InputAdornment';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';

import { fData } from 'src/utils/format-number';
import { fDateTime } from 'src/utils/format-time';

import { cortexClient, cortexFetcher, cortexEndpoints } from 'src/lib/cortex-client';

import { Iconify } from 'src/components/iconify';
import { Markdown } from 'src/components/markdown';
import { useSnackbar } from 'src/components/snackbar';

// ----------------------------------------------------------------------

type ViewMode = 'list' | 'grid';
type FileTypeFilter = 'all' | 'code' | 'document' | 'binary' | 'directory';

const FILE_TYPE_FILTERS: { value: FileTypeFilter; label: string }[] = [
  { value: 'all', label: 'All Files' },
  { value: 'code', label: 'Code Files' },
  { value: 'document', label: 'Documents' },
  { value: 'binary', label: 'Binary Files' },
  { value: 'directory', label: 'Directories' },
];

const CODE_EXTENSIONS = ['ts', 'tsx', 'js', 'jsx', 'rs', 'py', 'go', 'java', 'c', 'cpp', 'h', 'hpp', 'cs', 'rb', 'php'];
const DOCUMENT_EXTENSIONS = ['md', 'txt', 'pdf', 'doc', 'docx', 'rtf'];
const BINARY_EXTENSIONS = ['png', 'jpg', 'jpeg', 'gif', 'svg', 'ico', 'woff', 'woff2', 'ttf', 'eot'];

// ----------------------------------------------------------------------

export function VfsBrowserView() {
  const params = useParams();
  const workspaceId = params.id as string;
  const { enqueueSnackbar } = useSnackbar();

  const [currentPath, setCurrentPath] = useState('/');
  const [selectedFile, setSelectedFile] = useState<VfsEntry | null>(null);
  const [fileContent, setFileContent] = useState<string | null>(null);
  const [isViewerOpen, setIsViewerOpen] = useState(false);
  const [viewMode, setViewMode] = useState<ViewMode>('list');
  const [searchQuery, setSearchQuery] = useState('');
  const [fileTypeFilter, setFileTypeFilter] = useState<FileTypeFilter>('all');
  const [anchorEl, setAnchorEl] = useState<null | HTMLElement>(null);
  const [contextMenuEntry, setContextMenuEntry] = useState<VfsEntry | null>(null);

  // Fetch directory listing
  const { data: listing, isLoading } = useSWR<DirectoryListing>(
    workspaceId ? cortexEndpoints.vfs.list(workspaceId, currentPath) : null,
    cortexFetcher,
    { refreshInterval: 10000 }
  );

  const pathSegments = currentPath.split('/').filter(Boolean);

  // ----------------------------------------------------------------------
  // File Type Detection
  // ----------------------------------------------------------------------

  const getFileExtension = (filename: string) => filename.split('.').pop()?.toLowerCase() || '';

  const getFileCategory = (entry: VfsEntry): FileTypeFilter => {
    if (entry.file_type === 'directory') return 'directory';

    const ext = getFileExtension(entry.name);
    if (CODE_EXTENSIONS.includes(ext)) return 'code';
    if (DOCUMENT_EXTENSIONS.includes(ext)) return 'document';
    if (BINARY_EXTENSIONS.includes(ext)) return 'binary';

    return 'all';
  };

  const getLanguageFromExtension = (ext: string): string | null => {
    const langMap: Record<string, string> = {
      ts: 'TypeScript',
      tsx: 'TypeScript',
      js: 'JavaScript',
      jsx: 'JavaScript',
      rs: 'Rust',
      py: 'Python',
      go: 'Go',
      java: 'Java',
      c: 'C',
      cpp: 'C++',
      h: 'C/C++',
      hpp: 'C++',
      cs: 'C#',
      rb: 'Ruby',
      php: 'PHP',
      md: 'Markdown',
    };
    return langMap[ext] || null;
  };

  // ----------------------------------------------------------------------
  // Filtering & Search
  // ----------------------------------------------------------------------

  const filteredEntries = useMemo(() => {
    if (!listing?.entries) return [];

    let filtered = listing.entries;

    // Apply file type filter
    if (fileTypeFilter !== 'all') {
      filtered = filtered.filter((entry) => getFileCategory(entry) === fileTypeFilter);
    }

    // Apply search query
    if (searchQuery) {
      filtered = filtered.filter((entry) =>
        entry.name.toLowerCase().includes(searchQuery.toLowerCase())
      );
    }

    // Sort: directories first, then alphabetically
    filtered.sort((a, b) => {
      if (a.file_type === 'directory' && b.file_type !== 'directory') return -1;
      if (a.file_type !== 'directory' && b.file_type === 'directory') return 1;
      return a.name.localeCompare(b.name);
    });

    return filtered;
  }, [listing, fileTypeFilter, searchQuery]);

  // ----------------------------------------------------------------------
  // File Stats
  // ----------------------------------------------------------------------

  const fileStats = useMemo(() => {
    if (!listing?.entries) return null;

    const stats = {
      total: listing.entries.length,
      directories: 0,
      files: 0,
      totalSize: 0,
    };

    listing.entries.forEach((entry) => {
      if (entry.file_type === 'directory') {
        stats.directories += 1;
      } else {
        stats.files += 1;
        stats.totalSize += entry.size;
      }
    });

    return stats;
  }, [listing]);

  // ----------------------------------------------------------------------
  // Handlers
  // ----------------------------------------------------------------------

  const handleNavigate = useCallback((path: string) => {
    setCurrentPath(path);
    setSearchQuery(''); // Reset search when navigating
  }, []);

  const handleBreadcrumbClick = useCallback((index: number) => {
    if (index === -1) {
      setCurrentPath('/');
    } else {
      const newPath = '/' + pathSegments.slice(0, index + 1).join('/');
      setCurrentPath(newPath);
    }
  }, [pathSegments]);

  const handleFileClick = useCallback(
    async (entry: VfsEntry) => {
      if (entry.file_type === 'directory') {
        const newPath = currentPath === '/' ? `/${entry.name}` : `${currentPath}/${entry.name}`;
        handleNavigate(newPath);
      } else {
        setSelectedFile(entry);
        setIsViewerOpen(true);

        // Load file content
        try {
          const fullPath = currentPath === '/' ? `/${entry.name}` : `${currentPath}/${entry.name}`;
          const content = await cortexClient.readFileContent(workspaceId, fullPath);
          setFileContent(content);
        } catch (err) {
          setFileContent(`Error loading file: ${err}`);
        }
      }
    },
    [workspaceId, currentPath, handleNavigate]
  );

  const handleContextMenu = useCallback((event: React.MouseEvent, entry: VfsEntry) => {
    event.preventDefault();
    setContextMenuEntry(entry);
    setAnchorEl(event.currentTarget as HTMLElement);
  }, []);

  const handleCloseContextMenu = useCallback(() => {
    setAnchorEl(null);
    setContextMenuEntry(null);
  }, []);

  const handleDeleteFile = useCallback(async () => {
    if (!contextMenuEntry) return;

    try {
      const fullPath = currentPath === '/' ? `/${contextMenuEntry.name}` : `${currentPath}/${contextMenuEntry.name}`;
      await cortexClient.deleteFile(workspaceId, fullPath);
      mutate(cortexEndpoints.vfs.list(workspaceId, currentPath));
      enqueueSnackbar('File deleted successfully', 'success');
    } catch (err) {
      enqueueSnackbar('Failed to delete file', 'error');
    } finally {
      handleCloseContextMenu();
    }
  }, [contextMenuEntry, workspaceId, currentPath, enqueueSnackbar, handleCloseContextMenu]);

  const handleDownloadFile = useCallback(async () => {
    if (!contextMenuEntry || contextMenuEntry.file_type === 'directory') return;

    try {
      const fullPath = currentPath === '/' ? `/${contextMenuEntry.name}` : `${currentPath}/${contextMenuEntry.name}`;
      const content = await cortexClient.readFileContent(workspaceId, fullPath);

      // Create blob and download
      const blob = new Blob([content], { type: 'text/plain' });
      const url = window.URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = contextMenuEntry.name;
      a.click();
      window.URL.revokeObjectURL(url);

      enqueueSnackbar('File downloaded', 'success');
    } catch (err) {
      enqueueSnackbar('Failed to download file', 'error');
    } finally {
      handleCloseContextMenu();
    }
  }, [contextMenuEntry, workspaceId, currentPath, enqueueSnackbar, handleCloseContextMenu]);

  const handleCloseViewer = useCallback(() => {
    setIsViewerOpen(false);
    setSelectedFile(null);
    setFileContent(null);
  }, []);

  const handleViewModeChange = useCallback((_event: React.MouseEvent<HTMLElement>, newMode: ViewMode | null) => {
    if (newMode !== null) {
      setViewMode(newMode);
    }
  }, []);

  const getFileIcon = (entry: VfsEntry) => {
    if (entry.file_type === 'directory') {
      return 'solar:folder-bold-duotone';
    }

    const ext = getFileExtension(entry.name);
    const iconMap: Record<string, string> = {
      ts: 'vscode-icons:file-type-typescript',
      tsx: 'vscode-icons:file-type-typescript',
      js: 'vscode-icons:file-type-js',
      jsx: 'vscode-icons:file-type-js',
      rs: 'vscode-icons:file-type-rust',
      py: 'vscode-icons:file-type-python',
      json: 'vscode-icons:file-type-json',
      md: 'vscode-icons:file-type-markdown',
      txt: 'solar:document-text-bold-duotone',
      go: 'vscode-icons:file-type-go',
      java: 'vscode-icons:file-type-java',
      c: 'vscode-icons:file-type-c',
      cpp: 'vscode-icons:file-type-cpp',
      h: 'vscode-icons:file-type-c',
      cs: 'vscode-icons:file-type-csharp',
      rb: 'vscode-icons:file-type-ruby',
      php: 'vscode-icons:file-type-php',
      pdf: 'vscode-icons:file-type-pdf',
      png: 'vscode-icons:file-type-image',
      jpg: 'vscode-icons:file-type-image',
      svg: 'vscode-icons:file-type-svg',
    };

    return iconMap[ext] || 'solar:file-bold-duotone';
  };

  const isMarkdown = (filename: string) => filename.endsWith('.md') || filename.endsWith('.markdown');

  const isTextFile = (entry: VfsEntry) => {
    const ext = getFileExtension(entry.name);
    return CODE_EXTENSIONS.includes(ext) || DOCUMENT_EXTENSIONS.includes(ext) || ext === 'txt';
  };

  // ----------------------------------------------------------------------
  // Render Functions
  // ----------------------------------------------------------------------

  const renderFileList = () => (
    <List>
      {filteredEntries.map((entry) => (
        <ListItem
          key={entry.id}
          disablePadding
          secondaryAction={
            <Stack direction="row" spacing={1}>
              {entry.file_type !== 'directory' && (
                <Tooltip title={fData(entry.size)}>
                  <Chip label={fData(entry.size)} size="small" variant="outlined" />
                </Tooltip>
              )}
              <IconButton
                edge="end"
                onClick={(e) => handleContextMenu(e, entry)}
              >
                <Iconify icon="eva:more-vertical-fill" />
              </IconButton>
            </Stack>
          }
        >
          <ListItemButton onClick={() => handleFileClick(entry)}>
            <ListItemIcon>
              <Iconify icon={getFileIcon(entry)} width={24} />
            </ListItemIcon>
            <ListItemText
              primary={
                <Stack direction="row" spacing={1} alignItems="center">
                  <Typography variant="body2">{entry.name}</Typography>
                  {entry.file_type !== 'directory' && getLanguageFromExtension(getFileExtension(entry.name)) && (
                    <Chip
                      label={getLanguageFromExtension(getFileExtension(entry.name))}
                      size="small"
                      color="primary"
                      variant="outlined"
                      sx={{ height: 20 }}
                    />
                  )}
                </Stack>
              }
              secondary={
                <Stack direction="row" spacing={1} alignItems="center">
                  <Typography variant="caption" color="text.secondary">
                    {fDateTime(entry.updated_at)}
                  </Typography>
                  {entry.metadata && Object.keys(entry.metadata).length > 0 && (
                    <>
                      <Typography variant="caption">•</Typography>
                      <Typography variant="caption" color="text.secondary">
                        {Object.keys(entry.metadata).length} metadata
                      </Typography>
                    </>
                  )}
                </Stack>
              }
            />
            {entry.file_type === 'directory' && (
              <Iconify icon="eva:arrow-ios-forward-fill" />
            )}
          </ListItemButton>
        </ListItem>
      ))}
    </List>
  );

  const renderFileGrid = () => (
    <Box sx={{ display: "flex", gap: 2, flexWrap: "wrap", mt: 1 }}>
      {filteredEntries.map((entry) => (
        <Box sx={{ flex: "1 1 auto", minWidth: { xs: "100%", sm: "calc(50% - 8px)", md: "calc(33.33% - 8px)", lg: "calc(25% - 8px)" } }} key={entry.id}>
          <Card
            sx={{
              p: 2,
              cursor: 'pointer',
              '&:hover': {
                bgcolor: 'action.hover',
              },
            }}
            onClick={() => handleFileClick(entry)}
            onContextMenu={(e) => handleContextMenu(e, entry)}
          >
            <Stack spacing={1} alignItems="center">
              <Iconify icon={getFileIcon(entry)} width={48} />
              <Typography
                variant="body2"
                align="center"
                sx={{
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                  width: '100%',
                }}
              >
                {entry.name}
              </Typography>
              {entry.file_type !== 'directory' && (
                <Typography variant="caption" color="text.secondary">
                  {fData(entry.size)}
                </Typography>
              )}
              {entry.file_type !== 'directory' && getLanguageFromExtension(getFileExtension(entry.name)) && (
                <Chip
                  label={getLanguageFromExtension(getFileExtension(entry.name))}
                  size="small"
                  color="primary"
                  variant="outlined"
                />
              )}
            </Stack>
          </Card>
        </Box>
      ))}
    </Box>
  );

  // ----------------------------------------------------------------------

  return (
    <Box sx={{ p: 3 }}>
      <Stack direction="row" alignItems="center" spacing={2} sx={{ mb: 3 }}>
        <Typography variant="h4" sx={{ flexGrow: 1 }}>
          File Browser
        </Typography>

        <ToggleButtonGroup
          value={viewMode}
          exclusive
          onChange={handleViewModeChange}
          size="small"
        >
          <ToggleButton value="list">
            <Iconify icon="solar:list-bold-duotone" />
          </ToggleButton>
          <ToggleButton value="grid">
            <Iconify icon="solar:grid-bold-duotone" />
          </ToggleButton>
        </ToggleButtonGroup>
      </Stack>

      <Card sx={{ p: 3 }}>
        {/* Breadcrumbs */}
        <Breadcrumbs sx={{ mb: 2 }}>
          <Link
            component="button"
            variant="body1"
            onClick={() => handleBreadcrumbClick(-1)}
            sx={{ cursor: 'pointer', textDecoration: 'none', display: 'flex', alignItems: 'center' }}
          >
            <Iconify icon="solar:home-bold-duotone" sx={{ mr: 0.5 }} />
            Root
          </Link>
          {pathSegments.map((segment, index) => (
            <Link
              key={index}
              component="button"
              variant="body1"
              onClick={() => handleBreadcrumbClick(index)}
              sx={{ cursor: 'pointer', textDecoration: 'none' }}
            >
              {segment}
            </Link>
          ))}
        </Breadcrumbs>

        <Divider sx={{ mb: 2 }} />

        {/* Filters and Search */}
        <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2} sx={{ mb: 3 }}>
          <TextField
            fullWidth
            placeholder="Search files..."
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
            <InputLabel>File Type</InputLabel>
            <Select
              value={fileTypeFilter}
              label="File Type"
              onChange={(e) => setFileTypeFilter(e.target.value as FileTypeFilter)}
            >
              {FILE_TYPE_FILTERS.map((filter) => (
                <MenuItem key={filter.value} value={filter.value}>
                  {filter.label}
                </MenuItem>
              ))}
            </Select>
          </FormControl>
        </Stack>

        {/* File Stats */}
        {fileStats && (
          <Stack direction="row" spacing={2} sx={{ mb: 2 }}>
            <Chip
              icon={<Iconify icon="solar:folder-bold-duotone" />}
              label={`${fileStats.directories} directories`}
              size="small"
            />
            <Chip
              icon={<Iconify icon="solar:file-bold-duotone" />}
              label={`${fileStats.files} files`}
              size="small"
            />
            {fileStats.totalSize > 0 && (
              <Chip
                icon={<Iconify icon="solar:database-bold-duotone" />}
                label={fData(fileStats.totalSize)}
                size="small"
              />
            )}
          </Stack>
        )}

        {/* File List/Grid */}
        {isLoading ? (
          <Typography align="center" sx={{ py: 10 }}>Loading...</Typography>
        ) : filteredEntries.length > 0 ? (
          viewMode === 'list' ? renderFileList() : renderFileGrid()
        ) : (
          <Box sx={{ py: 10, textAlign: 'center' }}>
            <Typography variant="body2" color="text.secondary">
              {searchQuery || fileTypeFilter !== 'all' ? 'No files match your filters' : 'Empty directory'}
            </Typography>
          </Box>
        )}

        {listing && (
          <Box sx={{ mt: 2, display: 'flex', justifyContent: 'space-between' }}>
            <Typography variant="caption" color="text.secondary">
              {filteredEntries.length} of {listing.total_count} items
            </Typography>
            <Typography variant="caption" color="text.secondary">
              {currentPath}
            </Typography>
          </Box>
        )}
      </Card>

      {/* Context Menu */}
      <Menu
        anchorEl={anchorEl}
        open={Boolean(anchorEl)}
        onClose={handleCloseContextMenu}
      >
        <MenuItem
          onClick={() => {
            if (contextMenuEntry) handleFileClick(contextMenuEntry);
            handleCloseContextMenu();
          }}
        >
          <Iconify icon="solar:eye-bold-duotone" sx={{ mr: 1 }} />
          {contextMenuEntry?.file_type === 'directory' ? 'Open' : 'View'}
        </MenuItem>
        {contextMenuEntry?.file_type !== 'directory' && contextMenuEntry && isTextFile(contextMenuEntry) && (
          <MenuItem onClick={handleDownloadFile}>
            <Iconify icon="solar:download-bold-duotone" sx={{ mr: 1 }} />
            Download
          </MenuItem>
        )}
        <Divider />
        <MenuItem onClick={handleDeleteFile} sx={{ color: 'error.main' }}>
          <Iconify icon="solar:trash-bin-trash-bold-duotone" sx={{ mr: 1 }} />
          Delete
        </MenuItem>
      </Menu>

      {/* File Viewer Dialog */}
      <Dialog
        open={isViewerOpen}
        onClose={handleCloseViewer}
        maxWidth="lg"
        fullWidth
        PaperProps={{ sx: { height: '80vh' } }}
      >
        <DialogTitle>
          <Stack direction="row" alignItems="center" justifyContent="space-between">
            <Stack direction="row" alignItems="center" spacing={1}>
              <Iconify icon={selectedFile ? getFileIcon(selectedFile) : ''} width={24} />
              <Typography variant="h6">{selectedFile?.name}</Typography>
            </Stack>
            <Stack direction="row" spacing={1}>
              {selectedFile && (
                <>
                  <Chip label={fData(selectedFile.size)} size="small" />
                  {getLanguageFromExtension(getFileExtension(selectedFile.name)) && (
                    <Chip
                      label={getLanguageFromExtension(getFileExtension(selectedFile.name))}
                      size="small"
                      color="primary"
                    />
                  )}
                </>
              )}
              <IconButton onClick={handleCloseViewer}>
                <Iconify icon="eva:close-fill" />
              </IconButton>
            </Stack>
          </Stack>
        </DialogTitle>

        <DialogContent dividers sx={{ p: 3 }}>
          {fileContent === null ? (
            <Typography>Loading...</Typography>
          ) : selectedFile && isMarkdown(selectedFile.name) ? (
            <Markdown content={fileContent} />
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
              }}
            >
              {fileContent}
            </Box>
          )}
        </DialogContent>

        <DialogActions>
          <Button onClick={handleCloseViewer}>Close</Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}
