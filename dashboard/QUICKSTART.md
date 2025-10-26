# Axon Dashboard Quick Start Guide

This guide will help you get the Axon Dashboard up and running with the Axon Multi-Agent System.

## Prerequisites

- Node.js (v18 or higher)
- npm or yarn
- Axon server running on `http://127.0.0.1:9090`

## Step 1: Install Dependencies

```bash
cd /Users/taaliman/projects/luxquant/ry-ht/ryht/dashboard
npm install
```

## Step 2: Configure Environment

Copy the example environment file:

```bash
cp .env.example .env
```

Edit `.env` and update the configuration if needed:

```bash
# Axon Multi-Agent System API
VITE_AXON_API_URL=http://127.0.0.1:9090/api/v1
VITE_AXON_WS_URL=ws://127.0.0.1:9090/api/v1/ws
VITE_AXON_API_KEY=axon-dev-key-change-in-production
```

## Step 3: Start the Axon Server

In a separate terminal, start the Axon server:

```bash
cd /Users/taaliman/projects/luxquant/ry-ht/ryht/axon
cargo run -- server start --host 127.0.0.1 --port 9090
```

Wait for the server to be ready. You should see:
```
Axon API server running on http://127.0.0.1:9090
```

## Step 4: Start the Dashboard

```bash
npm run dev
```

The dashboard will start at `http://localhost:5173`

## Step 5: Explore the Dashboard

### Main Dashboard
Navigate to `http://localhost:5173/dashboard` to see:
- System health status
- Active agents count
- Running workflows
- WebSocket connection status

### Create Your First Agent

1. Go to **Agents** from the sidebar navigation
2. Click **Create Agent**
3. Fill in the form:
   - **Name:** `my-first-developer`
   - **Agent Type:** `Developer`
   - **Capabilities:** Select `Code Generation` and `Code Review`
   - **Max Concurrent Tasks:** `1`
4. Click **Create Agent**

The agent will be created and you'll see it in the agent list.

### Run Your First Workflow

1. Go to **Workflows** from the sidebar navigation
2. Click **Run Workflow**
3. Use the example workflow or create your own:

```yaml
name: Simple Code Review
description: Review code changes

tasks:
  - id: review
    agent_type: Reviewer
    action: review_code
    inputs:
      files: ["src/**/*.ts"]
```

4. Provide input parameters as JSON:

```json
{
  "repository": "https://github.com/example/repo",
  "branch": "main"
}
```

5. Click **Run Workflow**

The workflow will start executing and you can monitor its progress in real-time.

## Features

### Agent Management
- **List Agents:** View all running agents with their status
- **Create Agent:** Launch new agents with specific capabilities
- **Control Agents:** Pause, resume, restart, or stop agents
- **View Metrics:** See task completion stats and performance

### Workflow Orchestration
- **List Workflows:** View all workflows and their status
- **Run Workflow:** Execute YAML-defined workflows
- **Monitor Progress:** Real-time progress tracking
- **Cancel Workflows:** Stop running workflows

### Real-time Updates
- WebSocket connection for live updates
- Automatic UI refresh when agents or workflows change
- Connection status indicator

## Common Tasks

### Viewing Agent Details

1. Go to **Agents**
2. Click on an agent in the list
3. View detailed information including:
   - Current status
   - Task completion stats
   - Average task duration
   - Capabilities

### Monitoring Workflows

1. Go to **Workflows**
2. Find your workflow in the list
3. See real-time progress updates:
   - Completed tasks / Total tasks
   - Progress bar
   - Current status

### Checking System Health

The main dashboard shows:
- System health status (healthy/unhealthy)
- Number of active agents
- Number of running workflows
- Total tasks executed
- System uptime

## Troubleshooting

### Dashboard won't start
- Check that port 5173 is available
- Run `npm install` to ensure dependencies are installed
- Check for errors in the terminal

### Can't connect to Axon server
- Verify the Axon server is running
- Check the API URL in `.env` matches the server
- Ensure no firewall is blocking port 9090

### WebSocket disconnected
- Check that the Axon server is running
- Verify the WebSocket URL in `.env`
- Check browser console for errors
- The dashboard will automatically attempt to reconnect

### Authentication errors
- Verify `VITE_AXON_API_KEY` in `.env`
- Ensure it matches the Axon server configuration
- Restart the dashboard after changing `.env`

## Next Steps

- Read the full [Integration Guide](./AXON_INTEGRATION.md)
- Explore the [Axon API Documentation](../axon/docs/api.md)
- Create custom workflows for your use cases
- Set up multiple agents for complex tasks

## Support

For issues or questions:
1. Check the [Troubleshooting](#troubleshooting) section
2. Review the [Integration Guide](./AXON_INTEGRATION.md)
3. Check the Axon server logs
4. Open an issue on the project repository
