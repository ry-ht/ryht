#!/usr/bin/env bash
#
# Control Script for RYHT Development Environment
# Manages Axon (multi-agent system), Cortex (cognitive system), and Dashboard
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIST_DIR="$SCRIPT_DIR/dist"
DASHBOARD_DIR="$SCRIPT_DIR/dashboard"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
print_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
print_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
print_error() { echo -e "${RED}[ERROR]${NC} $1"; }

command_exists() {
    command -v "$1" >/dev/null 2>&1
}

check_requirements() {
    print_info "Checking requirements..."
    local missing=()
    if ! command_exists cargo; then missing+=("cargo (Rust)"); fi
    if ! command_exists npm; then missing+=("npm (Node.js)"); fi
    if [ ${#missing[@]} -ne 0 ]; then
        print_error "Missing: ${missing[*]}"
        exit 1
    fi
    print_success "Requirements met"
}

build_all() {
    print_info "Building all projects..."
    cd "$SCRIPT_DIR/axon" && cargo build --release
    cd "$SCRIPT_DIR/cortex" && cargo build --release
    cd "$DASHBOARD_DIR" && npm run build
    mkdir -p "$DIST_DIR"/{axon,cortex,dashboard}
    cp "$SCRIPT_DIR/axon/target/release/axon" "$DIST_DIR/axon/"
    cp "$SCRIPT_DIR/cortex/target/release/cortex" "$DIST_DIR/cortex/"
    cp -r "$DASHBOARD_DIR/dist/"* "$DIST_DIR/dashboard/"
    print_success "Build completed"
}

start_axon() {
    print_info "Starting Axon..."
    cd "$DIST_DIR/axon"
    export PATH="/Users/taaliman/.cargo/bin:/usr/local/bin:/usr/bin:/bin:$PATH"
    ./axon &
    echo $! > "$DIST_DIR/axon.pid"
    print_success "Axon started (PID: $(cat "$DIST_DIR/axon.pid"))"
}

start_cortex() {
    print_info "Starting Cortex..."
    cd "$DIST_DIR/cortex"
    export PATH="/Users/taaliman/.cargo/bin:/usr/local/bin:/usr/bin:/bin:$PATH"
    ./cortex &
    echo $! > "$DIST_DIR/cortex.pid"
    print_success "Cortex started (PID: $(cat "$DIST_DIR/cortex.pid"))"
}

start_dashboard_dev() {
    print_info "Starting Dashboard (dev)..."
    cd "$DASHBOARD_DIR"
    npm run dev &
    echo $! > "$DIST_DIR/dashboard.pid"
    print_success "Dashboard started (PID: $(cat "$DIST_DIR/dashboard.pid"))"
}

stop_service() {
    local pid_file="$DIST_DIR/${1}.pid"
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid"
            print_success "$1 stopped"
        fi
        rm -f "$pid_file"
    fi
}

stop_all() {
    stop_service "axon"
    stop_service "cortex"
    stop_service "dashboard"
}

status_all() {
    for svc in axon cortex dashboard; do
        local pid_file="$DIST_DIR/${svc}.pid"
        if [ -f "$pid_file" ] && kill -0 "$(cat "$pid_file")" 2>/dev/null; then
            print_success "$svc running (PID: $(cat "$pid_file"))"
        else
            print_info "$svc not running"
        fi
    done
}

case "${1:-}" in
    build) check_requirements; build_all ;;
    start)
        case "${2:-all}" in
            axon) start_axon ;;
            cortex) start_cortex ;;
            dashboard) start_dashboard_dev ;;
            *) start_axon; sleep 2; start_cortex; sleep 2; start_dashboard_dev ;;
        esac
        ;;
    stop) stop_all ;;
    restart) stop_all; sleep 2; $0 start ;;
    status) status_all ;;
    *)
        echo "Usage: $0 {build|start|stop|restart|status} [service]"
        echo "Services: axon, cortex, dashboard, all (default)"
        ;;
esac
