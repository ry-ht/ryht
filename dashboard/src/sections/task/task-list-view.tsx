import type { TaskInfo, TaskStatus, TaskPriority } from 'src/types/axon';

import useSWR, { mutate } from 'swr';
import { useState, useCallback } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import InputLabel from '@mui/material/InputLabel';
import FormControl from '@mui/material/FormControl';
import LinearProgress from '@mui/material/LinearProgress';

import { paths } from 'src/routes/paths';
import { useRouter } from 'src/routes/hooks';

import { getTaskStatusColor, getTaskPriorityColor } from 'src/utils/status-colors';

import { axonClient, axonFetcher, axonEndpoints } from 'src/lib/axon-client';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { TaskCreateDialog } from './task-create-dialog';

// ----------------------------------------------------------------------

const TASK_COLUMNS: { status: TaskStatus; label: string }[] = [
  { status: 'Pending', label: 'Pending' },
  { status: 'InProgress', label: 'In Progress' },
  { status: 'Blocked', label: 'Blocked' },
  { status: 'Done', label: 'Done' },
  { status: 'Cancelled', label: 'Cancelled' },
];

// ----------------------------------------------------------------------

export function TaskListView() {
  const router = useRouter();

  const [openCreateDialog, setOpenCreateDialog] = useState(false);
  const [filterPriority, setFilterPriority] = useState<TaskPriority | 'All'>('All');
  const [filterAgent, setFilterAgent] = useState<string>('All');
  const [searchTerm, setSearchTerm] = useState('');

  // Fetch tasks
  const { data: tasks = [], isLoading } = useSWR<TaskInfo[]>(
    axonEndpoints.tasks.list,
    axonFetcher,
    { refreshInterval: 5000 }
  );

  const handleStatusChange = useCallback(async (taskId: string, newStatus: TaskStatus) => {
    try {
      await axonClient.updateTask(taskId, { status: newStatus });
      mutate(axonEndpoints.tasks.list);
    } catch (err) {
      console.error('Failed to update task status:', err);
    }
  }, []);

  const handleTaskClick = useCallback(
    (taskId: string) => {
      router.push(paths.dashboard.tasks.details(taskId));
    },
    [router]
  );

  // Filter tasks
  const filteredTasks = tasks.filter((task) => {
    if (filterPriority !== 'All' && task.priority !== filterPriority) return false;
    if (filterAgent !== 'All' && !task.assigned_agents.includes(filterAgent)) return false;
    if (searchTerm && !task.title.toLowerCase().includes(searchTerm.toLowerCase())) return false;
    return true;
  });

  // Group tasks by status
  const tasksByStatus = TASK_COLUMNS.reduce(
    (acc, column) => {
      acc[column.status] = filteredTasks.filter((task) => task.status === column.status);
      return acc;
    },
    {} as Record<TaskStatus, TaskInfo[]>
  );

  // Get unique agents for filter
  const allAgents = Array.from(
    new Set(tasks.flatMap((task) => task.assigned_agents))
  ).sort();

  return (
    <>
      <CustomBreadcrumbs
        heading="Tasks"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Tasks' },
        ]}
        action={
          <Button
            variant="contained"
            startIcon={<Iconify icon="mingcute:add-line" />}
            onClick={() => setOpenCreateDialog(true)}
          >
            New Task
          </Button>
        }
        sx={{ mb: 3 }}
      />

      <Box sx={{ mb: 5 }}>

        <Stack spacing={2} direction={{ xs: 'column', sm: 'row' }}>
          <TextField
            fullWidth
            placeholder="Search tasks..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            InputProps={{
              startAdornment: <Iconify icon="eva:search-fill" sx={{ mr: 1, color: 'text.disabled' }} />,
            }}
          />
          <FormControl sx={{ minWidth: 150 }}>
            <InputLabel>Priority</InputLabel>
            <Select
              value={filterPriority}
              label="Priority"
              onChange={(e) => setFilterPriority(e.target.value as TaskPriority | 'All')}
            >
              <MenuItem value="All">All</MenuItem>
              <MenuItem value="Critical">Critical</MenuItem>
              <MenuItem value="High">High</MenuItem>
              <MenuItem value="Medium">Medium</MenuItem>
              <MenuItem value="Low">Low</MenuItem>
            </Select>
          </FormControl>
          <FormControl sx={{ minWidth: 200 }}>
            <InputLabel>Agent</InputLabel>
            <Select
              value={filterAgent}
              label="Agent"
              onChange={(e) => setFilterAgent(e.target.value)}
            >
              <MenuItem value="All">All</MenuItem>
              {allAgents.map((agent) => (
                <MenuItem key={agent} value={agent}>
                  {agent}
                </MenuItem>
              ))}
            </Select>
          </FormControl>
        </Stack>
      </Box>

      {isLoading ? (
        <LinearProgress />
      ) : (
        <Box
          sx={{
            display: 'grid',
            gap: 2,
            gridTemplateColumns: {
              xs: '1fr',
              sm: 'repeat(2, 1fr)',
              md: 'repeat(3, 1fr)',
              lg: 'repeat(5, 1fr)',
            },
          }}
        >
          {TASK_COLUMNS.map((column) => (
            <TaskColumn
              key={column.status}
              status={column.status}
              label={column.label}
              tasks={tasksByStatus[column.status] || []}
              onStatusChange={handleStatusChange}
              onTaskClick={handleTaskClick}
            />
          ))}
        </Box>
      )}

      <TaskCreateDialog
        open={openCreateDialog}
        onClose={() => setOpenCreateDialog(false)}
        onSuccess={() => mutate(axonEndpoints.tasks.list)}
      />
    </>
  );
}

// ----------------------------------------------------------------------

type TaskColumnProps = {
  status: TaskStatus;
  label: string;
  tasks: TaskInfo[];
  onStatusChange: (taskId: string, newStatus: TaskStatus) => void;
  onTaskClick: (taskId: string) => void;
};

function TaskColumn({ status, label, tasks, onStatusChange, onTaskClick }: TaskColumnProps) {
  return (
    <Box>
      <Box
        sx={{
          mb: 2,
          px: 2,
          py: 1,
          borderRadius: 1,
          bgcolor: 'background.neutral',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
        }}
      >
        <Typography variant="subtitle2">{label}</Typography>
        <Label color={getTaskStatusColor(status)}>{tasks.length}</Label>
      </Box>

      <Stack spacing={2}>
        {tasks.map((task) => (
          <TaskCard
            key={task.id}
            task={task}
            onStatusChange={onStatusChange}
            onClick={() => onTaskClick(task.id)}
          />
        ))}
      </Stack>
    </Box>
  );
}

// ----------------------------------------------------------------------

type TaskCardProps = {
  task: TaskInfo;
  onStatusChange: (taskId: string, newStatus: TaskStatus) => void;
  onClick: () => void;
};

function TaskCard({ task, onStatusChange, onClick }: TaskCardProps) {
  const [anchorEl, setAnchorEl] = useState<null | HTMLElement>(null);

  const handleMenuOpen = (event: React.MouseEvent<HTMLElement>) => {
    event.stopPropagation();
    setAnchorEl(event.currentTarget);
  };

  const handleMenuClose = () => {
    setAnchorEl(null);
  };

  const handleMoveToStatus = (newStatus: TaskStatus) => {
    onStatusChange(task.id, newStatus);
    handleMenuClose();
  };

  return (
    <Card
      sx={{
        p: 2,
        cursor: 'pointer',
        '&:hover': {
          boxShadow: (theme) => theme.customShadows.z8,
        },
      }}
      onClick={onClick}
    >
      <Box sx={{ display: 'flex', alignItems: 'flex-start', mb: 1 }}>
        <Label variant="soft" color={getTaskPriorityColor(task.priority)} sx={{ mr: 'auto' }}>
          {task.priority}
        </Label>
        <Box
          component="button"
          onClick={handleMenuOpen}
          sx={{
            p: 0.5,
            border: 'none',
            bgcolor: 'transparent',
            cursor: 'pointer',
            borderRadius: 0.5,
            '&:hover': { bgcolor: 'action.hover' },
          }}
        >
          <Iconify icon="eva:more-vertical-fill" width={20} />
        </Box>
      </Box>

      <Typography variant="subtitle2" sx={{ mb: 1 }}>
        {task.title}
      </Typography>

      {task.description && (
        <Typography
          variant="caption"
          sx={{
            mb: 1.5,
            display: '-webkit-box',
            overflow: 'hidden',
            WebkitBoxOrient: 'vertical',
            WebkitLineClamp: 2,
            color: 'text.secondary',
          }}
        >
          {task.description}
        </Typography>
      )}

      <Box sx={{ mb: 1.5 }}>
        <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 0.5 }}>
          <Typography variant="caption" sx={{ color: 'text.secondary' }}>
            Progress
          </Typography>
          <Typography variant="caption" sx={{ color: 'text.secondary' }}>
            {task.progress}%
          </Typography>
        </Box>
        <LinearProgress variant="determinate" value={task.progress} sx={{ height: 6, borderRadius: 1 }} />
      </Box>

      {task.assigned_agents.length > 0 && (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, mb: 1 }}>
          <Iconify icon="solar:user-bold" width={16} sx={{ color: 'text.disabled' }} />
          <Typography variant="caption" sx={{ color: 'text.secondary' }}>
            {task.assigned_agents.join(', ')}
          </Typography>
        </Box>
      )}

      {task.estimated_hours && (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, mb: 1 }}>
          <Iconify icon="solar:clock-circle-bold" width={16} sx={{ color: 'text.disabled' }} />
          <Typography variant="caption" sx={{ color: 'text.secondary' }}>
            {task.actual_hours || 0} / {task.estimated_hours}h
          </Typography>
        </Box>
      )}

      {task.dependencies.length > 0 && (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, mb: 1 }}>
          <Iconify icon="solar:link-circle-bold" width={16} sx={{ color: 'warning.main' }} />
          <Typography variant="caption" sx={{ color: 'warning.main' }}>
            {task.dependencies.length} dependencies
          </Typography>
        </Box>
      )}

      {task.tags.length > 0 && (
        <Box sx={{ display: 'flex', gap: 0.5, flexWrap: 'wrap' }}>
          {task.tags.map((tag) => (
            <Label key={tag} variant="soft" color="default" sx={{ fontSize: 10 }}>
              {tag}
            </Label>
          ))}
        </Box>
      )}

      {/* Move to status menu */}
      {anchorEl && (
        <Card
          sx={{
            position: 'absolute',
            zIndex: 1300,
            minWidth: 150,
            boxShadow: (theme) => theme.customShadows.dropdown,
          }}
        >
          <Stack>
            {TASK_COLUMNS.filter((col) => col.status !== task.status).map((col) => (
              <Box
                key={col.status}
                component="button"
                onClick={(e) => {
                  e.stopPropagation();
                  handleMoveToStatus(col.status);
                }}
                sx={{
                  px: 2,
                  py: 1,
                  border: 'none',
                  bgcolor: 'transparent',
                  cursor: 'pointer',
                  textAlign: 'left',
                  '&:hover': { bgcolor: 'action.hover' },
                }}
              >
                <Typography variant="body2">Move to {col.label}</Typography>
              </Box>
            ))}
          </Stack>
        </Card>
      )}
    </Card>
  );
}
