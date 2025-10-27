#!/usr/bin/env bash
# Development environment control script for Cortex/Axon/Dashboard
set -e

# Set PATH to include cargo and standard utilities
export PATH="/Users/taaliman/.cargo/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Directories
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIST_DIR="$PROJECT_ROOT/dist"
DASHBOARD_DIR="$PROJECT_ROOT/dashboard"
PID_DIR="$PROJECT_ROOT/.pids"

# PID files
CORTEX_PID="$PID_DIR/cortex.pid"
AXON_PID="$PID_DIR/axon.pid"

# Configuration
CORTEX_PORT="${CORTEX_PORT:-9090}"
AXON_PORT="${AXON_PORT:-8080}"
DASHBOARD_PORT="${DASHBOARD_PORT:-5173}"

# ----------------------------------------------------------------------
# Helper functions
# ----------------------------------------------------------------------

print_header() {
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Create PID directory
mkdir -p "$PID_DIR"

# ----------------------------------------------------------------------
# Build functions
# ----------------------------------------------------------------------

build_rust() {
    print_header "Building Rust Binaries (Cortex & Axon)"

    local PROFILE="${1:-release}"
    local BUILD_CMD="cargo build"
    local TARGET_DIR="target/debug"

    if [ "$PROFILE" = "release" ]; then
        BUILD_CMD="cargo build --release"
        TARGET_DIR="target/release"
    fi

    print_info "Build profile: $PROFILE"

    # Create dist directory
    mkdir -p "$DIST_DIR"

    # Build Cortex
    print_info "Building Cortex..."
    cd "$PROJECT_ROOT/cortex/cortex"
    $BUILD_CMD
    if [ -f "$TARGET_DIR/cortex" ]; then
        cp "$TARGET_DIR/cortex" "$DIST_DIR/"
        print_success "Cortex built and copied"
    else
        print_error "Cortex binary not found"
    fi

    # Build Axon
    print_info "Building Axon..."
    cd "$PROJECT_ROOT/axon"
    $BUILD_CMD
    if [ -f "$TARGET_DIR/axon" ]; then
        cp "$TARGET_DIR/axon" "$DIST_DIR/"
        print_success "Axon built and copied"
    else
        print_error "Axon binary not found"
    fi

    cd "$PROJECT_ROOT"
}

build_dashboard() {
    print_header "Building Dashboard"

    if [ ! -d "$DASHBOARD_DIR" ]; then
        print_error "Dashboard directory not found"
        return 1
    fi

    cd "$DASHBOARD_DIR"

    if [ ! -d "node_modules" ]; then
        print_info "Installing dependencies..."
        npm install --legacy-peer-deps
    fi

    print_info "Building dashboard..."
    npm run build

    # Copy to dist
    rm -rf "$DIST_DIR/dashboard"
    cp -r "$DASHBOARD_DIR/dist" "$DIST_DIR/dashboard"

    local file_count=$(find "$DIST_DIR/dashboard" -type f | wc -l | tr -d ' ')
    print_success "Dashboard built ($file_count files)"

    cd "$PROJECT_ROOT"
}

build_all() {
    build_rust "release"
    build_dashboard
    print_success "All components built successfully"
}

# ----------------------------------------------------------------------
# Run functions
# ----------------------------------------------------------------------

start_cortex() {
    print_header "Starting Cortex"

    if [ -f "$CORTEX_PID" ] && kill -0 $(cat "$CORTEX_PID") 2>/dev/null; then
        print_warning "Cortex is already running (PID: $(cat "$CORTEX_PID"))"
        return 0
    fi

    if [ ! -f "$DIST_DIR/cortex" ]; then
        print_error "Cortex binary not found. Run: ./control.sh build"
        return 1
    fi

    print_info "Starting Cortex on port $CORTEX_PORT..."

    cd "$DIST_DIR"
    PORT="$CORTEX_PORT" ./cortex > "../logs/cortex.log" 2>&1 &
    echo $! > "$CORTEX_PID"
    cd "$PROJECT_ROOT"

    sleep 2

    if kill -0 $(cat "$CORTEX_PID") 2>/dev/null; then
        print_success "Cortex started (PID: $(cat "$CORTEX_PID"))"
        print_info "API: http://localhost:$CORTEX_PORT"
        print_info "Logs: logs/cortex.log"
    else
        print_error "Cortex failed to start"
        rm -f "$CORTEX_PID"
        return 1
    fi
}

start_axon() {
    print_header "Starting Axon"

    if [ -f "$AXON_PID" ] && kill -0 $(cat "$AXON_PID") 2>/dev/null; then
        print_warning "Axon is already running (PID: $(cat "$AXON_PID"))"
        return 0
    fi

    if [ ! -f "$DIST_DIR/axon" ]; then
        print_error "Axon binary not found. Run: ./control.sh build"
        return 1
    fi

    print_info "Starting Axon on port $AXON_PORT..."

    cd "$DIST_DIR"
    PORT="$AXON_PORT" ./axon > "../logs/axon.log" 2>&1 &
    echo $! > "$AXON_PID"
    cd "$PROJECT_ROOT"

    sleep 2

    if kill -0 $(cat "$AXON_PID") 2>/dev/null; then
        print_success "Axon started (PID: $(cat "$AXON_PID"))"
        print_info "API: http://localhost:$AXON_PORT"
        print_info "Logs: logs/axon.log"
    else
        print_error "Axon failed to start"
        rm -f "$AXON_PID"
        return 1
    fi
}

start_dashboard() {
    print_header "Starting Dashboard"

    if [ ! -d "$DASHBOARD_DIR" ]; then
        print_error "Dashboard directory not found"
        return 1
    fi

    cd "$DASHBOARD_DIR"

    print_info "Starting dashboard dev server on port $DASHBOARD_PORT..."
    print_info "Dashboard will run in foreground. Press Ctrl+C to stop."

    VITE_CORTEX_API_URL="http://localhost:$CORTEX_PORT" \
    VITE_AXON_API_URL="http://localhost:$AXON_PORT" \
    npm run dev
}

start_all() {
    mkdir -p logs
    start_cortex
    start_axon
    print_success "Backend services started"
    echo ""
    print_info "To start dashboard: ./control.sh dashboard"
}

# ----------------------------------------------------------------------
# Stop functions
# ----------------------------------------------------------------------

stop_cortex() {
    if [ -f "$CORTEX_PID" ]; then
        local pid=$(cat "$CORTEX_PID")
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid"
            print_success "Cortex stopped (PID: $pid)"
        fi
        rm -f "$CORTEX_PID"
    else
        print_info "Cortex is not running"
    fi
}

stop_axon() {
    if [ -f "$AXON_PID" ]; then
        local pid=$(cat "$AXON_PID")
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid"
            print_success "Axon stopped (PID: $pid)"
        fi
        rm -f "$AXON_PID"
    else
        print_info "Axon is not running"
    fi
}

stop_all() {
    print_header "Stopping All Services"
    stop_cortex
    stop_axon
}

# ----------------------------------------------------------------------
# Status functions
# ----------------------------------------------------------------------

status() {
    print_header "Services Status"

    echo ""
    echo "Cortex:"
    if [ -f "$CORTEX_PID" ] && kill -0 $(cat "$CORTEX_PID") 2>/dev/null; then
        print_success "Running (PID: $(cat "$CORTEX_PID"))"
        echo "  API: http://localhost:$CORTEX_PORT"
    else
        print_info "Not running"
    fi

    echo ""
    echo "Axon:"
    if [ -f "$AXON_PID" ] && kill -0 $(cat "$AXON_PID") 2>/dev/null; then
        print_success "Running (PID: $(cat "$AXON_PID"))"
        echo "  API: http://localhost:$AXON_PORT"
    else
        print_info "Not running"
    fi

    echo ""
    echo "Dashboard:"
    if lsof -Pi :$DASHBOARD_PORT -sTCP:LISTEN -t >/dev/null 2>&1; then
        print_success "Running on port $DASHBOARD_PORT"
        echo "  URL: http://localhost:$DASHBOARD_PORT"
    else
        print_info "Not running"
    fi
}

# ----------------------------------------------------------------------
# Logs functions
# ----------------------------------------------------------------------

logs() {
    local service="${1:-all}"

    case "$service" in
        cortex)
            tail -f logs/cortex.log
            ;;
        axon)
            tail -f logs/axon.log
            ;;
        all|*)
            print_info "Following logs (Ctrl+C to stop)"
            tail -f logs/cortex.log logs/axon.log
            ;;
    esac
}

# ----------------------------------------------------------------------
# Main command dispatcher
# ----------------------------------------------------------------------

show_help() {
    cat << EOF
Usage: ./control.sh <command> [options]

Commands:
  build [profile]       Build Rust binaries (default: release)
  build-dashboard       Build dashboard only
  build-all             Build everything

  start                 Start Cortex and Axon
  start-cortex          Start Cortex only
  start-axon            Start Axon only
  dashboard             Start dashboard dev server

  stop                  Stop all services
  stop-cortex           Stop Cortex only
  stop-axon             Stop Axon only

  restart               Restart all services
  status                Show services status
  logs [service]        Show logs (cortex|axon|all)

  dev                   Full dev mode: build & start all
  clean                 Clean build artifacts

Environment Variables:
  CORTEX_PORT           Cortex API port (default: 9090)
  AXON_PORT             Axon API port (default: 8080)
  DASHBOARD_PORT        Dashboard port (default: 5173)

Examples:
  ./control.sh build              # Build release binaries
  ./control.sh build debug        # Build debug binaries
  ./control.sh start              # Start backend services
  ./control.sh dashboard          # Start dashboard
  ./control.sh dev                # Build and start everything
  ./control.sh logs cortex        # Follow Cortex logs
  ./control.sh status             # Check status

EOF
}

# ----------------------------------------------------------------------
# Main
# ----------------------------------------------------------------------

case "${1:-}" in
    build)
        build_rust "${2:-release}"
        ;;
    build-dashboard)
        build_dashboard
        ;;
    build-all)
        build_all
        ;;
    start)
        start_all
        ;;
    start-cortex)
        start_cortex
        ;;
    start-axon)
        start_axon
        ;;
    dashboard)
        start_dashboard
        ;;
    stop)
        stop_all
        ;;
    stop-cortex)
        stop_cortex
        ;;
    stop-axon)
        stop_axon
        ;;
    restart)
        stop_all
        sleep 1
        start_all
        ;;
    status)
        status
        ;;
    logs)
        logs "${2:-all}"
        ;;
    dev)
        build_all
        start_all
        print_success "Dev environment ready!"
        echo ""
        print_info "Run './control.sh dashboard' to start dashboard"
        ;;
    clean)
        print_header "Cleaning Build Artifacts"
        rm -rf target dist dashboard/dist
        print_success "Cleaned"
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        print_error "Unknown command: $1"
        echo ""
        show_help
        exit 1
        ;;
esac
