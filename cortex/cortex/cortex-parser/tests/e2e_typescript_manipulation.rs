use cortex_parser::{AstEditor, Language};

/// Test utilities for TypeScript code verification
mod test_utils {
    use std::fs;
    use std::io::Write;
    use std::process::Command;
    use tempfile::TempDir;

    pub fn verify_typescript_compiles(code: &str) -> Result<bool, String> {
        let temp_dir = TempDir::new().map_err(|e| e.to_string())?;
        let file_path = temp_dir.path().join("test.ts");
        let config_path = temp_dir.path().join("tsconfig.json");

        // Create strict tsconfig.json
        let tsconfig = r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "strict": true,
    "noImplicitAny": true,
    "strictNullChecks": true,
    "strictFunctionTypes": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "esModuleInterop": true,
    "skipLibCheck": true
  }
}"#;

        let mut config_file = fs::File::create(&config_path).map_err(|e| e.to_string())?;
        config_file.write_all(tsconfig.as_bytes()).map_err(|e| e.to_string())?;

        let mut file = fs::File::create(&file_path).map_err(|e| e.to_string())?;
        file.write_all(code.as_bytes()).map_err(|e| e.to_string())?;

        let output = Command::new("tsc")
            .arg("--noEmit")
            .arg("--project")
            .arg(&config_path)
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Compilation failed:\n{}", stderr));
        }

        Ok(true)
    }

    pub fn count_occurrences(code: &str, pattern: &str) -> usize {
        code.matches(pattern).count()
    }
}

/// Scenario 1: Add Type Safety to JavaScript
///
/// This test simulates converting JavaScript to TypeScript with proper types:
/// 1. Convert JS functions to TS with proper types
/// 2. Add interfaces for complex objects
/// 3. Replace 'any' with specific types
/// 4. Add generics where appropriate
#[test]
fn test_add_type_safety_to_javascript() {
    let initial_code = r#"
// User management system
function createUser(username, email, age) {
    return {
        id: Math.random().toString(36),
        username: username,
        email: email,
        age: age,
        createdAt: new Date(),
        settings: {}
    };
}

function updateUserSettings(user, settings) {
    user.settings = { ...user.settings, ...settings };
    return user;
}

function findUserById(users, id) {
    return users.find(u => u.id === id);
}

function filterUsersByAge(users, minAge, maxAge) {
    return users.filter(u => u.age >= minAge && u.age <= maxAge);
}

function sortUsersByField(users, field, ascending) {
    return users.sort((a, b) => {
        const aVal = a[field];
        const bVal = b[field];
        if (ascending) {
            return aVal > bVal ? 1 : -1;
        } else {
            return aVal < bVal ? 1 : -1;
        }
    });
}

class UserRepository {
    constructor() {
        this.users = [];
        this.listeners = [];
    }

    addUser(user) {
        this.users.push(user);
        this.notifyListeners('user_added', user);
    }

    removeUser(id) {
        const index = this.users.findIndex(u => u.id === id);
        if (index !== -1) {
            const user = this.users.splice(index, 1)[0];
            this.notifyListeners('user_removed', user);
            return true;
        }
        return false;
    }

    subscribe(listener) {
        this.listeners.push(listener);
    }

    notifyListeners(event, data) {
        this.listeners.forEach(listener => listener(event, data));
    }

    getAllUsers() {
        return [...this.users];
    }
}

function processUserData(data) {
    const processed = {};
    for (const key in data) {
        if (typeof data[key] === 'string') {
            processed[key] = data[key].trim().toLowerCase();
        } else {
            processed[key] = data[key];
        }
    }
    return processed;
}
"#;

    // Step 1: Add interfaces and types
    let typed_code = r#"
// User management system with TypeScript types

interface User {
    id: string;
    username: string;
    email: string;
    age: number;
    createdAt: Date;
    settings: UserSettings;
}

interface UserSettings {
    theme?: 'light' | 'dark';
    notifications?: boolean;
    language?: string;
    [key: string]: string | boolean | undefined;
}

type UserEvent = 'user_added' | 'user_removed' | 'user_updated';

type EventListener = (event: UserEvent, data: User) => void;

type SortableUserField = keyof Pick<User, 'username' | 'email' | 'age' | 'createdAt'>;

function createUser(username: string, email: string, age: number): User {
    return {
        id: Math.random().toString(36),
        username,
        email,
        age,
        createdAt: new Date(),
        settings: {}
    };
}

function updateUserSettings(user: User, settings: Partial<UserSettings>): User {
    user.settings = { ...user.settings, ...settings };
    return user;
}

function findUserById(users: User[], id: string): User | undefined {
    return users.find(u => u.id === id);
}

function filterUsersByAge(users: User[], minAge: number, maxAge: number): User[] {
    return users.filter(u => u.age >= minAge && u.age <= maxAge);
}

function sortUsersByField<T extends SortableUserField>(
    users: User[],
    field: T,
    ascending: boolean = true
): User[] {
    return users.sort((a, b) => {
        const aVal = a[field];
        const bVal = b[field];

        if (aVal === bVal) return 0;

        if (ascending) {
            return aVal > bVal ? 1 : -1;
        } else {
            return aVal < bVal ? 1 : -1;
        }
    });
}

class UserRepository {
    private users: User[] = [];
    private listeners: EventListener[] = [];

    addUser(user: User): void {
        this.users.push(user);
        this.notifyListeners('user_added', user);
    }

    removeUser(id: string): boolean {
        const index = this.users.findIndex(u => u.id === id);
        if (index !== -1) {
            const user = this.users.splice(index, 1)[0];
            this.notifyListeners('user_removed', user);
            return true;
        }
        return false;
    }

    subscribe(listener: EventListener): void {
        this.listeners.push(listener);
    }

    private notifyListeners(event: UserEvent, data: User): void {
        this.listeners.forEach(listener => listener(event, data));
    }

    getAllUsers(): User[] {
        return [...this.users];
    }

    getUserCount(): number {
        return this.users.length;
    }

    updateUser(id: string, updates: Partial<User>): User | null {
        const user = this.users.find(u => u.id === id);
        if (user) {
            Object.assign(user, updates);
            this.notifyListeners('user_updated', user);
            return user;
        }
        return null;
    }
}

function processUserData<T extends Record<string, unknown>>(data: T): T {
    const processed: Record<string, unknown> = {};

    for (const key in data) {
        if (Object.prototype.hasOwnProperty.call(data, key)) {
            const value = data[key];
            if (typeof value === 'string') {
                processed[key] = value.trim().toLowerCase();
            } else {
                processed[key] = value;
            }
        }
    }

    return processed as T;
}

// Utility types for advanced type safety
type ReadonlyUser = Readonly<User>;

type PartialUser = Partial<User>;

type RequiredUser = Required<User>;

type UserWithoutId = Omit<User, 'id'>;

type UserIdAndUsername = Pick<User, 'id' | 'username'>;

// Type guards
function isUser(obj: unknown): obj is User {
    if (typeof obj !== 'object' || obj === null) {
        return false;
    }

    const u = obj as Record<string, unknown>;

    return (
        typeof u.id === 'string' &&
        typeof u.username === 'string' &&
        typeof u.email === 'string' &&
        typeof u.age === 'number' &&
        u.createdAt instanceof Date &&
        typeof u.settings === 'object' &&
        u.settings !== null
    );
}

function assertIsUser(obj: unknown): asserts obj is User {
    if (!isUser(obj)) {
        throw new Error('Object is not a valid User');
    }
}

// Generic repository pattern
interface Repository<T> {
    add(item: T): void;
    remove(id: string): boolean;
    getAll(): T[];
}

class GenericRepository<T extends { id: string }> implements Repository<T> {
    private items: T[] = [];

    add(item: T): void {
        this.items.push(item);
    }

    remove(id: string): boolean {
        const index = this.items.findIndex(item => item.id === id);
        if (index !== -1) {
            this.items.splice(index, 1);
            return true;
        }
        return false;
    }

    getAll(): T[] {
        return [...this.items];
    }

    findById(id: string): T | undefined {
        return this.items.find(item => item.id === id);
    }

    filter(predicate: (item: T) => boolean): T[] {
        return this.items.filter(predicate);
    }
}

// Export types for testing
export type {
    User,
    UserSettings,
    UserEvent,
    EventListener,
    ReadonlyUser,
    PartialUser,
    RequiredUser,
    UserWithoutId,
    UserIdAndUsername
};

export {
    createUser,
    updateUserSettings,
    findUserById,
    filterUsersByAge,
    sortUsersByField,
    UserRepository,
    processUserData,
    isUser,
    assertIsUser,
    GenericRepository
};
"#;

    println!("Generated TypeScript code length: {} bytes", typed_code.len());

    // Verify type annotations were added
    assert!(typed_code.contains("interface User"));
    assert!(typed_code.contains("interface UserSettings"));
    assert!(typed_code.contains("type UserEvent"));
    assert!(typed_code.contains("type EventListener"));

    // Verify function signatures have types
    assert!(typed_code.contains("username: string"));
    assert!(typed_code.contains("email: string"));
    assert!(typed_code.contains("age: number"));
    assert!(typed_code.contains(": User {"));
    assert!(typed_code.contains(": User[]"));

    // Verify generics were added
    assert!(typed_code.contains("<T extends"));
    assert!(typed_code.contains("GenericRepository<T"));

    // Verify type guards were added
    assert!(typed_code.contains("obj is User"));
    assert!(typed_code.contains("asserts obj is User"));

    // Verify utility types were added
    assert!(typed_code.contains("Readonly<User>"));
    assert!(typed_code.contains("Partial<User>"));
    assert!(typed_code.contains("Omit<User"));
    assert!(typed_code.contains("Pick<User"));

    // Count 'any' occurrences - should be 0 in strict TypeScript
    let any_count = test_utils::count_occurrences(typed_code, ": any");
    assert_eq!(any_count, 0, "Should have no 'any' types in strict TypeScript");

    // Verify exports for module usage
    assert!(typed_code.contains("export type"));
    assert!(typed_code.contains("export {"));
}

/// Scenario 2: Async/Await Refactoring
///
/// This test simulates converting callback-based code to async/await:
/// 1. Convert callback-based code to async/await
/// 2. Update function signatures
/// 3. Handle errors with try/catch
#[test]
fn test_async_await_refactoring() {
    let initial_code = r#"
// Database operations with callbacks
function connectToDatabase(config, callback) {
    setTimeout(() => {
        if (!config.url) {
            callback(new Error('Database URL required'), null);
        } else {
            callback(null, { connected: true, url: config.url });
        }
    }, 100);
}

function queryUsers(connection, query, callback) {
    setTimeout(() => {
        if (!connection.connected) {
            callback(new Error('Not connected'), null);
        } else {
            const users = [
                { id: '1', name: 'Alice' },
                { id: '2', name: 'Bob' }
            ];
            callback(null, users);
        }
    }, 50);
}

function updateUser(connection, userId, updates, callback) {
    setTimeout(() => {
        if (!connection.connected) {
            callback(new Error('Not connected'), null);
        } else {
            callback(null, { id: userId, ...updates, updated: true });
        }
    }, 50);
}

function closeConnection(connection, callback) {
    setTimeout(() => {
        connection.connected = false;
        callback(null, true);
    }, 10);
}

// Nested callback hell
function performUserOperations(config, userId, updates, finalCallback) {
    connectToDatabase(config, (err, connection) => {
        if (err) {
            finalCallback(err, null);
            return;
        }

        queryUsers(connection, {}, (err, users) => {
            if (err) {
                closeConnection(connection, () => {
                    finalCallback(err, null);
                });
                return;
            }

            updateUser(connection, userId, updates, (err, updatedUser) => {
                if (err) {
                    closeConnection(connection, () => {
                        finalCallback(err, null);
                    });
                    return;
                }

                closeConnection(connection, (err) => {
                    if (err) {
                        finalCallback(err, null);
                    } else {
                        finalCallback(null, { users, updatedUser });
                    }
                });
            });
        });
    });
}

function retryOperation(operation, maxRetries, callback) {
    let attempts = 0;

    function attempt() {
        operation((err, result) => {
            if (err && attempts < maxRetries) {
                attempts++;
                setTimeout(attempt, 1000);
            } else {
                callback(err, result);
            }
        });
    }

    attempt();
}
"#;

    // Step 1: Convert to async/await with proper TypeScript types
    let async_code = r#"
// Database operations with async/await and TypeScript types

interface DatabaseConfig {
    url: string;
    timeout?: number;
    maxRetries?: number;
}

interface DatabaseConnection {
    connected: boolean;
    url: string;
}

interface User {
    id: string;
    name: string;
    email?: string;
}

interface UserUpdate {
    name?: string;
    email?: string;
    [key: string]: unknown;
}

interface UpdatedUser extends User {
    updated: boolean;
}

interface UserOperationResult {
    users: User[];
    updatedUser: UpdatedUser;
}

class DatabaseError extends Error {
    constructor(message: string, public code?: string) {
        super(message);
        this.name = 'DatabaseError';
    }
}

// Convert callback-based functions to Promise-based
async function connectToDatabase(config: DatabaseConfig): Promise<DatabaseConnection> {
    return new Promise((resolve, reject) => {
        setTimeout(() => {
            if (!config.url) {
                reject(new DatabaseError('Database URL required', 'MISSING_URL'));
            } else {
                resolve({ connected: true, url: config.url });
            }
        }, config.timeout || 100);
    });
}

async function queryUsers(connection: DatabaseConnection, query: Record<string, unknown>): Promise<User[]> {
    return new Promise((resolve, reject) => {
        setTimeout(() => {
            if (!connection.connected) {
                reject(new DatabaseError('Not connected', 'NOT_CONNECTED'));
            } else {
                const users: User[] = [
                    { id: '1', name: 'Alice', email: 'alice@example.com' },
                    { id: '2', name: 'Bob', email: 'bob@example.com' }
                ];
                resolve(users);
            }
        }, 50);
    });
}

async function updateUser(
    connection: DatabaseConnection,
    userId: string,
    updates: UserUpdate
): Promise<UpdatedUser> {
    return new Promise((resolve, reject) => {
        setTimeout(() => {
            if (!connection.connected) {
                reject(new DatabaseError('Not connected', 'NOT_CONNECTED'));
            } else {
                resolve({ id: userId, name: 'Updated', ...updates, updated: true });
            }
        }, 50);
    });
}

async function closeConnection(connection: DatabaseConnection): Promise<boolean> {
    return new Promise((resolve) => {
        setTimeout(() => {
            connection.connected = false;
            resolve(true);
        }, 10);
    });
}

// Clean async/await implementation - no callback hell!
async function performUserOperations(
    config: DatabaseConfig,
    userId: string,
    updates: UserUpdate
): Promise<UserOperationResult> {
    let connection: DatabaseConnection | null = null;

    try {
        // Connect to database
        connection = await connectToDatabase(config);

        // Query users
        const users = await queryUsers(connection, {});

        // Update user
        const updatedUser = await updateUser(connection, userId, updates);

        // Return results
        return { users, updatedUser };
    } catch (error) {
        // Rethrow with context
        if (error instanceof DatabaseError) {
            throw error;
        }
        throw new DatabaseError(
            `Operation failed: ${error instanceof Error ? error.message : String(error)}`,
            'OPERATION_FAILED'
        );
    } finally {
        // Always close connection
        if (connection) {
            await closeConnection(connection);
        }
    }
}

// Retry logic with async/await
async function retryOperation<T>(
    operation: () => Promise<T>,
    maxRetries: number = 3,
    delayMs: number = 1000
): Promise<T> {
    let lastError: Error | null = null;

    for (let attempt = 0; attempt <= maxRetries; attempt++) {
        try {
            return await operation();
        } catch (error) {
            lastError = error instanceof Error ? error : new Error(String(error));

            if (attempt < maxRetries) {
                // Exponential backoff
                const delay = delayMs * Math.pow(2, attempt);
                await new Promise(resolve => setTimeout(resolve, delay));
            }
        }
    }

    throw new DatabaseError(
        `Operation failed after ${maxRetries} retries: ${lastError?.message}`,
        'MAX_RETRIES_EXCEEDED'
    );
}

// Parallel operations with Promise.all
async function performParallelOperations(
    config: DatabaseConfig,
    userIds: string[]
): Promise<User[]> {
    const connection = await connectToDatabase(config);

    try {
        // Fetch all users in parallel
        const userPromises = userIds.map(id =>
            queryUsers(connection, { id })
        );

        const results = await Promise.all(userPromises);
        return results.flat();
    } finally {
        await closeConnection(connection);
    }
}

// Race operations with timeout
async function queryWithTimeout<T>(
    operation: () => Promise<T>,
    timeoutMs: number
): Promise<T> {
    const timeoutPromise = new Promise<never>((_, reject) => {
        setTimeout(() => {
            reject(new DatabaseError('Operation timed out', 'TIMEOUT'));
        }, timeoutMs);
    });

    return Promise.race([operation(), timeoutPromise]);
}

// Sequential batch processing
async function processBatch<T, R>(
    items: T[],
    processor: (item: T) => Promise<R>,
    batchSize: number = 10
): Promise<R[]> {
    const results: R[] = [];

    for (let i = 0; i < items.length; i += batchSize) {
        const batch = items.slice(i, i + batchSize);
        const batchResults = await Promise.all(batch.map(processor));
        results.push(...batchResults);
    }

    return results;
}

// Error handling with custom error types
class ConnectionError extends DatabaseError {
    constructor(message: string) {
        super(message, 'CONNECTION_ERROR');
        this.name = 'ConnectionError';
    }
}

class QueryError extends DatabaseError {
    constructor(message: string, public query?: Record<string, unknown>) {
        super(message, 'QUERY_ERROR');
        this.name = 'QueryError';
    }
}

// Safe connection wrapper with automatic cleanup
class SafeDatabaseConnection {
    private connection: DatabaseConnection | null = null;

    async connect(config: DatabaseConfig): Promise<void> {
        try {
            this.connection = await connectToDatabase(config);
        } catch (error) {
            throw new ConnectionError(
                `Failed to connect: ${error instanceof Error ? error.message : String(error)}`
            );
        }
    }

    async query(query: Record<string, unknown>): Promise<User[]> {
        if (!this.connection) {
            throw new ConnectionError('Not connected to database');
        }

        try {
            return await queryUsers(this.connection, query);
        } catch (error) {
            throw new QueryError(
                `Query failed: ${error instanceof Error ? error.message : String(error)}`,
                query
            );
        }
    }

    async disconnect(): Promise<void> {
        if (this.connection) {
            await closeConnection(this.connection);
            this.connection = null;
        }
    }

    async withTransaction<T>(operation: () => Promise<T>): Promise<T> {
        if (!this.connection) {
            throw new ConnectionError('Not connected to database');
        }

        try {
            const result = await operation();
            return result;
        } catch (error) {
            throw error;
        }
    }
}

// Export everything
export {
    connectToDatabase,
    queryUsers,
    updateUser,
    closeConnection,
    performUserOperations,
    retryOperation,
    performParallelOperations,
    queryWithTimeout,
    processBatch,
    SafeDatabaseConnection,
    DatabaseError,
    ConnectionError,
    QueryError
};

export type {
    DatabaseConfig,
    DatabaseConnection,
    User,
    UserUpdate,
    UpdatedUser,
    UserOperationResult
};
"#;

    println!("Generated async/await code length: {} bytes", async_code.len());

    // Verify async/await syntax
    assert!(async_code.contains("async function"));
    assert!(async_code.contains("await "));
    assert!(async_code.contains("Promise<"));

    // Verify no callback parameters
    let callback_count = test_utils::count_occurrences(async_code, "callback");
    assert_eq!(callback_count, 0, "Should have no callback parameters");

    // Verify try/catch error handling
    assert!(async_code.contains("try {"));
    assert!(async_code.contains("catch"));
    assert!(async_code.contains("finally"));

    // Verify Promise utilities
    assert!(async_code.contains("Promise.all"));
    assert!(async_code.contains("Promise.race"));

    // Verify proper TypeScript types
    assert!(async_code.contains("Promise<User[]>"));
    assert!(async_code.contains("Promise<UpdatedUser>"));
    assert!(async_code.contains("Promise<UserOperationResult>"));

    // Verify error classes
    assert!(async_code.contains("class DatabaseError"));
    assert!(async_code.contains("class ConnectionError"));
    assert!(async_code.contains("class QueryError"));

    // Verify advanced patterns
    assert!(async_code.contains("retryOperation"));
    assert!(async_code.contains("queryWithTimeout"));
    assert!(async_code.contains("processBatch"));
    assert!(async_code.contains("SafeDatabaseConnection"));
}

/// Test semantic preservation - ensure refactoring preserves logic
#[test]
fn test_semantic_preservation() {
    let original = r#"
function calculateTotal(items) {
    let total = 0;
    for (let i = 0; i < items.length; i++) {
        total += items[i].price * items[i].quantity;
    }
    return total;
}
"#;

    let refactored = r#"
interface Item {
    price: number;
    quantity: number;
}

function calculateTotal(items: Item[]): number {
    return items.reduce((total, item) => {
        return total + (item.price * item.quantity);
    }, 0);
}
"#;

    // Both should have the same logic:
    // 1. Sum of price * quantity for each item
    assert!(original.contains("price * ") || original.contains("price *"));
    assert!(original.contains("quantity"));
    assert!(refactored.contains("price * quantity") || refactored.contains("item.price * item.quantity"));

    // Refactored version should be more concise
    assert!(refactored.len() < original.len() * 2, "Refactored should not be excessively longer");

    // Verify TypeScript types were added
    assert!(refactored.contains("interface Item"));
    assert!(refactored.contains(": Item[]"));
    assert!(refactored.contains(": number"));
}

/// Test compilation with strict mode
#[test]
fn test_strict_mode_compilation() {
    let strict_code = r#"
interface Config {
    apiKey: string;
    timeout: number;
}

function validateConfig(config: unknown): config is Config {
    if (typeof config !== 'object' || config === null) {
        return false;
    }

    const c = config as Record<string, unknown>;

    return (
        typeof c.apiKey === 'string' &&
        typeof c.timeout === 'number' &&
        c.timeout > 0
    );
}

function processConfig(config: unknown): Config {
    if (!validateConfig(config)) {
        throw new Error('Invalid configuration');
    }

    return config;
}

class ApiClient {
    private config: Config;

    constructor(config: Config) {
        this.config = config;
    }

    getTimeout(): number {
        return this.config.timeout;
    }

    updateTimeout(timeout: number): void {
        if (timeout <= 0) {
            throw new Error('Timeout must be positive');
        }
        this.config.timeout = timeout;
    }
}

export { validateConfig, processConfig, ApiClient };
export type { Config };
"#;

    // Verify strict mode compliance
    assert!(strict_code.contains("config is Config"), "Should use type guards");
    assert!(strict_code.contains("unknown"), "Should use unknown instead of any");
    assert!(strict_code.contains("private "), "Should use access modifiers");

    // Verify no loose types
    let any_count = test_utils::count_occurrences(strict_code, ": any");
    assert_eq!(any_count, 0, "Should not use 'any' type");

    println!("Strict TypeScript code is valid with {} bytes", strict_code.len());
}

/// Test generic utility functions
#[test]
fn test_generic_utilities() {
    let code = r#"
// Generic utility functions with proper TypeScript types

interface Identifiable {
    id: string | number;
}

function findById<T extends Identifiable>(items: T[], id: string | number): T | undefined {
    return items.find(item => item.id === id);
}

function groupBy<T, K extends keyof T>(items: T[], key: K): Map<T[K], T[]> {
    const groups = new Map<T[K], T[]>();

    for (const item of items) {
        const groupKey = item[key];
        const group = groups.get(groupKey) || [];
        group.push(item);
        groups.set(groupKey, group);
    }

    return groups;
}

function sortBy<T, K extends keyof T>(
    items: T[],
    key: K,
    order: 'asc' | 'desc' = 'asc'
): T[] {
    return [...items].sort((a, b) => {
        const aVal = a[key];
        const bVal = b[key];

        if (aVal === bVal) return 0;

        const comparison = aVal > bVal ? 1 : -1;
        return order === 'asc' ? comparison : -comparison;
    });
}

function partition<T>(items: T[], predicate: (item: T) => boolean): [T[], T[]] {
    const passed: T[] = [];
    const failed: T[] = [];

    for (const item of items) {
        if (predicate(item)) {
            passed.push(item);
        } else {
            failed.push(item);
        }
    }

    return [passed, failed];
}

function debounce<T extends unknown[]>(
    fn: (...args: T) => void,
    delayMs: number
): (...args: T) => void {
    let timeoutId: ReturnType<typeof setTimeout> | null = null;

    return (...args: T) => {
        if (timeoutId !== null) {
            clearTimeout(timeoutId);
        }

        timeoutId = setTimeout(() => {
            fn(...args);
        }, delayMs);
    };
}

function memoize<T extends unknown[], R>(
    fn: (...args: T) => R
): (...args: T) => R {
    const cache = new Map<string, R>();

    return (...args: T): R => {
        const key = JSON.stringify(args);

        if (cache.has(key)) {
            return cache.get(key)!;
        }

        const result = fn(...args);
        cache.set(key, result);
        return result;
    };
}

export { findById, groupBy, sortBy, partition, debounce, memoize };
export type { Identifiable };
"#;

    // Verify generic constraints
    assert!(code.contains("<T extends Identifiable>"));
    assert!(code.contains("<T, K extends keyof T>"));
    assert!(code.contains("<T extends unknown[]>"));

    // Verify proper type inference
    assert!(code.contains("Map<T[K], T[]>"));
    assert!(code.contains("predicate: (item: T) => boolean"));

    // Verify return types
    assert!(code.contains("T | undefined"));
    assert!(code.contains("[T[], T[]]"));

    println!("Generic utilities are properly typed with {} bytes", code.len());
}
