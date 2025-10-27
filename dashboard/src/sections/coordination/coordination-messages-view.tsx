import { useState } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Chip from '@mui/material/Chip';
import Stack from '@mui/material/Stack';
import Avatar from '@mui/material/Avatar';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import InputLabel from '@mui/material/InputLabel';
import FormControl from '@mui/material/FormControl';
import LinearProgress from '@mui/material/LinearProgress';

import { Iconify } from 'src/components/iconify';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

interface Message {
  id: string;
  from_agent: string;
  to_agent: string;
  message_type: string;
  content: string;
  timestamp: string;
  priority: 'high' | 'medium' | 'low';
  status: 'sent' | 'delivered' | 'read';
}

const MOCK_MESSAGES: Message[] = [
  {
    id: '1',
    from_agent: 'orchestrator-001',
    to_agent: 'developer-001',
    message_type: 'task_assignment',
    content: 'Implement authentication feature',
    timestamp: new Date(Date.now() - 5 * 60000).toISOString(),
    priority: 'high',
    status: 'read',
  },
  {
    id: '2',
    from_agent: 'developer-001',
    to_agent: 'reviewer-001',
    message_type: 'review_request',
    content: 'Please review PR #123',
    timestamp: new Date(Date.now() - 15 * 60000).toISOString(),
    priority: 'medium',
    status: 'delivered',
  },
];

export function CoordinationMessagesView() {
  const [filterType, setFilterType] = useState<string>('all');
  const [filterPriority, setFilterPriority] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState<string>('');

  // In real implementation, fetch from API
  const messages = MOCK_MESSAGES;
  const isLoading = false;

  const filteredMessages = messages.filter((msg) => {
    if (filterType !== 'all' && msg.message_type !== filterType) return false;
    if (filterPriority !== 'all' && msg.priority !== filterPriority) return false;
    if (
      searchQuery &&
      !msg.content.toLowerCase().includes(searchQuery.toLowerCase()) &&
      !msg.from_agent.toLowerCase().includes(searchQuery.toLowerCase())
    )
      return false;
    return true;
  });

  const getPriorityColor = (priority: string) => {
    if (priority === 'high') return 'error';
    if (priority === 'medium') return 'warning';
    return 'success';
  };

  const getStatusColor = (status: string) => {
    if (status === 'read') return 'success';
    if (status === 'delivered') return 'info';
    return 'default';
  };

  return (
    <>
      <CustomBreadcrumbs
        heading="Inter-Agent Messages"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Coordination' },
          { name: 'Messages' },
        ]}
        sx={{ mb: 3 }}
      />

      <Card>
        <Box sx={{ p: 3 }}>
          <Stack spacing={2}>
            <Stack direction={{ xs: 'column', md: 'row' }} spacing={2}>
              <TextField
                fullWidth
                placeholder="Search messages..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                slotProps={{
                  input: {
                    startAdornment: (
                      <Iconify icon="eva:search-fill" sx={{ color: 'text.disabled', mr: 1 }} />
                    ),
                  },
                }}
              />
              <FormControl sx={{ minWidth: 200 }}>
                <InputLabel>Message Type</InputLabel>
                <Select
                  value={filterType}
                  label="Message Type"
                  onChange={(e) => setFilterType(e.target.value)}
                >
                  <MenuItem value="all">All Types</MenuItem>
                  <MenuItem value="task_assignment">Task Assignment</MenuItem>
                  <MenuItem value="review_request">Review Request</MenuItem>
                  <MenuItem value="status_update">Status Update</MenuItem>
                  <MenuItem value="notification">Notification</MenuItem>
                </Select>
              </FormControl>
              <FormControl sx={{ minWidth: 150 }}>
                <InputLabel>Priority</InputLabel>
                <Select
                  value={filterPriority}
                  label="Priority"
                  onChange={(e) => setFilterPriority(e.target.value)}
                >
                  <MenuItem value="all">All</MenuItem>
                  <MenuItem value="high">High</MenuItem>
                  <MenuItem value="medium">Medium</MenuItem>
                  <MenuItem value="low">Low</MenuItem>
                </Select>
              </FormControl>
            </Stack>

            <Typography variant="body2" color="text.secondary">
              {filteredMessages.length} messages
            </Typography>
          </Stack>
        </Box>

        {isLoading && <LinearProgress />}

        <Stack spacing={0} divider={<Box sx={{ borderBottom: 1, borderColor: 'divider' }} />}>
          {filteredMessages.map((message) => (
            <Box key={message.id} sx={{ p: 2.5, '&:hover': { bgcolor: 'action.hover' } }}>
              <Stack spacing={2}>
                <Stack direction="row" justifyContent="space-between" alignItems="flex-start">
                  <Stack direction="row" spacing={2} alignItems="center" flex={1}>
                    <Avatar sx={{ bgcolor: 'primary.main' }}>
                      <Iconify icon="mdi:robot" width={24} />
                    </Avatar>
                    <Box flex={1}>
                      <Stack direction="row" spacing={1} alignItems="center" flexWrap="wrap">
                        <Typography variant="subtitle2">{message.from_agent}</Typography>
                        <Iconify icon="mdi:arrow-right" width={16} color="text.disabled" />
                        <Typography variant="subtitle2">{message.to_agent}</Typography>
                      </Stack>
                      <Typography variant="caption" color="text.secondary">
                        {new Date(message.timestamp).toLocaleString()}
                      </Typography>
                    </Box>
                  </Stack>
                  <Stack direction="row" spacing={1}>
                    <Chip
                      label={message.priority}
                      size="small"
                      color={getPriorityColor(message.priority) as any}
                      variant="outlined"
                    />
                    <Chip
                      label={message.status}
                      size="small"
                      color={getStatusColor(message.status) as any}
                      variant="filled"
                    />
                  </Stack>
                </Stack>

                <Box>
                  <Chip label={message.message_type} size="small" sx={{ mb: 1 }} />
                  <Typography variant="body2">{message.content}</Typography>
                </Box>
              </Stack>
            </Box>
          ))}
        </Stack>

        {!isLoading && filteredMessages.length === 0 && (
          <Box sx={{ p: 5, textAlign: 'center' }}>
            <Iconify
              icon="mdi:message-off"
              width={64}
              sx={{ color: 'text.disabled', mb: 2 }}
            />
            <Typography variant="h6" color="text.secondary">
              No messages found
            </Typography>
          </Box>
        )}
      </Card>
    </>
  );
}
