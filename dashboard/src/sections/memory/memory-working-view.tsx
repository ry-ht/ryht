import { useState } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

interface WorkingMemoryItem {
  id: string;
  content: string;
  type: 'task' | 'context' | 'goal' | 'constraint';
  priority: number;
  lastAccessed: string;
}

const MOCK_WORKING_MEMORY: WorkingMemoryItem[] = [
  {
    id: '1',
    content: 'Implement user authentication system',
    type: 'task',
    priority: 1,
    lastAccessed: new Date().toISOString(),
  },
  {
    id: '2',
    content: 'Current workspace: /project/src',
    type: 'context',
    priority: 2,
    lastAccessed: new Date(Date.now() - 60000).toISOString(),
  },
  {
    id: '3',
    content: 'Maintain test coverage above 80%',
    type: 'constraint',
    priority: 3,
    lastAccessed: new Date(Date.now() - 120000).toISOString(),
  },
  {
    id: '4',
    content: 'Optimize API response time',
    type: 'goal',
    priority: 4,
    lastAccessed: new Date(Date.now() - 180000).toISOString(),
  },
];

export function MemoryWorkingView() {
  const [items] = useState<WorkingMemoryItem[]>(MOCK_WORKING_MEMORY);

  const getTypeColor = (type: string) => {
    const colors: Record<string, any> = {
      task: 'primary',
      context: 'info',
      goal: 'success',
      constraint: 'warning',
    };
    return colors[type] || 'default';
  };

  const getTypeIcon = (type: string) => {
    const icons: Record<string, string> = {
      task: 'mdi:clipboard-check',
      context: 'mdi:information',
      goal: 'mdi:target',
      constraint: 'mdi:alert-circle',
    };
    return icons[type] || 'mdi:circle';
  };

  return (
    <>
      <CustomBreadcrumbs
        heading="Working Memory"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Memory' },
          { name: 'Working' },
        ]}
        sx={{ mb: 3 }}
      />

      <Stack spacing={2}>
        <Card sx={{ p: 3 }}>
          <Stack direction="row" justifyContent="space-between" alignItems="center">
            <Box>
              <Typography variant="h6" sx={{ mb: 0.5 }}>
                Working Memory (7±2 Items)
              </Typography>
              <Typography variant="body2" color="text.secondary">
                Current active items in short-term memory. Limited to ~7 items for optimal
                cognitive performance.
              </Typography>
            </Box>
            <Button variant="outlined" startIcon={<Iconify icon="mdi:refresh" />}>
              Refresh
            </Button>
          </Stack>
        </Card>

        <Card sx={{ p: 3 }}>
          <Grid container spacing={2}>
            <Grid item xs={12} md={3}>
              <Stack spacing={1} alignItems="center" sx={{ p: 2, bgcolor: 'background.neutral', borderRadius: 2 }}>
                <Typography variant="h3">{items.length}</Typography>
                <Typography variant="body2" color="text.secondary">
                  Active Items
                </Typography>
              </Stack>
            </Grid>
            <Grid item xs={12} md={3}>
              <Stack spacing={1} alignItems="center" sx={{ p: 2, bgcolor: 'background.neutral', borderRadius: 2 }}>
                <Typography variant="h3">{items.filter((i) => i.type === 'task').length}</Typography>
                <Typography variant="body2" color="text.secondary">
                  Tasks
                </Typography>
              </Stack>
            </Grid>
            <Grid item xs={12} md={3}>
              <Stack spacing={1} alignItems="center" sx={{ p: 2, bgcolor: 'background.neutral', borderRadius: 2 }}>
                <Typography variant="h3">{items.filter((i) => i.type === 'context').length}</Typography>
                <Typography variant="body2" color="text.secondary">
                  Context
                </Typography>
              </Stack>
            </Grid>
            <Grid item xs={12} md={3}>
              <Stack spacing={1} alignItems="center" sx={{ p: 2, bgcolor: 'background.neutral', borderRadius: 2 }}>
                <Typography variant="h3">{7 - items.length}</Typography>
                <Typography variant="body2" color="text.secondary">
                  Available Slots
                </Typography>
              </Stack>
            </Grid>
          </Grid>
        </Card>

        <Grid container spacing={2}>
          {items
            .sort((a, b) => a.priority - b.priority)
            .map((item, index) => (
              <Grid item xs={12} key={item.id}>
                <Card sx={{ p: 2.5 }}>
                  <Stack direction="row" spacing={2} alignItems="flex-start">
                    <Box
                      sx={{
                        width: 48,
                        height: 48,
                        borderRadius: 1.5,
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        bgcolor: `${getTypeColor(item.type)}.lighter`,
                        color: `${getTypeColor(item.type)}.main`,
                        flexShrink: 0,
                      }}
                    >
                      <Iconify icon={getTypeIcon(item.type)} width={24} />
                    </Box>

                    <Stack spacing={1} flex={1}>
                      <Stack direction="row" spacing={1} alignItems="center">
                        <Typography variant="h6">#{index + 1}</Typography>
                        <Label variant="soft" color={getTypeColor(item.type)}>
                          {item.type}
                        </Label>
                        <Label variant="soft">Priority {item.priority}</Label>
                      </Stack>
                      <Typography variant="body1">{item.content}</Typography>
                      <Typography variant="caption" color="text.secondary">
                        Last accessed: {new Date(item.lastAccessed).toLocaleString()}
                      </Typography>
                    </Stack>
                  </Stack>
                </Card>
              </Grid>
            ))}
        </Grid>

        {items.length >= 7 && (
          <Card sx={{ p: 2, bgcolor: 'warning.lighter' }}>
            <Stack direction="row" spacing={2} alignItems="center">
              <Iconify icon="mdi:alert" width={24} color="warning.main" />
              <Typography variant="body2" color="warning.dark">
                Working memory is at capacity. Consider consolidating items to long-term memory.
              </Typography>
            </Stack>
          </Card>
        )}
      </Stack>
    </>
  );
}
