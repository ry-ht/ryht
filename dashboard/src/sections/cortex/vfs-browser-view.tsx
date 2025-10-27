import { useState, useCallback } from 'react';
import { useParams } from 'react-router';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Typography from '@mui/material/Typography';
import Stack from '@mui/material/Stack';
import Breadcrumbs from '@mui/material/Breadcrumbs';
import Link from '@mui/material/Link';
import List from '@mui/material/List';
import ListItem from '@mui/material/ListItem';
import ListItemButton from '@mui/material/ListItemButton';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import IconButton from '@mui/material/IconButton';
import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogContent from '@mui/material/DialogContent';
import Chip from '@mui/material/Chip';

import { fData } from 'src/utils/format-number';
import { fDateTime } from 'src/utils/format-time';

import { Iconify } from 'src/components/iconify';
import { Markdown } from 'src/components/markdown';

import useSWR from 'swr';
import { cortexClient, cortexFetcher, cortexEndpoints } from 'src/lib/cortex-client';
import type { DirectoryListing, VfsEntry } from 'src/types/cortex';

// ----------------------------------------------------------------------

export function VfsBrowserView() {
  const params = useParams();
  const workspaceId = params.id as string;

  const [currentPath, setCurrentPath] = useState('/');
  const [selectedFile, setSelectedFile] = useState<VfsEntry | null>(null);
  const [fileContent, setFileContent] = useState<string | null>(null);
  const [isViewerOpen, setIsViewerOpen] = useState(false);

  // Fetch directory listing
  const { data: listing, isLoading } = useSWR<DirectoryListing>(
    workspaceId ? cortexEndpoints.vfs.list(workspaceId, currentPath) : null,
    cortexFetcher,
    { refreshInterval: 10000 }
  );

  const pathSegments = currentPath.split('/').filter(Boolean);

  // ----------------------------------------------------------------------
  // Handlers
  // ----------------------------------------------------------------------

  const handleNavigate = useCallback((path: string) => {
    setCurrentPath(path);
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

  const handleCloseViewer = useCallback(() => {
    setIsViewerOpen(false);
    setSelectedFile(null);
    setFileContent(null);
  }, []);

  const getFileIcon = (entry: VfsEntry) => {
    if (entry.file_type === 'directory') {
      return 'solar:folder-bold-duotone';
    }

    const ext = entry.name.split('.').pop()?.toLowerCase();
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
    };

    return iconMap[ext || ''] || 'solar:file-bold-duotone';
  };

  const isMarkdown = (filename: string) => {
    return filename.endsWith('.md') || filename.endsWith('.markdown');
  };

  // ----------------------------------------------------------------------

  return (
    <Box sx={{ p: 3 }}>
      <Stack direction="row" alignItems="center" spacing={2} sx={{ mb: 3 }}>
        <Typography variant="h4" sx={{ flexGrow: 1 }}>
          File Browser
        </Typography>
      </Stack>

      <Card sx={{ p: 3 }}>
        {/* Breadcrumbs */}
        <Breadcrumbs sx={{ mb: 2 }}>
          <Link
            component="button"
            variant="body1"
            onClick={() => handleBreadcrumbClick(-1)}
            sx={{ cursor: 'pointer', textDecoration: 'none' }}
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

        {/* File List */}
        {isLoading ? (
          <Typography>Loading...</Typography>
        ) : listing && listing.entries.length > 0 ? (
          <List>
            {listing.entries.map((entry) => (
              <ListItem key={entry.id} disablePadding>
                <ListItemButton onClick={() => handleFileClick(entry)}>
                  <ListItemIcon>
                    <Iconify icon={getFileIcon(entry)} width={24} />
                  </ListItemIcon>
                  <ListItemText
                    primary={entry.name}
                    secondary={
                      <Stack direction="row" spacing={1} alignItems="center">
                        {entry.file_type !== 'directory' && (
                          <>
                            <Typography variant="caption">{fData(entry.size)}</Typography>
                            <Typography variant="caption">•</Typography>
                          </>
                        )}
                        <Typography variant="caption">{fDateTime(entry.updated_at)}</Typography>
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
        ) : (
          <Box sx={{ py: 10, textAlign: 'center' }}>
            <Typography variant="body2" color="text.secondary">
              Empty directory
            </Typography>
          </Box>
        )}

        {listing && (
          <Box sx={{ mt: 2, display: 'flex', justifyContent: 'space-between' }}>
            <Typography variant="caption" color="text.secondary">
              {listing.total_count} items
            </Typography>
            <Typography variant="caption" color="text.secondary">
              {currentPath}
            </Typography>
          </Box>
        )}
      </Card>

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
                <Chip label={fData(selectedFile.size)} size="small" />
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
      </Dialog>
    </Box>
  );
}
