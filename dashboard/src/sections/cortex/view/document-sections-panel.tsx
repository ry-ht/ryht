import type { DocumentSection } from 'src/types/cortex';

import { useState } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Chip from '@mui/material/Chip';
import Stack from '@mui/material/Stack';
import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';

import { cortexClient } from 'src/lib/cortex-client';

import { Iconify } from 'src/components/iconify';
import { Markdown } from 'src/components/markdown';
import { useSnackbar } from 'src/components/snackbar';

import { SectionEditorDialog } from './section-editor-dialog';

// ----------------------------------------------------------------------

interface DocumentSectionsPanelProps {
  documentId: string;
  sections: DocumentSection[];
  onRefresh: () => void;
  editMode: boolean;
}

interface SectionNode extends DocumentSection {
  children: SectionNode[];
}

// ----------------------------------------------------------------------

export function DocumentSectionsPanel({
  documentId,
  sections,
  onRefresh,
  editMode,
}: DocumentSectionsPanelProps) {
  const { enqueueSnackbar } = useSnackbar();
  const [editorOpen, setEditorOpen] = useState(false);
  const [editingSection, setEditingSection] = useState<DocumentSection | null>(null);

  // Build hierarchical tree
  const buildTree = (items: DocumentSection[]): SectionNode[] => {
    const itemMap: Record<string, SectionNode> = {};
    const rootItems: SectionNode[] = [];

    // Create nodes
    items.forEach((item) => {
      itemMap[item.id] = { ...item, children: [] };
    });

    // Build hierarchy
    items.forEach((item) => {
      const node = itemMap[item.id];
      if (item.parent_section_id && itemMap[item.parent_section_id]) {
        itemMap[item.parent_section_id].children.push(node);
      } else {
        rootItems.push(node);
      }
    });

    // Sort by order
    const sortByOrder = (nodes: SectionNode[]) => {
      nodes.sort((a, b) => a.order - b.order);
      nodes.forEach((node) => sortByOrder(node.children));
    };
    sortByOrder(rootItems);

    return rootItems;
  };

  const sectionTree = buildTree(sections);

  const handleCreateSection = () => {
    setEditingSection(null);
    setEditorOpen(true);
  };

  const handleEditSection = (section: DocumentSection) => {
    setEditingSection(section);
    setEditorOpen(true);
  };

  const handleDeleteSection = async (sectionId: string) => {
    if (!window.confirm('Are you sure you want to delete this section?')) {
      return;
    }

    try {
      await cortexClient.deleteSection(sectionId);
      enqueueSnackbar('Section deleted', 'success');
      onRefresh();
    } catch (error) {
      enqueueSnackbar('Failed to delete section', 'error');
    }
  };

  const handleMoveSection = async (sectionId: string, direction: 'up' | 'down') => {
    const section = sections.find((s) => s.id === sectionId);
    if (!section) return;

    const newOrder = direction === 'up' ? section.order - 1 : section.order + 1;

    try {
      await cortexClient.updateSection(sectionId, { order: newOrder });
      enqueueSnackbar('Section moved', 'success');
      onRefresh();
    } catch (error) {
      enqueueSnackbar('Failed to move section', 'error');
    }
  };

  const handleEditorClose = (saved: boolean) => {
    setEditorOpen(false);
    setEditingSection(null);
    if (saved) {
      onRefresh();
    }
  };

  const renderSection = (node: SectionNode, depth: number = 0) => {
    const levelColors: Record<number, string> = {
      1: 'primary',
      2: 'secondary',
      3: 'info',
      4: 'success',
      5: 'warning',
      6: 'error',
    };

    return (
      <Box key={node.id}>
        <Card
          sx={{
            p: 2,
            ml: depth * 4,
            mb: 1,
            borderLeft: 3,
            borderColor: `${levelColors[node.level] || 'default'}.main`,
            '&:hover': {
              bgcolor: 'action.hover',
            },
          }}
        >
          <Stack direction="row" alignItems="flex-start" spacing={2}>
            <Box sx={{ flexGrow: 1 }}>
              <Stack direction="row" spacing={1} alignItems="center" sx={{ mb: 1 }}>
                <Chip
                  label={`H${node.level}`}
                  size="small"
                  color={levelColors[node.level] as any}
                />
                <Chip label={`Order: ${node.order}`} size="small" variant="outlined" />
                <Typography variant="h6">{node.title}</Typography>
              </Stack>

              {!editMode && (
                <Box
                  sx={{
                    '& p': { mb: 1 },
                    '& pre': {
                      borderRadius: 1,
                      p: 1,
                      bgcolor: 'background.neutral',
                    },
                  }}
                >
                  <Markdown content={node.content} />
                </Box>
              )}

              {editMode && (
                <Typography variant="body2" color="text.secondary">
                  {node.content.substring(0, 200)}
                  {node.content.length > 200 ? '...' : ''}
                </Typography>
              )}
            </Box>

            {editMode && (
              <Stack direction="row" spacing={0.5}>
                <IconButton
                  size="small"
                  onClick={() => handleMoveSection(node.id, 'up')}
                  disabled={node.order === 0}
                >
                  <Iconify icon="eva:arrow-up-fill" />
                </IconButton>

                <IconButton
                  size="small"
                  onClick={() => handleMoveSection(node.id, 'down')}
                >
                  <Iconify icon="eva:arrow-down-fill" />
                </IconButton>

                <IconButton
                  size="small"
                  onClick={() => handleEditSection(node)}
                  color="primary"
                >
                  <Iconify icon="solar:pen-bold" />
                </IconButton>

                <IconButton
                  size="small"
                  onClick={() => handleDeleteSection(node.id)}
                  color="error"
                >
                  <Iconify icon="solar:trash-bin-trash-bold" />
                </IconButton>
              </Stack>
            )}
          </Stack>
        </Card>

        {node.children.map((child) => renderSection(child, depth + 1))}
      </Box>
    );
  };

  return (
    <Box>
      {editMode && (
        <Box sx={{ mb: 3 }}>
          <Button
            variant="contained"
            startIcon={<Iconify icon="eva:plus-fill" />}
            onClick={handleCreateSection}
          >
            Add Section
          </Button>
        </Box>
      )}

      {sections.length === 0 ? (
        <Alert severity="info">
          No sections found. {editMode && 'Click "Add Section" to create one.'}
        </Alert>
      ) : (
        <Box>{sectionTree.map((node) => renderSection(node))}</Box>
      )}

      <SectionEditorDialog
        open={editorOpen}
        documentId={documentId}
        section={editingSection}
        sections={sections}
        onClose={handleEditorClose}
      />
    </Box>
  );
}
