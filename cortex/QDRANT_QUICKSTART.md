# Qdrant Quick Start Guide

Get Qdrant running in 5 minutes!

## Prerequisites

- Docker & Docker Compose installed
- 8GB+ RAM available
- 10GB+ disk space

## 1. Setup Environment

```bash
# Copy environment template
cp .env.example .env

# Edit at minimum these variables:
# QDRANT_API_KEY=your-secret-key-here  # For production
# GRAFANA_ADMIN_PASSWORD=change-me     # Change default
# OPENAI_API_KEY=sk-...                # For embeddings
nano .env
```

## 2. Create Data Directories

```bash
mkdir -p data/qdrant data/surrealdb logs backups
```

## 3. Start Services

```bash
# Start core services
docker-compose up -d

# Check status
docker-compose ps

# View logs
docker-compose logs -f qdrant
```

## 4. Initialize Collections

```bash
# Using setup script
./scripts/setup-qdrant.sh

# OR build and use cortex-cli
cargo build --release
./target/release/cortex qdrant init
```

## 5. Verify Setup

```bash
# Check health
curl http://localhost:6333/healthz

# List collections
./target/release/cortex qdrant list

# View status
./target/release/cortex qdrant status --detailed
```

## 6. Access Dashboards

- **Qdrant Dashboard**: http://localhost:6333/dashboard
- **Grafana**: http://localhost:3000 (admin/admin)
- **Prometheus**: http://localhost:9090

## Quick Commands

```bash
# Check status
cortex qdrant status

# Run benchmark
cortex qdrant benchmark --num-queries 100

# Create backup
cortex qdrant snapshot

# View metrics
curl http://localhost:6333/metrics

# Stop all services
docker-compose down

# Stop and remove data
docker-compose down -v
```

## Collection Details

Cortex creates 5 collections automatically:

| Collection | Dimensions | Purpose |
|------------|------------|---------|
| code_vectors | 1536 | Code embeddings |
| memory_vectors | 1536 | Agent memory |
| document_vectors | 1536 | Documentation |
| ast_vectors | 768 | AST structures |
| dependency_vectors | 384 | Dependencies |

## Resource Usage

Expected resource usage:

- **Memory**: 4-8GB (depends on data volume)
- **CPU**: 2-4 cores (varies with load)
- **Disk**: ~500MB per 1M vectors

## Troubleshooting

### Services won't start

```bash
# Check Docker
docker info

# Check ports
netstat -an | grep 6333

# View logs
docker-compose logs
```

### Can't connect to Qdrant

```bash
# Verify service is running
docker-compose ps qdrant

# Check health
curl http://localhost:6333/healthz

# Check API key
echo $QDRANT_API_KEY
```

### Out of memory

```bash
# Increase memory limit in docker-compose.yml
# Under qdrant service:
#   deploy:
#     resources:
#       limits:
#         memory: 16G
```

## Next Steps

1. **Read Full Documentation**: [docs/QDRANT_SETUP.md](docs/QDRANT_SETUP.md)
2. **Migration Guide**: [docs/QDRANT_MIGRATION_GUIDE.md](docs/QDRANT_MIGRATION_GUIDE.md)
3. **Configure Monitoring**: Set up Grafana alerts
4. **Optimize Performance**: Tune HNSW parameters
5. **Setup Backups**: Configure automated snapshots

## Common Use Cases

### Insert Vectors

```python
from qdrant_client import QdrantClient
from qdrant_client.models import PointStruct

client = QdrantClient(host="localhost", port=6333)

points = [
    PointStruct(
        id=1,
        vector=[0.1, 0.2, ...],  # 1536 dimensions
        payload={"file": "main.rs", "language": "rust"}
    )
]

client.upsert(collection_name="code_vectors", points=points)
```

### Search Vectors

```python
results = client.search(
    collection_name="code_vectors",
    query_vector=[0.1, 0.2, ...],
    limit=10
)

for result in results:
    print(f"Score: {result.score}, File: {result.payload['file']}")
```

### Filtered Search

```python
from qdrant_client.models import Filter, FieldCondition, MatchValue

results = client.search(
    collection_name="code_vectors",
    query_vector=[0.1, 0.2, ...],
    query_filter=Filter(
        must=[
            FieldCondition(
                key="language",
                match=MatchValue(value="rust")
            )
        ]
    ),
    limit=10
)
```

## Performance Tips

1. **Batch Operations**: Use batch upsert for > 100 vectors
2. **Disable Indexing**: Set `m=0` during bulk import
3. **Use Filters**: Filter before vector search when possible
4. **Monitor Metrics**: Check Grafana regularly
5. **Optimize Segments**: Keep < 50 segments per collection

## Support

- **Documentation**: `docs/QDRANT_SETUP.md`
- **GitHub Issues**: Create an issue for bugs
- **Logs**: `docker-compose logs qdrant`
- **Metrics**: http://localhost:3000

---

**Version**: 1.0.0
**Last Updated**: 2025-10-23
**Cortex**: 0.1.0
