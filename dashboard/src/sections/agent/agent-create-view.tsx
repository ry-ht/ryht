import type { AgentType } from 'src/types/axon';

import { useForm } from 'react-hook-form';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import MenuItem from '@mui/material/MenuItem';
import Typography from '@mui/material/Typography';
import LoadingButton from '@mui/lab/LoadingButton';

import { paths } from 'src/routes/paths';
import { useRouter } from 'src/routes/hooks';

import { axonClient } from 'src/lib/axon-client';

import { Iconify } from 'src/components/iconify';
import { Form, Field } from 'src/components/hook-form';

// ----------------------------------------------------------------------

const AGENT_TYPES: { value: AgentType; label: string; description: string }[] = [
  {
    value: 'Orchestrator',
    label: 'Orchestrator',
    description: 'Master coordination and task delegation',
  },
  {
    value: 'Developer',
    label: 'Developer',
    description: 'Code generation, modification, and refactoring',
  },
  {
    value: 'Reviewer',
    label: 'Reviewer',
    description: 'Code review, quality assessment, and validation',
  },
  {
    value: 'Tester',
    label: 'Tester',
    description: 'Test generation, execution, and validation',
  },
  {
    value: 'Documenter',
    label: 'Documenter',
    description: 'Documentation generation and maintenance',
  },
  {
    value: 'Architect',
    label: 'Architect',
    description: 'System design and architecture planning',
  },
  {
    value: 'Researcher',
    label: 'Researcher',
    description: 'Information gathering and analysis',
  },
  {
    value: 'Optimizer',
    label: 'Optimizer',
    description: 'Performance and cost optimization',
  },
];

const CAPABILITIES = [
  { value: 'coding', label: 'Code Generation' },
  { value: 'review', label: 'Code Review' },
  { value: 'testing', label: 'Testing' },
  { value: 'docs', label: 'Documentation' },
  { value: 'debugging', label: 'Debugging' },
  { value: 'analysis', label: 'Code Analysis' },
];

// ----------------------------------------------------------------------

type FormValues = {
  name: string;
  agent_type: AgentType;
  capabilities: string[];
  max_concurrent_tasks: number;
};

export function AgentCreateView() {
  const router = useRouter();

  const defaultValues: FormValues = {
    name: '',
    agent_type: 'Developer',
    capabilities: ['coding'],
    max_concurrent_tasks: 1,
  };

  const methods = useForm<FormValues>({
    defaultValues,
  });

  const {
    handleSubmit,
    formState: { isSubmitting },
  } = methods;

  const onSubmit = handleSubmit(async (data) => {
    try {
      await axonClient.createAgent({
        name: data.name,
        agent_type: data.agent_type,
        capabilities: data.capabilities,
        max_concurrent_tasks: data.max_concurrent_tasks,
      });

      router.push(paths.dashboard.agents.list);
    } catch (error) {
      console.error('Failed to create agent:', error);
    }
  });

  return (
    <>
      <Box sx={{ display: 'flex', alignItems: 'center', mb: 5 }}>
        <Typography variant="h4" sx={{ flexGrow: 1 }}>
          Create New Agent
        </Typography>
        <Button
          variant="outlined"
          startIcon={<Iconify icon="eva:arrow-ios-back-fill" />}
          onClick={() => router.back()}
        >
          Back
        </Button>
      </Box>

      <Form methods={methods} onSubmit={onSubmit}>
        <Card sx={{ p: 3 }}>
          <Stack spacing={3}>
            <Field.Text
              name="name"
              label="Agent Name"
              placeholder="Enter a unique name for this agent"
              required
            />

            <Field.Select name="agent_type" label="Agent Type" required>
              {AGENT_TYPES.map((type) => (
                <MenuItem key={type.value} value={type.value}>
                  <Box>
                    <Typography variant="body2">{type.label}</Typography>
                    <Typography variant="caption" sx={{ color: 'text.secondary' }}>
                      {type.description}
                    </Typography>
                  </Box>
                </MenuItem>
              ))}
            </Field.Select>

            <Field.MultiSelect
              name="capabilities"
              label="Capabilities"
              options={CAPABILITIES}
              checkbox
              chip
              helperText="Select one or more capabilities for this agent"
            />

            <Field.Text
              name="max_concurrent_tasks"
              label="Max Concurrent Tasks"
              type="number"
              inputProps={{ min: 1, max: 10 }}
              helperText="Maximum number of tasks this agent can handle simultaneously"
            />

            <Box sx={{ display: 'flex', gap: 2, justifyContent: 'flex-end' }}>
              <Button variant="outlined" color="inherit" onClick={() => router.back()}>
                Cancel
              </Button>
              <LoadingButton
                type="submit"
                variant="contained"
                loading={isSubmitting}
                startIcon={<Iconify icon="mingcute:add-line" />}
              >
                Create Agent
              </LoadingButton>
            </Box>
          </Stack>
        </Card>
      </Form>
    </>
  );
}
