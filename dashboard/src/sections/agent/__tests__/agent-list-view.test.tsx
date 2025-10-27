import { it, vi, expect, describe, beforeEach } from 'vitest';

import * as axonClient from 'src/lib/axon-client';
import { render, screen, waitFor, fireEvent } from 'src/test/test-utils';

import { AgentListView } from '../agent-list-view';

// ----------------------------------------------------------------------
// Mock dependencies
// ----------------------------------------------------------------------

vi.mock('swr', () => ({
  default: vi.fn(),
  mutate: vi.fn(),
}));

vi.mock('src/lib/axon-client', () => ({
  axonClient: {
    deleteAgent: vi.fn(),
    pauseAgent: vi.fn(),
    resumeAgent: vi.fn(),
    restartAgent: vi.fn(),
  },
  axonFetcher: vi.fn(),
  axonEndpoints: {
    agents: {
      list: '/agents',
    },
  },
}));

vi.mock('src/components/snackbar', () => ({
  useSnackbar: () => ({
    showSnackbar: vi.fn(),
  }),
}));

vi.mock('src/utils/status-colors', () => ({
  getAgentStatusColor: (status: string) => {
    const colors: Record<string, any> = {
      Idle: 'default',
      Working: 'primary',
      Paused: 'warning',
      Failed: 'error',
    };
    return colors[status] || 'default';
  },
}));

// ----------------------------------------------------------------------
// Mock agent data
// ----------------------------------------------------------------------

const mockAgents = [
  {
    id: 'agent-1',
    name: 'Developer Agent',
    agent_type: 'Developer',
    status: 'Idle',
    capabilities: ['CodeGeneration', 'CodeReview', 'Testing'],
    metadata: {
      max_concurrent_tasks: 5,
      task_timeout_seconds: 300,
      tasks_completed: 15,
      tasks_failed: 2,
      total_execution_time_ms: 7500,
      avg_task_duration_ms: 500,
      created_at: '2024-01-01T00:00:00Z',
      last_active_at: '2024-01-02T00:00:00Z',
    },
    current_task: null,
  },
  {
    id: 'agent-2',
    name: 'Tester Agent',
    agent_type: 'Tester',
    status: 'Working',
    capabilities: ['Testing', 'CodeReview'],
    metadata: {
      max_concurrent_tasks: 3,
      task_timeout_seconds: 600,
      tasks_completed: 8,
      tasks_failed: 0,
      total_execution_time_ms: 4000,
      avg_task_duration_ms: 500,
      created_at: '2024-01-01T00:00:00Z',
      last_active_at: '2024-01-02T12:00:00Z',
    },
    current_task: 'Running tests',
  },
  {
    id: 'agent-3',
    name: 'Paused Agent',
    agent_type: 'Reviewer',
    status: 'Paused',
    capabilities: ['CodeReview'],
    metadata: {
      max_concurrent_tasks: 2,
      task_timeout_seconds: 300,
      tasks_completed: 5,
      tasks_failed: 1,
      total_execution_time_ms: 2500,
      avg_task_duration_ms: 500,
      created_at: '2024-01-01T00:00:00Z',
      last_active_at: '2024-01-01T18:00:00Z',
    },
    current_task: null,
  },
];

// ----------------------------------------------------------------------

describe('AgentListView', () => {
  beforeEach(async () => {
    vi.clearAllMocks();

    // Mock SWR to return our mock data
    const useSWR = await import('swr');
    vi.mocked(useSWR.default).mockReturnValue({
      data: mockAgents,
      error: undefined,
      isLoading: false,
      isValidating: false,
      mutate: vi.fn(),
    } as any);
  });

  // ----------------------------------------------------------------------
  // Rendering
  // ----------------------------------------------------------------------

  describe('Rendering', () => {
    it('should render the agents list', async () => {
      render(<AgentListView />);

      await waitFor(() => {
        expect(screen.getByText('Agents')).toBeInTheDocument();
      });
    });

    it('should render create agent button', async () => {
      render(<AgentListView />);

      await waitFor(() => {
        expect(screen.getByText('Create Agent')).toBeInTheDocument();
      });
    });

    it('should render all agents in the table', async () => {
      render(<AgentListView />);

      await waitFor(() => {
        expect(screen.getByText('Developer Agent')).toBeInTheDocument();
        expect(screen.getByText('Tester Agent')).toBeInTheDocument();
        expect(screen.getByText('Paused Agent')).toBeInTheDocument();
      });
    });

    it('should display agent types', async () => {
      render(<AgentListView />);

      await waitFor(() => {
        expect(screen.getByText('Developer')).toBeInTheDocument();
        expect(screen.getByText('Tester')).toBeInTheDocument();
        expect(screen.getByText('Reviewer')).toBeInTheDocument();
      });
    });

    it('should display agent statuses', async () => {
      render(<AgentListView />);

      await waitFor(() => {
        expect(screen.getByText('Idle')).toBeInTheDocument();
        expect(screen.getByText('Working')).toBeInTheDocument();
        expect(screen.getByText('Paused')).toBeInTheDocument();
      });
    });

    it('should display agent capabilities as chips', async () => {
      render(<AgentListView />);

      await waitFor(() => {
        expect(screen.getByText('CodeGeneration')).toBeInTheDocument();
        expect(screen.getByText('Testing')).toBeInTheDocument();
      });
    });

    it('should show +N chip for additional capabilities', async () => {
      render(<AgentListView />);

      await waitFor(() => {
        // Developer Agent has 3 capabilities, should show first 2 + "+1"
        expect(screen.getByText('+1')).toBeInTheDocument();
      });
    });

    it('should display task statistics', async () => {
      render(<AgentListView />);

      await waitFor(() => {
        expect(screen.getByText('15')).toBeInTheDocument(); // tasks_completed for agent-1
        expect(screen.getByText('8')).toBeInTheDocument(); // tasks_completed for agent-2
      });
    });

    it('should display failed tasks count', async () => {
      render(<AgentListView />);

      await waitFor(() => {
        expect(screen.getByText('(2 failed)')).toBeInTheDocument();
      });
    });

    it('should display average task duration', async () => {
      render(<AgentListView />);

      await waitFor(() => {
        const durations = screen.getAllByText('0.50s');
        expect(durations.length).toBeGreaterThan(0);
      });
    });
  });

  // ----------------------------------------------------------------------
  // Loading State
  // ----------------------------------------------------------------------

  describe('Loading State', () => {
    it('should handle loading state', async () => {
      const useSWR = await import('swr');
      vi.mocked(useSWR.default).mockReturnValue({
        data: undefined,
        error: undefined,
        isLoading: true,
        isValidating: false,
        mutate: vi.fn(),
      } as any);

      render(<AgentListView />);

      expect(screen.queryByText('Developer Agent')).not.toBeInTheDocument();
    });
  });

  // ----------------------------------------------------------------------
  // Empty State
  // ----------------------------------------------------------------------

  describe('Empty State', () => {
    it('should show empty state when no agents', async () => {
      const useSWR = await import('swr');
      vi.mocked(useSWR.default).mockReturnValue({
        data: [],
        error: undefined,
        isLoading: false,
        isValidating: false,
        mutate: vi.fn(),
      } as any);

      render(<AgentListView />);

      await waitFor(() => {
        expect(screen.getByText('Agents')).toBeInTheDocument();
      });
    });
  });

  // ----------------------------------------------------------------------
  // Agent Operations
  // ----------------------------------------------------------------------

  describe('Agent Operations', () => {
    it('should open popover menu when clicking more button', async () => {
      render(<AgentListView />);

      await waitFor(() => {
        const moreButtons = screen.getAllByRole('button');
        const moreButton = moreButtons.find(
          (btn) => btn.querySelector('svg') !== null
        );
        if (moreButton) {
          fireEvent.click(moreButton);
        }
      });

      await waitFor(() => {
        expect(screen.getByText('Delete')).toBeInTheDocument();
      });
    });

    it('should show pause option for active agents', async () => {
      render(<AgentListView />);

      await waitFor(() => {
        const moreButtons = screen.getAllByRole('button');
        const moreButton = moreButtons.find(
          (btn) => btn.querySelector('svg') !== null
        );
        if (moreButton) {
          fireEvent.click(moreButton);
        }
      });

      await waitFor(() => {
        // First agent should have Pause option
        expect(screen.getByText('Pause')).toBeInTheDocument();
      });
    });

    it('should call deleteAgent when delete is clicked', async () => {
      const mutate = await import('swr');
      const mockMutate = vi.fn();
      vi.mocked(mutate.mutate).mockImplementation(mockMutate);

      render(<AgentListView />);

      await waitFor(() => {
        const moreButtons = screen.getAllByRole('button');
        const moreButton = moreButtons.find(
          (btn) => btn.querySelector('svg') !== null
        );
        if (moreButton) {
          fireEvent.click(moreButton);
        }
      });

      await waitFor(() => {
        const deleteButton = screen.getByText('Delete');
        fireEvent.click(deleteButton);
      });

      await waitFor(() => {
        expect(axonClient.axonClient.deleteAgent).toHaveBeenCalled();
      });
    });

    it('should call pauseAgent when pause is clicked', async () => {
      render(<AgentListView />);

      await waitFor(() => {
        const moreButtons = screen.getAllByRole('button');
        const moreButton = moreButtons.find(
          (btn) => btn.querySelector('svg') !== null
        );
        if (moreButton) {
          fireEvent.click(moreButton);
        }
      });

      await waitFor(() => {
        const pauseButton = screen.getByText('Pause');
        fireEvent.click(pauseButton);
      });

      await waitFor(() => {
        expect(axonClient.axonClient.pauseAgent).toHaveBeenCalled();
      });
    });

    it('should call restartAgent when restart is clicked', async () => {
      render(<AgentListView />);

      await waitFor(() => {
        const moreButtons = screen.getAllByRole('button');
        const moreButton = moreButtons.find(
          (btn) => btn.querySelector('svg') !== null
        );
        if (moreButton) {
          fireEvent.click(moreButton);
        }
      });

      await waitFor(() => {
        const restartButton = screen.getByText('Restart');
        fireEvent.click(restartButton);
      });

      await waitFor(() => {
        expect(axonClient.axonClient.restartAgent).toHaveBeenCalled();
      });
    });
  });

  // ----------------------------------------------------------------------
  // Error Handling
  // ----------------------------------------------------------------------

  describe('Error Handling', () => {
    it('should handle delete error gracefully', async () => {
      vi.mocked(axonClient.axonClient.deleteAgent).mockRejectedValueOnce(
        new Error('Delete failed')
      );

      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      render(<AgentListView />);

      await waitFor(() => {
        const moreButtons = screen.getAllByRole('button');
        const moreButton = moreButtons.find(
          (btn) => btn.querySelector('svg') !== null
        );
        if (moreButton) {
          fireEvent.click(moreButton);
        }
      });

      await waitFor(() => {
        const deleteButton = screen.getByText('Delete');
        fireEvent.click(deleteButton);
      });

      await waitFor(() => {
        expect(consoleErrorSpy).toHaveBeenCalledWith(
          'Failed to delete agent:',
          expect.any(Error)
        );
      });

      consoleErrorSpy.mockRestore();
    });

    it('should handle pause error gracefully', async () => {
      vi.mocked(axonClient.axonClient.pauseAgent).mockRejectedValueOnce(
        new Error('Pause failed')
      );

      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      render(<AgentListView />);

      await waitFor(() => {
        const moreButtons = screen.getAllByRole('button');
        const moreButton = moreButtons.find(
          (btn) => btn.querySelector('svg') !== null
        );
        if (moreButton) {
          fireEvent.click(moreButton);
        }
      });

      await waitFor(() => {
        const pauseButton = screen.getByText('Pause');
        fireEvent.click(pauseButton);
      });

      await waitFor(() => {
        expect(consoleErrorSpy).toHaveBeenCalledWith(
          'Failed to pause agent:',
          expect.any(Error)
        );
      });

      consoleErrorSpy.mockRestore();
    });
  });

  // ----------------------------------------------------------------------
  // Pagination
  // ----------------------------------------------------------------------

  describe('Pagination', () => {
    it('should render pagination controls', async () => {
      render(<AgentListView />);

      await waitFor(() => {
        expect(screen.getByText(/rows per page/i)).toBeInTheDocument();
      });
    });

    it('should display correct total count', async () => {
      render(<AgentListView />);

      await waitFor(() => {
        expect(screen.getByText(/1–3 of 3/)).toBeInTheDocument();
      });
    });
  });
});
