#!/usr/bin/env /Users/taaliman/.bun/bin/bun
/**
 * Control Script for RYHT Development Environment
 * Manages Axon (multi-agent system), Cortex (cognitive system), and Dashboard
 */

import { existsSync, mkdirSync, rmSync, cpSync, readFileSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";
import { homedir } from "os";

// Setup paths
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const SCRIPT_DIR = __dirname;
const DIST_DIR = join(SCRIPT_DIR, "dist");
const DASHBOARD_DIR = join(SCRIPT_DIR, "dashboard");
const LOG_DIR = join(DIST_DIR, "logs");

// Config paths
const CONFIG_DIR = join(homedir(), ".ryht");
const CONFIG_FILE = join(CONFIG_DIR, "config.toml");

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

    // Ensure we have access to basic utilities and cargo
    process.env.PATH = `${systemPaths.join(":")}:${process.env.PATH}`;
};

// Initialize PATH
setupPath();

// Utility functions (defined early for use in config loading)
const print = {
    info: (msg: string) => console.log(`${BLUE}[INFO]${NC} ${msg}`),
    success: (msg: string) => console.log(`${GREEN}[SUCCESS]${NC} ${msg}`),
    warning: (msg: string) => console.log(`${YELLOW}[WARNING]${NC} ${msg}`),
    error: (msg: string) => console.log(`${RED}[ERROR]${NC} ${msg}`),
};

// Simple TOML parser for our config structure
const parseTOML = (content: string): Record<string, any> => {
    const lines = content.split('\n');
    const result: Record<string, any> = {};
    let currentSection: string[] = [];

    for (const line of lines) {
        const trimmed = line.trim();

        // Skip empty lines and comments
        if (!trimmed || trimmed.startsWith('#')) continue;

        // Section header [section] or [section.subsection]
        if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
            const section = trimmed.slice(1, -1);
            currentSection = section.split('.');

            // Create nested object structure
            let obj = result;
            for (let i = 0; i < currentSection.length; i++) {
                const key = currentSection[i];
                if (i === currentSection.length - 1) {
                    if (!obj[key]) obj[key] = {};
                } else {
                    if (!obj[key]) obj[key] = {};
                    obj = obj[key];
                }
            }
            continue;
        }

        // Key-value pair
        const eqIndex = trimmed.indexOf('=');
        if (eqIndex > 0) {
            const key = trimmed.slice(0, eqIndex).trim();
            let value = trimmed.slice(eqIndex + 1).trim();

            // Remove quotes from strings
            if ((value.startsWith('"') && value.endsWith('"')) ||
                (value.startsWith("'") && value.endsWith("'"))) {
                value = value.slice(1, -1);
            }

            // Parse boolean
            if (value === 'true') value = true as any;
            else if (value === 'false') value = false as any;
            // Parse number
            else if (!isNaN(Number(value)) && value !== '') {
                value = Number(value) as any;
            }

            // Set value in nested structure
            let obj = result;
            for (let i = 0; i < currentSection.length; i++) {
                const section = currentSection[i];
                if (!obj[section]) obj[section] = {};
                if (i === currentSection.length - 1) {
                    obj[section][key] = value;
                } else {
                    obj = obj[section];
                }
            }
        }
    }

    return result;
};

// Configuration type
interface Config {
    cortex: {
        server: {
            host: string;
            port: number;
        };
        mcp: {
            server_bind: string;
        };
    };
    axon: {
        server: {
            host: string;
            port: number;
        };
    };
}

// Load and parse configuration
const loadConfig = (): Config => {
    const defaults: Config = {
        cortex: {
            server: {
                host: "127.0.0.1",
                port: 8080  // Cortex API server default
            },
            mcp: {
                server_bind: "127.0.0.1:3000"
            }
        },
        axon: {
            server: {
                host: "127.0.0.1",
                port: 9090  // Axon API server default
            }
        }
    };

    try {
        if (!existsSync(CONFIG_FILE)) {
            print.warning(`Config file not found at ${CONFIG_FILE}, using defaults`);
            return defaults;
        }

        const content = readFileSync(CONFIG_FILE, 'utf-8');
        const parsed = parseTOML(content) as any;

        // Extract configuration with defaults
        const config: Config = {
            cortex: {
                server: {
                    host: parsed.cortex?.server?.host || defaults.cortex.server.host,
                    port: parsed.cortex?.server?.port || defaults.cortex.server.port
                },
                mcp: {
                    server_bind: parsed.cortex?.mcp?.server_bind || defaults.cortex.mcp.server_bind
                }
            },
            axon: {
                server: {
                    host: parsed.axon?.server?.host || defaults.axon.server.host,
                    port: parsed.axon?.server?.port || defaults.axon.server.port
                }
            }
        };

        return config;
    } catch (error) {
        print.warning(`Failed to load config: ${error}. Using defaults.`);
        return defaults;
    }
};

// Helper to build URLs from config
const getAxonUrls = (config: Config) => {
    const { host, port } = config.axon.server;
    return {
        health: `http://${host}:${port}/api/v1/health`,
        api: `http://${host}:${port}`
    };
};

const getCortexUrls = (config: Config) => {
    // Use cortex.server (API server), not cortex.mcp (MCP server)
    const { host, port } = config.cortex.server;
    return {
        health: `http://${host}:${port}/api/v1/health`,
        api: `http://${host}:${port}`
    };
};

// Load config at startup
const CONFIG = loadConfig();
const AXON_URLS = getAxonUrls(CONFIG);
const CORTEX_URLS = getCortexUrls(CONFIG);

// Log configuration on startup (useful for debugging)
if (process.env.DEBUG) {
    print.info(`Loaded configuration from ${CONFIG_FILE}`);
    print.info(`Axon API: ${AXON_URLS.api}`);
    print.info(`Cortex API: ${CORTEX_URLS.api}`);
}

// Helper function to execute shell commands
const exec = async (cmd: string[], options: any = {}) => {
    const proc = Bun.spawn(cmd, {
        stdout: "pipe",
        stderr: "pipe",
        env: process.env,
        ...options,
    });

    const output = await new Response(proc.stdout).text();
    await proc.exited;
    return { output, exitCode: proc.exitCode };
};

// Helper to execute and get text output
const execText = async (cmd: string[], options: any = {}) => {
    const result = await exec(cmd, options);
    if (result.exitCode !== 0) {
        throw new Error(`Command failed: ${cmd.join(" ")}`);
    }
    return result.output;
};

// Helper to execute silently
const execQuiet = async (cmd: string[], options: any = {}) => {
    const proc = Bun.spawn(cmd, {
        stdout: "ignore",
        stderr: "ignore",
        env: process.env,
        ...options,
    });
    await proc.exited;
    return proc.exitCode === 0;
};

// Ensure log directory exists
if (!existsSync(LOG_DIR)) {
    mkdirSync(LOG_DIR, { recursive: true });
}

const commandExists = async (cmd: string): Promise<boolean> => {
    try {
        // Check in common locations
        if (cmd === "cargo") {
            return existsSync(`${process.env.HOME}/.cargo/bin/cargo`);
        }

        // For other commands, try which
        const proc = Bun.spawn(["which", cmd], {
            stdout: "pipe",
            stderr: "pipe",
            env: process.env,
        });

        await proc.exited;
        return proc.exitCode === 0;
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
        const cargoPath = `${process.env.HOME}/.cargo/bin/cargo`;
        const proc = Bun.spawn([cargoPath, "build", "--release", "-p", "axon"], {
            cwd: SCRIPT_DIR,
            stdout: "pipe",
            stderr: "pipe",
            env: process.env,
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
        const cargoPath = `${process.env.HOME}/.cargo/bin/cargo`;
        const proc = Bun.spawn([cargoPath, "build", "--release", "-p", "cortex"], {
            cwd: SCRIPT_DIR,
            stdout: "pipe",
            stderr: "pipe",
            env: process.env,
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
            const response = await fetch(AXON_URLS.health);
            if (response.ok) {
                // Find the PID
                try {
                    const psResult = await execText(["sh", "-c", "ps aux | grep 'axon internal-server-run' | grep -v grep | head -1"]);
                    const pid = psResult.split(/\s+/)[1];

                    if (pid) {
                        await Bun.write(join(DIST_DIR, "axon.pid"), pid);
                        print.success(`Axon server started (PID: ${pid})`);
                        print.info(`Logs: ${logFile.name}`);
                        print.info(`API: ${AXON_URLS.api}`);
                        return;
                    }
                } catch {
                    // Failed to get PID
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
            const response = await fetch(CORTEX_URLS.health);
            if (response.ok) {
                // Find the PID
                try {
                    const psResult = await execText(["sh", "-c", "ps aux | grep 'cortex internal-server-run' | grep -v grep | head -1"]);
                    const pid = psResult.split(/\s+/)[1];

                    if (pid) {
                        await Bun.write(join(DIST_DIR, "cortex.pid"), pid);
                        print.success(`Cortex server started (PID: ${pid})`);
                        print.info(`Logs: ${logFile.name}`);
                        print.info(`API: ${CORTEX_URLS.api}`);
                        return;
                    }
                } catch {
                    // Failed to get PID
                }
            }
        } catch {
            // Server not ready yet
        }
        await Bun.sleep(2000);
    }

    // Check if process exists even if health check failed
    try {
        const psResult = await execText(["sh", "-c", "ps aux | grep 'cortex internal-server-run' | grep -v grep | head -1"]);
        const pid = psResult.split(/\s+/)[1];

        if (pid) {
            await Bun.write(join(DIST_DIR, "cortex.pid"), pid);
            print.warning(`Cortex server started but health check timed out (PID: ${pid})`);
            print.info(`Server may still be initializing. Check logs: ${logFile.name}`);
            print.info(`API: ${CORTEX_URLS.api}`);
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
            await execQuiet(["kill", pid.trim()]);
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
        if (await execQuiet(["pkill", "-f", "axon internal-server-run"])) {
            print.info("Killed remaining axon processes");
        }
    } catch {
        // No processes to kill
    }

    try {
        if (await execQuiet(["pkill", "-f", "cortex internal-server-run"])) {
            print.info("Killed remaining cortex processes");
        }
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
                if (await execQuiet(["kill", "-0", pid.trim()])) {
                    print.success(`  ${svc} running (PID: ${pid.trim()})`);
                } else {
                    print.info(`  ${svc} not running`);
                }
            } catch {
                print.info(`  ${svc} not running`);
            }
        } else {
            print.info(`  ${svc} not running`);
        }
    }

    // Check if dashboard is built
    if (existsSync(join(DIST_DIR, "dashboard"))) {
        print.success(`  dashboard built (served via Axon at ${AXON_URLS.api})`);
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
        const proc = Bun.spawn(["npm", "run", "build"], {
            cwd: DASHBOARD_DIR,
            stdout: "pipe",
            stderr: "pipe",
        });

        const output = await new Response(proc.stdout).text();
        const errors = await new Response(proc.stderr).text();
        await Bun.write(logFile, output + errors);

        await proc.exited;

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
    const cargoPath = `${process.env.HOME}/.cargo/bin/cargo`;
    await execQuiet([cargoPath, "clean"], { cwd: SCRIPT_DIR });

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
        await execQuiet(["npm", "run", "build"], { cwd: DASHBOARD_DIR });

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
    print.info(`   Dashboard:  ${AXON_URLS.api}`);
    print.info(`   Axon API:   ${AXON_URLS.api}/api/v1`);
    print.info(`   Cortex API: ${CORTEX_URLS.api}/api/v1`);
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
                    if (await execQuiet(["kill", "-0", pid.trim()])) {
                        servicesWereRunning = true;
                        print.warning("Services are running. They will be restarted after build.");
                        await stopAll();
                        await Bun.sleep(2000);
                    }
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