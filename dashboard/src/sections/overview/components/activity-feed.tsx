import type { Document, Workspace } from 'src/types/cortex';
import type { AgentInfo, WorkflowInfo } from 'src/types/axon';

import { useState } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Chip from '@mui/material/Chip';
import Stack from '@mui/material/Stack';
import Alert from '@mui/material/Alert';
import Divider from '@mui/material/Divider';
import MenuItem from '@mui/material/MenuItem';
import CardHeader from '@mui/material/CardHeader';
import Typography from '@mui/material/Typography';
import { alpha, useTheme } from '@mui/material/styles';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { usePopover, CustomPopover } from 'src/components/custom-popover';

// ----------------------------------------------------------------------

interface ActivityEvent {
  id: string;
  type: string;
  service: 'axon' | 'cortex';
  timestamp: string;
  data: any;
}

interface ActivityFeedProps {
  events: ActivityEvent[];
  agents: AgentInfo[];
  workflows: WorkflowInfo[];
  documents: Document[];
  workspaces: Workspace[];
}

export function ActivityFeed({
  events,
  agents,
  workflows,
  documents,
  workspaces,
}: ActivityFeedProps) {
  const theme = useTheme();
  const popover = usePopover();
  const [serviceFilter, setServiceFilter] = useState<'all' | 'axon' | 'cortex'>('all');
  const [eventTypeFilter, setEventTypeFilter] = useState<string>('all');

  // Filter events
  const filteredEvents = events.filter((event) => {
    if (serviceFilter !== 'all' && event.service !== serviceFilter) return false;
    if (eventTypeFilter !== 'all' && event.type !== eventTypeFilter) return false;
    return true;
  });

  // Get unique event types
  const eventTypes = Array.from(new Set(events.map((e) => e.type)));

  return (
    <Card sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <CardHeader
        title="Recent Activity"
        subheader="Real-time system events and updates"
        action={
          <Stack direction="row" spacing={1}>
            <Chip
              label={serviceFilter === 'all' ? 'All Services' : serviceFilter}
              size="small"
              onClick={popover.onOpen}
              onDelete={serviceFilter !== 'all' ? () => setServiceFilter('all') : undefined}
            />
          </Stack>
        }
      />
      <Divider />

      <Box sx={{ flexGrow: 1, overflow: 'auto', p: 3 }}>
        {filteredEvents.length === 0 ? (
          <Alert severity="info">
            No recent activity. System events will appear here in real-time.
          </Alert>
        ) : (
          <Stack spacing={2}>
            {filteredEvents.slice(0, 20).map((event) => (
              <ActivityItem key={event.id} event={event} />
            ))}
          </Stack>
        )}
      </Box>

      {filteredEvents.length > 20 && (
        <>
          <Divider />
          <Box sx={{ p: 2, textAlign: 'center' }}>
            <Typography variant="body2" color="text.secondary">
              Showing 20 of {filteredEvents.length} events
            </Typography>
          </Box>
        </>
      )}

      <CustomPopover open={popover.open} anchorEl={popover.anchorEl} onClose={popover.onClose}>
        <MenuItem
          selected={serviceFilter === 'all'}
          onClick={() => {
            setServiceFilter('all');
            popover.onClose();
          }}
        >
          All Services
        </MenuItem>
        <MenuItem
          selected={serviceFilter === 'axon'}
          onClick={() => {
            setServiceFilter('axon');
            popover.onClose();
          }}
        >
          Axon Only
        </MenuItem>
        <MenuItem
          selected={serviceFilter === 'cortex'}
          onClick={() => {
            setServiceFilter('cortex');
            popover.onClose();
          }}
        >
          Cortex Only
        </MenuItem>
      </CustomPopover>
    </Card>
  );
}

// ----------------------------------------------------------------------

function ActivityItem({ event }: { event: ActivityEvent }) {
  const theme = useTheme();
  const { icon, color, title, description } = getEventInfo(event);

  return (
    <Stack
      direction="row"
      spacing={2}
      sx={{
        p: 2,
        borderRadius: 1.5,
        border: `1px solid ${theme.palette.divider}`,
        bgcolor: alpha(theme.palette.grey[500], 0.08),
        '&:hover': {
          bgcolor: alpha(theme.palette.grey[500], 0.16),
        },
      }}
    >
      <Box
        sx={{
          width: 40,
          height: 40,
          flexShrink: 0,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          borderRadius: 1,
          bgcolor: alpha(theme.palette[color].main, 0.16),
        }}
      >
        <Iconify icon={icon} width={24} sx={{ color: theme.palette[color].main }} />
      </Box>

      <Box sx={{ flexGrow: 1, minWidth: 0 }}>
        <Stack direction="row" spacing={1} alignItems="center" sx={{ mb: 0.5 }}>
          <Typography variant="subtitle2" noWrap>
            {title}
          </Typography>
          <Label variant="soft" color={event.service === 'axon' ? 'info' : 'success'}>
            {event.service}
          </Label>
        </Stack>
        <Typography variant="body2" color="text.secondary" noWrap>
          {description}
        </Typography>
        <Typography variant="caption" color="text.disabled">
          {formatTimestamp(event.timestamp)}
        </Typography>
      </Box>
    </Stack>
  );
}

// ----------------------------------------------------------------------

function getEventInfo(event: ActivityEvent): {
  icon: string;
  color: 'primary' | 'secondary' | 'info' | 'success' | 'warning' | 'error';
  title: string;
  description: string;
} {
  const { type, data } = event;

  // Agent events
  if (type === 'agent_started') {
    return {
      icon: 'solar:play-circle-bold',
      color: 'success',
      title: 'Agent Started',
      description: `${data.agent_name} (${data.agent_id.slice(0, 8)})`,
    };
  }

  if (type === 'agent_stopped') {
    return {
      icon: 'solar:stop-circle-bold',
      color: 'error',
      title: 'Agent Stopped',
      description: `${data.agent_name} (${data.agent_id.slice(0, 8)})`,
    };
  }

  if (type === 'agent_status_changed') {
    return {
      icon: 'solar:refresh-circle-bold',
      color: 'info',
      title: 'Agent Status Changed',
      description: `${data.agent_name} is now ${data.status}`,
    };
  }

  // Workflow events
  if (type === 'workflow_started') {
    return {
      icon: 'solar:routing-2-bold',
      color: 'info',
      title: 'Workflow Started',
      description: `${data.workflow_name} (${data.workflow_id.slice(0, 8)})`,
    };
  }

  if (type === 'workflow_completed') {
    return {
      icon: 'solar:check-circle-bold',
      color: 'success',
      title: 'Workflow Completed',
      description: `${data.workflow_name} finished successfully`,
    };
  }

  if (type === 'workflow_failed') {
    return {
      icon: 'solar:close-circle-bold',
      color: 'error',
      title: 'Workflow Failed',
      description: `${data.workflow_name} encountered an error`,
    };
  }

  // Task events
  if (type === 'task_started') {
    return {
      icon: 'solar:widget-5-bold',
      color: 'info',
      title: 'Task Started',
      description: `Task ${data.task_id.slice(0, 8)} in workflow ${data.workflow_id.slice(0, 8)}`,
    };
  }

  if (type === 'task_completed') {
    return {
      icon: 'solar:check-square-bold',
      color: 'success',
      title: 'Task Completed',
      description: `Task ${data.task_id.slice(0, 8)} finished`,
    };
  }

  if (type === 'task_failed') {
    return {
      icon: 'solar:close-square-bold',
      color: 'error',
      title: 'Task Failed',
      description: `Task ${data.task_id.slice(0, 8)} encountered an error`,
    };
  }

  // Default
  return {
    icon: 'solar:info-circle-bold',
    color: 'info',
    title: type.replace(/_/g, ' ').replace(/\b\w/g, (l) => l.toUpperCase()),
    description: JSON.stringify(data).slice(0, 50),
  };
}

// ----------------------------------------------------------------------

function formatTimestamp(timestamp: string): string {
  const date = new Date(timestamp);
  const now = new Date();
  const diff = now.getTime() - date.getTime();

  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (seconds < 60) return 'just now';
  if (minutes < 60) return `${minutes}m ago`;
  if (hours < 24) return `${hours}h ago`;
  if (days < 7) return `${days}d ago`;

  return date.toLocaleDateString();
}
