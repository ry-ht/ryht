# Qdrant Production Setup Guide

This guide provides comprehensive instructions for setting up and operating Qdrant vector database in production for the Cortex project.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Architecture](#architecture)
- [Installation](#installation)
- [Configuration](#configuration)
- [Operations](#operations)
- [Monitoring](#monitoring)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)
- [Performance Tuning](#performance-tuning)

## Overview

Cortex uses Qdrant as its primary vector database for storing and searching embeddings. This setup includes:

- **Qdrant** - Vector database (v1.12.5)
- **Prometheus** - Metrics collection
- **Grafana** - Visualization and dashboards
- **SurrealDB** - Metadata and relational data
- **Redis** (optional) - Distributed caching
- **Jaeger** (optional) - Distributed tracing

## Quick Start

### 1. Prerequisites

```bash
# Install Docker and Docker Compose
docker --version  # Should be 20.10+
docker-compose --version  # Should be 1.29+

# Clone the repository
cd /path/to/cortex
```

### 2. Environment Setup

```bash
# Copy environment template
cp .env.example .env

# Edit .env and set your configuration
# At minimum, set:
# - QDRANT_API_KEY (for production)
# - GRAFANA_ADMIN_PASSWORD (change from default)
# - OPENAI_API_KEY (for embeddings)
```

### 3. Create Data Directories

```bash
# Create required directories
mkdir -p data/qdrant data/surrealdb logs backups

# Set permissions (Linux/Mac)
chmod -R 755 data logs backups
```

### 4. Start Services

```bash
# Start core services (Qdrant, SurrealDB, Prometheus, Grafana)
docker-compose up -d

# OR start all services including optional ones
docker-compose --profile full up -d

# Check status
docker-compose ps
```

### 5. Initialize Qdrant Collections

```bash
# Using the setup script
./scripts/setup-qdrant.sh

# OR using cortex
cargo build --release
./target/release/cortex qdrant init

# Verify setup
./target/release/cortex qdrant status
```

### 6. Access Services

- **Qdrant UI**: http://localhost:6333/dashboard
- **Grafana**: http://localhost:3000 (admin/admin)
- **Prometheus**: http://localhost:9090
- **Jaeger UI**: http://localhost:16686 (if using --profile full)

## Architecture

### Service Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   Application Layer                      │
│                    (cortex)                          │
└─────────────────────────────────────────────────────────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
        ▼                  ▼                  ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│   Qdrant     │  │  SurrealDB   │  │    Redis     │
│  (Vectors)   │  │  (Metadata)  │  │   (Cache)    │
└──────────────┘  └──────────────┘  └──────────────┘
        │
        │ metrics
        ▼
┌──────────────┐
│  Prometheus  │
│  (Metrics)   │
└──────────────┘
        │
        ▼
┌──────────────┐
│   Grafana    │
│(Dashboards)  │
└──────────────┘
```

### Collection Architecture

Cortex uses 5 specialized collections:

1. **code_vectors** (1536 dims) - Code embeddings
   - Function/class/module embeddings
   - Symbol and identifier vectors
   - AST-based semantic representations

2. **memory_vectors** (1536 dims) - Episodic and semantic memory
   - Agent interaction history
   - Long-term semantic memories
   - Context consolidation

3. **document_vectors** (1536 dims) - Documentation embeddings
   - README files
   - API documentation
   - Comments and docstrings

4. **ast_vectors** (768 dims) - AST structure embeddings
   - Syntax tree representations
   - Code structure patterns

5. **dependency_vectors** (384 dims) - Dependency graph embeddings
   - Import relationships
   - Call graphs
   - Module dependencies

## Installation

### Docker Compose Installation (Recommended)

The recommended way to run Qdrant is using the provided Docker Compose setup:

```bash
# 1. Review docker-compose.yml configuration
cat docker-compose.yml

# 2. Customize resource limits in .env
nano .env

# 3. Start services
docker-compose up -d qdrant

# 4. Check logs
docker-compose logs -f qdrant
```

### Standalone Installation

For development or testing:

```bash
# Using Docker directly
docker run -p 6333:6333 -p 6334:6334 \
  -v $(pwd)/data/qdrant:/qdrant/storage:z \
  qdrant/qdrant:v1.12.5
```

### Kubernetes Installation

For production Kubernetes deployments, see `docs/k8s/qdrant-deployment.yaml` (coming soon).

## Configuration

### Environment Variables

Key environment variables in `.env`:

```bash
# Qdrant Connection
QDRANT_HOST=localhost
QDRANT_HTTP_PORT=6333
QDRANT_GRPC_PORT=6334
QDRANT_API_KEY=your-secret-key-here  # REQUIRED for production

# Performance Tuning
QDRANT_CPU_LIMIT=4
QDRANT_MEMORY_LIMIT=8G
QDRANT_LOG_LEVEL=INFO

# Feature Flags
ENABLE_VECTOR_SEARCH=true
ENABLE_METRICS=true
```

### Collection Configuration

Edit `config/qdrant.toml` for environment-specific settings:

```toml
[production]
host = "localhost"
port = 6333

[production.hnsw]
m = 16                      # Edges per node
ef_construct = 100          # Construction quality
ef = 128                    # Search quality
full_scan_threshold = 10000

[production.optimizers]
default_segment_number = 8
max_segment_size = 200000
indexing_threshold = 20000
max_optimization_threads = 16
```

### HNSW Parameters

Optimal HNSW settings for 1M+ vectors:

| Parameter | Development | Production | Description |
|-----------|-------------|------------|-------------|
| m | 16 | 16 | Number of bi-directional links per element |
| ef_construct | 100 | 100 | Size of dynamic candidate list during construction |
| ef | 64 | 128 | Size of dynamic candidate list during search |
| full_scan_threshold | 10000 | 10000 | Switch to HNSW after this many vectors |

**Guidelines:**
- Higher `m` = Better recall, more memory
- Higher `ef_construct` = Better quality, slower indexing
- Higher `ef` = Better accuracy, slower search
- Typical ranges: m=4-64, ef_construct=100-500, ef=10-500

## Operations

### CLI Commands

Cortex provides comprehensive CLI commands for Qdrant operations:

```bash
# Initialize collections
cortex qdrant init

# Check status
cortex qdrant status
cortex qdrant status --detailed
cortex qdrant status --collection code_vectors

# List collections
cortex qdrant list
cortex qdrant list --detailed

# Verify data consistency
cortex qdrant verify
cortex qdrant verify --collection memory_vectors --fix

# Create snapshots
cortex qdrant snapshot
cortex qdrant snapshot --collection code_vectors --output ./backups

# Run benchmarks
cortex qdrant benchmark
cortex qdrant benchmark --num-queries 1000 --dimensions 1536

# Optimize collections
cortex qdrant optimize code_vectors --wait
```

### Direct API Access

Using curl for direct API access:

```bash
# Health check
curl http://localhost:6333/healthz

# List collections
curl http://localhost:6333/collections

# Get collection info
curl http://localhost:6333/collections/code_vectors

# Count points
curl -X POST http://localhost:6333/collections/code_vectors/points/count \
  -H "Content-Type: application/json" \
  -d '{"exact": false}'

# Search vectors
curl -X POST http://localhost:6333/collections/code_vectors/points/search \
  -H "Content-Type: application/json" \
  -d '{
    "vector": [0.1, 0.2, ...],  # 1536 dimensions
    "limit": 10,
    "with_payload": true
  }'
```

### Backup and Recovery

#### Create Backup

```bash
# Using CLI
cortex qdrant snapshot --output ./backups/$(date +%Y%m%d)

# Using API
curl -X POST http://localhost:6333/collections/code_vectors/snapshots
```

#### Restore Backup

```bash
# Using CLI (coming soon)
cortex qdrant restore ./backups/20250123/snapshot.tar

# Using API
curl -X PUT http://localhost:6333/collections/code_vectors/snapshots/upload \
  -F "snapshot=@./backups/snapshot.tar"
```

#### Automated Backups

Add to crontab:

```bash
# Daily backup at 2 AM
0 2 * * * cd /path/to/cortex && ./scripts/backup-qdrant.sh >> logs/backup.log 2>&1
```

## Monitoring

### Grafana Dashboards

Access Grafana at http://localhost:3000:

1. **Cortex Qdrant Overview**
   - Vector count per collection
   - Search latency (P50, P95, P99)
   - Memory usage
   - Request rate
   - Indexing progress
   - Segment counts

2. Navigate to: Dashboards → Cortex Dashboards → Qdrant Overview

### Key Metrics

Monitor these critical metrics:

| Metric | Alert Threshold | Description |
|--------|----------------|-------------|
| `qdrant_collections_vectors_total` | N/A | Total vectors per collection |
| `qdrant_collections_indexed_vectors_total` | N/A | Indexed vectors |
| `qdrant_search_duration_seconds` | P95 > 1s | Search latency |
| `qdrant_app_memory_bytes` | > 6GB | Memory usage |
| `qdrant_collections_segments_total` | > 50 | Segment count |

### Prometheus Queries

Useful queries:

```promql
# Indexing lag
qdrant_collections_vectors_total - qdrant_collections_indexed_vectors_total

# 95th percentile search latency
histogram_quantile(0.95, rate(qdrant_search_duration_seconds_bucket[5m]))

# Memory usage in GB
qdrant_app_memory_bytes / (1024^3)

# Request rate
rate(qdrant_http_requests_total[5m])
```

### Alerts

Pre-configured alerts in `config/prometheus-alerts.yml`:

- **QdrantDown** - Instance unavailable
- **QdrantHighMemoryUsage** - Memory > 6GB
- **QdrantIndexingLag** - > 10k unindexed vectors
- **QdrantHighSearchLatency** - P95 > 1s
- **QdrantSegmentOptimizationNeeded** - > 50 segments

## Best Practices

### 1. Collection Design

✅ **DO:**
- Use separate collections for different vector types
- Choose appropriate dimensions (smaller = faster)
- Set proper distance metrics (Cosine for normalized vectors)
- Create payload indexes for frequently filtered fields

❌ **DON'T:**
- Mix different embedding models in one collection
- Use unnecessarily large dimensions
- Create too many payload indexes (impacts write performance)

### 2. Indexing Strategy

For bulk ingestion (> 10k vectors):

```rust
// Disable indexing during bulk load
// Set m=0 in collection config

// Ingest all vectors
batch_upsert(vectors).await?;

// Re-enable indexing
// Set m=16 and trigger optimization
```

### 3. Search Optimization

```rust
// Use appropriate ef parameter
// Higher ef = better accuracy, slower search
search_params.ef = 128;  // Production
search_params.ef = 64;   // Development

// Use filters efficiently
filter = Filter {
    must: vec![
        Condition::matches("workspace_id", "abc123"),
        Condition::range("created_at", 1640000000, 1650000000),
    ],
};

// Limit results
limit = 10;  // Usually sufficient
```

### 4. Resource Management

**Memory:**
- 1GB per 1M vectors (approximate, depends on dimensions)
- Reserve 2x for indexing operations
- Monitor `qdrant_app_memory_bytes` metric

**CPU:**
- Reserve 1 core per 10 QPS
- Indexing uses `max_optimization_threads` (default: 16)
- Reduce if sharing host with other services

**Disk:**
- 500MB per 1M vectors (approximate)
- Use SSD for optimal performance
- Enable compression for backups

### 5. Security

Production security checklist:

- ✅ Set `QDRANT_API_KEY` in environment
- ✅ Use HTTPS/TLS in production
- ✅ Restrict network access (firewall rules)
- ✅ Enable authentication on Grafana
- ✅ Rotate API keys regularly
- ✅ Use secrets management (Vault, AWS Secrets Manager)

## Troubleshooting

### High Memory Usage

**Problem:** Qdrant consuming > 8GB RAM

**Solutions:**
1. Enable on-disk storage for vectors:
   ```yaml
   on_disk_vectors: true
   ```

2. Reduce HNSW parameters:
   ```yaml
   m: 12  # Instead of 16
   ef_construct: 80  # Instead of 100
   ```

3. Enable mmap for large segments:
   ```yaml
   memmap_threshold: 50000
   ```

### Slow Searches

**Problem:** P95 latency > 500ms

**Solutions:**
1. Increase `ef` parameter (but not too high):
   ```rust
   search_params.ef = 96;  # Try 64, 96, 128
   ```

2. Check indexing status:
   ```bash
   cortex qdrant verify
   ```

3. Optimize collections:
   ```bash
   cortex qdrant optimize code_vectors
   ```

4. Check segment count:
   ```bash
   # If > 50 segments, optimization needed
   cortex qdrant status --detailed
   ```

### Indexing Lag

**Problem:** Unindexed vectors accumulating

**Solutions:**
1. Check optimizer status:
   ```bash
   curl http://localhost:6333/collections/code_vectors | jq .optimizer_status
   ```

2. Increase indexing threshold:
   ```yaml
   indexing_threshold: 20000  # Batch more vectors
   ```

3. Increase optimization threads:
   ```yaml
   max_optimization_threads: 24  # If CPU available
   ```

### Connection Failures

**Problem:** Cannot connect to Qdrant

**Solutions:**
1. Check if service is running:
   ```bash
   docker-compose ps qdrant
   curl http://localhost:6333/healthz
   ```

2. Check logs:
   ```bash
   docker-compose logs qdrant
   ```

3. Verify ports:
   ```bash
   netstat -an | grep 6333
   ```

4. Check API key:
   ```bash
   # Ensure QDRANT_API_KEY matches in .env and client
   echo $QDRANT_API_KEY
   ```

## Performance Tuning

### For High Throughput (> 100 QPS)

```yaml
# docker-compose.yml
services:
  qdrant:
    deploy:
      resources:
        limits:
          cpus: '8'
          memory: 16G

# config/qdrant.yaml
storage:
  optimizers:
    default_segment_number: 16  # More parallelism
    max_optimization_threads: 24
    indexing_threshold: 50000  # Larger batches
```

### For Low Latency (< 50ms P95)

```yaml
# config/qdrant.yaml
storage:
  hnsw_index:
    m: 32  # More edges = faster search
    ef_construct: 200  # Better graph quality
  on_disk_vectors: false  # Keep in memory
  performance:
    max_search_threads: 8
```

### For Large Scale (> 10M vectors)

```yaml
# Use sharding
service:
  shard_number: 4

# Increase segment size
storage:
  optimizers:
    max_segment_size: 500000
    default_segment_number: 16

# Enable compression
quantization_config:
  scalar:
    type: "int8"
    always_ram: true
```

### Benchmark Results

Expected performance on modern hardware (8 cores, 16GB RAM):

| Operation | Throughput | Latency (P95) |
|-----------|------------|---------------|
| Search (1536d) | 500 QPS | 45ms |
| Upsert (batch 100) | 10k vectors/s | N/A |
| Point retrieval | 2000 QPS | 5ms |
| Filtered search | 300 QPS | 80ms |

## Additional Resources

- **Qdrant Documentation**: https://qdrant.tech/documentation/
- **Prometheus Querying**: https://prometheus.io/docs/prometheus/latest/querying/basics/
- **Grafana Tutorials**: https://grafana.com/tutorials/
- **HNSW Algorithm**: https://arxiv.org/abs/1603.09320

## Support

For issues and questions:

1. Check this documentation
2. Review logs: `docker-compose logs qdrant`
3. Check metrics: http://localhost:3000
4. Search GitHub issues
5. Contact the team

---

**Last Updated:** 2025-10-23
**Version:** 1.0.0
**Cortex Version:** 0.1.0
