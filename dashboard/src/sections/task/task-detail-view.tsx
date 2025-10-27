import type { TaskInfo, TaskStatus, TaskPriority, TaskActivity } from 'src/types/axon';

import useSWR, { mutate } from 'swr';
import { useState, useCallback } from 'react';
import { useParams } from 'react-router';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Chip from '@mui/material/Chip';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Slider from '@mui/material/Slider';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import LoadingButton from '@mui/lab/LoadingButton';
import FormControl from '@mui/material/FormControl';
import InputLabel from '@mui/material/InputLabel';
import Autocomplete from '@mui/material/Autocomplete';
import LinearProgress from '@mui/material/LinearProgress';

import { paths } from 'src/routes/paths';
import { useRouter } from 'src/routes/hooks';

import { axonClient, axonFetcher, axonEndpoints } from 'src/lib/axon-client';
import { getTaskPriorityColor, getTaskStatusColor } from 'src/utils/status-colors';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';

// ----------------------------------------------------------------------

export function TaskDetailView() {
  const router = useRouter();
  const params = useParams();
  const taskId = params.id as string;

  const [isEditing, setIsEditing] = useState(false);
  const [editedTask, setEditedTask] = useState<Partial<TaskInfo>>({});

  // Fetch task details
  const { data: task, isLoading } = useSWR<TaskInfo>(
    taskId ? axonEndpoints.tasks.details(taskId) : null,
    axonFetcher,
    { refreshInterval: 5000 }
  );

  // Fetch task activities
  const { data: activities = [] } = useSWR<TaskActivity[]>(
    taskId ? axonEndpoints.tasks.activities(taskId) : null,
    axonFetcher,
    { refreshInterval: 5000 }
  );

  // Fetch agents for assignment
  const { data: agents = [] } = useSWR(axonEndpoints.agents.list, axonFetcher);

  // Fetch tasks for dependencies
  const { data: tasks = [] } = useSWR(axonEndpoints.tasks.list, axonFetcher);

  const handleEdit = useCallback(() => {
    if (task) {
      setEditedTask(task);
      setIsEditing(true);
    }
  }, [task]);

  const handleCancelEdit = useCallback(() => {
    setEditedTask({});
    setIsEditing(false);
  }, []);

  const handleSave = useCallback(async () => {
    if (!task || !editedTask) return;

    try {
      await axonClient.updateTask(task.id, editedTask);
      mutate(axonEndpoints.tasks.details(task.id));
      mutate(axonEndpoints.tasks.list);
      setIsEditing(false);
    } catch (error) {
      console.error('Failed to update task:', error);
    }
  }, [task, editedTask]);

  const handleDelete = useCallback(async () => {
    if (!task) return;

    if (window.confirm('Are you sure you want to delete this task?')) {
      try {
        await axonClient.deleteTask(task.id);
        router.push(paths.dashboard.tasks.list);
      } catch (error) {
        console.error('Failed to delete task:', error);
      }
    }
  }, [task, router]);

  if (isLoading) {
    return <LinearProgress />;
  }

  if (!task) {
    return (
      <Box sx={{ textAlign: 'center', py: 10 }}>
        <Typography variant="h6" color="text.secondary">
          Task not found
        </Typography>
      </Box>
    );
  }

  const displayTask = isEditing ? { ...task, ...editedTask } : task;

  return (
    <>
      <Box sx={{ display: 'flex', alignItems: 'center', mb: 5 }}>
        <Typography variant="h4" sx={{ flexGrow: 1 }}>
          Task Details
        </Typography>
        <Stack direction="row" spacing={2}>
          <Button
            variant="outlined"
            startIcon={<Iconify icon="eva:arrow-ios-back-fill" />}
            onClick={() => router.push(paths.dashboard.tasks.list)}
          >
            Back
          </Button>
          {!isEditing && (
            <>
              <Button
                variant="outlined"
                startIcon={<Iconify icon="solar:pen-bold" />}
                onClick={handleEdit}
              >
                Edit
              </Button>
              <Button
                variant="outlined"
                color="error"
                startIcon={<Iconify icon="solar:trash-bin-trash-bold" />}
                onClick={handleDelete}
              >
                Delete
              </Button>
            </>
          )}
          {isEditing && (
            <>
              <Button variant="outlined" color="inherit" onClick={handleCancelEdit}>
                Cancel
              </Button>
              <LoadingButton variant="contained" onClick={handleSave}>
                Save Changes
              </LoadingButton>
            </>
          )}
        </Stack>
      </Box>

      <Grid container spacing={3}>
        <Grid item xs={12} md={8}>
          <Stack spacing={3}>
            {/* Task Information */}
            <Card sx={{ p: 3 }}>
              <Typography variant="h6" sx={{ mb: 3 }}>
                Task Information
              </Typography>

              <Stack spacing={3}>
                {isEditing ? (
                  <TextField
                    fullWidth
                    label="Title"
                    value={editedTask.title || task.title}
                    onChange={(e) => setEditedTask({ ...editedTask, title: e.target.value })}
                  />
                ) : (
                  <Box>
                    <Typography variant="caption" sx={{ color: 'text.secondary', mb: 0.5 }}>
                      Title
                    </Typography>
                    <Typography variant="h6">{task.title}</Typography>
                  </Box>
                )}

                {isEditing ? (
                  <TextField
                    fullWidth
                    label="Description"
                    multiline
                    rows={4}
                    value={editedTask.description || task.description}
                    onChange={(e) => setEditedTask({ ...editedTask, description: e.target.value })}
                  />
                ) : (
                  <Box>
                    <Typography variant="caption" sx={{ color: 'text.secondary', mb: 0.5 }}>
                      Description
                    </Typography>
                    <Typography variant="body2">{task.description || 'No description'}</Typography>
                  </Box>
                )}

                <Box>
                  <Typography variant="caption" sx={{ color: 'text.secondary', mb: 1 }}>
                    Progress: {displayTask.progress}%
                  </Typography>
                  {isEditing ? (
                    <Slider
                      value={editedTask.progress ?? task.progress}
                      onChange={(_, value) => setEditedTask({ ...editedTask, progress: value as number })}
                      min={0}
                      max={100}
                      valueLabelDisplay="auto"
                    />
                  ) : (
                    <LinearProgress
                      variant="determinate"
                      value={task.progress}
                      sx={{ height: 8, borderRadius: 1 }}
                    />
                  )}
                </Box>

                {isEditing ? (
                  <TextField
                    fullWidth
                    label="Completion Notes"
                    multiline
                    rows={3}
                    value={editedTask.completion_notes || task.completion_notes || ''}
                    onChange={(e) => setEditedTask({ ...editedTask, completion_notes: e.target.value })}
                  />
                ) : task.completion_notes ? (
                  <Box>
                    <Typography variant="caption" sx={{ color: 'text.secondary', mb: 0.5 }}>
                      Completion Notes
                    </Typography>
                    <Typography variant="body2">{task.completion_notes}</Typography>
                  </Box>
                ) : null}
              </Stack>
            </Card>

            {/* Activity Timeline */}
            <Card sx={{ p: 3 }}>
              <Typography variant="h6" sx={{ mb: 3 }}>
                Activity Timeline
              </Typography>

              <Stack spacing={2}>
                {activities.length === 0 ? (
                  <Typography variant="body2" color="text.secondary">
                    No activity yet
                  </Typography>
                ) : (
                  activities.map((activity) => (
                    <Box
                      key={activity.id}
                      sx={{
                        display: 'flex',
                        gap: 2,
                        pb: 2,
                        borderBottom: '1px dashed',
                        borderColor: 'divider',
                        '&:last-child': { borderBottom: 'none', pb: 0 },
                      }}
                    >
                      <Box
                        sx={{
                          width: 40,
                          height: 40,
                          borderRadius: '50%',
                          bgcolor: 'primary.lighter',
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'center',
                          flexShrink: 0,
                        }}
                      >
                        <Iconify icon="solar:history-bold" width={20} />
                      </Box>
                      <Box sx={{ flexGrow: 1 }}>
                        <Typography variant="subtitle2">{activity.action}</Typography>
                        <Typography variant="body2" sx={{ color: 'text.secondary', mb: 0.5 }}>
                          {activity.description}
                        </Typography>
                        <Typography variant="caption" sx={{ color: 'text.disabled' }}>
                          {new Date(activity.timestamp).toLocaleString()} • {activity.user}
                        </Typography>
                      </Box>
                    </Box>
                  ))
                )}
              </Stack>
            </Card>
          </Stack>
        </Grid>

        <Grid item xs={12} md={4}>
          <Stack spacing={3}>
            {/* Status & Priority */}
            <Card sx={{ p: 3 }}>
              <Typography variant="h6" sx={{ mb: 3 }}>
                Status & Priority
              </Typography>

              <Stack spacing={2}>
                {isEditing ? (
                  <>
                    <FormControl fullWidth>
                      <InputLabel>Status</InputLabel>
                      <Select
                        value={editedTask.status || task.status}
                        label="Status"
                        onChange={(e) => setEditedTask({ ...editedTask, status: e.target.value as TaskStatus })}
                      >
                        <MenuItem value="Pending">Pending</MenuItem>
                        <MenuItem value="InProgress">In Progress</MenuItem>
                        <MenuItem value="Blocked">Blocked</MenuItem>
                        <MenuItem value="Done">Done</MenuItem>
                        <MenuItem value="Cancelled">Cancelled</MenuItem>
                      </Select>
                    </FormControl>

                    <FormControl fullWidth>
                      <InputLabel>Priority</InputLabel>
                      <Select
                        value={editedTask.priority || task.priority}
                        label="Priority"
                        onChange={(e) => setEditedTask({ ...editedTask, priority: e.target.value as TaskPriority })}
                      >
                        <MenuItem value="Critical">Critical</MenuItem>
                        <MenuItem value="High">High</MenuItem>
                        <MenuItem value="Medium">Medium</MenuItem>
                        <MenuItem value="Low">Low</MenuItem>
                      </Select>
                    </FormControl>
                  </>
                ) : (
                  <>
                    <Box>
                      <Typography variant="caption" sx={{ color: 'text.secondary', mb: 1, display: 'block' }}>
                        Status
                      </Typography>
                      <Label variant="soft" color={getTaskStatusColor(task.status)}>
                        {task.status}
                      </Label>
                    </Box>

                    <Box>
                      <Typography variant="caption" sx={{ color: 'text.secondary', mb: 1, display: 'block' }}>
                        Priority
                      </Typography>
                      <Label variant="soft" color={getTaskPriorityColor(task.priority)}>
                        {task.priority}
                      </Label>
                    </Box>
                  </>
                )}
              </Stack>
            </Card>

            {/* Assignment */}
            <Card sx={{ p: 3 }}>
              <Typography variant="h6" sx={{ mb: 3 }}>
                Assignment
              </Typography>

              {isEditing ? (
                <Autocomplete
                  multiple
                  value={editedTask.assigned_agents || task.assigned_agents}
                  onChange={(_, value) => setEditedTask({ ...editedTask, assigned_agents: value })}
                  options={agents.map((agent: any) => agent.name)}
                  renderInput={(params) => <TextField {...params} label="Assigned Agents" />}
                  renderTags={(value, getTagProps) =>
                    value.map((option, index) => (
                      <Chip {...getTagProps({ index })} key={option} label={option} size="small" />
                    ))
                  }
                />
              ) : task.assigned_agents.length > 0 ? (
                <Stack spacing={1}>
                  {task.assigned_agents.map((agent) => (
                    <Box key={agent} sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                      <Iconify icon="solar:user-bold" width={20} />
                      <Typography variant="body2">{agent}</Typography>
                    </Box>
                  ))}
                </Stack>
              ) : (
                <Typography variant="body2" color="text.secondary">
                  No agents assigned
                </Typography>
              )}
            </Card>

            {/* Time Tracking */}
            <Card sx={{ p: 3 }}>
              <Typography variant="h6" sx={{ mb: 3 }}>
                Time Tracking
              </Typography>

              <Stack spacing={2}>
                {isEditing ? (
                  <>
                    <TextField
                      fullWidth
                      type="number"
                      label="Estimated Hours"
                      value={editedTask.estimated_hours ?? task.estimated_hours ?? ''}
                      onChange={(e) =>
                        setEditedTask({ ...editedTask, estimated_hours: parseFloat(e.target.value) || undefined })
                      }
                      inputProps={{ min: 0, step: 0.5 }}
                    />
                    <TextField
                      fullWidth
                      type="number"
                      label="Actual Hours"
                      value={editedTask.actual_hours ?? task.actual_hours ?? ''}
                      onChange={(e) =>
                        setEditedTask({ ...editedTask, actual_hours: parseFloat(e.target.value) || undefined })
                      }
                      inputProps={{ min: 0, step: 0.5 }}
                    />
                  </>
                ) : (
                  <>
                    <Box>
                      <Typography variant="caption" sx={{ color: 'text.secondary' }}>
                        Estimated
                      </Typography>
                      <Typography variant="h6">{task.estimated_hours || 0}h</Typography>
                    </Box>
                    <Box>
                      <Typography variant="caption" sx={{ color: 'text.secondary' }}>
                        Actual
                      </Typography>
                      <Typography variant="h6">{task.actual_hours || 0}h</Typography>
                    </Box>
                  </>
                )}
              </Stack>
            </Card>

            {/* Tags */}
            <Card sx={{ p: 3 }}>
              <Typography variant="h6" sx={{ mb: 3 }}>
                Tags
              </Typography>

              {isEditing ? (
                <Autocomplete
                  multiple
                  freeSolo
                  value={editedTask.tags || task.tags}
                  onChange={(_, value) => setEditedTask({ ...editedTask, tags: value })}
                  options={[]}
                  renderInput={(params) => <TextField {...params} label="Tags" />}
                  renderTags={(value, getTagProps) =>
                    value.map((option, index) => (
                      <Chip {...getTagProps({ index })} key={option} label={option} size="small" />
                    ))
                  }
                />
              ) : task.tags.length > 0 ? (
                <Box sx={{ display: 'flex', gap: 0.5, flexWrap: 'wrap' }}>
                  {task.tags.map((tag) => (
                    <Chip key={tag} label={tag} size="small" />
                  ))}
                </Box>
              ) : (
                <Typography variant="body2" color="text.secondary">
                  No tags
                </Typography>
              )}
            </Card>

            {/* Dependencies */}
            <Card sx={{ p: 3 }}>
              <Typography variant="h6" sx={{ mb: 3 }}>
                Dependencies
              </Typography>

              {isEditing ? (
                <Autocomplete
                  multiple
                  value={editedTask.dependencies || task.dependencies}
                  onChange={(_, value) => setEditedTask({ ...editedTask, dependencies: value })}
                  options={tasks.filter((t: any) => t.id !== task.id).map((t: any) => t.id)}
                  getOptionLabel={(option) => {
                    const t = tasks.find((task: any) => task.id === option);
                    return t ? `${t.title} (${t.id})` : option;
                  }}
                  renderInput={(params) => <TextField {...params} label="Dependencies" />}
                  renderTags={(value, getTagProps) =>
                    value.map((option, index) => (
                      <Chip {...getTagProps({ index })} key={option} label={option} size="small" />
                    ))
                  }
                />
              ) : task.dependencies.length > 0 ? (
                <Stack spacing={1}>
                  {task.dependencies.map((depId) => (
                    <Box key={depId} sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                      <Iconify icon="solar:link-circle-bold" width={20} />
                      <Typography variant="body2">{depId}</Typography>
                    </Box>
                  ))}
                </Stack>
              ) : (
                <Typography variant="body2" color="text.secondary">
                  No dependencies
                </Typography>
              )}
            </Card>

            {/* Spec Reference */}
            <Card sx={{ p: 3 }}>
              <Typography variant="h6" sx={{ mb: 3 }}>
                Spec Reference
              </Typography>

              {isEditing ? (
                <TextField
                  fullWidth
                  label="Spec Reference"
                  value={editedTask.spec_reference || task.spec_reference || ''}
                  onChange={(e) => setEditedTask({ ...editedTask, spec_reference: e.target.value })}
                />
              ) : task.spec_reference ? (
                <Button
                  variant="outlined"
                  startIcon={<Iconify icon="solar:link-bold" />}
                  href={task.spec_reference}
                  target="_blank"
                  fullWidth
                >
                  View Spec
                </Button>
              ) : (
                <Typography variant="body2" color="text.secondary">
                  No spec reference
                </Typography>
              )}
            </Card>

            {/* Metadata */}
            <Card sx={{ p: 3 }}>
              <Typography variant="h6" sx={{ mb: 3 }}>
                Metadata
              </Typography>

              <Stack spacing={1}>
                <Box>
                  <Typography variant="caption" sx={{ color: 'text.secondary' }}>
                    Created
                  </Typography>
                  <Typography variant="body2">{new Date(task.created_at).toLocaleString()}</Typography>
                </Box>
                <Box>
                  <Typography variant="caption" sx={{ color: 'text.secondary' }}>
                    Updated
                  </Typography>
                  <Typography variant="body2">{new Date(task.updated_at).toLocaleString()}</Typography>
                </Box>
                {task.started_at && (
                  <Box>
                    <Typography variant="caption" sx={{ color: 'text.secondary' }}>
                      Started
                    </Typography>
                    <Typography variant="body2">{new Date(task.started_at).toLocaleString()}</Typography>
                  </Box>
                )}
                {task.completed_at && (
                  <Box>
                    <Typography variant="caption" sx={{ color: 'text.secondary' }}>
                      Completed
                    </Typography>
                    <Typography variant="body2">{new Date(task.completed_at).toLocaleString()}</Typography>
                  </Box>
                )}
                <Box>
                  <Typography variant="caption" sx={{ color: 'text.secondary' }}>
                    Task ID
                  </Typography>
                  <Typography variant="body2" sx={{ fontFamily: 'monospace', fontSize: 11 }}>
                    {task.id}
                  </Typography>
                </Box>
              </Stack>
            </Card>
          </Stack>
        </Grid>
      </Grid>
    </>
  );
}
