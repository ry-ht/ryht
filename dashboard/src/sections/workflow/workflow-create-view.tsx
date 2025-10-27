import { useForm } from 'react-hook-form';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import LoadingButton from '@mui/lab/LoadingButton';

import { paths } from 'src/routes/paths';
import { useRouter } from 'src/routes/hooks';

import { axonClient } from 'src/lib/axon-client';

import { Iconify } from 'src/components/iconify';
import { Form, Field } from 'src/components/hook-form';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

type FormValues = {
  name: string;
  workflow_def: string;
  input_params: string;
};

const EXAMPLE_WORKFLOW = `# Example Workflow Definition
name: Code Review Workflow
description: Review and test code changes

tasks:
  - id: review
    agent_type: Reviewer
    action: review_code
    inputs:
      files: ["src/**/*.ts"]

  - id: test
    agent_type: Tester
    action: run_tests
    dependencies: [review]
    inputs:
      test_pattern: "**/*.test.ts"

  - id: document
    agent_type: Documenter
    action: update_docs
    dependencies: [review, test]
    inputs:
      output: "docs/api.md"
`;

const EXAMPLE_PARAMS = `{
  "repository": "https://github.com/example/repo",
  "branch": "main",
  "pull_request": "123"
}`;

export function WorkflowCreateView() {
  const router = useRouter();

  const defaultValues: FormValues = {
    name: '',
    workflow_def: EXAMPLE_WORKFLOW,
    input_params: EXAMPLE_PARAMS,
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
      // Parse input params as JSON
      let inputParams = {};
      try {
        inputParams = JSON.parse(data.input_params);
      } catch (err) {
        console.error('Invalid JSON in input params:', err);
        return;
      }

      await axonClient.runWorkflow({
        workflow_def: data.workflow_def,
        input_params: inputParams,
      });

      router.push(paths.dashboard.workflows.list);
    } catch (error) {
      console.error('Failed to run workflow:', error);
    }
  });

  return (
    <>
      <CustomBreadcrumbs
        heading="Run Workflow"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Workflows', href: '/workflows' },
          { name: 'Create' },
        ]}
        sx={{ mb: 3 }}
      />

      <Form methods={methods} onSubmit={onSubmit}>
        <Stack spacing={3}>
          <Alert severity="info">
            Define your workflow using YAML format. Specify tasks, dependencies, and agent types to
            orchestrate multi-agent collaboration.
          </Alert>

          <Card sx={{ p: 3 }}>
            <Stack spacing={3}>
              <Field.Text
                name="name"
                label="Workflow Name"
                placeholder="Enter a descriptive name for this workflow"
                helperText="Optional: If not provided, the name from the workflow definition will be used"
              />

              <Field.Text
                name="workflow_def"
                label="Workflow Definition (YAML)"
                multiline
                rows={16}
                placeholder="Enter your workflow definition in YAML format"
                required
              />

              <Field.Text
                name="input_params"
                label="Input Parameters (JSON)"
                multiline
                rows={8}
                placeholder='{"key": "value"}'
                helperText="Provide input parameters as a JSON object"
                required
              />

              <Box sx={{ display: 'flex', gap: 2, justifyContent: 'flex-end' }}>
                <Button variant="outlined" color="inherit" onClick={() => router.back()}>
                  Cancel
                </Button>
                <LoadingButton
                  type="submit"
                  variant="contained"
                  loading={isSubmitting}
                  startIcon={<Iconify icon="solar:play-bold" />}
                >
                  Run Workflow
                </LoadingButton>
              </Box>
            </Stack>
          </Card>
        </Stack>
      </Form>
    </>
  );
}
