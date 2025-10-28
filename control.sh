#!/usr/bin/env bash
#
# Control Script for RYHT Development Environment
# Manages Axon (multi-agent system), Cortex (cognitive system), and Dashboard
#

set -euo pipefail

# Ensure PATH includes cargo/rustc and essential utilities
# Use dynamic detection for cargo path to ensure portability
if [ -d "${HOME}/.cargo/bin" ]; then
    CARGO_BIN="${HOME}/.cargo/bin"
else
    # Try to find cargo in system
    CARGO_BIN="$(dirname "$(which cargo 2>/dev/null || echo /usr/local/bin/cargo)")"
fi

# Export PATH with all essential system directories
export PATH="${CARGO_BIN}:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:${PATH}"

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

    # Build both Rust projects from workspace root
    print_info "Building Axon and Cortex (Rust workspace)..."
    cd "$SCRIPT_DIR"
    if ! cargo build --release -p axon 2>&1 | tee "$LOG_DIR/build-axon.log"; then
        print_error "Axon build failed. Check $LOG_DIR/build-axon.log"
        exit 1
    fi
    if [ ! -f "$SCRIPT_DIR/target/release/axon" ]; then
        print_error "Axon binary not found after build at $SCRIPT_DIR/target/release/axon"
        exit 1
    fi
    print_success "Axon built successfully"

    if ! cargo build --release -p cortex 2>&1 | tee "$LOG_DIR/build-cortex.log"; then
        print_error "Cortex build failed. Check $LOG_DIR/build-cortex.log"
        exit 1
    fi
    if [ ! -f "$SCRIPT_DIR/target/release/cortex" ]; then
        print_error "Cortex binary not found after build at $SCRIPT_DIR/target/release/cortex"
        exit 1
    fi
    print_success "Cortex built successfully"

    # Build Dashboard
    print_info "Building Dashboard (TypeScript)..."
    cd "$DASHBOARD_DIR"
    if ! npm run build 2>&1 | tee "$LOG_DIR/build-dashboard.log"; then
        print_error "Dashboard build failed. Check $LOG_DIR/build-dashboard.log"
        exit 1
    fi
    if [ ! -d "$DASHBOARD_DIR/dist" ]; then
        print_error "Dashboard dist directory not found after build"
        exit 1
    fi
    print_success "Dashboard built successfully"

    # Copy binaries to dist
    print_info "Copying binaries to $DIST_DIR..."
    mkdir -p "$DIST_DIR"
    cp "$SCRIPT_DIR/target/release/axon" "$DIST_DIR/"
    cp "$SCRIPT_DIR/target/release/cortex" "$DIST_DIR/"
    rm -rf "$DIST_DIR/dashboard"
    cp -r "$DASHBOARD_DIR/dist" "$DIST_DIR/dashboard"

    print_success "✓ Build completed successfully"
    print_info "Binaries location: $DIST_DIR"
    print_info "  - Axon:      $DIST_DIR/axon"
    print_info "  - Cortex:    $DIST_DIR/cortex"
    print_info "  - Dashboard: $DIST_DIR/dashboard/"
}

start_axon() {
    print_info "Starting Axon server..."
    if [ ! -f "$DIST_DIR/axon" ]; then
        print_error "Axon binary not found. Run './control.sh build' first"
        exit 1
    fi
    cd "$DIST_DIR"
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

    # Kill any remaining server processes
    pkill -f "axon internal-server-run" 2>/dev/null && print_info "Killed remaining axon processes"
    pkill -f "cortex internal-server-run" 2>/dev/null && print_info "Killed remaining cortex processes"

    print_success "All services stopped"
}

status_all() {
    print_info "Service status:"
    for svc in axon cortex; do
        local pid_file="$DIST_DIR/${svc}.pid"
        if [ -f "$pid_file" ] && kill -0 "$(cat "$pid_file")" 2>/dev/null; then
            print_success "  $svc running (PID: $(cat "$pid_file"))"
        else
            print_info "  $svc not running"
        fi
    done

    # Check if dashboard is built
    if [ -d "$DIST_DIR/dashboard" ]; then
        print_success "  dashboard built (served via Axon at http://localhost:3000)"
    else
        print_warning "  dashboard not built (run './control.sh build' first)"
    fi
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

rebuild_dashboard() {
    print_info "Rebuilding Dashboard..."

    # Build dashboard
    cd "$DASHBOARD_DIR"
    if ! npm run build 2>&1 | tee "$LOG_DIR/build-dashboard.log"; then
        print_error "Dashboard build failed. Check $LOG_DIR/build-dashboard.log"
        exit 1
    fi

    # Copy to dist
    rm -rf "$DIST_DIR/dashboard"
    cp -r "$DASHBOARD_DIR/dist" "$DIST_DIR/dashboard"
    print_success "Dashboard rebuilt successfully in $DIST_DIR/dashboard"

    print_info "Note: Axon will serve the updated files automatically."
    print_info "If changes don't appear, try hard-refreshing your browser (Ctrl+Shift+R or Cmd+Shift+R)"
}

clean() {
    print_info "Cleaning build artifacts and logs..."
    stop_all
    rm -rf "$DIST_DIR"
    # Clean Rust workspace target directory (shared by axon and cortex)
    cd "$SCRIPT_DIR" && cargo clean
    # Clean Dashboard
    cd "$DASHBOARD_DIR" && rm -rf dist node_modules/.vite
    print_success "Clean completed"
}

case "${1:-}" in
    build)
        check_requirements

        # Check if services are running
        services_were_running=false
        if [ -f "$DIST_DIR/axon.pid" ] && kill -0 "$(cat "$DIST_DIR/axon.pid")" 2>/dev/null; then
            services_were_running=true
            print_warning "Services are running. They will be restarted after build."
            stop_all
            sleep 2
        fi

        build_all

        # Restart if they were running
        if [ "$services_were_running" = true ]; then
            print_info "Restarting services..."
            $0 start
        fi
        ;;
    start)
        case "${2:-all}" in
            axon) start_axon ;;
            cortex) start_cortex ;;
            all)
                # Check if dashboard build exists
                if [ ! -d "$DIST_DIR/dashboard" ]; then
                    print_warning "Dashboard build not found, building..."
                    cd "$DASHBOARD_DIR" && npm run build
                    rm -rf "$DIST_DIR/dashboard"
                    cp -r "$DASHBOARD_DIR/dist" "$DIST_DIR/dashboard"
                    print_success "Dashboard built"
                fi

                start_axon
                sleep 2
                set +e
                start_cortex
                set -e

                print_info ""
                print_info "🚀 Services started:"
                print_info "   Dashboard:  http://localhost:3000"
                print_info "   Axon API:   http://localhost:3000/api/v1"
                print_info "   Cortex API: http://localhost:8080/api/v1"
                ;;
            *)
                print_error "Unknown service: ${2} (available: axon, cortex, all)"
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
    rebuild-dashboard)
        rebuild_dashboard
        ;;
    clean)
        clean
        ;;
    *)
        echo "RYHT Development Control Script"
        echo ""
        echo "Usage: $0 <command> [options]"
        echo ""
        echo "Commands:"
        echo "  build                - Build all components (auto-restarts if running)"
        echo "  rebuild-dashboard    - Rebuild only dashboard to dist/dashboard (no restart)"
        echo "  start [service]      - Start service(s)"
        echo "    all                - Start all services (default)"
        echo "    axon               - Start Axon only"
        echo "    cortex             - Start Cortex only"
        echo "  stop                 - Stop all services"
        echo "  restart [service]    - Restart service(s)"
        echo "  status               - Show status of all services"
        echo "  logs <service>       - Tail logs for a service (axon, cortex)"
        echo "  clean                - Clean all build artifacts and stop services"
        echo ""
        echo "Examples:"
        echo "  $0 build             - Build everything (restarts services if running)"
        echo "  $0 rebuild-dashboard - Quick dashboard rebuild (no service restart)"
        echo "  $0 start             - Start all services (Axon + Cortex + Dashboard)"
        echo "  $0 start axon        - Start only Axon"
        echo "  $0 restart           - Restart all services"
        echo "  $0 logs axon         - View Axon logs"
        echo "  $0 status            - Check service status"
        echo "  $0 stop              - Stop all services"
        echo ""
        echo "Dashboard updates:"
        echo "  After changing dashboard code, run './control.sh rebuild-dashboard'"
        echo "  This rebuilds only the dashboard to dist/dashboard (services keep running)"
        echo "  Refresh browser (Ctrl+Shift+R) to see changes"
        ;;
esac
