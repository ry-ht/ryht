# Axon REST API Server

Complete REST API server implementation for the Axon multi-agent system.

## Features

### Core API Capabilities
- **Agent Management**: Create, start, stop, pause, resume, and restart agents
- **Workflow Execution**: Run, monitor, and cancel workflows
- **System Monitoring**: Real-time metrics, telemetry, and health checks
- **Configuration**: System configuration management

### Production-Ready Features
- **Authentication**: API key-based authentication with configurable keys
- **Rate Limiting**: Tiered rate limiting to prevent abuse
- **CORS Support**: Full CORS configuration for web dashboards
- **WebSocket Support**: Real-time updates via WebSocket connections
- **Request Logging**: Detailed request/response logging
- **Error Handling**: Comprehensive error handling with proper HTTP status codes
- **OpenAPI Documentation**: Full API documentation in OpenAPI 3.0 format

## Quick Start

### Starting the Server

```bash
# Using default configuration
axon server start

# Custom host and port
axon server start --host 0.0.0.0 --port 8080

# With workers
axon server start --workers 4
```

### Environment Variables

- `AXON_API_KEY`: Custom API key (default: `axon-dev-key-change-in-production`)

### API Authentication

Include the API key in the `Authorization` header:

```bash
# Using Bearer format
curl -H "Authorization: Bearer axon-dev-key-change-in-production" \
  http://localhost:3000/api/v1/agents

# Using ApiKey format
curl -H "Authorization: ApiKey axon-dev-key-change-in-production" \
  http://localhost:3000/api/v1/agents
```

## API Endpoints

### System Endpoints

- `GET /api/v1/` - API information and endpoint list
- `GET /api/v1/health` - Health check (no auth required)
- `GET /api/v1/status` - Detailed system status

### Agent Management

- `GET /api/v1/agents` - List all agents
- `POST /api/v1/agents` - Create a new agent
- `GET /api/v1/agents/:id` - Get agent information
- `PUT /api/v1/agents/:id` - Update agent configuration
- `DELETE /api/v1/agents/:id` - Stop and remove agent
- `POST /api/v1/agents/:id/pause` - Pause agent
- `POST /api/v1/agents/:id/resume` - Resume agent
- `POST /api/v1/agents/:id/restart` - Restart agent
- `GET /api/v1/agents/:id/logs` - Get agent logs

### Workflow Management

- `GET /api/v1/workflows` - List all workflows
- `POST /api/v1/workflows` - Execute a workflow
- `GET /api/v1/workflows/:id` - Get workflow status
- `POST /api/v1/workflows/:id/cancel` - Cancel workflow
- `POST /api/v1/workflows/:id/pause` - Pause workflow
- `POST /api/v1/workflows/:id/resume` - Resume workflow

### Monitoring

- `GET /api/v1/metrics` - Get system metrics
- `POST /api/v1/metrics/export` - Export metrics to file
- `GET /api/v1/telemetry` - Get telemetry data
- `GET /api/v1/telemetry/summary` - Get telemetry summary

### Configuration

- `GET /api/v1/config` - Get current configuration
- `PUT /api/v1/config` - Update configuration
- `POST /api/v1/config/validate` - Validate configuration

### WebSocket

- `WS /api/v1/ws` - WebSocket endpoint for real-time updates

## WebSocket Usage

Connect to the WebSocket endpoint and subscribe to channels:

```javascript
const ws = new WebSocket('ws://localhost:3000/api/v1/ws');

// Subscribe to channels
ws.send(JSON.stringify({
  type: 'Subscribe',
  channels: ['agents', 'workflows', 'metrics']
}));

// Receive events
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  console.log('Received:', message);
};
```

### Available Channels

- `agents` - All agent events
- `agent:<id>` - Specific agent events
- `workflows` - All workflow events
- `workflow:<id>` - Specific workflow events
- `metrics` - System metrics updates
- `alerts` - System alerts
- `tasks` - Task updates
- `user:<id>` - User-specific events

### Event Types

```typescript
// Agent events
{
  type: "Event",
  channel: "agents",
  event: {
    type: "AgentStatusChange",
    data: {
      agent_id: "uuid",
      agent_name: "test-agent",
      status: "Working",
      timestamp: "2025-10-26T12:00:00Z"
    }
  }
}

// Workflow events
{
  type: "Event",
  channel: "workflows",
  event: {
    type: "WorkflowProgress",
    data: {
      workflow_id: "uuid",
      workflow_name: "example-workflow",
      status: "Running",
      progress: 50,
      timestamp: "2025-10-26T12:00:00Z"
    }
  }
}

// Metrics events
{
  type: "Event",
  channel: "metrics",
  event: {
    type: "MetricsUpdate",
    data: {
      active_agents: 5,
      running_workflows: 2,
      total_tasks: 10,
      cpu_usage: 45.5,
      memory_usage: 60.2,
      timestamp: "2025-10-26T12:00:00Z"
    }
  }
}
```

## Rate Limiting

The API implements tiered rate limiting:

| Tier | Limit | Window |
|------|-------|--------|
| Auth | 10 requests | 1 minute |
| Read | 1000 requests | 1 minute |
| Write | 100 requests | 1 minute |
| Execute | 50 requests | 1 minute |
| Admin | 200 requests | 1 minute |

Rate limit headers are included in responses:
- `Retry-After` - Seconds until rate limit resets

## Examples

### Creating an Agent

```bash
curl -X POST http://localhost:3000/api/v1/agents \
  -H "Authorization: Bearer axon-dev-key-change-in-production" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test-agent",
    "agent_type": "Tester",
    "capabilities": ["testing", "validation"],
    "max_concurrent_tasks": 2
  }'
```

Response:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "test-agent"
}
```

### Running a Workflow

```bash
curl -X POST http://localhost:3000/api/v1/workflows \
  -H "Authorization: Bearer axon-dev-key-change-in-production" \
  -H "Content-Type: application/json" \
  -d '{
    "workflow_def": "example-workflow",
    "input_params": {
      "target": "example-project",
      "options": {
        "verbose": true
      }
    }
  }'
```

Response:
```json
{
  "workflow_id": "660e8400-e29b-41d4-a716-446655440000"
}
```

### Getting System Status

```bash
curl http://localhost:3000/api/v1/status \
  -H "Authorization: Bearer axon-dev-key-change-in-production"
```

Response:
```json
{
  "active_agents": 5,
  "running_workflows": 2,
  "total_tasks": 10,
  "cpu_usage": 45.5,
  "memory_usage": 60.2,
  "thread_count": 8
}
```

## Architecture

### Middleware Stack

1. **CORS Layer** - Handles cross-origin requests
2. **Trace Layer** - HTTP request tracing and logging
3. **Logging Middleware** - Request/response logging
4. **Authentication Middleware** - API key validation (optional on most routes)

### Component Overview

```
┌─────────────────────────────────────┐
│         REST API Server             │
├─────────────────────────────────────┤
│  Routes                             │
│  ├─ System (/health, /status)       │
│  ├─ Agents (/agents/*)              │
│  ├─ Workflows (/workflows/*)        │
│  ├─ Monitoring (/metrics/*)         │
│  └─ Config (/config/*)              │
├─────────────────────────────────────┤
│  Middleware                         │
│  ├─ CORS                            │
│  ├─ Authentication                  │
│  ├─ Rate Limiting                   │
│  └─ Logging                         │
├─────────────────────────────────────┤
│  WebSocket Manager                  │
│  ├─ Connection Management           │
│  ├─ Channel Subscriptions           │
│  └─ Event Broadcasting              │
├─────────────────────────────────────┤
│  Runtime Manager                    │
│  ├─ Agent Lifecycle                 │
│  ├─ Workflow Execution              │
│  └─ Metrics Collection              │
└─────────────────────────────────────┘
```

## Error Handling

All errors return a consistent JSON structure:

```json
{
  "error": "ERROR_CODE",
  "message": "Human-readable error description"
}
```

### HTTP Status Codes

- `200 OK` - Successful request
- `204 No Content` - Successful request with no response body
- `400 Bad Request` - Invalid request parameters
- `401 Unauthorized` - Missing or invalid API key
- `404 Not Found` - Resource not found
- `409 Conflict` - Resource conflict
- `429 Too Many Requests` - Rate limit exceeded
- `500 Internal Server Error` - Server error

## Security Considerations

1. **API Keys**: Always use custom API keys in production
2. **HTTPS**: Deploy behind a reverse proxy with TLS in production
3. **Rate Limiting**: Prevents abuse and ensures fair resource allocation
4. **Input Validation**: All inputs are validated before processing
5. **Error Messages**: Errors don't expose sensitive system information

## Development

### File Structure

```
api/
├── mod.rs              - Module exports
├── server.rs           - Server setup and configuration
├── routes.rs           - Route handlers
├── middleware.rs       - Middleware implementations
├── error.rs            - Error types and handling
├── websocket.rs        - WebSocket support
├── openapi.yaml        - OpenAPI specification
└── README.md           - This file
```

### Adding New Endpoints

1. Define the handler function in `routes.rs`
2. Add the route in `create_routes()`
3. Update `openapi.yaml` with the new endpoint
4. Add tests in the handler's `#[cfg(test)]` section

## OpenAPI Documentation

View the full API specification in [openapi.yaml](./openapi.yaml).

You can use tools like Swagger UI or Redoc to render the documentation:

```bash
# Using swagger-ui
docker run -p 8081:8080 -e SWAGGER_JSON=/api/openapi.yaml \
  -v $(pwd)/openapi.yaml:/api/openapi.yaml swaggerapi/swagger-ui
```

## Testing

```bash
# Run API tests
cargo test --package axon --lib commands::api

# Run integration tests
cargo test --package axon --test runtime_integration_test
```

## Performance

The server is built on Axum and Tokio, providing:
- Async/await for efficient resource usage
- Connection pooling
- Request pipelining
- Automatic backpressure handling

Expected performance (single instance):
- 1000+ requests/second on modest hardware
- Sub-10ms response times for simple operations
- WebSocket support for 10000+ concurrent connections

## Deployment

### Docker

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --package axon

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/axon /usr/local/bin/
EXPOSE 3000
CMD ["axon", "server", "start", "--host", "0.0.0.0"]
```

### Systemd Service

```ini
[Unit]
Description=Axon API Server
After=network.target

[Service]
Type=simple
User=axon
Environment="AXON_API_KEY=your-secure-key"
ExecStart=/usr/local/bin/axon server start --host 0.0.0.0
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

## Monitoring

The API provides built-in monitoring via:
- Health endpoint for liveness probes
- Metrics endpoint for Prometheus integration
- Telemetry endpoint for request statistics
- WebSocket events for real-time monitoring

## Support

For issues, questions, or contributions, see the main Axon repository.
