import { useState, useEffect } from 'react';

import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogContent from '@mui/material/DialogContent';
import DialogActions from '@mui/material/DialogActions';
import Button from '@mui/material/Button';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Stack from '@mui/material/Stack';
import Chip from '@mui/material/Chip';
import Alert from '@mui/material/Alert';
import CircularProgress from '@mui/material/CircularProgress';

import { fDateTime } from 'src/utils/format-time';

import { useSnackbar } from 'src/components/snackbar';

import useSWR from 'swr';
import { cortexFetcher } from 'src/lib/cortex-client';
import type { DocumentVersion } from 'src/types/cortex';

// ----------------------------------------------------------------------

interface VersionCompareDialogProps {
  open: boolean;
  documentId: string;
  versionIds: [string, string];
  onClose: () => void;
}

// ----------------------------------------------------------------------

export function VersionCompareDialog({
  open,
  documentId,
  versionIds,
  onClose,
}: VersionCompareDialogProps) {
  const { enqueueSnackbar } = useSnackbar();

  // Fetch both versions
  const { data: versions, isLoading } = useSWR<DocumentVersion[]>(
    open ? `/api/v1/documents/${documentId}/versions` : null,
    cortexFetcher
  );

  const version1 = versions?.find((v) => v.id === versionIds[0]);
  const version2 = versions?.find((v) => v.id === versionIds[1]);

  const renderVersionInfo = (version: DocumentVersion | undefined, label: string) => {
    if (!version) {
      return (
        <Box>
          <Typography variant="subtitle2" color="text.secondary">
            {label}
          </Typography>
          <Alert severity="error">Version not found</Alert>
        </Box>
      );
    }

    return (
      <Box>
        <Stack direction="row" spacing={1} alignItems="center" sx={{ mb: 1 }}>
          <Typography variant="subtitle2">{label}</Typography>
          <Chip label={version.version} size="small" color="primary" />
        </Stack>

        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1 }}>
          <strong>Created:</strong> {fDateTime(version.created_at)}
        </Typography>

        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1 }}>
          <strong>Author:</strong> {version.author}
        </Typography>

        <Typography variant="body2" sx={{ mb: 2 }}>
          {version.message}
        </Typography>

        {version.content_snapshot && (
          <Box
            sx={{
              p: 2,
              bgcolor: 'background.neutral',
              borderRadius: 1,
              maxHeight: 300,
              overflow: 'auto',
            }}
          >
            <Typography variant="caption" component="pre" sx={{ whiteSpace: 'pre-wrap' }}>
              {version.content_snapshot.substring(0, 500)}
              {version.content_snapshot.length > 500 ? '...' : ''}
            </Typography>
          </Box>
        )}
      </Box>
    );
  };

  return (
    <Dialog open={open} onClose={onClose} maxWidth="lg" fullWidth>
      <DialogTitle>Compare Versions</DialogTitle>

      <DialogContent>
        {isLoading ? (
          <Box sx={{ display: 'flex', justifyContent: 'center', py: 4 }}>
            <CircularProgress />
          </Box>
        ) : (
          <Box>
            <Alert severity="info" sx={{ mb: 3 }}>
              Comparing two versions. Full diff functionality coming soon.
            </Alert>

            <Stack direction="row" spacing={3}>
              <Box sx={{ flex: 1 }}>
                {renderVersionInfo(version1, 'Newer Version')}
              </Box>

              <Box sx={{ flex: 1 }}>
                {renderVersionInfo(version2, 'Older Version')}
              </Box>
            </Stack>
          </Box>
        )}
      </DialogContent>

      <DialogActions>
        <Button onClick={onClose}>Close</Button>
      </DialogActions>
    </Dialog>
  );
}
