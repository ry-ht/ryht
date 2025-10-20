# Axon: Dashboard UI - Complete Specification

## Overview

The Axon Dashboard is a native desktop application built with Tauri (Rust backend) + React (TypeScript frontend), providing real-time visualization and control of the multi-agent orchestration system. The dashboard connects to both Axon's orchestration engine and Cortex's REST API for comprehensive system monitoring and management.

**Technology Stack:**
- **Backend**: Tauri (Rust) - Native desktop runtime
- **Frontend**: React 18 + TypeScript
- **State Management**: Zustand
- **UI Framework**: Tailwind CSS + shadcn/ui components
- **Charts**: Recharts + D3.js for advanced visualizations
- **Real-time**: WebSocket connections to Cortex
- **Build**: Vite

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    Axon Desktop App (Tauri)                   │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                 React Frontend Layer                    │  │
│  │              (TypeScript + Tailwind)                    │  │
│  │                                                          │  │
│  │  ┌──────────────────────────────────────────────────┐  │  │
│  │  │             Dashboard Views                       │  │  │
│  │  │                                                    │  │  │
│  │  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌───────────┐ │  │  │
│  │  │  │Overview│ │ Agents │ │Workflow│ │  Metrics  │ │  │  │
│  │  │  │  View  │ │  View  │ │  View  │ │   View    │ │  │  │
│  │  │  └────────┘ └────────┘ └────────┘ └───────────┘ │  │  │
│  │  │                                                    │  │  │
│  │  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌───────────┐ │  │  │
│  │  │  │ Memory │ │  Logs  │ │Settings│ │ Terminal  │ │  │  │
│  │  │  │  View  │ │  View  │ │  View  │ │   View    │ │  │  │
│  │  │  └────────┘ └────────┘ └────────┘ └───────────┘ │  │  │
│  │  └──────────────────────────────────────────────────┘  │  │
│  │                                                          │  │
│  │  ┌──────────────────────────────────────────────────┐  │  │
│  │  │          State Management (Zustand)              │  │  │
│  │  │                                                    │  │  │
│  │  │  - Agent Store      - Workflow Store             │  │  │
│  │  │  - Metrics Store    - Memory Store               │  │  │
│  │  │  - UI Store         - Settings Store             │  │  │
│  │  └──────────────────────────────────────────────────┘  │  │
│  │                                                          │  │
│  │  ┌──────────────────────────────────────────────────┐  │  │
│  │  │           Real-time Data Hooks                   │  │  │
│  │  │                                                    │  │  │
│  │  │  - useAgents()      - useWorkflows()             │  │  │
│  │  │  - useMetrics()     - useCortexData()            │  │  │
│  │  │  - useWebSocket()   - useSystemHealth()          │  │  │
│  │  └──────────────────────────────────────────────────┘  │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                               │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                Tauri Backend Layer (Rust)              │  │
│  │                                                          │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │  │
│  │  │   Axon API   │  │  Cortex API  │  │   System     │ │  │
│  │  │   Commands   │  │   Proxy      │  │   Commands   │ │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘ │  │
│  │                                                          │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │  │
│  │  │  WebSocket   │  │    Event     │  │    File      │ │  │
│  │  │   Manager    │  │   Emitter    │  │   System     │ │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘ │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                               │
└──────────────────────────────────────────────────────────────┘
                           │         │
                REST API   │         │  WebSocket
                           ▼         ▼
        ┌──────────────────────────────────────────┐
        │      Axon Backend + Cortex REST API      │
        └──────────────────────────────────────────┘
```

## Tauri Backend Integration

### Command Handlers

```rust
// src-tauri/src/main.rs

use tauri::{Manager, State};
use std::sync::Arc;
use tokio::sync::Mutex;

// Global state
pub struct AppState {
    cortex_bridge: Arc<CortexBridge>,
    workflow_engine: Arc<Mutex<WorkflowEngine>>,
    agent_pool: Arc<Mutex<AgentPool>>,
}

#[tauri::command]
async fn get_system_health(
    state: State<'_, AppState>
) -> Result<SystemHealth, String> {
    let health = state.cortex_bridge
        .health_check()
        .await
        .map_err(|e| e.to_string())?;

    let agent_stats = state.agent_pool.lock().await.get_statistics();

    Ok(SystemHealth {
        cortex_status: health,
        agent_pool_stats: agent_stats,
        timestamp: Utc::now(),
    })
}

#[tauri::command]
async fn get_agents(
    state: State<'_, AppState>
) -> Result<Vec<AgentInfo>, String> {
    let pool = state.agent_pool.lock().await;
    let agents = pool.list_agents();

    Ok(agents.into_iter().map(|a| AgentInfo::from(a)).collect())
}

#[tauri::command]
async fn execute_workflow(
    workflow: Workflow,
    state: State<'_, AppState>,
) -> Result<WorkflowResult, String> {
    let mut engine = state.workflow_engine.lock().await;

    // Create schedule
    let scheduler = TaskScheduler::new(
        state.cortex_bridge.clone(),
        state.agent_pool.clone(),
    );
    let schedule = scheduler.create_schedule(&workflow)
        .await
        .map_err(|e| e.to_string())?;

    // Execute workflow
    let executor = WorkflowExecutor::new(
        state.cortex_bridge.clone(),
        state.agent_pool.clone(),
    );

    let result = executor.execute(workflow, schedule)
        .await
        .map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
async fn get_cortex_episodes(
    query: String,
    limit: usize,
    state: State<'_, AppState>,
) -> Result<Vec<Episode>, String> {
    state.cortex_bridge
        .search_episodes(&query, limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_metrics(
    state: State<'_, AppState>
) -> Result<MetricsSnapshot, String> {
    let metrics = state.cortex_bridge.metrics.export();
    Ok(metrics)
}

// WebSocket event handler
#[tauri::command]
async fn subscribe_cortex_events(
    window: tauri::Window,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.cortex_bridge.subscribe_events(
        EventFilter::All,
        move |event| {
            // Emit event to frontend
            window.emit("cortex-event", event).ok();
        }
    ).await;

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Initialize Axon backend
            let cortex_config = CortexConfig::default();
            let cortex_bridge = Arc::new(
                futures::executor::block_on(CortexBridge::new(cortex_config))
                    .expect("Failed to connect to Cortex")
            );

            let agent_pool = Arc::new(Mutex::new(AgentPool::new()));
            let workflow_engine = Arc::new(Mutex::new(WorkflowEngine::new()));

            app.manage(AppState {
                cortex_bridge,
                workflow_engine,
                agent_pool,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_system_health,
            get_agents,
            execute_workflow,
            get_cortex_episodes,
            get_metrics,
            subscribe_cortex_events,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

## Frontend Architecture

### State Management with Zustand

```typescript
// src/stores/agentStore.ts

import create from 'zustand';
import { invoke } from '@tauri-apps/api/tauri';

export interface Agent {
  id: string;
  name: string;
  type: AgentType;
  status: AgentStatus;
  capabilities: Capability[];
  currentTask: Task | null;
  metrics: AgentMetrics;
}

export interface AgentMetrics {
  tasksCompleted: number;
  tasksActive: number;
  successRate: number;
  avgDuration: number;
}

export enum AgentStatus {
  Idle = 'idle',
  Working = 'working',
  Completed = 'completed',
  Failed = 'failed',
}

export enum AgentType {
  Developer = 'developer',
  Reviewer = 'reviewer',
  Tester = 'tester',
  Architect = 'architect',
  Documenter = 'documenter',
}

interface AgentStore {
  // State
  agents: Map<string, Agent>;
  selectedAgent: string | null;
  loading: boolean;
  error: string | null;

  // Actions
  fetchAgents: () => Promise<void>;
  selectAgent: (id: string) => void;
  updateAgentStatus: (id: string, status: AgentStatus) => void;
}

export const useAgentStore = create<AgentStore>((set, get) => ({
  agents: new Map(),
  selectedAgent: null,
  loading: false,
  error: null,

  fetchAgents: async () => {
    set({ loading: true, error: null });
    try {
      const agents = await invoke<Agent[]>('get_agents');
      const agentMap = new Map(agents.map(a => [a.id, a]));
      set({ agents: agentMap, loading: false });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  selectAgent: (id: string) => {
    set({ selectedAgent: id });
  },

  updateAgentStatus: (id: string, status: AgentStatus) => {
    const agents = new Map(get().agents);
    const agent = agents.get(id);
    if (agent) {
      agents.set(id, { ...agent, status });
      set({ agents });
    }
  },
}));
```

```typescript
// src/stores/workflowStore.ts

import create from 'zustand';
import { invoke } from '@tauri-apps/api/tauri';

export interface Workflow {
  id: string;
  name: string;
  description: string;
  tasks: Task[];
  dependencies: Map<string, string[]>;
  status: WorkflowStatus;
  progress: number;
  startedAt?: Date;
  completedAt?: Date;
}

export interface Task {
  id: string;
  name: string;
  type: TaskType;
  status: TaskStatus;
  assignedAgent: string | null;
  progress: number;
}

export enum WorkflowStatus {
  Pending = 'pending',
  Running = 'running',
  Completed = 'completed',
  Failed = 'failed',
  Cancelled = 'cancelled',
}

interface WorkflowStore {
  workflows: Map<string, Workflow>;
  activeWorkflow: string | null;
  loading: boolean;
  error: string | null;

  executeWorkflow: (workflow: Workflow) => Promise<void>;
  cancelWorkflow: (id: string) => Promise<void>;
  updateWorkflowProgress: (id: string, progress: number) => void;
}

export const useWorkflowStore = create<WorkflowStore>((set, get) => ({
  workflows: new Map(),
  activeWorkflow: null,
  loading: false,
  error: null,

  executeWorkflow: async (workflow: Workflow) => {
    set({ loading: true, error: null, activeWorkflow: workflow.id });

    try {
      const result = await invoke<WorkflowResult>('execute_workflow', {
        workflow,
      });

      const workflows = new Map(get().workflows);
      workflows.set(workflow.id, {
        ...workflow,
        status: result.success ? WorkflowStatus.Completed : WorkflowStatus.Failed,
        completedAt: new Date(),
        progress: 100,
      });

      set({ workflows, loading: false, activeWorkflow: null });
    } catch (error) {
      const workflows = new Map(get().workflows);
      const failedWorkflow = workflows.get(workflow.id);
      if (failedWorkflow) {
        workflows.set(workflow.id, {
          ...failedWorkflow,
          status: WorkflowStatus.Failed,
        });
      }
      set({ error: String(error), loading: false, workflows });
    }
  },

  cancelWorkflow: async (id: string) => {
    // Implementation
  },

  updateWorkflowProgress: (id: string, progress: number) => {
    const workflows = new Map(get().workflows);
    const workflow = workflows.get(id);
    if (workflow) {
      workflows.set(id, { ...workflow, progress });
      set({ workflows });
    }
  },
}));
```

```typescript
// src/stores/metricsStore.ts

import create from 'zustand';
import { invoke } from '@tauri-apps/api/tauri';

export interface MetricsData {
  timestamp: Date;
  sessionsCreated: number;
  successfulMerges: number;
  mergeConflicts: number;
  filesRead: number;
  filesWritten: number;
  cacheHitRate: number;
  semanticSearches: number;
  episodesStored: number;
  locksAcquired: number;
  locksReleased: number;
}

interface MetricsStore {
  current: MetricsData | null;
  history: MetricsData[];
  loading: boolean;

  fetchMetrics: () => Promise<void>;
  addToHistory: (metrics: MetricsData) => void;
}

export const useMetricsStore = create<MetricsStore>((set, get) => ({
  current: null,
  history: [],
  loading: false,

  fetchMetrics: async () => {
    set({ loading: true });
    try {
      const metrics = await invoke<MetricsData>('get_metrics');
      set({
        current: metrics,
        loading: false,
      });

      // Add to history
      get().addToHistory(metrics);
    } catch (error) {
      console.error('Failed to fetch metrics:', error);
      set({ loading: false });
    }
  },

  addToHistory: (metrics: MetricsData) => {
    const history = [...get().history, metrics];
    // Keep only last 100 data points
    if (history.length > 100) {
      history.shift();
    }
    set({ history });
  },
}));
```

### Real-time Data Hooks

```typescript
// src/hooks/useWebSocket.ts

import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/tauri';

export interface CortexEvent {
  type: string;
  data: any;
}

export function useWebSocket() {
  const [connected, setConnected] = useState(false);
  const [events, setEvents] = useState<CortexEvent[]>([]);

  useEffect(() => {
    // Subscribe to Cortex events
    invoke('subscribe_cortex_events')
      .then(() => setConnected(true))
      .catch(console.error);

    // Listen for events from backend
    const unlisten = listen<CortexEvent>('cortex-event', (event) => {
      setEvents((prev) => [...prev, event.payload]);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return {
    connected,
    events,
    latestEvent: events[events.length - 1],
  };
}
```

```typescript
// src/hooks/useAgents.ts

import { useEffect } from 'react';
import { useAgentStore } from '../stores/agentStore';
import { useWebSocket } from './useWebSocket';

export function useAgents() {
  const { agents, loading, error, fetchAgents, updateAgentStatus } = useAgentStore();
  const { events } = useWebSocket();

  useEffect(() => {
    // Initial fetch
    fetchAgents();

    // Refresh every 5 seconds
    const interval = setInterval(fetchAgents, 5000);

    return () => clearInterval(interval);
  }, [fetchAgents]);

  useEffect(() => {
    // Update agent status based on WebSocket events
    events.forEach((event) => {
      if (event.type === 'AgentStatusChanged') {
        updateAgentStatus(event.data.agentId, event.data.status);
      }
    });
  }, [events, updateAgentStatus]);

  return {
    agents: Array.from(agents.values()),
    loading,
    error,
  };
}
```

```typescript
// src/hooks/useCortexData.ts

import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

export interface Episode {
  taskDescription: string;
  agentId: string;
  outcome: string;
  durationSeconds: number;
  solutionSummary: string;
  entitiesModified: string[];
  filesTouched: string[];
  patternsLearned: string[];
}

export function useCortexData(query: string, limit: number = 10) {
  const [episodes, setEpisodes] = useState<Episode[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!query) return;

    setLoading(true);
    setError(null);

    invoke<Episode[]>('get_cortex_episodes', { query, limit })
      .then(setEpisodes)
      .catch((err) => setError(String(err)))
      .finally(() => setLoading(false));
  }, [query, limit]);

  return { episodes, loading, error };
}
```

## Dashboard Views

### Overview Dashboard

```typescript
// src/views/OverviewDashboard.tsx

import React from 'react';
import { useAgents } from '../hooks/useAgents';
import { useWorkflowStore } from '../stores/workflowStore';
import { useMetricsStore } from '../stores/metricsStore';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '../components/ui/card';
import { AgentStatusCard } from '../components/AgentStatusCard';
import { WorkflowProgressCard } from '../components/WorkflowProgressCard';
import { MetricsChart } from '../components/MetricsChart';

export const OverviewDashboard: React.FC = () => {
  const { agents } = useAgents();
  const { workflows, activeWorkflow } = useWorkflowStore();
  const { current: metrics, history } = useMetricsStore();

  const idleAgents = agents.filter((a) => a.status === 'idle').length;
  const workingAgents = agents.filter((a) => a.status === 'working').length;

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-3xl font-bold">Axon Dashboard</h1>
        <p className="text-muted-foreground">
          Multi-agent orchestration with cognitive memory
        </p>
      </div>

      {/* Stats Overview */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total Agents</CardTitle>
            <UsersIcon className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{agents.length}</div>
            <p className="text-xs text-muted-foreground">
              {workingAgents} active, {idleAgents} idle
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Active Workflows</CardTitle>
            <ActivityIcon className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {Array.from(workflows.values()).filter((w) => w.status === 'running').length}
            </div>
            <p className="text-xs text-muted-foreground">
              {workflows.size} total workflows
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Cache Hit Rate</CardTitle>
            <DatabaseIcon className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {metrics ? `${(metrics.cacheHitRate * 100).toFixed(1)}%` : '-'}
            </div>
            <p className="text-xs text-muted-foreground">
              Memory optimization
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Episodes Stored</CardTitle>
            <BrainIcon className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {metrics?.episodesStored || 0}
            </div>
            <p className="text-xs text-muted-foreground">
              Cognitive learning
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Agent Status */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card>
          <CardHeader>
            <CardTitle>Agent Status</CardTitle>
            <CardDescription>Real-time agent activity</CardDescription>
          </CardHeader>
          <CardContent>
            <AgentStatusCard agents={agents} />
          </CardContent>
        </Card>

        {/* Active Workflow */}
        {activeWorkflow && (
          <Card>
            <CardHeader>
              <CardTitle>Active Workflow</CardTitle>
              <CardDescription>Current execution progress</CardDescription>
            </CardHeader>
            <CardContent>
              <WorkflowProgressCard workflowId={activeWorkflow} />
            </CardContent>
          </Card>
        )}
      </div>

      {/* Metrics Chart */}
      <Card>
        <CardHeader>
          <CardTitle>System Metrics</CardTitle>
          <CardDescription>Performance over time</CardDescription>
        </CardHeader>
        <CardContent>
          <MetricsChart data={history} />
        </CardContent>
      </Card>
    </div>
  );
};
```

### Agents View

```typescript
// src/views/AgentsView.tsx

import React from 'react';
import { useAgents } from '../hooks/useAgents';
import { useAgentStore } from '../stores/agentStore';
import {
  Table,
  TableBody,
  TableCaption,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '../components/ui/table';
import { Badge } from '../components/ui/badge';
import { Progress } from '../components/ui/progress';

export const AgentsView: React.FC = () => {
  const { agents, loading } = useAgents();
  const { selectedAgent, selectAgent } = useAgentStore();

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'idle':
        return 'bg-gray-500';
      case 'working':
        return 'bg-blue-500';
      case 'completed':
        return 'bg-green-500';
      case 'failed':
        return 'bg-red-500';
      default:
        return 'bg-gray-500';
    }
  };

  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Agents</h1>
        <p className="text-muted-foreground">
          Monitor and manage all agents in the system
        </p>
      </div>

      {loading ? (
        <div className="flex items-center justify-center h-64">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary"></div>
        </div>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* Agent List */}
          <div className="lg:col-span-2">
            <Card>
              <CardHeader>
                <CardTitle>Agent Pool</CardTitle>
              </CardHeader>
              <CardContent>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Name</TableHead>
                      <TableHead>Type</TableHead>
                      <TableHead>Status</TableHead>
                      <TableHead>Current Task</TableHead>
                      <TableHead>Success Rate</TableHead>
                      <TableHead>Tasks</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {agents.map((agent) => (
                      <TableRow
                        key={agent.id}
                        onClick={() => selectAgent(agent.id)}
                        className={`cursor-pointer ${
                          selectedAgent === agent.id ? 'bg-muted' : ''
                        }`}
                      >
                        <TableCell className="font-medium">{agent.name}</TableCell>
                        <TableCell>
                          <Badge variant="outline">{agent.type}</Badge>
                        </TableCell>
                        <TableCell>
                          <Badge className={getStatusColor(agent.status)}>
                            {agent.status}
                          </Badge>
                        </TableCell>
                        <TableCell>
                          {agent.currentTask?.name || '-'}
                        </TableCell>
                        <TableCell>
                          <div className="flex items-center gap-2">
                            <Progress
                              value={agent.metrics.successRate * 100}
                              className="w-16"
                            />
                            <span className="text-sm">
                              {(agent.metrics.successRate * 100).toFixed(0)}%
                            </span>
                          </div>
                        </TableCell>
                        <TableCell>
                          {agent.metrics.tasksCompleted} / {agent.metrics.tasksActive}
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </CardContent>
            </Card>
          </div>

          {/* Agent Details */}
          {selectedAgent && (
            <div>
              <AgentDetailsPanel agentId={selectedAgent} />
            </div>
          )}
        </div>
      )}
    </div>
  );
};
```

### Workflow Visualization

```typescript
// src/components/WorkflowDAG.tsx

import React, { useEffect, useRef } from 'react';
import * as d3 from 'd3';
import { Workflow, Task } from '../stores/workflowStore';

interface WorkflowDAGProps {
  workflow: Workflow;
}

export const WorkflowDAG: React.FC<WorkflowDAGProps> = ({ workflow }) => {
  const svgRef = useRef<SVGSVGElement>(null);

  useEffect(() => {
    if (!svgRef.current) return;

    const svg = d3.select(svgRef.current);
    const width = 800;
    const height = 600;

    svg.attr('width', width).attr('height', height);

    // Clear previous content
    svg.selectAll('*').remove();

    // Create DAG layout
    const tasks = workflow.tasks;
    const dependencies = workflow.dependencies;

    // Calculate positions using topological sort
    const levels = calculateLevels(tasks, dependencies);

    // Draw edges
    const g = svg.append('g');

    dependencies.forEach((deps, taskId) => {
      const task = tasks.find((t) => t.id === taskId);
      if (!task) return;

      deps.forEach((depId) => {
        const depTask = tasks.find((t) => t.id === depId);
        if (!depTask) return;

        const source = getNodePosition(depTask, levels, width, height);
        const target = getNodePosition(task, levels, width, height);

        g.append('line')
          .attr('x1', source.x)
          .attr('y1', source.y)
          .attr('x2', target.x)
          .attr('y2', target.y)
          .attr('stroke', '#64748b')
          .attr('stroke-width', 2)
          .attr('marker-end', 'url(#arrowhead)');
      });
    });

    // Draw nodes
    tasks.forEach((task) => {
      const pos = getNodePosition(task, levels, width, height);

      const node = g.append('g')
        .attr('transform', `translate(${pos.x}, ${pos.y})`);

      // Node circle
      node.append('circle')
        .attr('r', 30)
        .attr('fill', getTaskColor(task.status))
        .attr('stroke', '#1e293b')
        .attr('stroke-width', 2);

      // Task name
      node.append('text')
        .attr('text-anchor', 'middle')
        .attr('dy', '0.3em')
        .attr('fill', 'white')
        .attr('font-size', '10px')
        .text(task.name.substring(0, 8));

      // Progress indicator
      if (task.status === 'running') {
        node.append('circle')
          .attr('r', 35)
          .attr('fill', 'none')
          .attr('stroke', '#3b82f6')
          .attr('stroke-width', 3)
          .attr('stroke-dasharray', `${task.progress * 2 * Math.PI * 35} ${2 * Math.PI * 35}`);
      }
    });

    // Add arrowhead marker
    svg.append('defs').append('marker')
      .attr('id', 'arrowhead')
      .attr('markerWidth', 10)
      .attr('markerHeight', 10)
      .attr('refX', 8)
      .attr('refY', 3)
      .attr('orient', 'auto')
      .append('polygon')
      .attr('points', '0 0, 10 3, 0 6')
      .attr('fill', '#64748b');

  }, [workflow]);

  return (
    <div className="border rounded-lg p-4">
      <svg ref={svgRef}></svg>
    </div>
  );
};

function calculateLevels(
  tasks: Task[],
  dependencies: Map<string, string[]>
): Map<string, number> {
  const levels = new Map<string, number>();
  const visited = new Set<string>();

  function visit(taskId: string): number {
    if (visited.has(taskId)) {
      return levels.get(taskId)!;
    }

    visited.add(taskId);

    const deps = dependencies.get(taskId) || [];
    if (deps.length === 0) {
      levels.set(taskId, 0);
      return 0;
    }

    const maxDepLevel = Math.max(...deps.map(visit));
    const level = maxDepLevel + 1;
    levels.set(taskId, level);
    return level;
  }

  tasks.forEach((task) => visit(task.id));
  return levels;
}

function getNodePosition(
  task: Task,
  levels: Map<string, number>,
  width: number,
  height: number
): { x: number; y: number } {
  const level = levels.get(task.id) || 0;
  const maxLevel = Math.max(...Array.from(levels.values()));

  const tasksAtLevel = Array.from(levels.entries())
    .filter(([_, l]) => l === level)
    .map(([id]) => id);

  const indexAtLevel = tasksAtLevel.indexOf(task.id);

  const x = ((level + 1) / (maxLevel + 2)) * width;
  const y = ((indexAtLevel + 1) / (tasksAtLevel.length + 1)) * height;

  return { x, y };
}

function getTaskColor(status: string): string {
  switch (status) {
    case 'pending':
      return '#64748b';
    case 'running':
      return '#3b82f6';
    case 'completed':
      return '#10b981';
    case 'failed':
      return '#ef4444';
    default:
      return '#64748b';
  }
}
```

### Memory Visualization

```typescript
// src/components/MemoryHeatmap.tsx

import React from 'react';
import { ResponsiveContainer, LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip } from 'recharts';
import { useCortexData } from '../hooks/useCortexData';

export const MemoryHeatmap: React.FC = () => {
  const { episodes, loading } = useCortexData('', 50);

  if (loading) {
    return <div className="animate-pulse h-64 bg-muted rounded"></div>;
  }

  // Group episodes by hour
  const episodesByHour = episodes.reduce((acc, ep) => {
    const hour = new Date(ep.timestamp).getHours();
    acc[hour] = (acc[hour] || 0) + 1;
    return acc;
  }, {} as Record<number, number>);

  const chartData = Array.from({ length: 24 }, (_, hour) => ({
    hour,
    episodes: episodesByHour[hour] || 0,
  }));

  return (
    <ResponsiveContainer width="100%" height={300}>
      <LineChart data={chartData}>
        <CartesianGrid strokeDasharray="3 3" />
        <XAxis
          dataKey="hour"
          label={{ value: 'Hour of Day', position: 'insideBottom', offset: -5 }}
        />
        <YAxis label={{ value: 'Episodes', angle: -90, position: 'insideLeft' }} />
        <Tooltip />
        <Line
          type="monotone"
          dataKey="episodes"
          stroke="#3b82f6"
          strokeWidth={2}
          dot={{ r: 4 }}
        />
      </LineChart>
    </ResponsiveContainer>
  );
};
```

### System Metrics Dashboard

```typescript
// src/components/MetricsChart.tsx

import React from 'react';
import { ResponsiveContainer, LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend } from 'recharts';
import { MetricsData } from '../stores/metricsStore';

interface MetricsChartProps {
  data: MetricsData[];
}

export const MetricsChart: React.FC<MetricsChartProps> = ({ data }) => {
  const chartData = data.map((d, idx) => ({
    time: idx,
    sessions: d.sessionsCreated,
    merges: d.successfulMerges,
    conflicts: d.mergeConflicts,
    cacheHitRate: d.cacheHitRate * 100,
    episodes: d.episodesStored,
  }));

  return (
    <ResponsiveContainer width="100%" height={400}>
      <LineChart data={chartData}>
        <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
        <XAxis
          dataKey="time"
          stroke="#9ca3af"
          label={{ value: 'Time', position: 'insideBottom', offset: -5 }}
        />
        <YAxis stroke="#9ca3af" />
        <Tooltip
          contentStyle={{
            backgroundColor: '#1f2937',
            border: '1px solid #374151',
            borderRadius: '8px',
          }}
        />
        <Legend />
        <Line
          type="monotone"
          dataKey="sessions"
          stroke="#3b82f6"
          name="Sessions Created"
          strokeWidth={2}
        />
        <Line
          type="monotone"
          dataKey="merges"
          stroke="#10b981"
          name="Successful Merges"
          strokeWidth={2}
        />
        <Line
          type="monotone"
          dataKey="conflicts"
          stroke="#ef4444"
          name="Merge Conflicts"
          strokeWidth={2}
        />
        <Line
          type="monotone"
          dataKey="cacheHitRate"
          stroke="#8b5cf6"
          name="Cache Hit Rate %"
          strokeWidth={2}
        />
        <Line
          type="monotone"
          dataKey="episodes"
          stroke="#f59e0b"
          name="Episodes Stored"
          strokeWidth={2}
        />
      </LineChart>
    </ResponsiveContainer>
  );
};
```

## UI Components Library

### Agent Status Card

```typescript
// src/components/AgentStatusCard.tsx

import React from 'react';
import { Agent, AgentStatus } from '../stores/agentStore';
import { Avatar, AvatarFallback } from './ui/avatar';
import { Badge } from './ui/badge';
import { Progress } from './ui/progress';

interface AgentStatusCardProps {
  agents: Agent[];
}

export const AgentStatusCard: React.FC<AgentStatusCardProps> = ({ agents }) => {
  const getAgentIcon = (type: string) => {
    switch (type) {
      case 'developer':
        return '💻';
      case 'reviewer':
        return '🔍';
      case 'tester':
        return '🧪';
      case 'architect':
        return '🏗️';
      case 'documenter':
        return '📝';
      default:
        return '🤖';
    }
  };

  return (
    <div className="space-y-4">
      {agents.map((agent) => (
        <div
          key={agent.id}
          className="flex items-center justify-between p-4 border rounded-lg hover:bg-muted/50 transition-colors"
        >
          <div className="flex items-center gap-3">
            <Avatar>
              <AvatarFallback>{getAgentIcon(agent.type)}</AvatarFallback>
            </Avatar>
            <div>
              <p className="font-medium">{agent.name}</p>
              <p className="text-sm text-muted-foreground">{agent.type}</p>
            </div>
          </div>

          <div className="flex items-center gap-4">
            {agent.currentTask && (
              <div className="text-right max-w-xs">
                <p className="text-sm font-medium truncate">
                  {agent.currentTask.name}
                </p>
                <Progress
                  value={agent.currentTask.progress}
                  className="w-24 h-2"
                />
              </div>
            )}
            <Badge
              variant={agent.status === 'working' ? 'default' : 'secondary'}
            >
              {agent.status}
            </Badge>
          </div>
        </div>
      ))}
    </div>
  );
};
```

### Live Logs Viewer

```typescript
// src/components/LogsViewer.tsx

import React, { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';

interface LogEntry {
  timestamp: string;
  level: 'info' | 'warn' | 'error' | 'debug';
  message: string;
  context?: Record<string, any>;
}

export const LogsViewer: React.FC = () => {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [filter, setFilter] = useState<string>('');
  const [levelFilter, setLevelFilter] = useState<string>('all');
  const logsEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const unlisten = listen<LogEntry>('log-entry', (event) => {
      setLogs((prev) => [...prev, event.payload].slice(-100)); // Keep last 100 logs
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    logsEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [logs]);

  const filteredLogs = logs.filter((log) => {
    const matchesText = log.message.toLowerCase().includes(filter.toLowerCase());
    const matchesLevel = levelFilter === 'all' || log.level === levelFilter;
    return matchesText && matchesLevel;
  });

  const getLevelColor = (level: string) => {
    switch (level) {
      case 'info':
        return 'text-blue-400';
      case 'warn':
        return 'text-yellow-400';
      case 'error':
        return 'text-red-400';
      case 'debug':
        return 'text-gray-400';
      default:
        return 'text-gray-400';
    }
  };

  return (
    <div className="space-y-4">
      {/* Filters */}
      <div className="flex gap-4">
        <input
          type="text"
          placeholder="Filter logs..."
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          className="flex-1 px-3 py-2 border rounded-md"
        />
        <select
          value={levelFilter}
          onChange={(e) => setLevelFilter(e.target.value)}
          className="px-3 py-2 border rounded-md"
        >
          <option value="all">All Levels</option>
          <option value="info">Info</option>
          <option value="warn">Warning</option>
          <option value="error">Error</option>
          <option value="debug">Debug</option>
        </select>
      </div>

      {/* Logs */}
      <div className="h-96 overflow-y-auto bg-gray-900 rounded-lg p-4 font-mono text-sm">
        {filteredLogs.map((log, idx) => (
          <div key={idx} className="mb-2">
            <span className="text-gray-500">{log.timestamp}</span>
            {' '}
            <span className={getLevelColor(log.level)}>
              [{log.level.toUpperCase()}]
            </span>
            {' '}
            <span className="text-gray-300">{log.message}</span>
            {log.context && (
              <pre className="ml-8 text-xs text-gray-400">
                {JSON.stringify(log.context, null, 2)}
              </pre>
            )}
          </div>
        ))}
        <div ref={logsEndRef} />
      </div>
    </div>
  );
};
```

## Configuration and Settings

```typescript
// src/views/SettingsView.tsx

import React from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card';
import { Label } from '../components/ui/label';
import { Input } from '../components/ui/input';
import { Switch } from '../components/ui/switch';
import { Button } from '../components/ui/button';

export const SettingsView: React.FC = () => {
  const [cortexUrl, setCortexUrl] = React.useState('http://localhost:8081');
  const [enableWebSocket, setEnableWebSocket] = React.useState(true);
  const [cacheSizeMb, setCacheSizeMb] = React.useState(100);
  const [maxRetries, setMaxRetries] = React.useState(3);

  const handleSave = async () => {
    await invoke('update_config', {
      config: {
        cortexUrl,
        enableWebSocket,
        cacheSizeMb,
        maxRetries,
      },
    });
  };

  return (
    <div className="p-6 space-y-6 max-w-4xl">
      <div>
        <h1 className="text-3xl font-bold">Settings</h1>
        <p className="text-muted-foreground">
          Configure Axon and Cortex integration
        </p>
      </div>

      {/* Cortex Connection */}
      <Card>
        <CardHeader>
          <CardTitle>Cortex Connection</CardTitle>
          <CardDescription>
            Configure connection to Cortex REST API
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="cortex-url">Cortex URL</Label>
            <Input
              id="cortex-url"
              value={cortexUrl}
              onChange={(e) => setCortexUrl(e.target.value)}
              placeholder="http://localhost:8081"
            />
          </div>

          <div className="flex items-center justify-between">
            <Label htmlFor="websocket">Enable WebSocket</Label>
            <Switch
              id="websocket"
              checked={enableWebSocket}
              onCheckedChange={setEnableWebSocket}
            />
          </div>
        </CardContent>
      </Card>

      {/* Performance */}
      <Card>
        <CardHeader>
          <CardTitle>Performance</CardTitle>
          <CardDescription>
            Optimize caching and retry behavior
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="cache-size">Cache Size (MB)</Label>
            <Input
              id="cache-size"
              type="number"
              value={cacheSizeMb}
              onChange={(e) => setCacheSizeMb(Number(e.target.value))}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="max-retries">Max Retries</Label>
            <Input
              id="max-retries"
              type="number"
              value={maxRetries}
              onChange={(e) => setMaxRetries(Number(e.target.value))}
            />
          </div>
        </CardContent>
      </Card>

      {/* Save Button */}
      <Button onClick={handleSave} className="w-full">
        Save Settings
      </Button>
    </div>
  );
};
```

## Main Application Layout

```typescript
// src/App.tsx

import React from 'react';
import { BrowserRouter as Router, Routes, Route, Link } from 'react-router-dom';
import { OverviewDashboard } from './views/OverviewDashboard';
import { AgentsView } from './views/AgentsView';
import { WorkflowsView } from './views/WorkflowsView';
import { MetricsView } from './views/MetricsView';
import { MemoryView } from './views/MemoryView';
import { LogsView } from './views/LogsView';
import { SettingsView } from './views/SettingsView';

export const App: React.FC = () => {
  return (
    <Router>
      <div className="flex h-screen bg-background">
        {/* Sidebar */}
        <aside className="w-64 border-r bg-card">
          <div className="p-6">
            <h1 className="text-2xl font-bold">Axon</h1>
            <p className="text-sm text-muted-foreground">Multi-Agent System</p>
          </div>

          <nav className="space-y-2 px-4">
            <NavLink to="/" icon="🏠" label="Overview" />
            <NavLink to="/agents" icon="🤖" label="Agents" />
            <NavLink to="/workflows" icon="🔄" label="Workflows" />
            <NavLink to="/metrics" icon="📊" label="Metrics" />
            <NavLink to="/memory" icon="🧠" label="Memory" />
            <NavLink to="/logs" icon="📝" label="Logs" />
            <NavLink to="/settings" icon="⚙️" label="Settings" />
          </nav>
        </aside>

        {/* Main Content */}
        <main className="flex-1 overflow-y-auto">
          <Routes>
            <Route path="/" element={<OverviewDashboard />} />
            <Route path="/agents" element={<AgentsView />} />
            <Route path="/workflows" element={<WorkflowsView />} />
            <Route path="/metrics" element={<MetricsView />} />
            <Route path="/memory" element={<MemoryView />} />
            <Route path="/logs" element={<LogsView />} />
            <Route path="/settings" element={<SettingsView />} />
          </Routes>
        </main>
      </div>
    </Router>
  );
};

const NavLink: React.FC<{ to: string; icon: string; label: string }> = ({
  to,
  icon,
  label,
}) => {
  return (
    <Link
      to={to}
      className="flex items-center gap-3 px-4 py-2 rounded-lg hover:bg-muted transition-colors"
    >
      <span className="text-xl">{icon}</span>
      <span>{label}</span>
    </Link>
  );
};
```

## Build and Development

### Package Configuration

```json
// package.json

{
  "name": "axon-dashboard",
  "version": "1.0.0",
  "description": "Axon Multi-Agent Orchestration Dashboard",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "tauri:dev": "tauri dev",
    "tauri:build": "tauri build"
  },
  "dependencies": {
    "@radix-ui/react-avatar": "^1.0.4",
    "@radix-ui/react-badge": "^1.0.4",
    "@radix-ui/react-progress": "^1.0.3",
    "@radix-ui/react-switch": "^1.0.3",
    "@tauri-apps/api": "^1.5.0",
    "d3": "^7.8.5",
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "react-router-dom": "^6.20.0",
    "recharts": "^2.10.3",
    "zustand": "^4.4.7"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^1.5.0",
    "@types/d3": "^7.4.3",
    "@types/react": "^18.2.45",
    "@types/react-dom": "^18.2.18",
    "@vitejs/plugin-react": "^4.2.1",
    "autoprefixer": "^10.4.16",
    "postcss": "^8.4.32",
    "tailwindcss": "^3.3.6",
    "typescript": "^5.3.3",
    "vite": "^5.0.7"
  }
}
```

### Tauri Configuration

```json
// src-tauri/tauri.conf.json

{
  "build": {
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build",
    "devPath": "http://localhost:5173",
    "distDir": "../dist"
  },
  "package": {
    "productName": "Axon",
    "version": "1.0.0"
  },
  "tauri": {
    "allowlist": {
      "all": false,
      "shell": {
        "all": false,
        "open": true
      },
      "window": {
        "all": false,
        "close": true,
        "hide": true,
        "show": true,
        "maximize": true,
        "minimize": true,
        "unmaximize": true,
        "unminimize": true,
        "startDragging": true
      }
    },
    "bundle": {
      "active": true,
      "category": "DeveloperTool",
      "copyright": "",
      "deb": {
        "depends": []
      },
      "externalBin": [],
      "icon": [
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/128x128@2x.png",
        "icons/icon.icns",
        "icons/icon.ico"
      ],
      "identifier": "dev.axon.app",
      "longDescription": "",
      "macOS": {
        "entitlements": null,
        "exceptionDomain": "",
        "frameworks": [],
        "providerShortName": null,
        "signingIdentity": null
      },
      "resources": [],
      "shortDescription": "",
      "targets": "all",
      "windows": {
        "certificateThumbprint": null,
        "digestAlgorithm": "sha256",
        "timestampUrl": ""
      }
    },
    "security": {
      "csp": null
    },
    "updater": {
      "active": false
    },
    "windows": [
      {
        "fullscreen": false,
        "height": 800,
        "resizable": true,
        "title": "Axon",
        "width": 1400,
        "minWidth": 1200,
        "minHeight": 700
      }
    ]
  }
}
```

## Deployment

### Development Mode

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri:dev
```

### Production Build

```bash
# Build for production
npm run tauri:build

# Output locations:
# - macOS: src-tauri/target/release/bundle/dmg/
# - Windows: src-tauri/target/release/bundle/msi/
# - Linux: src-tauri/target/release/bundle/deb/
```

## Conclusion

The Axon Dashboard provides a comprehensive, production-ready UI for monitoring and managing the multi-agent orchestration system:

1. **Native Performance**: Tauri + Rust backend for desktop-class performance
2. **Real-time Updates**: WebSocket integration with Cortex for live data
3. **Rich Visualizations**: D3.js for DAG workflows, Recharts for metrics
4. **Type Safety**: Full TypeScript implementation
5. **State Management**: Zustand for predictable state updates
6. **Modern UI**: Tailwind CSS + shadcn/ui components
7. **Responsive Design**: Works on all desktop screen sizes
8. **Developer Experience**: Hot reload, TypeScript, and comprehensive hooks

The dashboard seamlessly integrates with both Axon's orchestration engine and Cortex's REST API, providing a unified interface for multi-agent system management and cognitive memory visualization.
