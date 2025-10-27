import type { AgentInfo, AgentType } from 'src/types/axon';

import useSWR from 'swr';
import { useMemo } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import LinearProgress from '@mui/material/LinearProgress';

import { axonFetcher, axonEndpoints } from 'src/lib/axon-client';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { getAgentStatusColor } from 'src/utils/status-colors';

// ----------------------------------------------------------------------

type AgentTypeViewProps = {
  agentType: AgentType;
};

const AGENT_TYPE_INFO: Record<AgentType, { description: string; icon: string; color: string }> = {
  Orchestrator: {
    description: 'Coordinates workflows and delegates tasks across the multi-agent system',
    icon: 'mdi:sitemap',
    color: '#9C27B0',
  },
  Developer: {
    description: 'Generates code, implements features, and performs programming tasks',
    icon: 'mdi:code-braces',
    color: '#2196F3',
  },
  Reviewer: {
    description: 'Reviews code quality, identifies issues, and suggests improvements',
    icon: 'mdi:magnify',
    color: '#FF9800',
  },
  Tester: {
    description: 'Creates and executes tests to ensure code quality and correctness',
    icon: 'mdi:test-tube',
    color: '#4CAF50',
  },
  Documenter: {
    description: 'Generates documentation, comments, and technical specifications',
    icon: 'mdi:file-document',
    color: '#00BCD4',
  },
  Architect: {
    description: 'Designs system architecture and makes high-level technical decisions',
    icon: 'mdi:architecture',
    color: '#673AB7',
  },
  Researcher: {
    description: 'Investigates solutions, analyzes patterns, and explores new approaches',
    icon: 'mdi:book-search',
    color: '#E91E63',
  },
  Optimizer: {
    description: 'Optimizes performance, improves efficiency, and reduces resource usage',
    icon: 'mdi:speedometer',
    color: '#FF5722',
  },
};

export function AgentTypeView({ agentType }: AgentTypeViewProps) {
  const { data: agents = [], isLoading } = useSWR<AgentInfo[]>(
    axonEndpoints.agents.list,
    axonFetcher,
    { refreshInterval: 5000 }
  );

  const filteredAgents = useMemo(
    () => agents.filter((agent) => agent.agent_type === agentType),
    [agents, agentType]
  );

  const typeInfo = AGENT_TYPE_INFO[agentType];

  const stats = useMemo(() => {
    const total = filteredAgents.length;
    const idle = filteredAgents.filter((a) => a.status === 'Idle').length;
    const working = filteredAgents.filter((a) => a.status === 'Working').length;
    const paused = filteredAgents.filter((a) => a.status === 'Paused').length;
    const failed = filteredAgents.filter((a) => a.status === 'Failed').length;

    const totalTasks = filteredAgents.reduce((sum, a) => sum + a.metadata.tasks_completed, 0);
    const totalFailed = filteredAgents.reduce((sum, a) => sum + a.metadata.tasks_failed, 0);
    const avgDuration = filteredAgents.length
      ? filteredAgents.reduce((sum, a) => sum + a.metadata.avg_task_duration_ms, 0) /
        filteredAgents.length
      : 0;
    const successRate =
      totalTasks + totalFailed > 0 ? (totalTasks / (totalTasks + totalFailed)) * 100 : 0;

    return {
      total,
      idle,
      working,
      paused,
      failed,
      totalTasks,
      totalFailed,
      avgDuration,
      successRate,
    };
  }, [filteredAgents]);

  return (
    <>
      <CustomBreadcrumbs
        heading={`${agentType} Agents`}
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Agents', href: '/agents' },
          { name: agentType },
        ]}
        sx={{ mb: 3 }}
      />

      <Stack spacing={2} sx={{ mb: 3 }}>
        <Card sx={{ p: 3 }}>
          <Stack direction="row" spacing={2} alignItems="center" sx={{ mb: 2 }}>
            <Box
              sx={{
                width: 64,
                height: 64,
                borderRadius: 2,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                bgcolor: `${typeInfo.color}20`,
                color: typeInfo.color,
              }}
            >
              <Iconify icon={typeInfo.icon} width={40} />
            </Box>
            <Box>
              <Typography variant="h4">{agentType} Agents</Typography>
              <Typography variant="body2" color="text.secondary">
                {typeInfo.description}
              </Typography>
            </Box>
          </Stack>

          {isLoading && <LinearProgress sx={{ mb: 2 }} />}

          <Grid container spacing={2}>
            <Grid size={{ xs: 6, md: 3 }}>
              <Stack spacing={0.5}>
                <Typography variant="body2" color="text.secondary">
                  Total Agents
                </Typography>
                <Typography variant="h4">{stats.total}</Typography>
              </Stack>
            </Grid>
            <Grid size={{ xs: 6, md: 3 }}>
              <Stack spacing={0.5}>
                <Typography variant="body2" color="text.secondary">
                  Working
                </Typography>
                <Typography variant="h4" color="info.main">
                  {stats.working}
                </Typography>
              </Stack>
            </Grid>
            <Grid size={{ xs: 6, md: 3 }}>
              <Stack spacing={0.5}>
                <Typography variant="body2" color="text.secondary">
                  Tasks Completed
                </Typography>
                <Typography variant="h4">{stats.totalTasks}</Typography>
              </Stack>
            </Grid>
            <Grid size={{ xs: 6, md: 3 }}>
              <Stack spacing={0.5}>
                <Typography variant="body2" color="text.secondary">
                  Success Rate
                </Typography>
                <Typography
                  variant="h4"
                  color={stats.successRate >= 80 ? 'success.main' : 'warning.main'}
                >
                  {stats.successRate.toFixed(1)}%
                </Typography>
              </Stack>
            </Grid>
          </Grid>
        </Card>

        <Grid container spacing={2}>
          {filteredAgents.map((agent) => (
            <Grid size={{ xs: 12, md: 6, lg: 4 }} key={agent.id}>
              <Card sx={{ p: 2.5, height: '100%' }}>
                <Stack spacing={2}>
                  <Stack direction="row" justifyContent="space-between" alignItems="flex-start">
                    <Box>
                      <Typography variant="h6">{agent.name}</Typography>
                      <Typography variant="caption" color="text.disabled">
                        {agent.id.substring(0, 8)}
                      </Typography>
                    </Box>
                    <Label variant="soft" color={getAgentStatusColor(agent.status)}>
                      {agent.status}
                    </Label>
                  </Stack>

                  {agent.current_task && (
                    <Box>
                      <Typography variant="caption" color="text.secondary">
                        Current Task:
                      </Typography>
                      <Typography variant="body2" noWrap>
                        {agent.current_task}
                      </Typography>
                    </Box>
                  )}

                  <Stack direction="row" spacing={2}>
                    <Stack spacing={0.5} flex={1}>
                      <Typography variant="caption" color="text.secondary">
                        Completed
                      </Typography>
                      <Typography variant="subtitle2">
                        {agent.metadata.tasks_completed}
                      </Typography>
                    </Stack>
                    <Stack spacing={0.5} flex={1}>
                      <Typography variant="caption" color="text.secondary">
                        Failed
                      </Typography>
                      <Typography variant="subtitle2" color="error.main">
                        {agent.metadata.tasks_failed}
                      </Typography>
                    </Stack>
                    <Stack spacing={0.5} flex={1}>
                      <Typography variant="caption" color="text.secondary">
                        Avg Duration
                      </Typography>
                      <Typography variant="subtitle2">
                        {agent.metadata.avg_task_duration_ms > 0
                          ? `${(agent.metadata.avg_task_duration_ms / 1000).toFixed(1)}s`
                          : '-'}
                      </Typography>
                    </Stack>
                  </Stack>

                  <Box>
                    <Typography variant="caption" color="text.secondary">
                      Capabilities
                    </Typography>
                    <Stack direction="row" spacing={0.5} flexWrap="wrap" gap={0.5} mt={0.5}>
                      {agent.capabilities.map((cap) => (
                        <Label key={cap} variant="soft">
                          {cap}
                        </Label>
                      ))}
                    </Stack>
                  </Box>
                </Stack>
              </Card>
            </Grid>
          ))}
        </Grid>

        {!isLoading && filteredAgents.length === 0 && (
          <Card sx={{ p: 5, textAlign: 'center' }}>
            <Iconify icon="mdi:robot-off" width={64} sx={{ color: 'text.disabled', mb: 2 }} />
            <Typography variant="h6" color="text.secondary">
              No {agentType} agents found
            </Typography>
            <Typography variant="body2" color="text.disabled" sx={{ mt: 1 }}>
              Create a new {agentType} agent to get started
            </Typography>
          </Card>
        )}
      </Stack>
    </>
  );
}
