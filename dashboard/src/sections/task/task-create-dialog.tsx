import type { TaskPriority } from 'src/types/axon';

import useSWR from 'swr';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z as zod } from 'zod';

import Box from '@mui/material/Box';
import Chip from '@mui/material/Chip';
import Stack from '@mui/material/Stack';
import Dialog from '@mui/material/Dialog';
import Button from '@mui/material/Button';
import MenuItem from '@mui/material/MenuItem';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import LoadingButton from '@mui/lab/LoadingButton';
import DialogTitle from '@mui/material/DialogTitle';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import Autocomplete from '@mui/material/Autocomplete';

import { axonClient, axonFetcher, axonEndpoints } from 'src/lib/axon-client';

import { Form, Field } from 'src/components/hook-form';

// ----------------------------------------------------------------------

const TaskSchema = zod.object({
  title: zod.string().min(1, 'Title is required'),
  description: zod.string().optional(),
  priority: zod.enum(['Critical', 'High', 'Medium', 'Low']),
  assigned_agents: zod.array(zod.string()).optional(),
  estimated_hours: zod.number().min(0).optional(),
  tags: zod.array(zod.string()).optional(),
  dependencies: zod.array(zod.string()).optional(),
  spec_reference: zod.string().optional(),
});

type TaskFormValues = zod.infer<typeof TaskSchema>;

// ----------------------------------------------------------------------

type TaskCreateDialogProps = {
  open: boolean;
  onClose: () => void;
  onSuccess: () => void;
};

export function TaskCreateDialog({ open, onClose, onSuccess }: TaskCreateDialogProps) {
  // Fetch agents for assignment
  const { data: agents = [] } = useSWR(axonEndpoints.agents.list, axonFetcher);

  // Fetch existing tasks for dependencies
  const { data: tasks = [] } = useSWR(axonEndpoints.tasks.list, axonFetcher);

  const defaultValues: TaskFormValues = {
    title: '',
    description: '',
    priority: 'Medium',
    assigned_agents: [],
    estimated_hours: undefined,
    tags: [],
    dependencies: [],
    spec_reference: '',
  };

  const methods = useForm<TaskFormValues>({
    resolver: zodResolver(TaskSchema),
    defaultValues,
  });

  const {
    handleSubmit,
    reset,
    formState: { isSubmitting },
  } = methods;

  const onSubmit = handleSubmit(async (data) => {
    try {
      await axonClient.createTask({
        title: data.title,
        description: data.description,
        priority: data.priority,
        assigned_agents: data.assigned_agents,
        estimated_hours: data.estimated_hours,
        tags: data.tags,
        dependencies: data.dependencies,
        spec_reference: data.spec_reference,
      });

      reset();
      onSuccess();
      onClose();
    } catch (error) {
      console.error('Failed to create task:', error);
    }
  });

  const handleClose = () => {
    reset();
    onClose();
  };

  return (
    <Dialog open={open} onClose={handleClose} maxWidth="md" fullWidth>
      <DialogTitle>Create New Task</DialogTitle>

      <Form methods={methods} onSubmit={onSubmit}>
        <DialogContent>
          <Stack spacing={3}>
            <Field.Text
              name="title"
              label="Title"
              placeholder="Enter task title"
              required
            />

            <Field.Text
              name="description"
              label="Description"
              placeholder="Enter task description"
              multiline
              rows={4}
            />

            <Field.Select name="priority" label="Priority" required>
              <MenuItem value="Critical">Critical</MenuItem>
              <MenuItem value="High">High</MenuItem>
              <MenuItem value="Medium">Medium</MenuItem>
              <MenuItem value="Low">Low</MenuItem>
            </Field.Select>

            <Field.Autocomplete
              name="assigned_agents"
              label="Assign Agents"
              multiple
              options={agents.map((agent: any) => agent.name)}
              placeholder="Select agents"
              ChipProps={{ size: 'small' }}
            />

            <Field.Text
              name="estimated_hours"
              label="Estimated Hours"
              type="number"
              placeholder="0"
              inputProps={{ min: 0, step: 0.5 }}
            />

            <Field.Autocomplete
              name="tags"
              label="Tags"
              multiple
              freeSolo
              options={[]}
              placeholder="Add tags"
              ChipProps={{ size: 'small' }}
            />

            <Field.Autocomplete
              name="dependencies"
              label="Dependencies"
              multiple
              options={tasks.map((task: any) => task.id)}
              getOptionLabel={(option) => {
                const task = tasks.find((t: any) => t.id === option);
                return task ? `${task.title} (${task.id})` : option;
              }}
              placeholder="Select dependent tasks"
              ChipProps={{ size: 'small' }}
            />

            <Field.Text
              name="spec_reference"
              label="Spec Reference"
              placeholder="URL or reference to specification"
            />
          </Stack>
        </DialogContent>

        <DialogActions>
          <Button variant="outlined" color="inherit" onClick={handleClose}>
            Cancel
          </Button>
          <LoadingButton type="submit" variant="contained" loading={isSubmitting}>
            Create Task
          </LoadingButton>
        </DialogActions>
      </Form>
    </Dialog>
  );
}
