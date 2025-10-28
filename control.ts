#!/usr/bin/env /Users/taaliman/.bun/bin/bun
/**
 * Control Script for RYHT Development Environment
 * Manages Axon (multi-agent system), Cortex (cognitive system), and Dashboard
 */

import { existsSync, mkdirSync, rmSync, cpSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";
import { $ } from "bun";

// Setup paths
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const SCRIPT_DIR = __dirname;
const DIST_DIR = join(SCRIPT_DIR, "dist");
const DASHBOARD_DIR = join(SCRIPT_DIR, "dashboard");
const LOG_DIR = join(DIST_DIR, "logs");

// Colors for output
const RED = '\x1b[0;31m';
const GREEN = '\x1b[0;32m';
const YELLOW = '\x1b[1;33m';
const BLUE = '\x1b[0;34m';
const NC = '\x1b[0m'; // No Color

// Setup PATH with cargo and essential utilities
const setupPath = () => {
    const homeCargoBin = `${process.env.HOME}/.cargo/bin`;
    const systemPaths = [
        homeCargoBin,
        "/usr/local/bin",
        "/usr/bin",
        "/bin",
        "/usr/sbin",
        "/sbin"
    ];

    process.env.PATH = `${systemPaths.join(":")}:${process.env.PATH}`;
};

// Initialize PATH
setupPath();

// Utility functions
const print = {
    info: (msg: string) => console.log(`${BLUE}[INFO]${NC} ${msg}`),
    success: (msg: string) => console.log(`${GREEN}[SUCCESS]${NC} ${msg}`),
    warning: (msg: string) => console.log(`${YELLOW}[WARNING]${NC} ${msg}`),
    error: (msg: string) => console.log(`${RED}[ERROR]${NC} ${msg}`),
};

// Ensure log directory exists
if (!existsSync(LOG_DIR)) {
    mkdirSync(LOG_DIR, { recursive: true });
}

const commandExists = async (cmd: string): Promise<boolean> => {
    try {
        await $`which ${cmd}`.quiet();
        return true;
    } catch {
        return false;
    }
};

const checkRequirements = async () => {
    print.info("Checking requirements...");
    const missing: string[] = [];

    if (!await commandExists("cargo")) missing.push("cargo (Rust)");
    if (!await commandExists("npm")) missing.push("npm (Node.js)");

    if (missing.length > 0) {
        print.error(`Missing: ${missing.join(", ")}`);
        process.exit(1);
    }

    print.success("Requirements met");
};

const buildAll = async () => {
    print.info("Building all projects...");

    // Build Axon and Cortex
    print.info("Building Axon and Cortex (Rust workspace)...");
    process.chdir(SCRIPT_DIR);

    // Build Axon
    const axonBuildLog = Bun.file(join(LOG_DIR, "build-axon.log"));
    try {
        const proc = Bun.spawn(["cargo", "build", "--release", "-p", "axon"], {
            cwd: SCRIPT_DIR,
            stdout: "pipe",
            stderr: "pipe",
        });

        const output = await new Response(proc.stdout).text();
        const errors = await new Response(proc.stderr).text();
        await Bun.write(axonBuildLog, output + errors);

        await proc.exited;

        if (!existsSync(join(SCRIPT_DIR, "target/release/axon"))) {
            throw new Error("Axon binary not found after build");
        }
        print.success("Axon built successfully");
    } catch (e) {
        print.error(`Axon build failed. Check ${axonBuildLog.name}`);
        console.error(e);
        process.exit(1);
    }

    // Build Cortex
    const cortexBuildLog = Bun.file(join(LOG_DIR, "build-cortex.log"));
    try {
        const proc = Bun.spawn(["cargo", "build", "--release", "-p", "cortex"], {
            cwd: SCRIPT_DIR,
            stdout: "pipe",
            stderr: "pipe",
        });

        const output = await new Response(proc.stdout).text();
        const errors = await new Response(proc.stderr).text();
        await Bun.write(cortexBuildLog, output + errors);

        await proc.exited;

        if (!existsSync(join(SCRIPT_DIR, "target/release/cortex"))) {
            throw new Error("Cortex binary not found after build");
        }
        print.success("Cortex built successfully");
    } catch (e) {
        print.error(`Cortex build failed. Check ${cortexBuildLog.name}`);
        console.error(e);
        process.exit(1);
    }

    // Build Dashboard
    print.info("Building Dashboard (TypeScript)...");
    process.chdir(DASHBOARD_DIR);

    const dashboardBuildLog = Bun.file(join(LOG_DIR, "build-dashboard.log"));
    try {
        const proc = Bun.spawn(["npm", "run", "build"], {
            cwd: DASHBOARD_DIR,
            stdout: "pipe",
            stderr: "pipe",
        });

        const output = await new Response(proc.stdout).text();
        const errors = await new Response(proc.stderr).text();
        await Bun.write(dashboardBuildLog, output + errors);

        await proc.exited;

        if (!existsSync(join(DASHBOARD_DIR, "dist"))) {
            throw new Error("Dashboard dist directory not found after build");
        }
        print.success("Dashboard built successfully");
    } catch (e) {
        print.error(`Dashboard build failed. Check ${dashboardBuildLog.name}`);
        console.error(e);
        process.exit(1);
    }

    // Copy binaries to dist
    print.info(`Copying binaries to ${DIST_DIR}...`);
    if (!existsSync(DIST_DIR)) {
        mkdirSync(DIST_DIR, { recursive: true });
    }

    cpSync(join(SCRIPT_DIR, "target/release/axon"), join(DIST_DIR, "axon"));
    cpSync(join(SCRIPT_DIR, "target/release/cortex"), join(DIST_DIR, "cortex"));

    if (existsSync(join(DIST_DIR, "dashboard"))) {
        rmSync(join(DIST_DIR, "dashboard"), { recursive: true, force: true });
    }
    cpSync(join(DASHBOARD_DIR, "dist"), join(DIST_DIR, "dashboard"), { recursive: true });

    print.success("✓ Build completed successfully");
    print.info(`Binaries location: ${DIST_DIR}`);
    print.info(`  - Axon:      ${join(DIST_DIR, "axon")}`);
    print.info(`  - Cortex:    ${join(DIST_DIR, "cortex")}`);
    print.info(`  - Dashboard: ${join(DIST_DIR, "dashboard")}/`);
};

const startAxon = async () => {
    print.info("Starting Axon server...");

    if (!existsSync(join(DIST_DIR, "axon"))) {
        print.error("Axon binary not found. Run './control.ts build' first");
        process.exit(1);
    }

    process.chdir(DIST_DIR);
    const logFile = Bun.file(join(LOG_DIR, "axon.log"));

    // Start Axon in background
    const proc = Bun.spawn(["./axon", "server", "start"], {
        stdout: logFile,
        stderr: logFile,
        cwd: DIST_DIR,
    });

    // Wait for server to start
    await Bun.sleep(3000);

    for (let i = 0; i < 5; i++) {
        try {
            const response = await fetch("http://127.0.0.1:3000/api/v1/health");
            if (response.ok) {
                // Find the PID
                const psResult = await $`ps aux | grep "axon internal-server-run" | grep -v grep | head -1`.text();
                const pid = psResult.split(/\s+/)[1];

                if (pid) {
                    await Bun.write(join(DIST_DIR, "axon.pid"), pid);
                    print.success(`Axon server started (PID: ${pid})`);
                    print.info(`Logs: ${logFile.name}`);
                    print.info("API: http://localhost:3000");
                    return;
                }
            }
        } catch {
            // Server not ready yet
        }
        await Bun.sleep(1000);
    }

    print.error(`Axon failed to start. Check ${logFile.name}`);
    process.exit(1);
};

const startCortex = async () => {
    print.info("Starting Cortex server...");

    if (!existsSync(join(DIST_DIR, "cortex"))) {
        print.error("Cortex binary not found. Run './control.ts build' first");
        process.exit(1);
    }

    process.chdir(DIST_DIR);
    const logFile = Bun.file(join(LOG_DIR, "cortex.log"));

    // Start Cortex in background
    const proc = Bun.spawn(["./cortex", "server", "start"], {
        stdout: logFile,
        stderr: logFile,
        cwd: DIST_DIR,
    });

    print.info("Waiting for Cortex to initialize...");
    await Bun.sleep(5000);

    for (let i = 0; i < 10; i++) {
        try {
            const response = await fetch("http://127.0.0.1:8080/api/v1/health");
            if (response.ok) {
                // Find the PID
                const psResult = await $`ps aux | grep "cortex internal-server-run" | grep -v grep | head -1`.text();
                const pid = psResult.split(/\s+/)[1];

                if (pid) {
                    await Bun.write(join(DIST_DIR, "cortex.pid"), pid);
                    print.success(`Cortex server started (PID: ${pid})`);
                    print.info(`Logs: ${logFile.name}`);
                    print.info("API: http://localhost:8080");
                    return;
                }
            }
        } catch {
            // Server not ready yet
        }
        await Bun.sleep(2000);
    }

    // Check if process exists even if health check failed
    try {
        const psResult = await $`ps aux | grep "cortex internal-server-run" | grep -v grep | head -1`.text();
        const pid = psResult.split(/\s+/)[1];

        if (pid) {
            await Bun.write(join(DIST_DIR, "cortex.pid"), pid);
            print.warning(`Cortex server started but health check timed out (PID: ${pid})`);
            print.info(`Server may still be initializing. Check logs: ${logFile.name}`);
            print.info("API: http://localhost:8080");
            return;
        }
    } catch {
        // No process found
    }

    print.error(`Cortex failed to start. Check ${logFile.name}`);
};

const stopService = async (service: string) => {
    const pidFile = join(DIST_DIR, `${service}.pid`);

    if (existsSync(pidFile)) {
        const pid = await Bun.file(pidFile).text();
        try {
            await $`kill ${pid}`.quiet();
            print.success(`${service} stopped`);
        } catch {
            // Process might already be dead
        }
        rmSync(pidFile);
    }
};

const stopAll = async () => {
    print.info("Stopping all services...");

    await stopService("axon");
    await stopService("cortex");

    // Kill any remaining server processes
    try {
        await $`pkill -f "axon internal-server-run"`.quiet();
        print.info("Killed remaining axon processes");
    } catch {
        // No processes to kill
    }

    try {
        await $`pkill -f "cortex internal-server-run"`.quiet();
        print.info("Killed remaining cortex processes");
    } catch {
        // No processes to kill
    }

    print.success("All services stopped");
};

const statusAll = async () => {
    print.info("Service status:");

    for (const svc of ["axon", "cortex"]) {
        const pidFile = join(DIST_DIR, `${svc}.pid`);
        if (existsSync(pidFile)) {
            const pid = await Bun.file(pidFile).text();
            try {
                await $`kill -0 ${pid}`.quiet();
                print.success(`  ${svc} running (PID: ${pid})`);
            } catch {
                print.info(`  ${svc} not running`);
            }
        } else {
            print.info(`  ${svc} not running`);
        }
    }

    // Check if dashboard is built
    if (existsSync(join(DIST_DIR, "dashboard"))) {
        print.success("  dashboard built (served via Axon at http://localhost:3000)");
    } else {
        print.warning("  dashboard not built (run './control.ts build' first)");
    }
};

const logs = async (service: string) => {
    if (!service) {
        print.error("Specify service: axon, cortex, or dashboard");
        process.exit(1);
    }

    const logFile = join(LOG_DIR, `${service}.log`);
    if (!existsSync(logFile)) {
        print.error(`Log file not found: ${logFile}`);
        process.exit(1);
    }

    print.info(`Showing logs for ${service} (Ctrl+C to exit)`);

    // Use tail -f for continuous log streaming
    const proc = Bun.spawn(["tail", "-f", logFile], {
        stdout: "inherit",
        stderr: "inherit",
    });

    await proc.exited;
};

const rebuildDashboard = async () => {
    print.info("Rebuilding Dashboard...");

    process.chdir(DASHBOARD_DIR);
    const logFile = Bun.file(join(LOG_DIR, "build-dashboard.log"));

    try {
        const build = await $`npm run build 2>&1`.text();
        await Bun.write(logFile, build);

        // Copy to dist
        if (existsSync(join(DIST_DIR, "dashboard"))) {
            rmSync(join(DIST_DIR, "dashboard"), { recursive: true, force: true });
        }
        cpSync(join(DASHBOARD_DIR, "dist"), join(DIST_DIR, "dashboard"), { recursive: true });

        print.success(`Dashboard rebuilt successfully in ${join(DIST_DIR, "dashboard")}`);
        print.info("Note: Axon will serve the updated files automatically.");
        print.info("If changes don't appear, try hard-refreshing your browser (Ctrl+Shift+R or Cmd+Shift+R)");
    } catch (e) {
        print.error(`Dashboard build failed. Check ${logFile.name}`);
        process.exit(1);
    }
};

const clean = async () => {
    print.info("Cleaning build artifacts and logs...");

    await stopAll();

    if (existsSync(DIST_DIR)) {
        rmSync(DIST_DIR, { recursive: true, force: true });
    }

    // Clean Rust workspace target directory
    process.chdir(SCRIPT_DIR);
    await $`cargo clean`.quiet();

    // Clean Dashboard
    process.chdir(DASHBOARD_DIR);
    if (existsSync(join(DASHBOARD_DIR, "dist"))) {
        rmSync(join(DASHBOARD_DIR, "dist"), { recursive: true, force: true });
    }
    if (existsSync(join(DASHBOARD_DIR, "node_modules/.vite"))) {
        rmSync(join(DASHBOARD_DIR, "node_modules/.vite"), { recursive: true, force: true });
    }

    print.success("Clean completed");
};

const startAll = async () => {
    // Check if dashboard build exists
    if (!existsSync(join(DIST_DIR, "dashboard"))) {
        print.warning("Dashboard build not found, building...");
        process.chdir(DASHBOARD_DIR);
        await $`npm run build`.quiet();

        if (existsSync(join(DIST_DIR, "dashboard"))) {
            rmSync(join(DIST_DIR, "dashboard"), { recursive: true, force: true });
        }
        cpSync(join(DASHBOARD_DIR, "dist"), join(DIST_DIR, "dashboard"), { recursive: true });
        print.success("Dashboard built");
    }

    await startAxon();
    await Bun.sleep(2000);

    try {
        await startCortex();
    } catch {
        // Continue even if Cortex fails
    }

    print.info("");
    print.info("🚀 Services started:");
    print.info("   Dashboard:  http://localhost:3000");
    print.info("   Axon API:   http://localhost:3000/api/v1");
    print.info("   Cortex API: http://localhost:8080/api/v1");
};

const showHelp = () => {
    console.log(`RYHT Development Control Script

Usage: ./control.ts <command> [options]

Commands:
  build                - Build all components (auto-restarts if running)
  rebuild-dashboard    - Rebuild only dashboard to dist/dashboard (no restart)
  start [service]      - Start service(s)
    all                - Start all services (default)
    axon               - Start Axon only
    cortex             - Start Cortex only
  stop                 - Stop all services
  restart [service]    - Restart service(s)
  status               - Show status of all services
  logs <service>       - Tail logs for a service (axon, cortex)
  clean                - Clean all build artifacts and stop services

Examples:
  ./control.ts build             - Build everything (restarts services if running)
  ./control.ts rebuild-dashboard - Quick dashboard rebuild (no service restart)
  ./control.ts start             - Start all services (Axon + Cortex + Dashboard)
  ./control.ts start axon        - Start only Axon
  ./control.ts restart           - Restart all services
  ./control.ts logs axon         - View Axon logs
  ./control.ts status            - Check service status
  ./control.ts stop              - Stop all services

Dashboard updates:
  After changing dashboard code, run './control.ts rebuild-dashboard'
  This rebuilds only the dashboard to dist/dashboard (services keep running)
  Refresh browser (Ctrl+Shift+R) to see changes`);
};

// Main command handler
const main = async () => {
    const [, , command, ...args] = process.argv;

    switch (command) {
        case "build": {
            await checkRequirements();

            // Check if services are running
            let servicesWereRunning = false;
            const axonPidFile = join(DIST_DIR, "axon.pid");

            if (existsSync(axonPidFile)) {
                const pid = await Bun.file(axonPidFile).text();
                try {
                    await $`kill -0 ${pid}`.quiet();
                    servicesWereRunning = true;
                    print.warning("Services are running. They will be restarted after build.");
                    await stopAll();
                    await Bun.sleep(2000);
                } catch {
                    // PID file exists but process is dead
                }
            }

            await buildAll();

            // Restart if they were running
            if (servicesWereRunning) {
                print.info("Restarting services...");
                await startAll();
            }
            break;
        }

        case "start": {
            const service = args[0] || "all";

            switch (service) {
                case "axon":
                    await startAxon();
                    break;
                case "cortex":
                    await startCortex();
                    break;
                case "all":
                    await startAll();
                    break;
                default:
                    print.error(`Unknown service: ${service} (available: axon, cortex, all)`);
                    process.exit(1);
            }
            break;
        }

        case "stop": {
            await stopAll();
            break;
        }

        case "restart": {
            const service = args[0] || "all";
            await stopAll();
            await Bun.sleep(2000);

            switch (service) {
                case "axon":
                    await startAxon();
                    break;
                case "cortex":
                    await startCortex();
                    break;
                case "all":
                    await startAll();
                    break;
                default:
                    print.error(`Unknown service: ${service} (available: axon, cortex, all)`);
                    process.exit(1);
            }
            break;
        }

        case "status": {
            await statusAll();
            break;
        }

        case "logs": {
            await logs(args[0]);
            break;
        }

        case "rebuild-dashboard": {
            await rebuildDashboard();
            break;
        }

        case "clean": {
            await clean();
            break;
        }

        default: {
            showHelp();
            break;
        }
    }
};

// Run main function
await main();