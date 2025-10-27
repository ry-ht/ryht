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

const WORKFLOW_TEMPLATES = [
  {
    id: 'full-feature',
    name: 'Full Feature Development',
    description: 'Complete feature implementation with code, tests, and documentation',
    requiredAgents: ['Orchestrator', 'Developer', 'Tester', 'Reviewer', 'Documenter'],
    estimatedDuration: '2-4 hours',
    complexity: 'high',
    icon: 'mdi:puzzle',
  },
  {
    id: 'bug-fix',
    name: 'Bug Fix Workflow',
    description: 'Identify, fix, and test bug resolutions',
    requiredAgents: ['Developer', 'Tester', 'Reviewer'],
    estimatedDuration: '30-60 min',
    complexity: 'medium',
    icon: 'mdi:bug-outline',
  },
  {
    id: 'code-review',
    name: 'Code Review',
    description: 'Comprehensive code review with suggestions',
    requiredAgents: ['Reviewer', 'Architect'],
    estimatedDuration: '15-30 min',
    complexity: 'low',
    icon: 'mdi:magnify',
  },
  {
    id: 'refactoring',
    name: 'Code Refactoring',
    description: 'Refactor existing code for better quality and performance',
    requiredAgents: ['Architect', 'Developer', 'Tester', 'Optimizer'],
    estimatedDuration: '1-3 hours',
    complexity: 'high',
    icon: 'mdi:refresh',
  },
  {
    id: 'documentation',
    name: 'Documentation Generation',
    description: 'Generate comprehensive documentation for codebase',
    requiredAgents: ['Documenter', 'Researcher'],
    estimatedDuration: '1-2 hours',
    complexity: 'medium',
    icon: 'mdi:file-document',
  },
  {
    id: 'testing',
    name: 'Test Suite Creation',
    description: 'Create comprehensive test coverage for code',
    requiredAgents: ['Tester', 'Developer'],
    estimatedDuration: '1-2 hours',
    complexity: 'medium',
    icon: 'mdi:test-tube',
  },
];

export function WorkflowTemplatesView() {
  const [selectedTemplate, setSelectedTemplate] = useState<string | null>(null);

  const getComplexityColor = (complexity: string) => {
    if (complexity === 'high') return 'error';
    if (complexity === 'medium') return 'warning';
    return 'success';
  };

  return (
    <>
      <CustomBreadcrumbs
        heading="Workflow Templates"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Workflows', href: '/workflows' },
          { name: 'Templates' },
        ]}
        sx={{ mb: 3 }}
      />

      <Stack spacing={2}>
        <Card sx={{ p: 3 }}>
          <Typography variant="h6" sx={{ mb: 1 }}>
            Available Workflow Templates
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Choose a pre-configured workflow template to quickly start a new workflow with the right
            agents and structure.
          </Typography>
        </Card>

        <Grid container spacing={2}>
          {WORKFLOW_TEMPLATES.map((template) => (
            <Grid item xs={12} md={6} lg={4} key={template.id}>
              <Card
                sx={{
                  p: 2.5,
                  height: '100%',
                  cursor: 'pointer',
                  border: 2,
                  borderColor:
                    selectedTemplate === template.id ? 'primary.main' : 'transparent',
                  transition: 'all 0.2s',
                  '&:hover': {
                    borderColor: 'primary.light',
                    transform: 'translateY(-4px)',
                    boxShadow: 3,
                  },
                }}
                onClick={() => setSelectedTemplate(template.id)}
              >
                <Stack spacing={2}>
                  <Stack direction="row" justifyContent="space-between" alignItems="flex-start">
                    <Box
                      sx={{
                        width: 48,
                        height: 48,
                        borderRadius: 1.5,
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        bgcolor: 'primary.lighter',
                        color: 'primary.main',
                      }}
                    >
                      <Iconify icon={template.icon} width={28} />
                    </Box>
                    <Label variant="soft" color={getComplexityColor(template.complexity)}>
                      {template.complexity}
                    </Label>
                  </Stack>

                  <Box>
                    <Typography variant="h6" sx={{ mb: 0.5 }}>
                      {template.name}
                    </Typography>
                    <Typography variant="body2" color="text.secondary">
                      {template.description}
                    </Typography>
                  </Box>

                  <Stack spacing={1}>
                    <Stack direction="row" spacing={1} alignItems="center">
                      <Iconify icon="mdi:clock-outline" width={16} color="text.secondary" />
                      <Typography variant="caption" color="text.secondary">
                        {template.estimatedDuration}
                      </Typography>
                    </Stack>
                    <Stack direction="row" spacing={1} alignItems="center">
                      <Iconify icon="mdi:account-group" width={16} color="text.secondary" />
                      <Typography variant="caption" color="text.secondary">
                        {template.requiredAgents.length} agents required
                      </Typography>
                    </Stack>
                  </Stack>

                  <Box>
                    <Typography variant="caption" color="text.secondary" sx={{ mb: 0.5, display: 'block' }}>
                      Required Agents:
                    </Typography>
                    <Stack direction="row" spacing={0.5} flexWrap="wrap" gap={0.5}>
                      {template.requiredAgents.map((agent) => (
                        <Label key={agent} variant="soft" size="small">
                          {agent}
                        </Label>
                      ))}
                    </Stack>
                  </Box>

                  <Button
                    variant={selectedTemplate === template.id ? 'contained' : 'outlined'}
                    fullWidth
                    startIcon={<Iconify icon="mdi:play" />}
                  >
                    Create Workflow
                  </Button>
                </Stack>
              </Card>
            </Grid>
          ))}
        </Grid>
      </Stack>
    </>
  );
}
