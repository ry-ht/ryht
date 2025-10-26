# Axon Multi-Agent System Integration

This document describes the integration between the Dashboard application and the Axon Multi-Agent System REST API.

## Overview

The dashboard provides a comprehensive UI for managing and monitoring the Axon multi-agent system, including:

- Real-time agent management (create, start, stop, pause, resume)
- Workflow orchestration and monitoring
- System metrics and telemetry
- WebSocket-based real-time updates

## Architecture

```
┌─────────────────────────────────────────┐
│                                         │
│         Dashboard (React + MUI)         │
│                                         │
├─────────────────────────────────────────┤
│                                         │
│  ┌──────────────┐  ┌─────────────────┐ │
│  │ Axon Client  │  │ Axon WebSocket  │ │
│  │   (HTTP)     │  │    (WS)         │ │
│  └──────┬───────┘  └────────┬────────┘ │
│         │                   │          │
└─────────┼───────────────────┼──────────┘
          │                   │
          │                   │
┌─────────▼───────────────────▼──────────┐
│                                         │
│      Axon REST API (port 9090)          │
│                                         │
│  ┌──────────┐  ┌──────────┐            │
│  │  Agents  │  │Workflows │            │
│  └──────────┘  └──────────┘            │
│                                         │
└─────────────────────────────────────────┘
```

## Components

### 1. API Client (`src/lib/axon-client.ts`)

HTTP client for communicating with the Axon REST API.

**Features:**
- Bearer token authentication
- Automatic error handling
- Type-safe API calls
- Support for all Axon endpoints

**Usage:**
```typescript
import { axonClient } from 'src/lib/axon-client';

// Create an agent
const result = await axonClient.createAgent({
  name: 'my-developer',
  agent_type: 'Developer',
  capabilities: ['coding', 'review'],
  max_concurrent_tasks: 2,
});

// List agents
const agents = await axonClient.listAgents();

// Run a workflow
const workflow = await axonClient.runWorkflow({
  workflow_def: yamlDefinition,
  input_params: { repo: 'example/repo' },
});
```

### 2. WebSocket Client (`src/lib/axon-websocket.ts`)

Real-time event streaming from the Axon system.

**Features:**
- Automatic reconnection with exponential backoff
- Event subscription system
- Connection status monitoring
- React hook for easy integration

**Usage:**
```typescript
import { axonWebSocket } from 'src/lib/axon-websocket';

// Subscribe to events
const unsubscribe = axonWebSocket.subscribe((event) => {
  console.log('Received event:', event.type, event.data);
});

// Or use the React hook
import { useAxonWebSocket } from 'src/lib/axon-websocket';

function MyComponent() {
  const { isConnected } = useAxonWebSocket((event) => {
    // Handle events
  });

  return <div>WebSocket: {isConnected ? 'Connected' : 'Disconnected'}</div>;
}
```

### 3. Type Definitions (`src/types/axon.ts`)

TypeScript type definitions for all Axon API data structures.

**Key Types:**
- `AgentInfo` - Agent metadata and status
- `WorkflowInfo` - Workflow status and progress
- `HealthResponse` - System health information
- `WebSocketEvent` - Real-time event types

### 4. UI Components

#### Agent Management
- **Agent List** (`src/sections/agent/agent-list-view.tsx`)
  - View all agents with status and metrics
  - Pause/resume/restart/delete operations
  - Real-time status updates

- **Create Agent** (`src/sections/agent/agent-create-view.tsx`)
  - Form to create new agents
  - Select agent type and capabilities
  - Configure concurrency limits

#### Workflow Management
- **Workflow List** (`src/sections/workflow/workflow-list-view.tsx`)
  - View all workflows with progress
  - Real-time status updates
  - Cancel running workflows

- **Run Workflow** (`src/sections/workflow/workflow-create-view.tsx`)
  - YAML workflow definition editor
  - JSON input parameters
  - Workflow submission

#### Dashboard Overview
- **Axon Overview** (`src/sections/overview/axon-overview.tsx`)
  - System health status
  - Active agents and workflows count
  - WebSocket connection status
  - Recent agents and workflows

## Configuration

### Environment Variables

Create a `.env` file in the dashboard root directory:

```bash
# Axon Multi-Agent System API
VITE_AXON_API_URL=http://127.0.0.1:9090/api/v1
VITE_AXON_WS_URL=ws://127.0.0.1:9090/api/v1/ws
VITE_AXON_API_KEY=axon-dev-key-change-in-production
```

### Authentication

The API uses Bearer token authentication. The token is configured via the `VITE_AXON_API_KEY` environment variable and automatically included in all requests.

**Default:** `axon-dev-key-change-in-production`

**Production:** Update this to a secure token and configure the Axon server accordingly.

## API Endpoints

### Health & Status
- `GET /api/v1/health` - Health check (no auth required)
- `GET /api/v1/status` - System status

### Agents
- `GET /api/v1/agents` - List all agents
- `POST /api/v1/agents` - Create a new agent
- `GET /api/v1/agents/{id}` - Get agent details
- `DELETE /api/v1/agents/{id}` - Stop/delete agent
- `POST /api/v1/agents/{id}/pause` - Pause agent
- `POST /api/v1/agents/{id}/resume` - Resume agent
- `POST /api/v1/agents/{id}/restart` - Restart agent
- `GET /api/v1/agents/{id}/logs` - Get agent logs

### Workflows
- `GET /api/v1/workflows` - List all workflows
- `POST /api/v1/workflows` - Run a workflow
- `GET /api/v1/workflows/{id}` - Get workflow status
- `POST /api/v1/workflows/{id}/cancel` - Cancel workflow
- `POST /api/v1/workflows/{id}/pause` - Pause workflow
- `POST /api/v1/workflows/{id}/resume` - Resume workflow

### Metrics
- `GET /api/v1/metrics` - Get system metrics
- `GET /api/v1/telemetry?range=60` - Get telemetry data
- `GET /api/v1/telemetry/summary` - Get telemetry summary
- `POST /api/v1/metrics/export` - Export metrics

### Configuration
- `GET /api/v1/config` - Get configuration
- `PUT /api/v1/config` - Update configuration
- `POST /api/v1/config/validate` - Validate configuration

### WebSocket
- `WS /api/v1/ws` - WebSocket connection for real-time events

## Running the Integration

### 1. Start the Axon Server

```bash
cd /Users/taaliman/projects/luxquant/ry-ht/ryht/axon
cargo run -- server start --host 127.0.0.1 --port 9090
```

Or use the internal server command:
```bash
cargo run -- internal-server-run --host 127.0.0.1 --port 9090
```

### 2. Start the Dashboard

```bash
cd /Users/taaliman/projects/luxquant/ry-ht/ryht/dashboard
npm install
npm run dev
```

The dashboard will be available at `http://localhost:5173`

### 3. Navigate to Axon Pages

- **Dashboard Overview:** `http://localhost:5173/dashboard`
- **Agents:** `http://localhost:5173/dashboard/agents`
- **Create Agent:** `http://localhost:5173/dashboard/agents/create`
- **Workflows:** `http://localhost:5173/dashboard/workflows`
- **Run Workflow:** `http://localhost:5173/dashboard/workflows/create`

## Development

### Adding New Features

1. **Update API Client** - Add new methods to `src/lib/axon-client.ts`
2. **Update Types** - Add/modify types in `src/types/axon.ts`
3. **Create UI Components** - Add views in `src/sections/`
4. **Add Routes** - Update `src/routes/sections/dashboard.tsx`
5. **Update Navigation** - Update `src/layouts/nav-config-dashboard.tsx`

### Data Fetching Pattern

The integration uses SWR for data fetching with automatic revalidation:

```typescript
import useSWR from 'swr';
import { axonFetcher, axonEndpoints } from 'src/lib/axon-client';

const { data, error, isLoading } = useSWR(
  axonEndpoints.agents.list,
  axonFetcher,
  { refreshInterval: 5000 } // Refresh every 5 seconds
);
```

### Real-time Updates

Components can subscribe to WebSocket events for real-time updates:

```typescript
import { useEffect } from 'react';
import { axonWebSocket } from 'src/lib/axon-websocket';
import { mutate } from 'swr';

useEffect(() => {
  const unsubscribe = axonWebSocket.subscribe((event) => {
    if (event.type === 'agent_status_changed') {
      // Refresh agent list
      mutate(axonEndpoints.agents.list);
    }
  });

  return unsubscribe;
}, []);
```

## Troubleshooting

### WebSocket Connection Issues

If the WebSocket fails to connect:
1. Verify the Axon server is running on port 9090
2. Check CORS settings in the Axon server
3. Ensure `VITE_AXON_WS_URL` is correct
4. Check browser console for WebSocket errors

### API Authentication Errors

If you get 401 Unauthorized errors:
1. Verify `VITE_AXON_API_KEY` matches the server configuration
2. Check the Authorization header in network requests
3. Ensure the Axon server is configured to accept the token

### CORS Issues

If you get CORS errors:
1. Configure Axon server to allow requests from `http://localhost:5173`
2. Add appropriate CORS headers in the Axon middleware
3. Check browser console for specific CORS errors

## Future Enhancements

Potential improvements to the integration:

1. **Agent Details View** - Detailed view for individual agents with task history
2. **Workflow Visualization** - DAG visualization of workflow tasks and dependencies
3. **Real-time Logs** - Stream agent logs in real-time using WebSocket
4. **Metrics Dashboard** - Charts and graphs for system metrics
5. **Agent Templates** - Pre-configured agent templates for common use cases
6. **Workflow Library** - Save and reuse workflow definitions
7. **Multi-tenant Support** - Workspace isolation and management
8. **Advanced Monitoring** - Performance profiling and optimization recommendations

## License

This integration is part of the Axon Multi-Agent System project.
