#!/usr/bin/env bash
#
# Cortex Qdrant Setup Script
#
# This script initializes Qdrant collections with optimal configurations
# for production use with 1M+ vectors.
#
# Usage:
#   ./scripts/setup-qdrant.sh [options]
#
# Options:
#   --host HOST      Qdrant host (default: localhost)
#   --port PORT      Qdrant HTTP port (default: 6333)
#   --api-key KEY    Qdrant API key (optional)
#   --skip-verify    Skip collection verification
#   --force          Force recreate existing collections
#   --help           Show this help message
#

set -euo pipefail

# Color output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m' # No Color

# Default configuration
QDRANT_HOST="${QDRANT_HOST:-localhost}"
QDRANT_PORT="${QDRANT_PORT:-6333}"
QDRANT_API_KEY="${QDRANT_API_KEY:-}"
SKIP_VERIFY=false
FORCE_RECREATE=false

# Collection configurations
declare -A COLLECTIONS=(
    ["code_vectors"]="1536"        # OpenAI text-embedding-3-small
    ["memory_vectors"]="1536"      # Episodic/semantic memory
    ["document_vectors"]="1536"    # Document embeddings
    ["ast_vectors"]="768"          # AST structure embeddings (smaller model)
    ["dependency_vectors"]="384"   # Dependency graph embeddings (smaller model)
)

# HNSW parameters optimized for production
readonly HNSW_M=16                        # Number of edges per node
readonly HNSW_EF_CONSTRUCT=100            # Neighbors during index construction
readonly HNSW_FULL_SCAN_THRESHOLD=10000   # Switch to HNSW after this many vectors

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $*"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*" >&2
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --host)
                QDRANT_HOST="$2"
                shift 2
                ;;
            --port)
                QDRANT_PORT="$2"
                shift 2
                ;;
            --api-key)
                QDRANT_API_KEY="$2"
                shift 2
                ;;
            --skip-verify)
                SKIP_VERIFY=true
                shift
                ;;
            --force)
                FORCE_RECREATE=true
                shift
                ;;
            --help)
                grep '^#' "$0" | tail -n +3 | head -n -1 | cut -c 3-
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
}

# Build curl command with optional API key
build_curl_cmd() {
    local url="$1"
    local method="${2:-GET}"
    local data="${3:-}"

    local cmd=(curl -s -w "\n%{http_code}")

    if [[ -n "$QDRANT_API_KEY" ]]; then
        cmd+=(-H "api-key: $QDRANT_API_KEY")
    fi

    cmd+=(-H "Content-Type: application/json")
    cmd+=(-X "$method")

    if [[ -n "$data" ]]; then
        cmd+=(--data "$data")
    fi

    cmd+=("$url")

    echo "${cmd[@]}"
}

# Check Qdrant health
check_health() {
    log_info "Checking Qdrant health at ${QDRANT_HOST}:${QDRANT_PORT}..."

    local response
    response=$(eval "$(build_curl_cmd "http://${QDRANT_HOST}:${QDRANT_PORT}/healthz")")
    local http_code="${response##*$'\n'}"

    if [[ "$http_code" -eq 200 ]]; then
        log_success "Qdrant is healthy"
        return 0
    else
        log_error "Qdrant health check failed (HTTP $http_code)"
        return 1
    fi
}

# Get collection info
get_collection() {
    local collection_name="$1"
    local response
    response=$(eval "$(build_curl_cmd "http://${QDRANT_HOST}:${QDRANT_PORT}/collections/${collection_name}")")
    local http_code="${response##*$'\n'}"
    local body="${response%$'\n'*}"

    if [[ "$http_code" -eq 200 ]]; then
        echo "$body"
        return 0
    else
        return 1
    fi
}

# Delete collection
delete_collection() {
    local collection_name="$1"
    log_warning "Deleting collection: $collection_name"

    local response
    response=$(eval "$(build_curl_cmd "http://${QDRANT_HOST}:${QDRANT_PORT}/collections/${collection_name}" DELETE)")
    local http_code="${response##*$'\n'}"

    if [[ "$http_code" -eq 200 ]]; then
        log_success "Deleted collection: $collection_name"
        return 0
    else
        log_error "Failed to delete collection: $collection_name (HTTP $http_code)"
        return 1
    fi
}

# Create collection with optimal settings
create_collection() {
    local collection_name="$1"
    local vector_size="$2"

    log_info "Creating collection: $collection_name (vector_size=$vector_size)"

    # Build collection configuration
    local config
    config=$(cat <<EOF
{
  "vectors": {
    "size": ${vector_size},
    "distance": "Cosine",
    "on_disk": false
  },
  "hnsw_config": {
    "m": ${HNSW_M},
    "ef_construct": ${HNSW_EF_CONSTRUCT},
    "full_scan_threshold": ${HNSW_FULL_SCAN_THRESHOLD},
    "on_disk": false
  },
  "optimizers_config": {
    "deleted_threshold": 0.2,
    "vacuum_min_vector_number": 1000,
    "default_segment_number": 8,
    "max_segment_size": 200000,
    "memmap_threshold": 50000,
    "indexing_threshold": 20000,
    "flush_interval_sec": 5,
    "max_optimization_threads": 16
  },
  "wal_config": {
    "wal_capacity_mb": 32,
    "wal_segments_ahead": 2
  },
  "quantization_config": null,
  "replication_factor": 1,
  "write_consistency_factor": 1,
  "on_disk_payload": false,
  "shard_number": 1
}
EOF
    )

    local response
    response=$(eval "$(build_curl_cmd "http://${QDRANT_HOST}:${QDRANT_PORT}/collections/${collection_name}" PUT "$config")")
    local http_code="${response##*$'\n'}"
    local body="${response%$'\n'*}"

    if [[ "$http_code" -eq 200 ]]; then
        log_success "Created collection: $collection_name"
        return 0
    else
        log_error "Failed to create collection: $collection_name (HTTP $http_code)"
        echo "$body" >&2
        return 1
    fi
}

# Create payload indexes for common fields
create_payload_indexes() {
    local collection_name="$1"

    log_info "Creating payload indexes for: $collection_name"

    # Common payload fields across collections
    local indexes=(
        "workspace_id:keyword"
        "project_id:keyword"
        "file_path:keyword"
        "language:keyword"
        "created_at:integer"
        "updated_at:integer"
    )

    # Collection-specific indexes
    case "$collection_name" in
        code_vectors)
            indexes+=("function_name:text" "class_name:text" "symbol_type:keyword")
            ;;
        memory_vectors)
            indexes+=("memory_type:keyword" "session_id:keyword" "importance:float")
            ;;
        document_vectors)
            indexes+=("doc_type:keyword" "section:text")
            ;;
    esac

    # Create each index
    for index_spec in "${indexes[@]}"; do
        IFS=':' read -r field_name field_type <<< "$index_spec"

        local index_config
        index_config=$(cat <<EOF
{
  "field_name": "${field_name}",
  "field_schema": "${field_type}"
}
EOF
        )

        local response
        response=$(eval "$(build_curl_cmd "http://${QDRANT_HOST}:${QDRANT_PORT}/collections/${collection_name}/index" PUT "$index_config")")
        local http_code="${response##*$'\n'}"

        if [[ "$http_code" -eq 200 ]]; then
            log_success "  Created index: ${field_name} (${field_type})"
        else
            log_warning "  Failed to create index: ${field_name} (may already exist)"
        fi
    done
}

# Verify collection configuration
verify_collection() {
    local collection_name="$1"
    local expected_size="$2"

    log_info "Verifying collection: $collection_name"

    local info
    if ! info=$(get_collection "$collection_name"); then
        log_error "Collection does not exist: $collection_name"
        return 1
    fi

    # Extract vector size using grep and basic text processing
    local actual_size
    actual_size=$(echo "$info" | grep -o '"size":[0-9]*' | head -1 | cut -d':' -f2)

    if [[ "$actual_size" -eq "$expected_size" ]]; then
        log_success "  Vector size: ${actual_size} ✓"
    else
        log_error "  Vector size mismatch: expected ${expected_size}, got ${actual_size}"
        return 1
    fi

    # Verify HNSW parameters
    if echo "$info" | grep -q "\"m\":${HNSW_M}"; then
        log_success "  HNSW m: ${HNSW_M} ✓"
    else
        log_warning "  HNSW m parameter mismatch"
    fi

    if echo "$info" | grep -q "\"ef_construct\":${HNSW_EF_CONSTRUCT}"; then
        log_success "  HNSW ef_construct: ${HNSW_EF_CONSTRUCT} ✓"
    else
        log_warning "  HNSW ef_construct parameter mismatch"
    fi

    return 0
}

# Main setup function
main() {
    parse_args "$@"

    echo ""
    log_info "========================================="
    log_info "  Cortex Qdrant Setup"
    log_info "========================================="
    log_info "Host: ${QDRANT_HOST}:${QDRANT_PORT}"
    log_info "Collections: ${#COLLECTIONS[@]}"
    echo ""

    # Check Qdrant health
    if ! check_health; then
        log_error "Cannot connect to Qdrant. Please ensure it's running."
        exit 1
    fi

    echo ""

    # Process each collection
    local failed_collections=0
    for collection_name in "${!COLLECTIONS[@]}"; do
        local vector_size="${COLLECTIONS[$collection_name]}"

        echo ""
        log_info "Processing: $collection_name"
        log_info "----------------------------------------"

        # Check if collection exists
        if get_collection "$collection_name" &>/dev/null; then
            if [[ "$FORCE_RECREATE" == true ]]; then
                delete_collection "$collection_name" || ((failed_collections++))
                create_collection "$collection_name" "$vector_size" || ((failed_collections++))
            else
                log_warning "Collection already exists: $collection_name (use --force to recreate)"
            fi
        else
            create_collection "$collection_name" "$vector_size" || ((failed_collections++))
        fi

        # Create payload indexes
        create_payload_indexes "$collection_name"

        # Verify configuration
        if [[ "$SKIP_VERIFY" == false ]]; then
            verify_collection "$collection_name" "$vector_size" || log_warning "Verification warnings"
        fi
    done

    echo ""
    log_info "========================================="
    if [[ "$failed_collections" -eq 0 ]]; then
        log_success "Setup completed successfully!"
        log_info "All collections are ready for production use."
    else
        log_error "Setup completed with $failed_collections failures"
        exit 1
    fi
    log_info "========================================="
    echo ""

    # Print next steps
    log_info "Next steps:"
    echo "  1. Verify collections: curl http://${QDRANT_HOST}:${QDRANT_PORT}/collections"
    echo "  2. Start ingesting vectors using cortex-cli"
    echo "  3. Monitor metrics: http://${QDRANT_HOST}:${QDRANT_PORT}/metrics"
    echo "  4. Access Grafana dashboards for monitoring"
    echo ""
}

# Run main function
main "$@"
