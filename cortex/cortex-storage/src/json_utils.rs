//! JSON utility functions for working with SurrealDB data.
//!
//! This module provides utilities to handle SurrealDB-specific constraints,
//! particularly around field naming conflicts with reserved identifiers.

use serde_json::Value;

/// Rename an ID field in a JSON object.
///
/// This is useful for working around SurrealDB's reserved field names.
/// For example, SurrealDB treats `id` as a special field for record IDs,
/// so we need to rename it to avoid conflicts with application-level ID fields.
///
/// # Arguments
///
/// * `json` - A mutable reference to a JSON value (expected to be an object)
/// * `from` - The source field name to rename
/// * `to` - The target field name
///
/// # Returns
///
/// `true` if the field was renamed, `false` if the field didn't exist or json is not an object
///
/// # Examples
///
/// ```
/// use serde_json::json;
/// use cortex_storage::json_utils::rename_id_field;
///
/// let mut data = json!({
///     "id": "user-123",
///     "name": "Alice"
/// });
///
/// rename_id_field(&mut data, "id", "cortex_id");
/// assert_eq!(data["cortex_id"], "user-123");
/// assert!(data.get("id").is_none());
/// ```
pub fn rename_id_field(json: &mut Value, from: &str, to: &str) -> bool {
    if let Some(obj) = json.as_object_mut() {
        if let Some(id_val) = obj.remove(from) {
            obj.insert(to.to_string(), id_val);
            return true;
        }
    }
    false
}

/// Restore the original `id` field from `cortex_id`.
///
/// This is a convenience function for the common case of converting
/// data retrieved from SurrealDB back to the application format.
///
/// # Arguments
///
/// * `json` - A mutable reference to a JSON value (expected to be an object)
///
/// # Returns
///
/// `true` if the field was restored, `false` if `cortex_id` didn't exist or json is not an object
///
/// # Examples
///
/// ```
/// use serde_json::json;
/// use cortex_storage::json_utils::restore_id_field;
///
/// let mut data = json!({
///     "cortex_id": "user-123",
///     "name": "Alice"
/// });
///
/// restore_id_field(&mut data);
/// assert_eq!(data["id"], "user-123");
/// assert!(data.get("cortex_id").is_none());
/// ```
pub fn restore_id_field(json: &mut Value) -> bool {
    rename_id_field(json, "cortex_id", "id")
}

/// Prepare data for storage in SurrealDB by renaming `id` to `cortex_id`.
///
/// This is a convenience function for the common case of converting
/// application data before storing it in SurrealDB.
///
/// # Arguments
///
/// * `json` - A mutable reference to a JSON value (expected to be an object)
///
/// # Returns
///
/// `true` if the field was prepared, `false` if `id` didn't exist or json is not an object
///
/// # Examples
///
/// ```
/// use serde_json::json;
/// use cortex_storage::json_utils::prepare_for_db;
///
/// let mut data = json!({
///     "id": "user-123",
///     "name": "Alice"
/// });
///
/// prepare_for_db(&mut data);
/// assert_eq!(data["cortex_id"], "user-123");
/// assert!(data.get("id").is_none());
/// ```
pub fn prepare_for_db(json: &mut Value) -> bool {
    rename_id_field(json, "id", "cortex_id")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_rename_id_field_success() {
        let mut data = json!({
            "id": "test-123",
            "name": "Test Item"
        });

        let result = rename_id_field(&mut data, "id", "cortex_id");

        assert!(result);
        assert_eq!(data["cortex_id"], "test-123");
        assert!(data.get("id").is_none());
        assert_eq!(data["name"], "Test Item");
    }

    #[test]
    fn test_rename_id_field_missing_field() {
        let mut data = json!({
            "name": "Test Item"
        });

        let result = rename_id_field(&mut data, "id", "cortex_id");

        assert!(!result);
        assert!(data.get("cortex_id").is_none());
    }

    #[test]
    fn test_rename_id_field_not_object() {
        let mut data = json!("string value");

        let result = rename_id_field(&mut data, "id", "cortex_id");

        assert!(!result);
    }

    #[test]
    fn test_rename_id_field_array() {
        let mut data = json!([1, 2, 3]);

        let result = rename_id_field(&mut data, "id", "cortex_id");

        assert!(!result);
    }

    #[test]
    fn test_restore_id_field_success() {
        let mut data = json!({
            "cortex_id": "test-456",
            "description": "Test description"
        });

        let result = restore_id_field(&mut data);

        assert!(result);
        assert_eq!(data["id"], "test-456");
        assert!(data.get("cortex_id").is_none());
        assert_eq!(data["description"], "Test description");
    }

    #[test]
    fn test_restore_id_field_missing_cortex_id() {
        let mut data = json!({
            "description": "Test description"
        });

        let result = restore_id_field(&mut data);

        assert!(!result);
        assert!(data.get("id").is_none());
    }

    #[test]
    fn test_prepare_for_db_success() {
        let mut data = json!({
            "id": "test-789",
            "metadata": {"key": "value"}
        });

        let result = prepare_for_db(&mut data);

        assert!(result);
        assert_eq!(data["cortex_id"], "test-789");
        assert!(data.get("id").is_none());
        assert_eq!(data["metadata"]["key"], "value");
    }

    #[test]
    fn test_prepare_for_db_missing_id() {
        let mut data = json!({
            "metadata": {"key": "value"}
        });

        let result = prepare_for_db(&mut data);

        assert!(!result);
        assert!(data.get("cortex_id").is_none());
    }

    #[test]
    fn test_round_trip() {
        let mut data = json!({
            "id": "round-trip-test",
            "field1": "value1",
            "field2": 42
        });

        // Prepare for DB storage
        assert!(prepare_for_db(&mut data));
        assert_eq!(data["cortex_id"], "round-trip-test");
        assert!(data.get("id").is_none());

        // Restore after retrieval
        assert!(restore_id_field(&mut data));
        assert_eq!(data["id"], "round-trip-test");
        assert!(data.get("cortex_id").is_none());
        assert_eq!(data["field1"], "value1");
        assert_eq!(data["field2"], 42);
    }

    #[test]
    fn test_custom_field_names() {
        let mut data = json!({
            "user_id": "user-123",
            "custom_field": "value"
        });

        let result = rename_id_field(&mut data, "user_id", "db_user_id");

        assert!(result);
        assert_eq!(data["db_user_id"], "user-123");
        assert!(data.get("user_id").is_none());
        assert_eq!(data["custom_field"], "value");
    }

    #[test]
    fn test_complex_id_value() {
        let mut data = json!({
            "id": {
                "prefix": "user",
                "number": 123,
                "uuid": "550e8400-e29b-41d4-a716-446655440000"
            },
            "name": "Test"
        });

        assert!(prepare_for_db(&mut data));
        assert_eq!(data["cortex_id"]["prefix"], "user");
        assert_eq!(data["cortex_id"]["number"], 123);
        assert!(data.get("id").is_none());

        assert!(restore_id_field(&mut data));
        assert_eq!(data["id"]["prefix"], "user");
        assert!(data.get("cortex_id").is_none());
    }
}
