import { useState } from 'react';

import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import Card from '@mui/material/Card';
import Chip from '@mui/material/Chip';
import Alert from '@mui/material/Alert';
import Timeline from '@mui/lab/Timeline';
import TimelineItem from '@mui/lab/TimelineItem';
import TimelineSeparator from '@mui/lab/TimelineSeparator';
import TimelineConnector from '@mui/lab/TimelineConnector';
import TimelineContent from '@mui/lab/TimelineContent';
import TimelineDot from '@mui/lab/TimelineDot';
import TimelineOppositeContent from '@mui/lab/TimelineOppositeContent';

import { fDateTime } from 'src/utils/format-time';

import { Iconify } from 'src/components/iconify';
import { useSnackbar } from 'src/components/snackbar';

import type { DocumentVersion } from 'src/types/cortex';

import { VersionEditorDialog } from './version-editor-dialog';
import { VersionCompareDialog } from './version-compare-dialog';

// ----------------------------------------------------------------------

interface DocumentVersionsPanelProps {
  documentId: string;
  versions: DocumentVersion[];
  onRefresh: () => void;
  currentVersion: string;
}

// ----------------------------------------------------------------------

export function DocumentVersionsPanel({
  documentId,
  versions,
  onRefresh,
  currentVersion,
}: DocumentVersionsPanelProps) {
  const { enqueueSnackbar } = useSnackbar();
  const [editorOpen, setEditorOpen] = useState(false);
  const [compareOpen, setCompareOpen] = useState(false);
  const [selectedVersions, setSelectedVersions] = useState<[string, string] | null>(null);

  // Sort versions by date (newest first)
  const sortedVersions = [...versions].sort(
    (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
  );

  const handleCreateVersion = () => {
    setEditorOpen(true);
  };

  const handleCompareVersions = (versionId: string) => {
    if (sortedVersions.length > 1) {
      const currentIndex = sortedVersions.findIndex((v) => v.id === versionId);
      if (currentIndex < sortedVersions.length - 1) {
        setSelectedVersions([versionId, sortedVersions[currentIndex + 1].id]);
        setCompareOpen(true);
      }
    }
  };

  const handleEditorClose = (saved: boolean) => {
    setEditorOpen(false);
    if (saved) {
      onRefresh();
    }
  };

  const handleCompareClose = () => {
    setCompareOpen(false);
    setSelectedVersions(null);
  };

  const renderVersion = (version: DocumentVersion, index: number) => {
    const isLatest = index === 0;
    const isCurrent = version.version === currentVersion;

    return (
      <TimelineItem key={version.id}>
        <TimelineOppositeContent color="text.secondary" sx={{ flex: 0.2 }}>
          <Typography variant="caption">{fDateTime(version.created_at)}</Typography>
        </TimelineOppositeContent>

        <TimelineSeparator>
          <TimelineDot color={isLatest ? 'primary' : isCurrent ? 'success' : 'grey'}>
            <Iconify
              icon={
                isLatest
                  ? 'eva:star-fill'
                  : isCurrent
                    ? 'eva:checkmark-fill'
                    : 'eva:clock-fill'
              }
            />
          </TimelineDot>
          {index < sortedVersions.length - 1 && <TimelineConnector />}
        </TimelineSeparator>

        <TimelineContent sx={{ flex: 1 }}>
          <Card
            sx={{
              p: 2,
              mb: 2,
              borderLeft: 3,
              borderColor: isLatest ? 'primary.main' : isCurrent ? 'success.main' : 'grey.300',
            }}
          >
            <Stack spacing={1}>
              <Stack direction="row" alignItems="center" spacing={1} flexWrap="wrap">
                <Typography variant="h6">{version.version}</Typography>
                {isLatest && <Chip label="Latest" size="small" color="primary" />}
                {isCurrent && <Chip label="Current" size="small" color="success" />}
              </Stack>

              <Typography variant="body2" color="text.secondary">
                {version.message}
              </Typography>

              {version.author && (
                <Typography variant="caption" color="text.secondary">
                  <strong>Author:</strong> {version.author}
                </Typography>
              )}

              <Stack direction="row" spacing={1} sx={{ mt: 1 }}>
                {version.content_snapshot && (
                  <Button
                    size="small"
                    startIcon={<Iconify icon="eva:eye-fill" />}
                    onClick={() => {
                      enqueueSnackbar('View version feature coming soon', 'info');
                    }}
                  >
                    View
                  </Button>
                )}

                {index < sortedVersions.length - 1 && (
                  <Button
                    size="small"
                    startIcon={<Iconify icon="eva:swap-fill" />}
                    onClick={() => handleCompareVersions(version.id)}
                  >
                    Compare
                  </Button>
                )}

                {!isCurrent && (
                  <Button
                    size="small"
                    startIcon={<Iconify icon="eva:refresh-fill" />}
                    onClick={() => {
                      enqueueSnackbar('Restore version feature coming soon', 'info');
                    }}
                  >
                    Restore
                  </Button>
                )}
              </Stack>
            </Stack>
          </Card>
        </TimelineContent>
      </TimelineItem>
    );
  };

  return (
    <Box>
      <Box sx={{ mb: 3 }}>
        <Stack direction="row" spacing={2} alignItems="center">
          <Button
            variant="contained"
            startIcon={<Iconify icon="eva:plus-fill" />}
            onClick={handleCreateVersion}
          >
            Create Version
          </Button>

          <Alert severity="info" sx={{ flex: 1 }}>
            Versions allow you to track changes and restore previous states of the document.
          </Alert>
        </Stack>
      </Box>

      {versions.length === 0 ? (
        <Alert severity="info">
          No versions found. Click "Create Version" to save the current state.
        </Alert>
      ) : (
        <Timeline position="right">
          {sortedVersions.map((version, index) => renderVersion(version, index))}
        </Timeline>
      )}

      <VersionEditorDialog
        open={editorOpen}
        documentId={documentId}
        onClose={handleEditorClose}
      />

      {selectedVersions && (
        <VersionCompareDialog
          open={compareOpen}
          documentId={documentId}
          versionIds={selectedVersions}
          onClose={handleCompareClose}
        />
      )}
    </Box>
  );
}
