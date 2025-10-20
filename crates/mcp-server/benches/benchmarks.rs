//! Performance benchmarks for the MCP server framework.
//!
//! This benchmark suite measures the performance of critical operations to ensure
//! they meet the target metrics from the specification:
//!
//! - Tool registration: < 1μs
//! - Schema generation: < 10μs
//! - Request parsing: < 100μs
//! - Tool execution overhead: < 50μs
//!
//! Run benchmarks with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mcp_server::{
    error::ToolError,
    hooks::{Hook, HookEvent, HookRegistry},
    middleware::{LoggingMiddleware, Middleware, MetricsMiddleware, RequestContext},
    protocol::{JsonRpcRequest, JsonRpcResponse},
    server::McpServer,
    tool::{Tool, ToolContext, ToolRegistry, ToolResult},
};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::runtime::Runtime;

// ================================================================================================
// Benchmark Tools
// ================================================================================================

/// Simple tool for benchmarking - minimal overhead
struct SimpleTool;

#[async_trait::async_trait]
impl Tool for SimpleTool {
    fn name(&self) -> &str {
        "simple"
    }

    fn description(&self) -> Option<&str> {
        Some("A simple benchmark tool")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "value": { "type": "string" }
            }
        })
    }

    async fn execute(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::success_text("ok"))
    }
}

/// Tool with complex schema for benchmarking schema generation
struct ComplexSchemaTool;

#[async_trait::async_trait]
impl Tool for ComplexSchemaTool {
    fn name(&self) -> &str {
        "complex"
    }

    fn description(&self) -> Option<&str> {
        Some("A tool with complex schema")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "string_field": { "type": "string", "description": "A string field" },
                "number_field": { "type": "number", "description": "A number field" },
                "boolean_field": { "type": "boolean", "description": "A boolean field" },
                "array_field": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "An array field"
                },
                "object_field": {
                    "type": "object",
                    "properties": {
                        "nested_string": { "type": "string" },
                        "nested_number": { "type": "number" }
                    },
                    "description": "An object field"
                },
                "enum_field": {
                    "type": "string",
                    "enum": ["option1", "option2", "option3"],
                    "description": "An enum field"
                }
            },
            "required": ["string_field", "number_field"]
        })
    }

    fn output_schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "result": { "type": "string" },
                "metadata": {
                    "type": "object",
                    "properties": {
                        "timestamp": { "type": "number" },
                        "duration": { "type": "number" }
                    }
                }
            }
        }))
    }

    async fn execute(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::success_json(json!({
            "result": "success",
            "metadata": {
                "timestamp": 1234567890,
                "duration": 123
            }
        })))
    }
}

// ================================================================================================
// Tool Registration Benchmarks
// ================================================================================================

fn bench_tool_registration(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("tool_registration");

    group.bench_function("single_tool", |b| {
        b.to_async(&rt).iter(|| async {
            let registry = ToolRegistry::new();
            registry.register(SimpleTool).await.unwrap();
            black_box(registry);
        });
    });

    group.bench_function("10_tools", |b| {
        b.to_async(&rt).iter(|| async {
            let registry = ToolRegistry::new();
            for i in 0..10 {
                struct DynamicTool(String);
                #[async_trait::async_trait]
                impl Tool for DynamicTool {
                    fn name(&self) -> &str {
                        &self.0
                    }
                    fn input_schema(&self) -> Value {
                        json!({})
                    }
                    async fn execute(
                        &self,
                        _: Value,
                        _: &ToolContext,
                    ) -> Result<ToolResult, ToolError> {
                        Ok(ToolResult::success_text("ok"))
                    }
                }
                registry
                    .register(DynamicTool(format!("tool_{}", i)))
                    .await
                    .unwrap();
            }
            black_box(registry);
        });
    });

    group.finish();
}

// ================================================================================================
// Tool Lookup Benchmarks
// ================================================================================================

fn bench_tool_lookup(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("tool_lookup");

    // Setup registry with varying numbers of tools
    for size in [1, 10, 50, 100].iter() {
        let registry = rt.block_on(async {
            let registry = ToolRegistry::new();
            for i in 0..*size {
                struct DynamicTool(String);
                #[async_trait::async_trait]
                impl Tool for DynamicTool {
                    fn name(&self) -> &str {
                        &self.0
                    }
                    fn input_schema(&self) -> Value {
                        json!({})
                    }
                    async fn execute(
                        &self,
                        _: Value,
                        _: &ToolContext,
                    ) -> Result<ToolResult, ToolError> {
                        Ok(ToolResult::success_text("ok"))
                    }
                }
                registry
                    .register(DynamicTool(format!("tool_{}", i)))
                    .await
                    .unwrap();
            }
            registry
        });

        group.bench_with_input(BenchmarkId::new("get", size), size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let tool = registry.get("tool_0").await;
                black_box(tool);
            });
        });

        group.bench_with_input(BenchmarkId::new("has", size), size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let exists = registry.has("tool_0").await;
                black_box(exists);
            });
        });

        group.bench_with_input(BenchmarkId::new("list", size), size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let tools = registry.list().await;
                black_box(tools);
            });
        });
    }

    group.finish();
}

// ================================================================================================
// Schema Generation Benchmarks
// ================================================================================================

fn bench_schema_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("schema_generation");

    group.bench_function("simple_schema", |b| {
        let tool = SimpleTool;
        b.iter(|| {
            let schema = tool.input_schema();
            black_box(schema);
        });
    });

    group.bench_function("complex_schema", |b| {
        let tool = ComplexSchemaTool;
        b.iter(|| {
            let schema = tool.input_schema();
            black_box(schema);
        });
    });

    group.bench_function("with_output_schema", |b| {
        let tool = ComplexSchemaTool;
        b.iter(|| {
            let input = tool.input_schema();
            let output = tool.output_schema();
            black_box((input, output));
        });
    });

    group.finish();
}

// ================================================================================================
// Request Parsing Benchmarks
// ================================================================================================

fn bench_request_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_parsing");

    let simple_request = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
    let request_with_params = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"echo","arguments":{"message":"hello"}}}"#;
    let complex_request = r#"{"jsonrpc":"2.0","id":3,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{"tools":{"listChanged":false},"resources":{},"prompts":{}},"clientInfo":{"name":"test-client","version":"1.0.0"}}}"#;

    group.throughput(Throughput::Bytes(simple_request.len() as u64));
    group.bench_function("simple_request", |b| {
        b.iter(|| {
            let req: JsonRpcRequest = serde_json::from_str(black_box(simple_request)).unwrap();
            black_box(req);
        });
    });

    group.throughput(Throughput::Bytes(request_with_params.len() as u64));
    group.bench_function("request_with_params", |b| {
        b.iter(|| {
            let req: JsonRpcRequest =
                serde_json::from_str(black_box(request_with_params)).unwrap();
            black_box(req);
        });
    });

    group.throughput(Throughput::Bytes(complex_request.len() as u64));
    group.bench_function("complex_request", |b| {
        b.iter(|| {
            let req: JsonRpcRequest = serde_json::from_str(black_box(complex_request)).unwrap();
            black_box(req);
        });
    });

    group.finish();
}

// ================================================================================================
// Tool Execution Overhead Benchmarks
// ================================================================================================

fn bench_tool_execution_overhead(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("tool_execution_overhead");

    // Measure the overhead of tool execution infrastructure vs actual work
    group.bench_function("minimal_tool", |b| {
        let tool = SimpleTool;
        let ctx = ToolContext::new();
        let input = json!({"value": "test"});

        b.to_async(&rt).iter(|| async {
            let result = tool.execute(black_box(input.clone()), &ctx).await.unwrap();
            black_box(result);
        });
    });

    group.bench_function("tool_with_json_output", |b| {
        let tool = ComplexSchemaTool;
        let ctx = ToolContext::new();
        let input = json!({
            "string_field": "test",
            "number_field": 42,
            "boolean_field": true,
            "array_field": ["a", "b", "c"],
            "object_field": {
                "nested_string": "nested",
                "nested_number": 123
            },
            "enum_field": "option1"
        });

        b.to_async(&rt).iter(|| async {
            let result = tool.execute(black_box(input.clone()), &ctx).await.unwrap();
            black_box(result);
        });
    });

    group.finish();
}

// ================================================================================================
// Middleware Chain Benchmarks
// ================================================================================================

fn bench_middleware_chain(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("middleware_chain");

    let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);
    let response = JsonRpcResponse::success(Some(json!(1)), json!({"ok": true}));

    group.bench_function("no_middleware", |b| {
        b.to_async(&rt).iter(|| async {
            let mut ctx = RequestContext::new("test".to_string());
            black_box(&mut ctx);
        });
    });

    group.bench_function("logging_middleware", |b| {
        let middleware = LoggingMiddleware::new();
        b.to_async(&rt).iter(|| async {
            let mut ctx = RequestContext::new("test".to_string());
            middleware
                .on_request(black_box(&request), &mut ctx)
                .await
                .unwrap();
            middleware
                .on_response(black_box(&response), &ctx)
                .await
                .unwrap();
            black_box(ctx);
        });
    });

    group.bench_function("metrics_middleware", |b| {
        let middleware = MetricsMiddleware::new();
        b.to_async(&rt).iter(|| async {
            let mut ctx = RequestContext::new("test".to_string());
            middleware
                .on_request(black_box(&request), &mut ctx)
                .await
                .unwrap();
            middleware
                .on_response(black_box(&response), &ctx)
                .await
                .unwrap();
            black_box(ctx);
        });
    });

    group.bench_function("two_middlewares", |b| {
        let logging = LoggingMiddleware::new();
        let metrics = MetricsMiddleware::new();

        b.to_async(&rt).iter(|| async {
            let mut ctx = RequestContext::new("test".to_string());
            logging
                .on_request(black_box(&request), &mut ctx)
                .await
                .unwrap();
            metrics
                .on_request(black_box(&request), &mut ctx)
                .await
                .unwrap();
            metrics
                .on_response(black_box(&response), &ctx)
                .await
                .unwrap();
            logging
                .on_response(black_box(&response), &ctx)
                .await
                .unwrap();
            black_box(ctx);
        });
    });

    group.finish();
}

// ================================================================================================
// Hook Emission Benchmarks
// ================================================================================================

fn bench_hook_emission(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("hook_emission");

    struct NoOpHook;

    #[async_trait::async_trait]
    impl Hook for NoOpHook {
        async fn on_event(
            &self,
            _event: &HookEvent,
        ) -> Result<(), mcp_server::error::MiddlewareError> {
            Ok(())
        }
    }

    group.bench_function("no_hooks", |b| {
        let registry = HookRegistry::new();
        b.to_async(&rt).iter(|| async {
            registry
                .emit(black_box(&HookEvent::ServerStarted))
                .await;
        });
    });

    group.bench_function("single_hook", |b| {
        let registry = rt.block_on(async {
            let registry = HookRegistry::new();
            registry.register(NoOpHook).await;
            registry
        });

        b.to_async(&rt).iter(|| async {
            registry
                .emit(black_box(&HookEvent::ServerStarted))
                .await;
        });
    });

    group.bench_function("five_hooks", |b| {
        let registry = rt.block_on(async {
            let registry = HookRegistry::new();
            for _ in 0..5 {
                registry.register(NoOpHook).await;
            }
            registry
        });

        b.to_async(&rt).iter(|| async {
            registry
                .emit(black_box(&HookEvent::ServerStarted))
                .await;
        });
    });

    group.bench_function("tool_event_emission", |b| {
        let registry = rt.block_on(async {
            let registry = HookRegistry::new();
            registry.register(NoOpHook).await;
            registry
        });

        b.to_async(&rt).iter(|| async {
            registry
                .emit(black_box(&HookEvent::ToolCalled {
                    name: "test".to_string(),
                    args: json!({}),
                }))
                .await;
        });
    });

    group.finish();
}

// ================================================================================================
// Full Request/Response Cycle Benchmarks
// ================================================================================================

fn bench_full_request_cycle(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("full_request_cycle");

    let server = McpServer::builder()
        .name("bench-server")
        .version("1.0.0")
        .tool(SimpleTool)
        .tool(ComplexSchemaTool)
        .build();

    group.bench_function("initialize", |b| {
        let request = JsonRpcRequest::new(
            Some(json!(1)),
            "initialize".to_string(),
            Some(json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            })),
        );

        b.to_async(&rt).iter(|| async {
            let response = server.handle_request(black_box(request.clone())).await;
            black_box(response);
        });
    });

    group.bench_function("tools_list", |b| {
        let request = JsonRpcRequest::new(Some(json!(2)), "tools/list".to_string(), None);

        b.to_async(&rt).iter(|| async {
            let response = server.handle_request(black_box(request.clone())).await;
            black_box(response);
        });
    });

    group.bench_function("tools_call_simple", |b| {
        let request = JsonRpcRequest::new(
            Some(json!(3)),
            "tools/call".to_string(),
            Some(json!({
                "name": "simple",
                "arguments": {"value": "test"}
            })),
        );

        b.to_async(&rt).iter(|| async {
            let response = server.handle_request(black_box(request.clone())).await;
            black_box(response);
        });
    });

    group.bench_function("tools_call_complex", |b| {
        let request = JsonRpcRequest::new(
            Some(json!(4)),
            "tools/call".to_string(),
            Some(json!({
                "name": "complex",
                "arguments": {
                    "string_field": "test",
                    "number_field": 42,
                    "boolean_field": true,
                    "array_field": ["a", "b", "c"],
                    "object_field": {
                        "nested_string": "nested",
                        "nested_number": 123
                    },
                    "enum_field": "option1"
                }
            })),
        );

        b.to_async(&rt).iter(|| async {
            let response = server.handle_request(black_box(request.clone())).await;
            black_box(response);
        });
    });

    group.finish();
}

// ================================================================================================
// Concurrent Request Benchmarks
// ================================================================================================

fn bench_concurrent_requests(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("concurrent_requests");

    let server = Arc::new(
        McpServer::builder()
            .name("bench-server")
            .version("1.0.0")
            .tool(SimpleTool)
            .build(),
    );

    for concurrency in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("tools_list", concurrency),
            concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| async {
                    let mut handles = vec![];
                    for i in 0..concurrency {
                        let server = Arc::clone(&server);
                        let handle = tokio::spawn(async move {
                            let request = JsonRpcRequest::new(
                                Some(json!(i)),
                                "tools/list".to_string(),
                                None,
                            );
                            server.handle_request(request).await
                        });
                        handles.push(handle);
                    }

                    let results = futures::future::join_all(handles).await;
                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

// ================================================================================================
// Benchmark Groups
// ================================================================================================

criterion_group!(
    benches,
    bench_tool_registration,
    bench_tool_lookup,
    bench_schema_generation,
    bench_request_parsing,
    bench_tool_execution_overhead,
    bench_middleware_chain,
    bench_hook_emission,
    bench_full_request_cycle,
    bench_concurrent_requests,
);

criterion_main!(benches);
