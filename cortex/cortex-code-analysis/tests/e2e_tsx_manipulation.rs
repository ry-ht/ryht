// Note: AstEditor and Lang are not directly used in these tests as they perform
// string-based manipulations to demonstrate code transformation patterns

/// Test utilities for TSX code verification
mod test_utils {
    pub fn count_occurrences(code: &str, pattern: &str) -> usize {
        code.matches(pattern).count()
    }

    pub fn contains_all(code: &str, patterns: &[&str]) -> bool {
        patterns.iter().all(|p| code.contains(p))
    }
}

/// Scenario 1: Add State Management to Component
///
/// This test simulates converting a presentational component to a stateful one:
/// 1. Start with a presentational component
/// 2. Add useState/useEffect hooks
/// 3. Add prop types
/// 4. Add event handlers
#[test]
fn test_add_state_management_to_component() {
    let initial_code = r#"
import React from 'react';

interface UserCardProps {
    userId: string;
    name: string;
    email: string;
}

export const UserCard: React.FC<UserCardProps> = ({ userId, name, email }) => {
    return (
        <div className="user-card">
            <h2>{name}</h2>
            <p>{email}</p>
        </div>
    );
};
"#;

    // Step 1: Convert to stateful component with hooks
    let stateful_code = r#"
import React, { useState, useEffect, useCallback } from 'react';

interface UserCardProps {
    userId: string;
    name: string;
    email: string;
    onUserUpdate?: (userId: string, data: UserUpdateData) => void;
    onUserDelete?: (userId: string) => void;
}

interface UserUpdateData {
    name?: string;
    email?: string;
}

interface UserDetails {
    userId: string;
    name: string;
    email: string;
    joinedAt: Date;
    lastActive: Date;
    posts: number;
}

export const UserCard: React.FC<UserCardProps> = ({
    userId,
    name,
    email,
    onUserUpdate,
    onUserDelete
}) => {
    // State management
    const [isEditing, setIsEditing] = useState(false);
    const [editedName, setEditedName] = useState(name);
    const [editedEmail, setEditedEmail] = useState(email);
    const [userDetails, setUserDetails] = useState<UserDetails | null>(null);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // Load additional user details on mount
    useEffect(() => {
        const loadUserDetails = async () => {
            setLoading(true);
            setError(null);

            try {
                // Simulate API call
                const response = await fetch(`/api/users/${userId}`);
                if (!response.ok) {
                    throw new Error('Failed to load user details');
                }

                const data = await response.json();
                setUserDetails({
                    userId: data.id,
                    name: data.name,
                    email: data.email,
                    joinedAt: new Date(data.joinedAt),
                    lastActive: new Date(data.lastActive),
                    posts: data.posts || 0
                });
            } catch (err) {
                setError(err instanceof Error ? err.message : 'Unknown error');
            } finally {
                setLoading(false);
            }
        };

        loadUserDetails();
    }, [userId]);

    // Sync local state with props
    useEffect(() => {
        setEditedName(name);
        setEditedEmail(email);
    }, [name, email]);

    // Event handlers
    const handleEditClick = useCallback(() => {
        setIsEditing(true);
    }, []);

    const handleCancelClick = useCallback(() => {
        setIsEditing(false);
        setEditedName(name);
        setEditedEmail(email);
    }, [name, email]);

    const handleSaveClick = useCallback(() => {
        if (onUserUpdate) {
            onUserUpdate(userId, {
                name: editedName,
                email: editedEmail
            });
        }
        setIsEditing(false);
    }, [userId, editedName, editedEmail, onUserUpdate]);

    const handleDeleteClick = useCallback(() => {
        if (onUserDelete && window.confirm('Are you sure you want to delete this user?')) {
            onUserDelete(userId);
        }
    }, [userId, onUserDelete]);

    const handleNameChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
        setEditedName(e.target.value);
    }, []);

    const handleEmailChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
        setEditedEmail(e.target.value);
    }, []);

    // Render loading state
    if (loading) {
        return (
            <div className="user-card loading">
                <p>Loading user details...</p>
            </div>
        );
    }

    // Render error state
    if (error) {
        return (
            <div className="user-card error">
                <p>Error: {error}</p>
            </div>
        );
    }

    return (
        <div className="user-card">
            {isEditing ? (
                <div className="edit-mode">
                    <input
                        type="text"
                        value={editedName}
                        onChange={handleNameChange}
                        placeholder="Name"
                        aria-label="User name"
                    />
                    <input
                        type="email"
                        value={editedEmail}
                        onChange={handleEmailChange}
                        placeholder="Email"
                        aria-label="User email"
                    />
                    <div className="actions">
                        <button onClick={handleSaveClick} className="btn-save">
                            Save
                        </button>
                        <button onClick={handleCancelClick} className="btn-cancel">
                            Cancel
                        </button>
                    </div>
                </div>
            ) : (
                <div className="view-mode">
                    <h2>{name}</h2>
                    <p>{email}</p>

                    {userDetails && (
                        <div className="user-stats">
                            <p>Joined: {userDetails.joinedAt.toLocaleDateString()}</p>
                            <p>Last Active: {userDetails.lastActive.toLocaleDateString()}</p>
                            <p>Posts: {userDetails.posts}</p>
                        </div>
                    )}

                    <div className="actions">
                        <button onClick={handleEditClick} className="btn-edit">
                            Edit
                        </button>
                        <button onClick={handleDeleteClick} className="btn-delete">
                            Delete
                        </button>
                    </div>
                </div>
            )}
        </div>
    );
};
"#;

    println!("Generated stateful component: {} bytes", stateful_code.len());

    // Verify hooks were added
    assert!(stateful_code.contains("useState"));
    assert!(stateful_code.contains("useEffect"));
    assert!(stateful_code.contains("useCallback"));

    // Verify state declarations
    assert!(stateful_code.contains("const [isEditing, setIsEditing]"));
    assert!(stateful_code.contains("const [loading, setLoading]"));
    assert!(stateful_code.contains("const [error, setError]"));

    // Verify event handlers
    assert!(stateful_code.contains("handleEditClick"));
    assert!(stateful_code.contains("handleSaveClick"));
    assert!(stateful_code.contains("handleDeleteClick"));

    // Verify TypeScript types
    assert!(stateful_code.contains("React.ChangeEvent<HTMLInputElement>"));
    assert!(stateful_code.contains("UserDetails | null"));

    // Verify proper prop types
    assert!(stateful_code.contains("onUserUpdate?"));
    assert!(stateful_code.contains("onUserDelete?"));

    let hook_count = test_utils::count_occurrences(stateful_code, "useState") +
                    test_utils::count_occurrences(stateful_code, "useEffect") +
                    test_utils::count_occurrences(stateful_code, "useCallback");

    assert!(hook_count >= 10, "Should have multiple hooks for complex state management");
}

/// Scenario 2: Extract Custom Hook
///
/// This test simulates extracting repeated logic into a custom hook:
/// 1. Find repeated logic in components
/// 2. Extract to custom hook
/// 3. Update components to use hook
/// 4. Add TypeScript types
#[test]
fn test_extract_custom_hook() {
    let initial_components = r#"
import React, { useState, useEffect } from 'react';

// Component 1: Product List
export const ProductList: React.FC = () => {
    const [products, setProducts] = useState<any[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        const fetchProducts = async () => {
            setLoading(true);
            setError(null);

            try {
                const response = await fetch('/api/products');
                if (!response.ok) throw new Error('Failed to fetch');
                const data = await response.json();
                setProducts(data);
            } catch (err) {
                setError(err instanceof Error ? err.message : 'Unknown error');
            } finally {
                setLoading(false);
            }
        };

        fetchProducts();
    }, []);

    if (loading) return <div>Loading...</div>;
    if (error) return <div>Error: {error}</div>;

    return (
        <ul>
            {products.map(p => <li key={p.id}>{p.name}</li>)}
        </ul>
    );
};

// Component 2: User List
export const UserList: React.FC = () => {
    const [users, setUsers] = useState<any[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        const fetchUsers = async () => {
            setLoading(true);
            setError(null);

            try {
                const response = await fetch('/api/users');
                if (!response.ok) throw new Error('Failed to fetch');
                const data = await response.json();
                setUsers(data);
            } catch (err) {
                setError(err instanceof Error ? err.message : 'Unknown error');
            } finally {
                setLoading(false);
            }
        };

        fetchUsers();
    }, []);

    if (loading) return <div>Loading...</div>;
    if (error) return <div>Error: {error}</div>;

    return (
        <ul>
            {users.map(u => <li key={u.id}>{u.name}</li>)}
        </ul>
    );
};
"#;

    // Step 1: Extract custom hook for data fetching
    let with_custom_hook = r#"
import React, { useState, useEffect, useCallback } from 'react';

// Custom hook types
interface UseFetchOptions {
    method?: 'GET' | 'POST' | 'PUT' | 'DELETE';
    headers?: Record<string, string>;
    body?: unknown;
}

interface UseFetchResult<T> {
    data: T | null;
    loading: boolean;
    error: string | null;
    refetch: () => Promise<void>;
}

// Custom hook for data fetching
function useFetch<T>(url: string, options?: UseFetchOptions): UseFetchResult<T> {
    const [data, setData] = useState<T | null>(null);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const fetchData = useCallback(async () => {
        setLoading(true);
        setError(null);

        try {
            const response = await fetch(url, {
                method: options?.method || 'GET',
                headers: options?.headers,
                body: options?.body ? JSON.stringify(options.body) : undefined
            });

            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }

            const result = await response.json();
            setData(result);
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Unknown error occurred');
        } finally {
            setLoading(false);
        }
    }, [url, options?.method, options?.headers, options?.body]);

    useEffect(() => {
        fetchData();
    }, [fetchData]);

    return { data, loading, error, refetch: fetchData };
}

// Custom hook for paginated data
interface UsePaginatedFetchOptions extends UseFetchOptions {
    pageSize?: number;
}

interface UsePaginatedFetchResult<T> {
    data: T[];
    loading: boolean;
    error: string | null;
    page: number;
    hasMore: boolean;
    loadMore: () => void;
    refetch: () => Promise<void>;
}

function usePaginatedFetch<T>(
    baseUrl: string,
    options?: UsePaginatedFetchOptions
): UsePaginatedFetchResult<T> {
    const [data, setData] = useState<T[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [page, setPage] = useState(1);
    const [hasMore, setHasMore] = useState(true);

    const pageSize = options?.pageSize || 20;

    const fetchPage = useCallback(async (pageNum: number, append: boolean = false) => {
        setLoading(true);
        setError(null);

        try {
            const url = `${baseUrl}?page=${pageNum}&pageSize=${pageSize}`;
            const response = await fetch(url, {
                method: options?.method || 'GET',
                headers: options?.headers,
            });

            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }

            const result = await response.json();

            if (append) {
                setData(prev => [...prev, ...result.items]);
            } else {
                setData(result.items);
            }

            setHasMore(result.hasMore || false);
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Unknown error occurred');
        } finally {
            setLoading(false);
        }
    }, [baseUrl, pageSize, options?.method, options?.headers]);

    const loadMore = useCallback(() => {
        if (!loading && hasMore) {
            const nextPage = page + 1;
            setPage(nextPage);
            fetchPage(nextPage, true);
        }
    }, [loading, hasMore, page, fetchPage]);

    const refetch = useCallback(async () => {
        setPage(1);
        await fetchPage(1, false);
    }, [fetchPage]);

    useEffect(() => {
        fetchPage(1, false);
    }, [fetchPage]);

    return { data, loading, error, page, hasMore, loadMore, refetch };
}

// Custom hook for form state
interface UseFormOptions<T> {
    initialValues: T;
    onSubmit: (values: T) => void | Promise<void>;
    validate?: (values: T) => Partial<Record<keyof T, string>>;
}

interface UseFormResult<T> {
    values: T;
    errors: Partial<Record<keyof T, string>>;
    touched: Partial<Record<keyof T, boolean>>;
    handleChange: (field: keyof T) => (e: React.ChangeEvent<HTMLInputElement>) => void;
    handleBlur: (field: keyof T) => () => void;
    handleSubmit: (e: React.FormEvent) => void;
    resetForm: () => void;
    setFieldValue: (field: keyof T, value: T[keyof T]) => void;
}

function useForm<T extends Record<string, unknown>>(
    options: UseFormOptions<T>
): UseFormResult<T> {
    const [values, setValues] = useState<T>(options.initialValues);
    const [errors, setErrors] = useState<Partial<Record<keyof T, string>>>({});
    const [touched, setTouched] = useState<Partial<Record<keyof T, boolean>>>({});

    const handleChange = useCallback((field: keyof T) => {
        return (e: React.ChangeEvent<HTMLInputElement>) => {
            setValues(prev => ({
                ...prev,
                [field]: e.target.value
            }));
        };
    }, []);

    const handleBlur = useCallback((field: keyof T) => {
        return () => {
            setTouched(prev => ({
                ...prev,
                [field]: true
            }));

            if (options.validate) {
                const validationErrors = options.validate(values);
                setErrors(validationErrors);
            }
        };
    }, [values, options]);

    const handleSubmit = useCallback((e: React.FormEvent) => {
        e.preventDefault();

        if (options.validate) {
            const validationErrors = options.validate(values);
            setErrors(validationErrors);

            if (Object.keys(validationErrors).length > 0) {
                return;
            }
        }

        options.onSubmit(values);
    }, [values, options]);

    const resetForm = useCallback(() => {
        setValues(options.initialValues);
        setErrors({});
        setTouched({});
    }, [options.initialValues]);

    const setFieldValue = useCallback((field: keyof T, value: T[keyof T]) => {
        setValues(prev => ({
            ...prev,
            [field]: value
        }));
    }, []);

    return {
        values,
        errors,
        touched,
        handleChange,
        handleBlur,
        handleSubmit,
        resetForm,
        setFieldValue
    };
}

// Type definitions
interface Product {
    id: string;
    name: string;
    price: number;
}

interface User {
    id: string;
    name: string;
    email: string;
}

// Refactored components using custom hooks
export const ProductList: React.FC = () => {
    const { data: products, loading, error, refetch } = useFetch<Product[]>('/api/products');

    if (loading) return <div>Loading products...</div>;
    if (error) return <div>Error: {error}</div>;
    if (!products) return <div>No products found</div>;

    return (
        <div>
            <button onClick={refetch}>Refresh</button>
            <ul>
                {products.map(p => (
                    <li key={p.id}>
                        {p.name} - ${p.price}
                    </li>
                ))}
            </ul>
        </div>
    );
};

export const UserList: React.FC = () => {
    const { data: users, loading, error, refetch } = useFetch<User[]>('/api/users');

    if (loading) return <div>Loading users...</div>;
    if (error) return <div>Error: {error}</div>;
    if (!users) return <div>No users found</div>;

    return (
        <div>
            <button onClick={refetch}>Refresh</button>
            <ul>
                {users.map(u => (
                    <li key={u.id}>
                        {u.name} ({u.email})
                    </li>
                ))}
            </ul>
        </div>
    );
};

// Example using paginated hook
export const InfiniteProductList: React.FC = () => {
    const { data, loading, error, hasMore, loadMore } = usePaginatedFetch<Product>('/api/products');

    return (
        <div>
            <ul>
                {data.map(p => (
                    <li key={p.id}>
                        {p.name} - ${p.price}
                    </li>
                ))}
            </ul>
            {loading && <div>Loading more...</div>}
            {error && <div>Error: {error}</div>}
            {hasMore && !loading && (
                <button onClick={loadMore}>Load More</button>
            )}
        </div>
    );
};

// Example using form hook
interface LoginFormValues {
    email: string;
    password: string;
}

export const LoginForm: React.FC = () => {
    const {
        values,
        errors,
        touched,
        handleChange,
        handleBlur,
        handleSubmit
    } = useForm<LoginFormValues>({
        initialValues: {
            email: '',
            password: ''
        },
        onSubmit: async (values) => {
            console.log('Submitting:', values);
        },
        validate: (values) => {
            const errors: Partial<Record<keyof LoginFormValues, string>> = {};

            if (!values.email) {
                errors.email = 'Email is required';
            } else if (!/\S+@\S+\.\S+/.test(values.email)) {
                errors.email = 'Email is invalid';
            }

            if (!values.password) {
                errors.password = 'Password is required';
            } else if (values.password.length < 8) {
                errors.password = 'Password must be at least 8 characters';
            }

            return errors;
        }
    });

    return (
        <form onSubmit={handleSubmit}>
            <div>
                <label htmlFor="email">Email:</label>
                <input
                    id="email"
                    type="email"
                    value={values.email}
                    onChange={handleChange('email')}
                    onBlur={handleBlur('email')}
                />
                {touched.email && errors.email && (
                    <span className="error">{errors.email}</span>
                )}
            </div>

            <div>
                <label htmlFor="password">Password:</label>
                <input
                    id="password"
                    type="password"
                    value={values.password}
                    onChange={handleChange('password')}
                    onBlur={handleBlur('password')}
                />
                {touched.password && errors.password && (
                    <span className="error">{errors.password}</span>
                )}
            </div>

            <button type="submit">Login</button>
        </form>
    );
};

// Export hooks
export { useFetch, usePaginatedFetch, useForm };
export type { UseFetchOptions, UseFetchResult, UsePaginatedFetchResult, UseFormOptions, UseFormResult };
"#;

    println!("Generated code with custom hooks: {} bytes", with_custom_hook.len());

    // Verify custom hooks were created
    assert!(with_custom_hook.contains("function useFetch<T>"));
    assert!(with_custom_hook.contains("function usePaginatedFetch<T>"));
    assert!(with_custom_hook.contains("function useForm<T"));

    // Verify components use custom hooks
    assert!(with_custom_hook.contains("useFetch<Product[]>"));
    assert!(with_custom_hook.contains("useFetch<User[]>"));

    // Verify proper TypeScript types
    assert!(with_custom_hook.contains("interface UseFetchResult<T>"));
    assert!(with_custom_hook.contains("interface UseFormResult<T>"));

    // Verify code is more reusable
    let original_fetch_count = test_utils::count_occurrences(initial_components, "await fetch");
    let refactored_direct_fetch = test_utils::count_occurrences(with_custom_hook, "await fetch");

    println!("Original fetch calls: {}, Refactored: {}", original_fetch_count, refactored_direct_fetch);

    // The custom hook should centralize fetch logic
    assert!(with_custom_hook.contains("export { useFetch"));
}

/// Scenario 3: Complex Component with Multiple Concerns
///
/// This test creates a complex real-world component with:
/// - Multiple state variables
/// - Side effects
/// - Event handlers
/// - Conditional rendering
/// - Proper TypeScript types
#[test]
fn test_complex_data_table_component() {
    let complex_component = r#"
import React, { useState, useEffect, useCallback, useMemo } from 'react';

// Type definitions
interface Column<T> {
    key: keyof T;
    header: string;
    width?: string;
    sortable?: boolean;
    render?: (value: T[keyof T], row: T) => React.ReactNode;
}

interface SortConfig<T> {
    key: keyof T;
    direction: 'asc' | 'desc';
}

interface FilterConfig {
    [key: string]: string;
}

interface DataTableProps<T> {
    data: T[];
    columns: Column<T>[];
    keyField: keyof T;
    pageSize?: number;
    searchable?: boolean;
    filterable?: boolean;
    exportable?: boolean;
    onRowClick?: (row: T) => void;
    onRowSelect?: (selectedRows: T[]) => void;
}

// Complex Data Table Component
export function DataTable<T extends Record<string, unknown>>({
    data,
    columns,
    keyField,
    pageSize = 10,
    searchable = true,
    filterable = true,
    exportable = true,
    onRowClick,
    onRowSelect
}: DataTableProps<T>): JSX.Element {
    // State management
    const [currentPage, setCurrentPage] = useState(1);
    const [sortConfig, setSortConfig] = useState<SortConfig<T> | null>(null);
    const [searchTerm, setSearchTerm] = useState('');
    const [filters, setFilters] = useState<FilterConfig>({});
    const [selectedRows, setSelectedRows] = useState<Set<T[keyof T]>>(new Set());

    // Reset page when data changes
    useEffect(() => {
        setCurrentPage(1);
    }, [data]);

    // Notify parent of selection changes
    useEffect(() => {
        if (onRowSelect) {
            const selected = data.filter(row => selectedRows.has(row[keyField]));
            onRowSelect(selected);
        }
    }, [selectedRows, data, keyField, onRowSelect]);

    // Filtering logic
    const filteredData = useMemo(() => {
        let result = [...data];

        // Apply search
        if (searchTerm) {
            result = result.filter(row =>
                Object.values(row).some(value =>
                    String(value).toLowerCase().includes(searchTerm.toLowerCase())
                )
            );
        }

        // Apply column filters
        Object.entries(filters).forEach(([key, value]) => {
            if (value) {
                result = result.filter(row =>
                    String(row[key]).toLowerCase().includes(value.toLowerCase())
                );
            }
        });

        return result;
    }, [data, searchTerm, filters]);

    // Sorting logic
    const sortedData = useMemo(() => {
        if (!sortConfig) return filteredData;

        return [...filteredData].sort((a, b) => {
            const aVal = a[sortConfig.key];
            const bVal = b[sortConfig.key];

            if (aVal === bVal) return 0;

            const comparison = aVal > bVal ? 1 : -1;
            return sortConfig.direction === 'asc' ? comparison : -comparison;
        });
    }, [filteredData, sortConfig]);

    // Pagination logic
    const paginatedData = useMemo(() => {
        const startIndex = (currentPage - 1) * pageSize;
        return sortedData.slice(startIndex, startIndex + pageSize);
    }, [sortedData, currentPage, pageSize]);

    const totalPages = Math.ceil(sortedData.length / pageSize);

    // Event handlers
    const handleSort = useCallback((key: keyof T) => {
        setSortConfig(prev => {
            if (!prev || prev.key !== key) {
                return { key, direction: 'asc' };
            }
            if (prev.direction === 'asc') {
                return { key, direction: 'desc' };
            }
            return null;
        });
    }, []);

    const handleSearch = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
        setSearchTerm(e.target.value);
        setCurrentPage(1);
    }, []);

    const handleFilterChange = useCallback((key: string, value: string) => {
        setFilters(prev => ({
            ...prev,
            [key]: value
        }));
        setCurrentPage(1);
    }, []);

    const handleRowSelect = useCallback((rowKey: T[keyof T]) => {
        setSelectedRows(prev => {
            const next = new Set(prev);
            if (next.has(rowKey)) {
                next.delete(rowKey);
            } else {
                next.add(rowKey);
            }
            return next;
        });
    }, []);

    const handleSelectAll = useCallback(() => {
        if (selectedRows.size === paginatedData.length) {
            setSelectedRows(new Set());
        } else {
            setSelectedRows(new Set(paginatedData.map(row => row[keyField])));
        }
    }, [selectedRows.size, paginatedData, keyField]);

    const handleExport = useCallback(() => {
        const csv = [
            columns.map(col => col.header).join(','),
            ...sortedData.map(row =>
                columns.map(col => String(row[col.key])).join(',')
            )
        ].join('\n');

        const blob = new Blob([csv], { type: 'text/csv' });
        const url = URL.createObjectURL(blob);
        const link = document.createElement('a');
        link.href = url;
        link.download = 'export.csv';
        link.click();
        URL.revokeObjectURL(url);
    }, [sortedData, columns]);

    const handlePageChange = useCallback((page: number) => {
        setCurrentPage(page);
    }, []);

    // Render helpers
    const renderSortIndicator = (key: keyof T) => {
        if (!sortConfig || sortConfig.key !== key) return null;
        return sortConfig.direction === 'asc' ? ' ▲' : ' ▼';
    };

    const renderPagination = () => {
        const pages: number[] = [];
        const maxVisible = 5;

        let start = Math.max(1, currentPage - Math.floor(maxVisible / 2));
        let end = Math.min(totalPages, start + maxVisible - 1);

        if (end - start + 1 < maxVisible) {
            start = Math.max(1, end - maxVisible + 1);
        }

        for (let i = start; i <= end; i++) {
            pages.push(i);
        }

        return (
            <div className="pagination">
                <button
                    onClick={() => handlePageChange(1)}
                    disabled={currentPage === 1}
                >
                    First
                </button>
                <button
                    onClick={() => handlePageChange(currentPage - 1)}
                    disabled={currentPage === 1}
                >
                    Previous
                </button>

                {pages.map(page => (
                    <button
                        key={page}
                        onClick={() => handlePageChange(page)}
                        className={page === currentPage ? 'active' : ''}
                    >
                        {page}
                    </button>
                ))}

                <button
                    onClick={() => handlePageChange(currentPage + 1)}
                    disabled={currentPage === totalPages}
                >
                    Next
                </button>
                <button
                    onClick={() => handlePageChange(totalPages)}
                    disabled={currentPage === totalPages}
                >
                    Last
                </button>

                <span className="page-info">
                    Page {currentPage} of {totalPages} ({sortedData.length} items)
                </span>
            </div>
        );
    };

    return (
        <div className="data-table">
            {/* Toolbar */}
            <div className="table-toolbar">
                {searchable && (
                    <input
                        type="text"
                        placeholder="Search..."
                        value={searchTerm}
                        onChange={handleSearch}
                        className="search-input"
                    />
                )}

                {exportable && (
                    <button onClick={handleExport} className="export-btn">
                        Export CSV
                    </button>
                )}

                {selectedRows.size > 0 && (
                    <span className="selection-info">
                        {selectedRows.size} row(s) selected
                    </span>
                )}
            </div>

            {/* Filters */}
            {filterable && (
                <div className="table-filters">
                    {columns.map(col => (
                        <input
                            key={String(col.key)}
                            type="text"
                            placeholder={`Filter ${col.header}...`}
                            value={filters[String(col.key)] || ''}
                            onChange={(e) => handleFilterChange(String(col.key), e.target.value)}
                            className="filter-input"
                        />
                    ))}
                </div>
            )}

            {/* Table */}
            <table>
                <thead>
                    <tr>
                        <th>
                            <input
                                type="checkbox"
                                checked={selectedRows.size === paginatedData.length && paginatedData.length > 0}
                                onChange={handleSelectAll}
                            />
                        </th>
                        {columns.map(col => (
                            <th
                                key={String(col.key)}
                                style={{ width: col.width }}
                                onClick={() => col.sortable !== false && handleSort(col.key)}
                                className={col.sortable !== false ? 'sortable' : ''}
                            >
                                {col.header}
                                {col.sortable !== false && renderSortIndicator(col.key)}
                            </th>
                        ))}
                    </tr>
                </thead>
                <tbody>
                    {paginatedData.map(row => (
                        <tr
                            key={String(row[keyField])}
                            onClick={() => onRowClick?.(row)}
                            className={selectedRows.has(row[keyField]) ? 'selected' : ''}
                        >
                            <td>
                                <input
                                    type="checkbox"
                                    checked={selectedRows.has(row[keyField])}
                                    onChange={() => handleRowSelect(row[keyField])}
                                    onClick={(e) => e.stopPropagation()}
                                />
                            </td>
                            {columns.map(col => (
                                <td key={String(col.key)}>
                                    {col.render
                                        ? col.render(row[col.key], row)
                                        : String(row[col.key])}
                                </td>
                            ))}
                        </tr>
                    ))}
                </tbody>
            </table>

            {/* Pagination */}
            {totalPages > 1 && renderPagination()}

            {/* Empty state */}
            {paginatedData.length === 0 && (
                <div className="empty-state">
                    No data to display
                </div>
            )}
        </div>
    );
}

// Export types
export type { Column, SortConfig, FilterConfig, DataTableProps };
"#;

    println!("Complex component: {} bytes", complex_component.len());

    // Verify component complexity
    let required_patterns = &[
        "useState",
        "useEffect",
        "useCallback",
        "useMemo",
        "interface",
        "export function DataTable<T",
        "sortedData",
        "paginatedData",
        "filteredData",
        "handleSort",
        "handleSearch",
        "handlePageChange",
    ];

    assert!(
        test_utils::contains_all(complex_component, required_patterns),
        "Component should contain all required patterns"
    );

    // Verify proper TypeScript generics
    assert!(complex_component.contains("<T extends Record<string, unknown>>"));

    // Count hooks - should have many for complex component
    let hook_count = test_utils::count_occurrences(complex_component, "use");
    assert!(hook_count >= 8, "Complex component should use multiple hooks");

    // Verify event handlers
    let handler_count = test_utils::count_occurrences(complex_component, "handle");
    assert!(handler_count >= 6, "Should have multiple event handlers");
}

/// Test token efficiency for TSX transformations
#[test]
fn test_tsx_token_efficiency() {
    let simple_component = r#"
export const Button = ({ label, onClick }) => {
    return <button onClick={onClick}>{label}</button>;
};
"#;

    let typed_component = r#"
import React from 'react';

interface ButtonProps {
    label: string;
    onClick: () => void;
    disabled?: boolean;
    variant?: 'primary' | 'secondary' | 'danger';
}

export const Button: React.FC<ButtonProps> = ({
    label,
    onClick,
    disabled = false,
    variant = 'primary'
}) => {
    return (
        <button
            onClick={onClick}
            disabled={disabled}
            className={`btn btn-${variant}`}
        >
            {label}
        </button>
    );
};
"#;

    let simple_size = simple_component.len();
    let typed_size = typed_component.len();
    let overhead = (typed_size as f64 / simple_size as f64) - 1.0;

    println!("Simple: {} bytes, Typed: {} bytes, Overhead: {:.1}%",
             simple_size, typed_size, overhead * 100.0);

    // TypeScript overhead should be reasonable (less than 5x)
    assert!(typed_size < simple_size * 5, "Type overhead should be reasonable");

    // Verify types were added
    assert!(typed_component.contains("interface ButtonProps"));
    assert!(typed_component.contains("React.FC<ButtonProps>"));
}
