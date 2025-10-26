#!/bin/bash
# Common Cortex CLI Workflows
# This script demonstrates common usage patterns for the Cortex CLI

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

success() {
    echo -e "${GREEN}✓${NC} $1"
}

warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

error() {
    echo -e "${RED}✗${NC} $1"
}

# Check if cortex is installed
check_cortex() {
    if ! command -v cortex &> /dev/null; then
        error "Cortex CLI not found. Please install it first."
        exit 1
    fi
    success "Cortex CLI found"
}

# Setup: Install and start database
setup_database() {
    info "Setting up SurrealDB..."

    # Check if already installed
    if cortex db status &> /dev/null; then
        warning "SurrealDB already running"
    else
        info "Installing SurrealDB..."
        cortex db install

        info "Starting SurrealDB..."
        cortex db start
    fi

    success "Database ready"
}

# Workflow 1: Initialize a new project
workflow_init_project() {
    info "=== Workflow 1: Initialize New Project ==="

    local project_name=${1:-"my-project"}

    info "Creating workspace: $project_name"
    cortex init "$project_name" --workspace-type project

    success "Project '$project_name' initialized"
}

# Workflow 2: Ingest codebase
workflow_ingest_codebase() {
    info "=== Workflow 2: Ingest Codebase ==="

    local path=${1:-.}
    local workspace=${2:-""}

    if [ -z "$workspace" ]; then
        info "Ingesting from: $path (default workspace)"
        cortex ingest "$path"
    else
        info "Ingesting from: $path into workspace: $workspace"
        cortex ingest "$path" --workspace "$workspace"
    fi

    success "Ingestion complete"

    # Show statistics
    cortex stats
}

# Workflow 3: Multi-workspace setup
workflow_multi_workspace() {
    info "=== Workflow 3: Multi-Workspace Setup ==="

    local base_dir=$1

    if [ -z "$base_dir" ]; then
        error "Base directory required"
        return 1
    fi

    # Create workspaces for each subdirectory
    for dir in "$base_dir"*/; do
        if [ -d "$dir" ]; then
            local workspace_name=$(basename "$dir")
            info "Creating workspace: $workspace_name"
            cortex workspace create "$workspace_name" --type project 2>/dev/null || true

            info "Ingesting: $dir"
            cortex ingest "$dir" --workspace "$workspace_name"
        fi
    done

    success "Multi-workspace setup complete"

    # List all workspaces
    cortex workspace list
}

# Workflow 4: Search and export
workflow_search_export() {
    info "=== Workflow 4: Search and Export ==="

    local query=$1
    local output_file=${2:-"search-results.json"}

    if [ -z "$query" ]; then
        error "Search query required"
        return 1
    fi

    info "Searching for: $query"
    cortex search "$query" --format json > "$output_file"

    success "Results saved to: $output_file"

    # Show summary
    local count=$(jq length "$output_file" 2>/dev/null || echo "0")
    info "Found $count results"
}

# Workflow 5: Batch operations
workflow_batch_operations() {
    info "=== Workflow 5: Batch Operations ==="

    # Get all workspaces
    local workspaces=$(cortex workspace list --format json | jq -r '.[].name' 2>/dev/null || echo "")

    if [ -z "$workspaces" ]; then
        warning "No workspaces found"
        return 0
    fi

    info "Processing workspaces:"
    echo "$workspaces" | while read -r workspace; do
        info "  - $workspace"

        # Consolidate memory for each workspace
        cortex memory consolidate --workspace "$workspace" 2>/dev/null || true
    done

    success "Batch operations complete"
}

# Workflow 6: Statistics monitoring
workflow_monitor_stats() {
    info "=== Workflow 6: Statistics Monitoring ==="

    local output_file=${1:-"stats-$(date +%Y%m%d-%H%M%S).json"}

    info "Collecting statistics..."
    cortex stats --format json > "$output_file"

    success "Statistics saved to: $output_file"

    # Display summary
    if command -v jq &> /dev/null; then
        info "Summary:"
        jq '{
            workspaces: .workspaces,
            files: .files,
            total_size: .total_size_bytes,
            timestamp: now
        }' "$output_file"
    fi
}

# Workflow 7: Backup and restore
workflow_backup() {
    info "=== Workflow 7: Backup Configuration ==="

    local backup_dir=${1:-"./cortex-backup-$(date +%Y%m%d-%H%M%S)"}

    info "Creating backup directory: $backup_dir"
    mkdir -p "$backup_dir"

    # Backup configuration
    info "Backing up configuration..."
    cortex config list > "$backup_dir/config.txt"
    cortex config list --format json > "$backup_dir/config.json"

    # Backup workspace list
    info "Backing up workspace list..."
    cortex workspace list --format json > "$backup_dir/workspaces.json"

    # Backup statistics
    info "Backing up statistics..."
    cortex stats --format json > "$backup_dir/stats.json"

    success "Backup complete: $backup_dir"
}

# Workflow 8: Daily maintenance
workflow_daily_maintenance() {
    info "=== Workflow 8: Daily Maintenance ==="

    # Check database status
    info "Checking database status..."
    cortex db status

    # Show statistics
    info "Current statistics:"
    cortex stats

    # Consolidate memory
    info "Consolidating memory..."
    cortex memory consolidate

    success "Daily maintenance complete"
}

# Workflow 9: CI/CD ingestion
workflow_ci_ingestion() {
    info "=== Workflow 9: CI/CD Ingestion ==="

    local project_name=${1:-$CI_PROJECT_NAME}
    local project_path=${2:-.}

    if [ -z "$project_name" ]; then
        error "Project name required"
        return 1
    fi

    info "CI/CD ingestion for: $project_name"

    # Create or switch to workspace
    cortex workspace create "$project_name" --type project 2>/dev/null || true
    cortex workspace switch "$project_name"

    # Ingest project
    cortex ingest "$project_path" --recursive true

    # Show results
    cortex list documents --workspace "$project_name" --format json

    success "CI/CD ingestion complete"
}

# Workflow 10: Interactive exploration
workflow_interactive() {
    info "=== Workflow 10: Interactive Exploration ==="

    while true; do
        echo ""
        echo "Choose an option:"
        echo "1) List workspaces"
        echo "2) Search memory"
        echo "3) Show statistics"
        echo "4) List documents"
        echo "5) List episodes"
        echo "6) Exit"
        echo ""

        read -p "Enter choice [1-6]: " choice

        case $choice in
            1)
                cortex workspace list
                ;;
            2)
                read -p "Enter search query: " query
                cortex search "$query"
                ;;
            3)
                cortex stats
                ;;
            4)
                read -p "Enter workspace (or leave empty): " workspace
                if [ -z "$workspace" ]; then
                    cortex list documents
                else
                    cortex list documents --workspace "$workspace"
                fi
                ;;
            5)
                read -p "Enter limit [20]: " limit
                limit=${limit:-20}
                cortex list episodes --limit "$limit"
                ;;
            6)
                info "Goodbye!"
                exit 0
                ;;
            *)
                error "Invalid choice"
                ;;
        esac
    done
}

# Main menu
show_menu() {
    echo ""
    echo "=== Cortex CLI Common Workflows ==="
    echo ""
    echo "Available workflows:"
    echo "  1) setup           - Setup database and environment"
    echo "  2) init            - Initialize new project"
    echo "  3) ingest          - Ingest codebase"
    echo "  4) multi           - Multi-workspace setup"
    echo "  5) search          - Search and export"
    echo "  6) batch           - Batch operations"
    echo "  7) monitor         - Statistics monitoring"
    echo "  8) backup          - Backup configuration"
    echo "  9) maintenance     - Daily maintenance"
    echo " 10) ci              - CI/CD ingestion"
    echo " 11) interactive     - Interactive exploration"
    echo ""
    echo "Usage: $0 <workflow> [args...]"
    echo ""
    echo "Examples:"
    echo "  $0 setup"
    echo "  $0 init my-project"
    echo "  $0 ingest ./src my-workspace"
    echo "  $0 multi ./projects"
    echo "  $0 search 'authentication' results.json"
    echo "  $0 monitor stats.json"
    echo ""
}

# Main script
main() {
    check_cortex

    if [ $# -eq 0 ]; then
        show_menu
        exit 0
    fi

    local workflow=$1
    shift

    case $workflow in
        setup)
            setup_database
            ;;
        init)
            workflow_init_project "$@"
            ;;
        ingest)
            workflow_ingest_codebase "$@"
            ;;
        multi)
            workflow_multi_workspace "$@"
            ;;
        search)
            workflow_search_export "$@"
            ;;
        batch)
            workflow_batch_operations "$@"
            ;;
        monitor)
            workflow_monitor_stats "$@"
            ;;
        backup)
            workflow_backup "$@"
            ;;
        maintenance)
            workflow_daily_maintenance "$@"
            ;;
        ci)
            workflow_ci_ingestion "$@"
            ;;
        interactive)
            workflow_interactive
            ;;
        help|--help|-h)
            show_menu
            ;;
        *)
            error "Unknown workflow: $workflow"
            show_menu
            exit 1
            ;;
    esac
}

# Run main
main "$@"
