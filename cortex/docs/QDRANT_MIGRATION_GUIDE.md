# Qdrant Migration Guide

This guide provides step-by-step instructions for migrating from HNSW or other vector storage solutions to Qdrant.

## Table of Contents

- [Overview](#overview)
- [Pre-Migration Checklist](#pre-migration-checklist)
- [Migration Strategies](#migration-strategies)
- [Step-by-Step Migration](#step-by-step-migration)
- [Validation and Testing](#validation-and-testing)
- [Rollback Procedures](#rollback-procedures)
- [Common Issues](#common-issues)

## Overview

### Why Migrate to Qdrant?

- **Performance**: 10-100x faster than traditional HNSW implementations
- **Scalability**: Handles billions of vectors efficiently
- **Features**: Advanced filtering, hybrid search, multi-vector support
- **Production-Ready**: Built-in monitoring, clustering, backups
- **Cloud-Native**: Easy deployment on Kubernetes
- **Cost-Effective**: Efficient memory usage and compression

### Migration Timeline

| Phase | Duration | Description |
|-------|----------|-------------|
| Planning | 1-2 days | Assess current setup, plan migration |
| Setup | 1 day | Deploy Qdrant infrastructure |
| Testing | 2-3 days | Test migration scripts, validate data |
| Migration | 1-7 days | Migrate data (depends on volume) |
| Validation | 1-2 days | Verify data integrity, performance |
| Cutover | 1 day | Switch production traffic |

## Pre-Migration Checklist

### 1. Inventory Your Data

```bash
# Count vectors in current system
# Example for file-based storage:
find ./vectors -name "*.vec" | wc -l

# Estimate total size
du -sh ./vectors
```

### 2. Document Current Schema

```python
# Document your current vector structure
current_schema = {
    "vector_dimension": 1536,
    "metadata_fields": {
        "workspace_id": "string",
        "file_path": "string",
        "created_at": "timestamp",
        "language": "string",
    },
    "total_vectors": 1500000,
    "storage_format": "numpy",
}
```

### 3. Performance Baseline

```bash
# Measure current performance
# - Average search latency
# - Throughput (QPS)
# - Memory usage
# - Disk usage
```

### 4. Backup Current Data

```bash
# Create full backup
tar -czf backup_vectors_$(date +%Y%m%d).tar.gz ./vectors

# Verify backup
tar -tzf backup_vectors_$(date +%Y%m%d).tar.gz | head -20
```

### 5. Infrastructure Requirements

Calculate Qdrant resource needs:

```python
# Estimate memory (rough formula)
num_vectors = 1_500_000
vector_dim = 1536
bytes_per_float = 4

# Vector storage
vector_memory = (num_vectors * vector_dim * bytes_per_float) / (1024**3)  # GB

# HNSW index overhead (2-3x vector size)
index_memory = vector_memory * 2.5

# Total memory needed
total_memory = vector_memory + index_memory
print(f"Estimated memory: {total_memory:.2f} GB")

# Recommendation: total_memory * 1.5 for headroom
recommended_memory = total_memory * 1.5
print(f"Recommended: {recommended_memory:.2f} GB")
```

## Migration Strategies

### Strategy 1: Blue-Green Migration (Recommended)

**Best for:** Production systems requiring zero downtime

**Process:**
1. Deploy Qdrant (green)
2. Migrate data to Qdrant
3. Run both systems in parallel
4. Validate Qdrant performance
5. Switch traffic to Qdrant
6. Decommission old system

**Pros:**
- Zero downtime
- Easy rollback
- Thorough validation

**Cons:**
- Requires 2x resources temporarily
- More complex setup

### Strategy 2: Rolling Migration

**Best for:** Large datasets, gradual migration

**Process:**
1. Deploy Qdrant
2. Migrate data in batches (by workspace, date range, etc.)
3. Route queries based on data location
4. Continue until fully migrated
5. Decommission old system

**Pros:**
- Lower resource requirements
- Gradual risk mitigation
- Can pause/resume

**Cons:**
- Longer migration period
- More complex routing logic

### Strategy 3: Snapshot and Restore

**Best for:** Smaller datasets, can tolerate downtime

**Process:**
1. Stop writes to old system
2. Create snapshot
3. Deploy Qdrant
4. Import snapshot
5. Validate
6. Resume operations

**Pros:**
- Simpler process
- Faster migration
- Guaranteed consistency

**Cons:**
- Requires downtime
- All-or-nothing approach

## Step-by-Step Migration

### Step 1: Deploy Qdrant Infrastructure

```bash
# 1. Clone Cortex repository
git clone https://github.com/your-org/cortex.git
cd cortex

# 2. Configure environment
cp .env.example .env
nano .env  # Set QDRANT_API_KEY, resource limits, etc.

# 3. Start Qdrant
docker-compose up -d qdrant

# 4. Verify deployment
curl http://localhost:6333/healthz

# 5. Initialize collections
./scripts/setup-qdrant.sh
```

### Step 2: Export Data from Current System

#### From Numpy Files

```python
import numpy as np
import json
from pathlib import Path

def export_vectors_to_jsonl(input_dir, output_file):
    """Export numpy vectors to JSONL format for Qdrant"""

    with open(output_file, 'w') as f:
        for vec_file in Path(input_dir).glob('**/*.npy'):
            # Load vector
            vector = np.load(vec_file).tolist()

            # Load metadata (assuming .json sidecar file)
            metadata_file = vec_file.with_suffix('.json')
            if metadata_file.exists():
                with open(metadata_file) as mf:
                    metadata = json.load(mf)
            else:
                metadata = {}

            # Create point
            point = {
                "id": str(vec_file.stem),  # Use filename as ID
                "vector": vector,
                "payload": metadata
            }

            f.write(json.dumps(point) + '\n')

# Export
export_vectors_to_jsonl('./vectors', 'export.jsonl')
```

#### From FAISS Index

```python
import faiss
import numpy as np
import json

def export_faiss_to_jsonl(index_file, metadata_file, output_file):
    """Export FAISS index to JSONL"""

    # Load FAISS index
    index = faiss.read_index(index_file)

    # Load metadata
    with open(metadata_file) as f:
        metadata = json.load(f)

    # Export vectors
    with open(output_file, 'w') as f:
        for i in range(index.ntotal):
            vector = index.reconstruct(i).tolist()

            point = {
                "id": str(i),
                "vector": vector,
                "payload": metadata.get(str(i), {})
            }

            f.write(json.dumps(point) + '\n')

# Export
export_faiss_to_jsonl('vectors.index', 'metadata.json', 'export.jsonl')
```

### Step 3: Import Data to Qdrant

#### Using Python Client

```python
from qdrant_client import QdrantClient
from qdrant_client.models import PointStruct, Batch
import json
from tqdm import tqdm

# Connect to Qdrant
client = QdrantClient(
    host="localhost",
    port=6333,
    api_key=os.getenv("QDRANT_API_KEY")
)

def import_jsonl_to_qdrant(input_file, collection_name, batch_size=500):
    """Import JSONL file to Qdrant collection"""

    points = []

    with open(input_file) as f:
        for line in tqdm(f, desc="Importing"):
            point_data = json.loads(line)

            point = PointStruct(
                id=point_data["id"],
                vector=point_data["vector"],
                payload=point_data.get("payload", {})
            )

            points.append(point)

            # Batch upsert
            if len(points) >= batch_size:
                client.upsert(
                    collection_name=collection_name,
                    points=points
                )
                points = []

    # Upload remaining points
    if points:
        client.upsert(
            collection_name=collection_name,
            points=points
        )

# Import
import_jsonl_to_qdrant('export.jsonl', 'code_vectors', batch_size=500)
```

#### Using CLI (Coming Soon)

```bash
# Using cortex migration command
cortex qdrant migrate \
  --source export.jsonl \
  --target code_vectors \
  --batch-size 500
```

### Step 4: Validate Migration

```python
from qdrant_client import QdrantClient

client = QdrantClient(host="localhost", port=6333)

def validate_migration(collection_name, expected_count):
    """Validate migration completeness and correctness"""

    # 1. Check count
    info = client.get_collection(collection_name)
    actual_count = info.points_count

    print(f"Expected: {expected_count}, Actual: {actual_count}")
    assert actual_count == expected_count, "Count mismatch!"

    # 2. Sample verification
    # Get a few random points and verify structure
    sample_ids = ["id1", "id2", "id3"]

    for point_id in sample_ids:
        point = client.retrieve(
            collection_name=collection_name,
            ids=[point_id],
            with_vectors=True
        )[0]

        print(f"Point {point_id}:")
        print(f"  Vector dim: {len(point.vector)}")
        print(f"  Payload keys: {list(point.payload.keys())}")

    # 3. Search validation
    # Perform known searches and compare results
    test_vector = [0.1] * 1536

    results = client.search(
        collection_name=collection_name,
        query_vector=test_vector,
        limit=10
    )

    print(f"Search returned {len(results)} results")

    # 4. Performance check
    import time

    start = time.time()
    for _ in range(100):
        client.search(
            collection_name=collection_name,
            query_vector=test_vector,
            limit=10
        )
    duration = time.time() - start

    avg_latency = duration / 100 * 1000  # ms
    print(f"Average search latency: {avg_latency:.2f}ms")

    print("✓ Validation passed!")

# Run validation
validate_migration('code_vectors', expected_count=1500000)
```

### Step 5: Performance Testing

```bash
# Run benchmark
cortex qdrant benchmark \
  --collection code_vectors \
  --num-queries 1000 \
  --dimensions 1536

# Expected output:
# Min latency:  23.45 ms
# Max latency:  187.32 ms
# Avg latency:  45.67 ms
# P50 latency:  42.11 ms
# P95 latency:  78.90 ms
# P99 latency:  125.44 ms
```

### Step 6: Cutover

For Blue-Green migration:

```python
# 1. Final sync (if using incremental migration)
sync_incremental_changes()

# 2. Enable read traffic to Qdrant
enable_qdrant_reads()

# 3. Monitor for 24-48 hours
monitor_metrics()

# 4. Enable write traffic to Qdrant
enable_qdrant_writes()

# 5. Disable old system
disable_old_system()

# 6. Archive old data
archive_old_vectors()
```

## Validation and Testing

### Data Integrity Checks

```bash
# 1. Count verification
cortex qdrant verify --collection code_vectors

# 2. Sample spot checks
# Compare random samples from old system vs Qdrant

# 3. Search result comparison
# Run same queries on both systems, compare results
```

### Performance Comparison

| Metric | Old System | Qdrant | Improvement |
|--------|-----------|--------|-------------|
| Search Latency (P95) | 250ms | 45ms | 5.5x faster |
| Indexing Throughput | 500/s | 10k/s | 20x faster |
| Memory Usage | 24GB | 12GB | 50% reduction |
| Storage Size | 50GB | 15GB | 70% reduction |

## Rollback Procedures

### If Migration Fails

```bash
# 1. Stop writes to Qdrant
docker-compose stop qdrant

# 2. Re-enable old system
enable_old_system()

# 3. Investigate issues
docker-compose logs qdrant

# 4. Fix and retry
# Address issues found in logs
# Re-run migration
```

### Partial Rollback

If using rolling migration:

```bash
# Revert traffic routing for specific workspaces
# Keep successfully migrated data in Qdrant
# Continue using old system for problematic data
```

## Common Issues

### Issue: Out of Memory During Import

**Symptoms:**
- Qdrant container crashes
- `oom-kill` events in logs

**Solutions:**
1. Reduce batch size:
   ```python
   batch_size = 100  # Instead of 500
   ```

2. Increase memory limit:
   ```yaml
   # docker-compose.yml
   memory: 16G  # Instead of 8G
   ```

3. Enable on-disk indexing:
   ```yaml
   # Temporarily disable HNSW during import
   hnsw_config:
     m: 0  # Disable indexing

   # Re-enable after import
   hnsw_config:
     m: 16
   ```

### Issue: Slow Import Speed

**Symptoms:**
- Import taking too long
- Low CPU utilization

**Solutions:**
1. Increase batch size:
   ```python
   batch_size = 1000  # Instead of 500
   ```

2. Parallel import:
   ```python
   from concurrent.futures import ThreadPoolExecutor

   with ThreadPoolExecutor(max_workers=4) as executor:
       futures = []
       for batch in batches:
           future = executor.submit(import_batch, batch)
           futures.append(future)
   ```

3. Disable indexing during import:
   ```python
   # Import with indexing disabled
   # Then trigger optimization
   client.update_collection(
       collection_name="code_vectors",
       optimizer_config=OptimizerConfig(indexing_threshold=20000)
   )
   ```

### Issue: Data Inconsistency

**Symptoms:**
- Search results differ from old system
- Count mismatch

**Solutions:**
1. Verify export:
   ```bash
   wc -l export.jsonl  # Should match vector count
   ```

2. Check for duplicates:
   ```python
   ids = set()
   duplicates = []

   with open('export.jsonl') as f:
       for line in f:
           point = json.loads(line)
           if point['id'] in ids:
               duplicates.append(point['id'])
           ids.add(point['id'])

   print(f"Found {len(duplicates)} duplicates")
   ```

3. Re-import problematic batches:
   ```python
   # Identify and re-import failed batches
   ```

### Issue: Performance Degradation

**Symptoms:**
- Search slower than expected
- High memory usage

**Solutions:**
1. Trigger optimization:
   ```bash
   cortex qdrant optimize code_vectors --wait
   ```

2. Check segment count:
   ```bash
   cortex qdrant status --detailed
   # If segments > 50, optimization needed
   ```

3. Adjust HNSW parameters:
   ```yaml
   hnsw_config:
     ef: 96  # Lower ef for faster search
     m: 12   # Lower m for less memory
   ```

## Next Steps

After successful migration:

1. **Monitor Performance**
   - Set up Grafana dashboards
   - Configure alerts
   - Review metrics daily for first week

2. **Optimize Configuration**
   - Fine-tune HNSW parameters based on metrics
   - Adjust resource limits
   - Enable advanced features (quantization, sharding)

3. **Backup and Recovery**
   - Set up automated backups
   - Test restore procedures
   - Document recovery playbook

4. **Team Training**
   - Train team on Qdrant operations
   - Document common tasks
   - Establish on-call procedures

5. **Decommission Old System**
   - Archive old data
   - Delete old infrastructure
   - Update documentation

---

**Need Help?**
- Check logs: `docker-compose logs qdrant`
- Review metrics: http://localhost:3000
- Contact support team
