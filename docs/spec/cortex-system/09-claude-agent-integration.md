# Cortex: Claude Agent SDK Integration

## Overview

This document details the integration between Cortex and the Claude Agent SDK, enabling sophisticated multi-agent workflows with Claude's advanced capabilities. The integration allows Claude agents to work directly in the cognitive memory layer through MCP tools.

## Claude Agent SDK Architecture

### SDK Components

```typescript
// From @anthropic-ai/claude-agent-sdk
interface ClaudeAgent {
  id: string;
  name: string;
  role: AgentRole;
  capabilities: Capability[];
  memory: AgentMemory;
  tools: Tool[];

  async execute(task: Task): Promise<Result>;
  async collaborate(agents: ClaudeAgent[]): Promise<CollaborationResult>;
}

interface AgentMemory {
  shortTerm: ShortTermMemory;
  longTerm: LongTermMemory;
  episodic: EpisodicMemory;

  async store(key: string, value: any): Promise<void>;
  async retrieve(key: string): Promise<any>;
  async search(query: string): Promise<SearchResult[]>;
}
```

### Integration Points

```
┌─────────────────────────────────────────────────────────┐
│                  Claude Agent SDK                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │    Agent    │  │   Memory    │  │   Tools     │    │
│  │  Orchestor  │  │   Manager   │  │  Registry   │    │
│  └─────────────┘  └─────────────┘  └─────────────┘    │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                    MCP Adapter Layer                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │   Protocol  │  │   Session   │  │   Memory    │    │
│  │   Adapter   │  │   Bridge    │  │   Sync      │    │
│  └─────────────┘  └─────────────┘  └─────────────┘    │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                    Cortex Core                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │  150+ MCP   │  │  Cognitive  │  │   Multi-    │    │
│  │    Tools    │  │   Memory    │  │   Agent     │    │
│  └─────────────┘  └─────────────┘  └─────────────┘    │
└─────────────────────────────────────────────────────────┘
```

## Implementation

### Claude Agent Adapter

```typescript
// claude-agent-adapter.ts

import { ClaudeAgent, AgentConfig } from '@anthropic-ai/claude-agent-sdk';
import { CortexClient } from '@cortex/client';

export class CortexClaudeAgent extends ClaudeAgent {
  private cortexClient: CortexClient;
  private sessionId: string | null = null;

  constructor(config: AgentConfig) {
    super(config);

    // Initialize Cortex connection
    this.cortexClient = new CortexClient({
      endpoint: process.env.CORTEX_ENDPOINT || 'stdio://',
      auth: {
        agentId: config.id,
        token: config.authToken
      }
    });

    // Register Cortex tools with Claude
    this.registerCortexTools();

    // Sync memory systems
    this.syncMemory();
  }

  private registerCortexTools(): void {
    // Map Cortex MCP tools to Claude tool format
    const cortexTools = this.cortexClient.listTools();

    for (const tool of cortexTools) {
      this.registerTool({
        name: tool.name,
        description: tool.description,
        parameters: tool.parameters,
        execute: async (params) => {
          return this.cortexClient.callTool(tool.name, params);
        }
      });
    }
  }

  private syncMemory(): void {
    // Bridge Claude's memory with Cortex's cognitive memory
    this.memory.setAdapter({
      store: async (key: string, value: any) => {
        await this.cortexClient.callTool('cortex.memory.store', {
          key,
          value,
          agent_id: this.id
        });
      },

      retrieve: async (key: string) => {
        return this.cortexClient.callTool('cortex.memory.retrieve', {
          key,
          agent_id: this.id
        });
      },

      search: async (query: string) => {
        return this.cortexClient.callTool('cortex.memory.find_similar_episodes', {
          query,
          limit: 10
        });
      }
    });
  }

  async startSession(scope: string[]): Promise<void> {
    // Create isolated Cortex session for this agent
    const result = await this.cortexClient.callTool('cortex.session.create', {
      agent_id: this.id,
      scope_paths: scope,
      isolation_level: 'snapshot',
      ttl_seconds: 3600
    });

    this.sessionId = result.session_id;
  }

  async execute(task: Task): Promise<Result> {
    // Start session if not already started
    if (!this.sessionId) {
      await this.startSession(task.scope || ['/']);
    }

    try {
      // Execute task using Cortex tools
      const result = await super.execute(task);

      // Record episode
      await this.recordEpisode(task, result);

      return result;
    } finally {
      // Merge session changes
      if (this.sessionId) {
        await this.cortexClient.callTool('cortex.session.merge', {
          session_id: this.sessionId,
          merge_strategy: 'auto'
        });
      }
    }
  }

  private async recordEpisode(task: Task, result: Result): Promise<void> {
    await this.cortexClient.callTool('cortex.memory.record_episode', {
      task_description: task.description,
      solution_summary: result.summary,
      outcome: result.success ? 'success' : 'failure',
      entities_affected: result.affectedEntities || [],
      duration_seconds: result.duration,
      agent_id: this.id
    });
  }
}
```

### Multi-Agent Orchestration

```typescript
// multi-agent-orchestrator.ts

export class CortexOrchestrator {
  private agents: Map<string, CortexClaudeAgent> = new Map();
  private cortexClient: CortexClient;

  constructor() {
    this.cortexClient = new CortexClient();
  }

  async createAgent(config: AgentConfig): Promise<CortexClaudeAgent> {
    const agent = new CortexClaudeAgent(config);

    // Register agent in Cortex
    await this.cortexClient.callTool('cortex.agent.register', {
      agent_id: config.id,
      agent_type: config.role,
      capabilities: config.capabilities
    });

    this.agents.set(config.id, agent);
    return agent;
  }

  async executeWorkflow(workflow: Workflow): Promise<WorkflowResult> {
    // Create agents for workflow
    const agents = await this.createWorkflowAgents(workflow);

    // Execute workflow steps
    const results = [];

    for (const step of workflow.steps) {
      const agent = agents.get(step.agentType);

      if (!agent) {
        throw new Error(`No agent available for ${step.agentType}`);
      }

      // Check dependencies
      if (step.dependencies) {
        await this.waitForDependencies(step.dependencies, results);
      }

      // Execute step
      const result = await agent.execute({
        description: step.description,
        action: step.action,
        parameters: step.parameters,
        scope: step.scope
      });

      results.push({
        stepId: step.id,
        result
      });

      // Handle parallel steps
      if (step.parallel) {
        await this.executeParallelSteps(step.parallel, agents);
      }
    }

    return {
      success: results.every(r => r.result.success),
      results
    };
  }

  private async createWorkflowAgents(workflow: Workflow): Promise<Map<string, CortexClaudeAgent>> {
    const agents = new Map();

    for (const agentType of workflow.requiredAgents) {
      const config = this.getAgentConfig(agentType);
      const agent = await this.createAgent(config);
      agents.set(agentType, agent);
    }

    return agents;
  }

  async collaborativeTask(task: CollaborativeTask): Promise<CollaborativeResult> {
    // Create specialized agents
    const developer = await this.createAgent({
      id: 'dev-' + generateId(),
      role: 'developer',
      capabilities: ['code-generation', 'refactoring']
    });

    const reviewer = await this.createAgent({
      id: 'rev-' + generateId(),
      role: 'reviewer',
      capabilities: ['code-review', 'best-practices']
    });

    const tester = await this.createAgent({
      id: 'test-' + generateId(),
      role: 'tester',
      capabilities: ['test-generation', 'test-execution']
    });

    // Developer implements feature
    const implementation = await developer.execute({
      description: `Implement ${task.feature}`,
      action: 'implement',
      parameters: task.requirements
    });

    // Reviewer reviews code
    const review = await reviewer.execute({
      description: `Review implementation of ${task.feature}`,
      action: 'review',
      parameters: {
        implementation: implementation.result,
        standards: task.codingStandards
      }
    });

    // Handle review feedback
    if (review.result.needsChanges) {
      const fixes = await developer.execute({
        description: `Address review feedback`,
        action: 'fix',
        parameters: {
          feedback: review.result.feedback
        }
      });

      implementation.result = fixes.result;
    }

    // Tester creates and runs tests
    const tests = await tester.execute({
      description: `Create tests for ${task.feature}`,
      action: 'test',
      parameters: {
        implementation: implementation.result,
        coverage_target: 0.8
      }
    });

    return {
      implementation,
      review,
      tests,
      success: tests.result.passed
    };
  }
}
```

## Agent Patterns

### Pattern 1: Specialized Agent Roles

```typescript
// agent-roles.ts

export const AGENT_ROLES = {
  ARCHITECT: {
    id: 'architect',
    capabilities: [
      'design-system',
      'architecture-review',
      'dependency-analysis'
    ],
    tools: [
      'cortex.deps.*',
      'cortex.graph.*',
      'cortex.quality.*'
    ]
  },

  DEVELOPER: {
    id: 'developer',
    capabilities: [
      'code-generation',
      'refactoring',
      'bug-fixing'
    ],
    tools: [
      'cortex.code.*',
      'cortex.vfs.*',
      'cortex.test.*'
    ]
  },

  REVIEWER: {
    id: 'reviewer',
    capabilities: [
      'code-review',
      'security-audit',
      'performance-review'
    ],
    tools: [
      'cortex.quality.*',
      'cortex.search.*',
      'cortex.analyze.*'
    ]
  },

  TESTER: {
    id: 'tester',
    capabilities: [
      'test-generation',
      'test-execution',
      'coverage-analysis'
    ],
    tools: [
      'cortex.test.*',
      'cortex.validate.*',
      'cortex.coverage.*'
    ]
  },

  DOCUMENTER: {
    id: 'documenter',
    capabilities: [
      'documentation-generation',
      'api-docs',
      'user-guides'
    ],
    tools: [
      'cortex.doc.*',
      'cortex.export.*'
    ]
  },

  ORCHESTRATOR: {
    id: 'orchestrator',
    capabilities: [
      'workflow-management',
      'agent-coordination',
      'task-distribution'
    ],
    tools: [
      'cortex.agent.*',
      'cortex.session.*',
      'cortex.workflow.*'
    ]
  }
};
```

### Pattern 2: Swarm Intelligence

```typescript
// swarm-intelligence.ts

export class SwarmIntelligence {
  private agents: CortexClaudeAgent[] = [];
  private sharedMemory: SharedMemory;

  async solveComplex(problem: ComplexProblem): Promise<Solution> {
    // Create swarm of agents
    const swarmSize = this.calculateSwarmSize(problem);

    for (let i = 0; i < swarmSize; i++) {
      const agent = await this.createSwarmAgent(i);
      this.agents.push(agent);
    }

    // Divide problem into subproblems
    const subproblems = this.decomposeProblem(problem);

    // Distribute subproblems to agents
    const assignments = this.distributeWork(subproblems, this.agents);

    // Execute in parallel with communication
    const solutions = await Promise.all(
      assignments.map(async ({ agent, subproblem }) => {
        // Each agent works on their subproblem
        const solution = await agent.execute({
          description: subproblem.description,
          action: 'solve',
          parameters: subproblem.parameters
        });

        // Share insights with swarm
        await this.shareInsights(agent.id, solution);

        // Learn from other agents
        const insights = await this.getSwarmInsights();

        // Refine solution based on swarm intelligence
        return agent.refine(solution, insights);
      })
    );

    // Combine solutions
    return this.combineSolutions(solutions);
  }

  private async shareInsights(agentId: string, solution: Solution): Promise<void> {
    await this.sharedMemory.store(`insights:${agentId}`, {
      patterns: solution.patterns,
      approach: solution.approach,
      confidence: solution.confidence
    });

    // Broadcast to other agents
    await this.broadcast({
      from: agentId,
      type: 'insight',
      content: solution.keyInsights
    });
  }

  private async getSwarmInsights(): Promise<Insight[]> {
    const insights = [];

    for (const agent of this.agents) {
      const agentInsights = await this.sharedMemory.retrieve(`insights:${agent.id}`);
      if (agentInsights) {
        insights.push(agentInsights);
      }
    }

    return this.synthesizeInsights(insights);
  }
}
```

### Pattern 3: Evolutionary Development

```typescript
// evolutionary-development.ts

export class EvolutionaryDevelopment {
  private population: Solution[] = [];
  private generation = 0;

  async evolve(problem: Problem, generations: number): Promise<Solution> {
    // Initialize population with diverse solutions
    await this.initializePopulation(problem);

    for (let gen = 0; gen < generations; gen++) {
      this.generation = gen;

      // Evaluate fitness of each solution
      await this.evaluateFitness();

      // Select best solutions
      const selected = this.selection();

      // Create new solutions through crossover
      const offspring = await this.crossover(selected);

      // Apply mutations
      await this.mutate(offspring);

      // Replace population
      this.population = [...selected, ...offspring];

      // Check for convergence
      if (this.hasConverged()) {
        break;
      }
    }

    // Return best solution
    return this.getBestSolution();
  }

  private async initializePopulation(problem: Problem): Promise<void> {
    const populationSize = 20;

    for (let i = 0; i < populationSize; i++) {
      const agent = await this.createEvolutionaryAgent(i);

      const solution = await agent.execute({
        description: problem.description,
        action: 'generate-solution',
        parameters: {
          approach: this.getRandomApproach(),
          constraints: problem.constraints
        }
      });

      this.population.push(solution);
    }
  }

  private async evaluateFitness(): Promise<void> {
    for (const solution of this.population) {
      // Test solution
      const testResults = await this.testSolution(solution);

      // Analyze code quality
      const qualityMetrics = await this.analyzeQuality(solution);

      // Calculate fitness score
      solution.fitness = this.calculateFitness(testResults, qualityMetrics);
    }
  }

  private async crossover(parents: Solution[]): Promise<Solution[]> {
    const offspring = [];

    for (let i = 0; i < parents.length - 1; i += 2) {
      const parent1 = parents[i];
      const parent2 = parents[i + 1];

      // Combine solutions
      const child = await this.combineSolutions(parent1, parent2);
      offspring.push(child);
    }

    return offspring;
  }
}
```

## Workflow Examples

### Example 1: Feature Development Workflow

```yaml
# feature-development.yaml
name: Feature Development
description: Complete feature development with review and testing

agents:
  - type: architect
    id: arch-001
    capabilities: [design, review]

  - type: developer
    id: dev-001
    capabilities: [implement, refactor]

  - type: tester
    id: test-001
    capabilities: [test-generation]

  - type: documenter
    id: doc-001
    capabilities: [documentation]

steps:
  - id: design
    agent: arch-001
    action: design_architecture
    parameters:
      requirements: ${requirements}
      constraints: ${constraints}

  - id: implement
    agent: dev-001
    action: implement_feature
    dependencies: [design]
    parameters:
      design: ${design.output}
      language: typescript

  - id: test
    agent: test-001
    action: generate_tests
    dependencies: [implement]
    parameters:
      implementation: ${implement.output}
      coverage_target: 0.8

  - id: document
    agent: doc-001
    action: generate_docs
    dependencies: [implement]
    parallel: true
    parameters:
      code: ${implement.output}
      format: markdown

  - id: review
    agent: arch-001
    action: review_implementation
    dependencies: [implement, test]
    parameters:
      implementation: ${implement.output}
      tests: ${test.output}
```

### Example 2: Refactoring Workflow

```typescript
// refactoring-workflow.ts

export async function refactorComplexModule(modulePath: string): Promise<RefactorResult> {
  const orchestrator = new CortexOrchestrator();

  // Phase 1: Analysis
  const analyzer = await orchestrator.createAgent({
    id: 'analyzer-001',
    role: 'analyst',
    capabilities: ['complexity-analysis', 'pattern-detection']
  });

  const analysis = await analyzer.execute({
    description: 'Analyze module complexity',
    action: 'analyze',
    parameters: {
      module: modulePath,
      metrics: ['cyclomatic', 'cognitive', 'coupling']
    }
  });

  // Phase 2: Planning
  const architect = await orchestrator.createAgent({
    id: 'architect-001',
    role: 'architect',
    capabilities: ['refactoring-planning']
  });

  const plan = await architect.execute({
    description: 'Create refactoring plan',
    action: 'plan',
    parameters: {
      analysis: analysis.result,
      goals: {
        max_complexity: 10,
        max_coupling: 5
      }
    }
  });

  // Phase 3: Implementation
  const developer = await orchestrator.createAgent({
    id: 'developer-001',
    role: 'developer',
    capabilities: ['refactoring']
  });

  const refactored = await developer.execute({
    description: 'Execute refactoring plan',
    action: 'refactor',
    parameters: {
      plan: plan.result,
      preserve_behavior: true
    }
  });

  // Phase 4: Validation
  const tester = await orchestrator.createAgent({
    id: 'tester-001',
    role: 'tester',
    capabilities: ['regression-testing']
  });

  const validation = await tester.execute({
    description: 'Validate refactoring',
    action: 'test',
    parameters: {
      original: modulePath,
      refactored: refactored.result,
      test_suite: 'existing'
    }
  });

  return {
    analysis,
    plan,
    refactored,
    validation,
    success: validation.result.passed
  };
}
```

## Memory Integration

### Shared Cognitive Memory

```typescript
// cognitive-memory-integration.ts

export class CognitiveMemoryIntegration {
  private cortexClient: CortexClient;

  async syncAgentMemory(agent: ClaudeAgent): Promise<void> {
    // Sync short-term memory
    const workingMemory = await agent.memory.getWorkingMemory();

    await this.cortexClient.callTool('cortex.memory.update_working_set', {
      agent_id: agent.id,
      items: workingMemory.items,
      attention_weights: workingMemory.weights
    });

    // Sync episodic memory
    const episodes = await agent.memory.getRecentEpisodes();

    for (const episode of episodes) {
      await this.cortexClient.callTool('cortex.memory.record_episode', {
        task_description: episode.task,
        solution_summary: episode.solution,
        outcome: episode.outcome,
        agent_id: agent.id
      });
    }
  }

  async loadAgentMemory(agent: ClaudeAgent): Promise<void> {
    // Load relevant episodes
    const episodes = await this.cortexClient.callTool(
      'cortex.memory.find_similar_episodes',
      {
        query: agent.currentTask?.description || '',
        limit: 20,
        agent_id: agent.id
      }
    );

    // Load into agent memory
    for (const episode of episodes) {
      await agent.memory.addEpisode({
        task: episode.task_description,
        solution: episode.solution_summary,
        outcome: episode.outcome,
        relevance: episode.similarity_score
      });
    }

    // Load working memory
    const workingSet = await this.cortexClient.callTool(
      'cortex.memory.get_working_set',
      {
        agent_id: agent.id
      }
    );

    await agent.memory.setWorkingMemory(workingSet);
  }
}
```

## Performance Optimization

### Agent Pool Management

```typescript
// agent-pool.ts

export class AgentPool {
  private availableAgents: Queue<CortexClaudeAgent> = new Queue();
  private busyAgents: Map<string, CortexClaudeAgent> = new Map();
  private maxAgents = 10;

  async getAgent(requirements: AgentRequirements): Promise<CortexClaudeAgent> {
    // Check for available agent with required capabilities
    let agent = this.findAvailableAgent(requirements);

    if (!agent && this.busyAgents.size < this.maxAgents) {
      // Create new agent if under limit
      agent = await this.createAgent(requirements);
    }

    if (!agent) {
      // Wait for an agent to become available
      agent = await this.waitForAgent(requirements);
    }

    // Mark as busy
    this.busyAgents.set(agent.id, agent);

    return agent;
  }

  async releaseAgent(agent: CortexClaudeAgent): Promise<void> {
    // Clean up agent session
    await agent.cleanup();

    // Move to available pool
    this.busyAgents.delete(agent.id);
    this.availableAgents.enqueue(agent);
  }

  private findAvailableAgent(requirements: AgentRequirements): CortexClaudeAgent | null {
    for (const agent of this.availableAgents) {
      if (this.meetsRequirements(agent, requirements)) {
        return this.availableAgents.dequeue();
      }
    }
    return null;
  }
}
```

### Caching Strategy

```typescript
// agent-cache.ts

export class AgentCache {
  private responseCache: LRUCache<string, any> = new LRUCache(1000);
  private ttl = 300; // 5 minutes

  async getCachedOrExecute(
    agent: CortexClaudeAgent,
    task: Task
  ): Promise<Result> {
    const cacheKey = this.generateCacheKey(agent.id, task);

    // Check cache
    const cached = this.responseCache.get(cacheKey);
    if (cached && !this.isExpired(cached)) {
      return cached.result;
    }

    // Execute task
    const result = await agent.execute(task);

    // Cache result if cacheable
    if (this.isCacheable(task, result)) {
      this.responseCache.set(cacheKey, {
        result,
        timestamp: Date.now()
      });
    }

    return result;
  }

  private isCacheable(task: Task, result: Result): boolean {
    // Don't cache mutations
    if (task.action.includes('create') ||
        task.action.includes('update') ||
        task.action.includes('delete')) {
      return false;
    }

    // Don't cache failed results
    if (!result.success) {
      return false;
    }

    return true;
  }
}
```

## Error Handling

### Agent Error Recovery

```typescript
// error-recovery.ts

export class AgentErrorRecovery {
  async executeWithRecovery(
    agent: CortexClaudeAgent,
    task: Task,
    maxRetries = 3
  ): Promise<Result> {
    let lastError: Error | null = null;

    for (let attempt = 1; attempt <= maxRetries; attempt++) {
      try {
        // Create checkpoint before execution
        const checkpoint = await this.createCheckpoint(agent);

        // Execute task
        const result = await agent.execute(task);

        // Validate result
        if (this.isValidResult(result)) {
          return result;
        }

        throw new Error('Invalid result');

      } catch (error) {
        lastError = error as Error;

        // Log error
        console.error(`Attempt ${attempt} failed:`, error);

        // Analyze error
        const errorType = this.classifyError(error);

        // Apply recovery strategy
        switch (errorType) {
          case 'SESSION_EXPIRED':
            await agent.startSession(task.scope || ['/']);
            break;

          case 'LOCK_CONFLICT':
            await this.waitAndRetry(attempt * 1000);
            break;

          case 'MERGE_CONFLICT':
            await this.resolveConflicts(agent, task);
            break;

          case 'SYNTAX_ERROR':
            // Modify task to fix syntax
            task = await this.fixSyntaxErrors(task, error);
            break;

          default:
            // Rollback to checkpoint
            await this.rollbackToCheckpoint(checkpoint);
        }
      }
    }

    throw new Error(`Failed after ${maxRetries} attempts: ${lastError}`);
  }

  private async resolveConflicts(agent: CortexClaudeAgent, task: Task): Promise<void> {
    // Get conflicts
    const conflicts = await agent.getConflicts();

    // Use AI to resolve
    for (const conflict of conflicts) {
      const resolution = await agent.resolveConflict({
        conflict,
        strategy: 'ai-assisted',
        context: task.context
      });

      await agent.applyResolution(resolution);
    }
  }
}
```

## Monitoring & Observability

### Agent Telemetry

```typescript
// agent-telemetry.ts

export class AgentTelemetry {
  private metrics: MetricsCollector;
  private tracer: Tracer;

  instrumentAgent(agent: CortexClaudeAgent): CortexClaudeAgent {
    // Wrap execute method
    const originalExecute = agent.execute.bind(agent);

    agent.execute = async (task: Task): Promise<Result> => {
      const span = this.tracer.startSpan('agent.execute', {
        attributes: {
          'agent.id': agent.id,
          'agent.role': agent.role,
          'task.action': task.action
        }
      });

      const startTime = Date.now();

      try {
        const result = await originalExecute(task);

        // Record metrics
        this.metrics.record({
          name: 'agent.task.duration',
          value: Date.now() - startTime,
          tags: {
            agent_id: agent.id,
            task_action: task.action,
            success: result.success
          }
        });

        span.setStatus({ code: SpanStatusCode.OK });
        return result;

      } catch (error) {
        // Record error
        this.metrics.record({
          name: 'agent.task.error',
          value: 1,
          tags: {
            agent_id: agent.id,
            task_action: task.action,
            error_type: error.constructor.name
          }
        });

        span.recordException(error);
        span.setStatus({ code: SpanStatusCode.ERROR });
        throw error;

      } finally {
        span.end();
      }
    };

    return agent;
  }
}
```

## Security

### Agent Authentication

```typescript
// agent-auth.ts

export class AgentAuthenticator {
  async authenticateAgent(credentials: AgentCredentials): Promise<AuthToken> {
    // Verify agent identity
    const agent = await this.verifyIdentity(credentials);

    // Check permissions
    const permissions = await this.getAgentPermissions(agent.id);

    // Generate JWT token
    const token = jwt.sign({
      agent_id: agent.id,
      role: agent.role,
      permissions,
      exp: Date.now() + 3600000 // 1 hour
    }, process.env.JWT_SECRET);

    // Register session
    await this.registerAgentSession(agent.id, token);

    return token;
  }

  async authorizeToolAccess(
    agentId: string,
    toolName: string
  ): Promise<boolean> {
    const permissions = await this.getAgentPermissions(agentId);

    // Check if tool is in allowed list
    if (permissions.allowedTools.includes(toolName)) {
      return true;
    }

    // Check pattern matching
    for (const pattern of permissions.toolPatterns) {
      if (new RegExp(pattern).test(toolName)) {
        return true;
      }
    }

    return false;
  }
}
```

## Deployment

### Docker Configuration

```dockerfile
# Dockerfile.claude-agent
FROM node:20-alpine as builder

WORKDIR /app

# Install dependencies
COPY package*.json ./
RUN npm ci

# Build TypeScript
COPY tsconfig.json ./
COPY src ./src
RUN npm run build

# Production image
FROM node:20-alpine

WORKDIR /app

# Copy built application
COPY --from=builder /app/dist ./dist
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/package*.json ./

# Install Claude Agent SDK
RUN npm install @anthropic-ai/claude-agent-sdk

ENV NODE_ENV=production
ENV CORTEX_ENDPOINT=stdio://

ENTRYPOINT ["node", "dist/index.js"]
```

### Kubernetes Deployment

```yaml
# claude-agent-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: claude-agent-pool
spec:
  replicas: 5
  selector:
    matchLabels:
      app: claude-agent
  template:
    metadata:
      labels:
        app: claude-agent
    spec:
      containers:
      - name: claude-agent
        image: cortex/claude-agent:v3
        env:
        - name: AGENT_ROLE
          value: developer
        - name: CORTEX_ENDPOINT
          value: cortex-service:8080
        - name: MAX_CONCURRENT_TASKS
          value: "3"
        resources:
          requests:
            memory: "1Gi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "1"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
```

## Best Practices

### 1. Agent Lifecycle Management

```typescript
// Always clean up agents after use
const agent = await orchestrator.createAgent(config);
try {
  const result = await agent.execute(task);
  return result;
} finally {
  await agent.cleanup();
  await orchestrator.releaseAgent(agent);
}
```

### 2. Session Isolation

```typescript
// Use separate sessions for each task
await agent.startSession({
  scope: ['/src/feature-x'],
  isolation: 'snapshot'
});
```

### 3. Memory Optimization

```typescript
// Clear working memory between tasks
await agent.memory.clearWorkingMemory();

// Compress episodic memory periodically
await agent.memory.compressEpisodes({
  olderThan: '30d',
  keepImportant: true
});
```

### 4. Error Handling

```typescript
// Always handle errors gracefully
try {
  const result = await agent.execute(task);
} catch (error) {
  if (error.code === 'MERGE_CONFLICT') {
    // Handle merge conflict
  } else if (error.code === 'SESSION_EXPIRED') {
    // Restart session
  } else {
    // Generic error handling
  }
}
```

## Conclusion

The integration between Cortex and Claude Agent SDK enables:

1. **Seamless Memory Integration**: Claude agents work directly with Cortex's cognitive memory
2. **Advanced Orchestration**: Complex multi-agent workflows with coordination
3. **Session Isolation**: Safe parallel development without conflicts
4. **Tool Accessibility**: 150+ MCP tools available to all agents
5. **Performance Optimization**: Agent pooling, caching, and telemetry
6. **Enterprise Ready**: Security, monitoring, and scalable deployment

This integration creates a powerful platform for AI-driven software development at scale.