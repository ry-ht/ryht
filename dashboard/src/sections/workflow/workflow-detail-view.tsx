import type { WorkflowStatusDetail, WorkflowTask } from 'src/types/axon';

import useSWR, { mutate } from 'swr';
import { useState, useCallback } from 'react';
import { useParams } from 'react-router';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import MenuItem from '@mui/material/MenuItem';
import Accordion from '@mui/material/Accordion';
import Typography from '@mui/material/Typography';
import LinearProgress from '@mui/material/LinearProgress';
import AccordionSummary from '@mui/material/AccordionSummary';
import AccordionDetails from '@mui/material/AccordionDetails';

import { paths } from 'src/routes/paths';
import { useRouter } from 'src/routes/hooks';

import { axonClient, axonFetcher, axonEndpoints } from 'src/lib/axon-client';
import { getWorkflowStatusColor } from 'src/utils/status-colors';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CustomPopover, usePopover } from 'src/components/custom-popover';

// import { WorkflowVisualizer } from './workflow-visualizer';

// ----------------------------------------------------------------------

export function WorkflowDetailView() {
  const router = useRouter();
  const params = useParams();
  const popover = usePopover();
  const workflowId = params.id as string;

  const [expandedTask, setExpandedTask] = useState<string | false>(false);

  // Fetch workflow details
  const { data: workflow, isLoading } = useSWR<WorkflowStatusDetail>(
    workflowId ? axonEndpoints.workflows.details(workflowId) : null,
    axonFetcher,
    { refreshInterval: 3000 }
  );

  const handleCancelWorkflow = useCallback(async () => {
    if (!workflow) return;

    try {
      await axonClient.cancelWorkflow(workflow.id);
      mutate(axonEndpoints.workflows.details(workflow.id));
      mutate(axonEndpoints.workflows.list);
      popover.onClose();
    } catch (err) {
      console.error('Failed to cancel workflow:', err);
    }
  }, [workflow, popover]);

  const handlePauseWorkflow = useCallback(async () => {
    if (!workflow) return;

    try {
      await axonClient.pauseWorkflow(workflow.id);
      mutate(axonEndpoints.workflows.details(workflow.id));
      mutate(axonEndpoints.workflows.list);
      popover.onClose();
    } catch (err) {
      console.error('Failed to pause workflow:', err);
    }
  }, [workflow, popover]);

  const handleResumeWorkflow = useCallback(async () => {
    if (!workflow) return;

    try {
      await axonClient.resumeWorkflow(workflow.id);
      mutate(axonEndpoints.workflows.details(workflow.id));
      mutate(axonEndpoints.workflows.list);
      popover.onClose();
    } catch (err) {
      console.error('Failed to resume workflow:', err);
    }
  }, [workflow, popover]);

  const calculateProgress = useCallback(() => {
    if (!workflow || workflow.tasks.length === 0) return 0;
    const completed = workflow.tasks.filter((t) => t.status === 'Completed').length;
    return (completed / workflow.tasks.length) * 100;
  }, [workflow]);

  const calculateDuration = useCallback(() => {
    if (!workflow || !workflow.started_at) return '-';

    const start = new Date(workflow.started_at).getTime();
    const end = workflow.completed_at ? new Date(workflow.completed_at).getTime() : Date.now();
    const durationMs = end - start;
    const seconds = Math.floor(durationMs / 1000);

    if (seconds < 60) return `${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m ${seconds % 60}s`;
    const hours = Math.floor(minutes / 60);
    return `${hours}h ${minutes % 60}m`;
  }, [workflow]);

  if (isLoading) {
    return <LinearProgress />;
  }

  if (!workflow) {
    return (
      <Box sx={{ textAlign: 'center', py: 10 }}>
        <Typography variant="h6" color="text.secondary">
          Workflow not found
        </Typography>
      </Box>
    );
  }

  const progress = calculateProgress();
  const duration = calculateDuration();

  return (
    <>
      <Box sx={{ display: 'flex', alignItems: 'center', mb: 5 }}>
        <Typography variant="h4" sx={{ flexGrow: 1 }}>
          Workflow Details
        </Typography>
        <Stack direction="row" spacing={2}>
          <Button
            variant="outlined"
            startIcon={<Iconify icon="eva:arrow-ios-back-fill" />}
            onClick={() => router.push(paths.dashboard.workflows.list)}
          >
            Back
          </Button>
          <Button
            variant="outlined"
            startIcon={<Iconify icon="eva:more-vertical-fill" />}
            onClick={popover.onOpen}
          >
            Actions
          </Button>
        </Stack>
      </Box>

      <Grid container spacing={3}>
        <Grid item xs={12} md={8}>
          <Stack spacing={3}>
            {/* Workflow Information */}
            <Card sx={{ p: 3 }}>
              <Stack spacing={3}>
                <Box>
                  <Typography variant="h5" sx={{ mb: 1 }}>
                    {workflow.name}
                  </Typography>
                  <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
                    <Label variant="soft" color={getWorkflowStatusColor(workflow.status)}>
                      {workflow.status}
                    </Label>
                    <Typography variant="caption" sx={{ color: 'text.disabled' }}>
                      ID: {workflow.id}
                    </Typography>
                  </Box>
                </Box>

                <Box>
                  <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 1 }}>
                    <Typography variant="body2" sx={{ color: 'text.secondary' }}>
                      Progress
                    </Typography>
                    <Typography variant="body2" sx={{ color: 'text.secondary' }}>
                      {progress.toFixed(0)}% ({workflow.tasks.filter((t) => t.status === 'Completed').length} /{' '}
                      {workflow.tasks.length} tasks)
                    </Typography>
                  </Box>
                  <LinearProgress variant="determinate" value={progress} sx={{ height: 8, borderRadius: 1 }} />
                </Box>

                {workflow.error && (
                  <Box
                    sx={{
                      p: 2,
                      borderRadius: 1,
                      bgcolor: 'error.lighter',
                      border: '1px solid',
                      borderColor: 'error.light',
                    }}
                  >
                    <Typography variant="subtitle2" sx={{ color: 'error.dark', mb: 0.5 }}>
                      Error
                    </Typography>
                    <Typography variant="body2" sx={{ color: 'error.dark' }}>
                      {workflow.error}
                    </Typography>
                  </Box>
                )}
              </Stack>
            </Card>

            {/* Workflow DAG Visualization */}
            {/* Uncomment when mermaid is installed */}
            {/* <WorkflowVisualizer tasks={workflow.tasks} /> */}

            {/* Task List */}
            <Card sx={{ p: 3 }}>
              <Typography variant="h6" sx={{ mb: 3 }}>
                Tasks
              </Typography>

              <Stack spacing={1}>
                {workflow.tasks.map((task) => (
                  <TaskItem
                    key={task.id}
                    task={task}
                    expanded={expandedTask === task.id}
                    onToggle={() => setExpandedTask(expandedTask === task.id ? false : task.id)}
                  />
                ))}
              </Stack>
            </Card>
          </Stack>
        </Grid>

        <Grid item xs={12} md={4}>
          <Stack spacing={3}>
            {/* Metadata */}
            <Card sx={{ p: 3 }}>
              <Typography variant="h6" sx={{ mb: 3 }}>
                Metadata
              </Typography>

              <Stack spacing={2}>
                <Box>
                  <Typography variant="caption" sx={{ color: 'text.secondary', mb: 0.5, display: 'block' }}>
                    Duration
                  </Typography>
                  <Typography variant="body2">{duration}</Typography>
                </Box>

                <Box>
                  <Typography variant="caption" sx={{ color: 'text.secondary', mb: 0.5, display: 'block' }}>
                    Created
                  </Typography>
                  <Typography variant="body2">{new Date(workflow.created_at).toLocaleString()}</Typography>
                </Box>

                {workflow.started_at && (
                  <Box>
                    <Typography variant="caption" sx={{ color: 'text.secondary', mb: 0.5, display: 'block' }}>
                      Started
                    </Typography>
                    <Typography variant="body2">{new Date(workflow.started_at).toLocaleString()}</Typography>
                  </Box>
                )}

                {workflow.completed_at && (
                  <Box>
                    <Typography variant="caption" sx={{ color: 'text.secondary', mb: 0.5, display: 'block' }}>
                      Completed
                    </Typography>
                    <Typography variant="body2">{new Date(workflow.completed_at).toLocaleString()}</Typography>
                  </Box>
                )}
              </Stack>
            </Card>

            {/* Statistics */}
            <Card sx={{ p: 3 }}>
              <Typography variant="h6" sx={{ mb: 3 }}>
                Statistics
              </Typography>

              <Stack spacing={2}>
                <Box>
                  <Typography variant="caption" sx={{ color: 'text.secondary' }}>
                    Total Tasks
                  </Typography>
                  <Typography variant="h4">{workflow.tasks.length}</Typography>
                </Box>

                <Box>
                  <Typography variant="caption" sx={{ color: 'text.secondary' }}>
                    Completed
                  </Typography>
                  <Typography variant="h4" sx={{ color: 'success.main' }}>
                    {workflow.tasks.filter((t) => t.status === 'Completed').length}
                  </Typography>
                </Box>

                <Box>
                  <Typography variant="caption" sx={{ color: 'text.secondary' }}>
                    Running
                  </Typography>
                  <Typography variant="h4" sx={{ color: 'info.main' }}>
                    {workflow.tasks.filter((t) => t.status === 'Running').length}
                  </Typography>
                </Box>

                <Box>
                  <Typography variant="caption" sx={{ color: 'text.secondary' }}>
                    Failed
                  </Typography>
                  <Typography variant="h4" sx={{ color: 'error.main' }}>
                    {workflow.tasks.filter((t) => t.status === 'Failed').length}
                  </Typography>
                </Box>
              </Stack>
            </Card>
          </Stack>
        </Grid>
      </Grid>

      <CustomPopover
        open={popover.open}
        anchorEl={popover.anchorEl}
        onClose={popover.onClose}
        slotProps={{ arrow: { placement: 'right-top' } }}
      >
        {workflow.status === 'Running' && (
          <>
            <MenuItem onClick={handlePauseWorkflow}>
              <Iconify icon="solar:pause-circle-bold" />
              Pause Workflow
            </MenuItem>
            <MenuItem onClick={handleCancelWorkflow} sx={{ color: 'error.main' }}>
              <Iconify icon="solar:close-circle-bold" />
              Cancel Workflow
            </MenuItem>
          </>
        )}

        {workflow.status === 'Paused' && (
          <MenuItem onClick={handleResumeWorkflow}>
            <Iconify icon="solar:play-circle-bold" />
            Resume Workflow
          </MenuItem>
        )}
      </CustomPopover>
    </>
  );
}

// ----------------------------------------------------------------------

type TaskItemProps = {
  task: WorkflowTask;
  expanded: boolean;
  onToggle: () => void;
};

function TaskItem({ task, expanded, onToggle }: TaskItemProps) {
  return (
    <Accordion expanded={expanded} onChange={onToggle}>
      <AccordionSummary expandIcon={<Iconify icon="eva:arrow-ios-downward-fill" />}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, width: '100%', pr: 2 }}>
          <Label variant="soft" color={getWorkflowStatusColor(task.status)}>
            {task.status}
          </Label>
          <Typography variant="subtitle2" sx={{ flexGrow: 1 }}>
            {task.name}
          </Typography>
          <Typography variant="caption" sx={{ color: 'text.disabled' }}>
            {task.agent_type}
          </Typography>
        </Box>
      </AccordionSummary>

      <AccordionDetails>
        <Stack spacing={2}>
          <Box>
            <Typography variant="caption" sx={{ color: 'text.secondary', mb: 0.5, display: 'block' }}>
              Task ID
            </Typography>
            <Typography variant="body2" sx={{ fontFamily: 'monospace', fontSize: 11 }}>
              {task.id}
            </Typography>
          </Box>

          {task.dependencies.length > 0 && (
            <Box>
              <Typography variant="caption" sx={{ color: 'text.secondary', mb: 0.5, display: 'block' }}>
                Dependencies
              </Typography>
              <Box sx={{ display: 'flex', gap: 0.5, flexWrap: 'wrap' }}>
                {task.dependencies.map((dep) => (
                  <Label key={dep} variant="soft" color="default" sx={{ fontSize: 10 }}>
                    {dep}
                  </Label>
                ))}
              </Box>
            </Box>
          )}

          {task.started_at && (
            <Box>
              <Typography variant="caption" sx={{ color: 'text.secondary', mb: 0.5, display: 'block' }}>
                Started
              </Typography>
              <Typography variant="body2">{new Date(task.started_at).toLocaleString()}</Typography>
            </Box>
          )}

          {task.completed_at && (
            <Box>
              <Typography variant="caption" sx={{ color: 'text.secondary', mb: 0.5, display: 'block' }}>
                Completed
              </Typography>
              <Typography variant="body2">{new Date(task.completed_at).toLocaleString()}</Typography>
            </Box>
          )}

          {task.error && (
            <Box
              sx={{
                p: 2,
                borderRadius: 1,
                bgcolor: 'error.lighter',
                border: '1px solid',
                borderColor: 'error.light',
              }}
            >
              <Typography variant="caption" sx={{ color: 'error.dark', mb: 0.5, display: 'block' }}>
                Error
              </Typography>
              <Typography variant="body2" sx={{ color: 'error.dark' }}>
                {task.error}
              </Typography>
            </Box>
          )}

          {task.result && (
            <Box>
              <Typography variant="caption" sx={{ color: 'text.secondary', mb: 0.5, display: 'block' }}>
                Result
              </Typography>
              <Box
                sx={{
                  p: 2,
                  borderRadius: 1,
                  bgcolor: 'background.neutral',
                  fontFamily: 'monospace',
                  fontSize: 11,
                  overflow: 'auto',
                }}
              >
                <pre style={{ margin: 0 }}>{JSON.stringify(task.result, null, 2)}</pre>
              </Box>
            </Box>
          )}
        </Stack>
      </AccordionDetails>
    </Accordion>
  );
}
