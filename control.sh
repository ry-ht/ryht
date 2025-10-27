#!/usr/bin/env bash
#
# Control Script for RYHT Development Environment
# Manages Axon (multi-agent system), Cortex (cognitive system), and Dashboard
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIST_DIR="$SCRIPT_DIR/dist"
DASHBOARD_DIR="$SCRIPT_DIR/dashboard"
LOG_DIR="$DIST_DIR/logs"

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

# Ensure log directory exists
mkdir -p "$LOG_DIR"

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

    print_info "Building Axon (Rust)..."
    cd "$SCRIPT_DIR/axon" && cargo build --release 2>&1 | tee "$LOG_DIR/build-axon.log"

    print_info "Building Cortex (Rust)..."
    cd "$SCRIPT_DIR/cortex" && cargo build --release 2>&1 | tee "$LOG_DIR/build-cortex.log"

    print_info "Building Dashboard (TypeScript)..."
    cd "$DASHBOARD_DIR" && npm run build 2>&1 | tee "$LOG_DIR/build-dashboard.log"

    mkdir -p "$DIST_DIR"
    cp "$SCRIPT_DIR/axon/target/release/axon" "$DIST_DIR/"
    cp "$SCRIPT_DIR/cortex/target/release/cortex" "$DIST_DIR/"
    rm -rf "$DIST_DIR/dashboard"
    cp -r "$DASHBOARD_DIR/dist" "$DIST_DIR/dashboard"

    print_success "Build completed successfully"
    print_info "Binaries location: $DIST_DIR"
}

start_axon() {
    print_info "Starting Axon server..."
    if [ ! -f "$DIST_DIR/axon" ]; then
        print_error "Axon binary not found. Run './control.sh build' first"
        exit 1
    fi
    cd "$DIST_DIR"
    export PATH="/Users/taaliman/.cargo/bin:/usr/local/bin:/usr/bin:/bin:$PATH"
    ./axon server start > "$LOG_DIR/axon.log" 2>&1 &

    # Wait for server to start and check health
    sleep 3
    for i in 1 2 3 4 5; do
        if curl -s http://127.0.0.1:3000/api/v1/health >/dev/null 2>&1; then
            # Extract PID from process list
            local pid=$(ps aux | grep "axon internal-server-run" | grep -v grep | awk '{print $2}' | head -1)
            if [ -n "$pid" ]; then
                echo $pid > "$DIST_DIR/axon.pid"
                print_success "Axon server started (PID: $pid)"
                print_info "Logs: $LOG_DIR/axon.log"
                print_info "API: http://localhost:3000"
                return 0
            fi
        fi
        sleep 1
    done

    print_error "Axon failed to start. Check $LOG_DIR/axon.log"
    exit 1
}

start_cortex() {
    print_info "Starting Cortex server..."
    if [ ! -f "$DIST_DIR/cortex" ]; then
        print_error "Cortex binary not found. Run './control.sh build' first"
        exit 1
    fi
    cd "$DIST_DIR"
    export PATH="/Users/taaliman/.cargo/bin:/usr/local/bin:/usr/bin:/bin:$PATH"
    ./cortex server start > "$LOG_DIR/cortex.log" 2>&1 &

    # Wait for server to start and check health (cortex needs more time)
    print_info "Waiting for Cortex to initialize..."
    sleep 5
    for i in 1 2 3 4 5 6 7 8 9 10; do
        if curl -s http://127.0.0.1:8080/api/v1/health >/dev/null 2>&1; then
            # Extract PID from process list
            local pid=$(ps aux | grep "cortex internal-server-run" | grep -v grep | awk '{print $2}' | head -1)
            if [ -n "$pid" ]; then
                echo $pid > "$DIST_DIR/cortex.pid"
                print_success "Cortex server started (PID: $pid)"
                print_info "Logs: $LOG_DIR/cortex.log"
                print_info "API: http://localhost:8080"
                return 0
            fi
        fi
        sleep 2
    done

    # Check if process exists even if health check failed
    local pid=$(ps aux | grep "cortex internal-server-run" | grep -v grep | awk '{print $2}' | head -1)
    if [ -n "$pid" ]; then
        echo $pid > "$DIST_DIR/cortex.pid"
        print_warning "Cortex server started but health check timed out (PID: $pid)"
        print_info "Server may still be initializing. Check logs: $LOG_DIR/cortex.log"
        print_info "API: http://localhost:8080"
        return 0
    fi

    print_error "Cortex failed to start. Check $LOG_DIR/cortex.log"
    return 1
}

start_dashboard_dev() {
    print_info "Starting Dashboard (dev mode)..."
    cd "$DASHBOARD_DIR"
    nohup npm run dev > "$LOG_DIR/dashboard.log" 2>&1 &
    echo $! > "$DIST_DIR/dashboard.pid"
    sleep 2
    if kill -0 $(cat "$DIST_DIR/dashboard.pid") 2>/dev/null; then
        print_success "Dashboard started (PID: $(cat "$DIST_DIR/dashboard.pid"))"
        print_info "Logs: $LOG_DIR/dashboard.log"
        print_info "URL: http://localhost:5173"
    else
        print_error "Dashboard failed to start. Check $LOG_DIR/dashboard.log"
        exit 1
    fi
}

start_dashboard_prod() {
    print_info "Starting Dashboard (production mode)..."
    if [ ! -d "$DIST_DIR/dashboard" ]; then
        print_error "Dashboard build not found. Run './control.sh build' first"
        exit 1
    fi
    cd "$DASHBOARD_DIR"
    nohup npm run start > "$LOG_DIR/dashboard-prod.log" 2>&1 &
    echo $! > "$DIST_DIR/dashboard.pid"
    sleep 2
    if kill -0 $(cat "$DIST_DIR/dashboard.pid") 2>/dev/null; then
        print_success "Dashboard started in production mode (PID: $(cat "$DIST_DIR/dashboard.pid"))"
        print_info "Logs: $LOG_DIR/dashboard-prod.log"
        print_info "URL: http://localhost:4173"
    else
        print_error "Dashboard failed to start. Check $LOG_DIR/dashboard-prod.log"
        exit 1
    fi
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
    print_info "Stopping all services..."

    # Stop by PID file
    stop_service "axon"
    stop_service "cortex"
    stop_service "dashboard"

    # Kill any remaining server processes
    pkill -f "axon internal-server-run" 2>/dev/null && print_info "Killed remaining axon processes"
    pkill -f "cortex internal-server-run" 2>/dev/null && print_info "Killed remaining cortex processes"

    print_success "All services stopped"
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

logs() {
    local service="${1:-}"
    if [ -z "$service" ]; then
        print_error "Specify service: axon, cortex, or dashboard"
        exit 1
    fi

    local log_file="$LOG_DIR/${service}.log"
    if [ ! -f "$log_file" ]; then
        print_error "Log file not found: $log_file"
        exit 1
    fi

    print_info "Showing logs for $service (Ctrl+C to exit)"
    tail -f "$log_file"
}

clean() {
    print_info "Cleaning build artifacts and logs..."
    stop_all
    rm -rf "$DIST_DIR"
    cd "$SCRIPT_DIR/axon" && cargo clean
    cd "$SCRIPT_DIR/cortex" && cargo clean
    cd "$DASHBOARD_DIR" && rm -rf dist node_modules/.vite
    print_success "Clean completed"
}

dev() {
    print_info "Starting development environment..."
    stop_all
    print_info "Starting all services in development mode..."

    # Start Axon (required)
    start_axon
    sleep 2

    # Start Cortex (optional - may fail if dependencies not ready)
    set +e  # Don't exit on error
    start_cortex
    cortex_status=$?
    set -e

    if [ $cortex_status -ne 0 ]; then
        print_warning "Cortex failed to start - continuing without it"
        print_info "You can start Cortex manually later with: ./control.sh start cortex"
    fi

    sleep 2

    # Start Dashboard (required)
    start_dashboard_dev

    print_success "Development environment started!"
    print_info "Axon API: http://localhost:3000"
    if [ $cortex_status -eq 0 ]; then
        print_info "Cortex API: http://localhost:8080"
    fi
    print_info "Dashboard: http://localhost:5173"
    print_info "View logs: ./control.sh logs [axon|cortex|dashboard]"
}

case "${1:-}" in
    build)
        check_requirements
        build_all
        ;;
    start)
        case "${2:-all}" in
            axon) start_axon ;;
            cortex) start_cortex ;;
            dashboard)
                if [ "${3:-dev}" = "prod" ]; then
                    start_dashboard_prod
                else
                    start_dashboard_dev
                fi
                ;;
            all)
                start_axon
                sleep 2
                start_cortex
                sleep 2
                if [ "${3:-dev}" = "prod" ]; then
                    start_dashboard_prod
                else
                    start_dashboard_dev
                fi
                ;;
            *)
                print_error "Unknown service: ${2}"
                exit 1
                ;;
        esac
        ;;
    stop)
        stop_all
        ;;
    restart)
        stop_all
        sleep 2
        $0 start "${2:-all}" "${3:-dev}"
        ;;
    status)
        status_all
        ;;
    logs)
        logs "${2:-}"
        ;;
    clean)
        clean
        ;;
    dev)
        dev
        ;;
    *)
        echo "RYHT Development Control Script"
        echo ""
        echo "Usage: $0 <command> [options]"
        echo ""
        echo "Commands:"
        echo "  build              - Build all components (axon, cortex, dashboard)"
        echo "  dev                - Start full development environment"
        echo "  start [service]    - Start service(s)"
        echo "    all              - Start all services (default)"
        echo "    axon             - Start Axon only"
        echo "    cortex           - Start Cortex only"
        echo "    dashboard [mode] - Start Dashboard (dev/prod, default: dev)"
        echo "  stop               - Stop all services"
        echo "  restart [service]  - Restart service(s)"
        echo "  status             - Show status of all services"
        echo "  logs <service>     - Tail logs for a service"
        echo "  clean              - Clean all build artifacts and stop services"
        echo ""
        echo "Examples:"
        echo "  $0 build           - Build everything"
        echo "  $0 dev             - Start development environment"
        echo "  $0 start axon      - Start only Axon"
        echo "  $0 logs dashboard  - View Dashboard logs"
        echo "  $0 stop            - Stop all services"
        ;;
esac
