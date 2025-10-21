//! Comprehensive tests for permission types and functionality.
//!
//! Tests permission modes, updates, results, and permission checking logic.

use cc_sdk::permissions::{
    PermissionBehavior, PermissionMode, PermissionResult, PermissionResultAllow,
    PermissionResultDeny, PermissionRuleValue, PermissionUpdate, PermissionUpdateDestination,
    PermissionUpdateType, ToolPermissionContext,
};

#[test]
fn test_permission_mode_default() {
    let mode = PermissionMode::default();
    assert_eq!(mode, PermissionMode::Default);
}

#[test]
fn test_permission_mode_variants() {
    let modes = vec![
        PermissionMode::Default,
        PermissionMode::AcceptEdits,
        PermissionMode::Plan,
        PermissionMode::BypassPermissions,
    ];

    // All modes should be distinct
    for (i, mode1) in modes.iter().enumerate() {
        for (j, mode2) in modes.iter().enumerate() {
            if i == j {
                assert_eq!(mode1, mode2);
            } else {
                assert_ne!(mode1, mode2);
            }
        }
    }
}

#[test]
fn test_permission_mode_serialization() {
    let test_cases = vec![
        (PermissionMode::Default, r#""default""#),
        (PermissionMode::AcceptEdits, r#""acceptEdits""#),
        (PermissionMode::Plan, r#""plan""#),
        (PermissionMode::BypassPermissions, r#""bypassPermissions""#),
    ];

    for (mode, expected_json) in test_cases {
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, expected_json);

        let deserialized: PermissionMode = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, mode);
    }
}

#[test]
fn test_permission_behavior_variants() {
    let allow = PermissionBehavior::Allow;
    let deny = PermissionBehavior::Deny;
    let ask = PermissionBehavior::Ask;

    assert_eq!(allow, PermissionBehavior::Allow);
    assert_eq!(deny, PermissionBehavior::Deny);
    assert_eq!(ask, PermissionBehavior::Ask);

    assert_ne!(allow, deny);
    assert_ne!(allow, ask);
    assert_ne!(deny, ask);
}

#[test]
fn test_permission_behavior_serialization() {
    let test_cases = vec![
        (PermissionBehavior::Allow, r#""allow""#),
        (PermissionBehavior::Deny, r#""deny""#),
        (PermissionBehavior::Ask, r#""ask""#),
    ];

    for (behavior, expected_json) in test_cases {
        let json = serde_json::to_string(&behavior).unwrap();
        assert_eq!(json, expected_json);

        let deserialized: PermissionBehavior = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, behavior);
    }
}

#[test]
fn test_permission_update_destination_variants() {
    let destinations = vec![
        PermissionUpdateDestination::UserSettings,
        PermissionUpdateDestination::ProjectSettings,
        PermissionUpdateDestination::LocalSettings,
        PermissionUpdateDestination::Session,
    ];

    // All should be distinct
    for (i, dest1) in destinations.iter().enumerate() {
        for (j, dest2) in destinations.iter().enumerate() {
            if i == j {
                assert_eq!(dest1, dest2);
            } else {
                assert_ne!(dest1, dest2);
            }
        }
    }
}

#[test]
fn test_permission_update_destination_serialization() {
    let test_cases = vec![
        (PermissionUpdateDestination::UserSettings, r#""userSettings""#),
        (
            PermissionUpdateDestination::ProjectSettings,
            r#""projectSettings""#,
        ),
        (
            PermissionUpdateDestination::LocalSettings,
            r#""localSettings""#,
        ),
        (PermissionUpdateDestination::Session, r#""session""#),
    ];

    for (dest, expected_json) in test_cases {
        let json = serde_json::to_string(&dest).unwrap();
        assert_eq!(json, expected_json);

        let deserialized: PermissionUpdateDestination = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, dest);
    }
}

#[test]
fn test_permission_update_type_variants() {
    let types = vec![
        PermissionUpdateType::AddRules,
        PermissionUpdateType::ReplaceRules,
        PermissionUpdateType::RemoveRules,
        PermissionUpdateType::SetMode,
        PermissionUpdateType::AddDirectories,
        PermissionUpdateType::RemoveDirectories,
    ];

    // All should be distinct
    for (i, type1) in types.iter().enumerate() {
        for (j, type2) in types.iter().enumerate() {
            if i == j {
                assert_eq!(type1, type2);
            } else {
                assert_ne!(type1, type2);
            }
        }
    }
}

#[test]
fn test_permission_update_type_serialization() {
    let test_cases = vec![
        (PermissionUpdateType::AddRules, r#""addRules""#),
        (PermissionUpdateType::ReplaceRules, r#""replaceRules""#),
        (PermissionUpdateType::RemoveRules, r#""removeRules""#),
        (PermissionUpdateType::SetMode, r#""setMode""#),
        (PermissionUpdateType::AddDirectories, r#""addDirectories""#),
        (PermissionUpdateType::RemoveDirectories, r#""removeDirectories""#),
    ];

    for (update_type, expected_json) in test_cases {
        let json = serde_json::to_string(&update_type).unwrap();
        assert_eq!(json, expected_json);

        let deserialized: PermissionUpdateType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, update_type);
    }
}

#[test]
fn test_permission_rule_value_creation() {
    let rule = PermissionRuleValue {
        tool_name: "Bash".to_string(),
        rule_content: Some("Allow all bash commands".to_string()),
    };

    assert_eq!(rule.tool_name, "Bash");
    assert_eq!(
        rule.rule_content,
        Some("Allow all bash commands".to_string())
    );
}

#[test]
fn test_permission_rule_value_serialization() {
    let rule = PermissionRuleValue {
        tool_name: "Read".to_string(),
        rule_content: Some("Allow reading /tmp/*".to_string()),
    };

    let json = serde_json::to_string(&rule).unwrap();
    assert!(json.contains(r#""tool_name":"Read""#));
    assert!(json.contains(r#""rule_content":"Allow reading /tmp/*""#));

    let deserialized: PermissionRuleValue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.tool_name, rule.tool_name);
    assert_eq!(deserialized.rule_content, rule.rule_content);
}

#[test]
fn test_permission_update_add_rules() {
    let update = PermissionUpdate {
        update_type: PermissionUpdateType::AddRules,
        rules: Some(vec![
            PermissionRuleValue {
                tool_name: "Bash".to_string(),
                rule_content: None,
            },
            PermissionRuleValue {
                tool_name: "Read".to_string(),
                rule_content: Some("*.txt".to_string()),
            },
        ]),
        behavior: Some(PermissionBehavior::Allow),
        mode: None,
        directories: None,
        destination: Some(PermissionUpdateDestination::Session),
    };

    assert_eq!(update.update_type, PermissionUpdateType::AddRules);
    assert!(update.rules.is_some());
    assert_eq!(update.rules.as_ref().unwrap().len(), 2);
    assert_eq!(update.behavior, Some(PermissionBehavior::Allow));
    assert_eq!(update.destination, Some(PermissionUpdateDestination::Session));
}

#[test]
fn test_permission_update_set_mode() {
    let update = PermissionUpdate {
        update_type: PermissionUpdateType::SetMode,
        rules: None,
        behavior: None,
        mode: Some(PermissionMode::AcceptEdits),
        directories: None,
        destination: Some(PermissionUpdateDestination::UserSettings),
    };

    assert_eq!(update.update_type, PermissionUpdateType::SetMode);
    assert_eq!(update.mode, Some(PermissionMode::AcceptEdits));
    assert_eq!(
        update.destination,
        Some(PermissionUpdateDestination::UserSettings)
    );
}

#[test]
fn test_permission_update_add_directories() {
    let update = PermissionUpdate {
        update_type: PermissionUpdateType::AddDirectories,
        rules: None,
        behavior: None,
        mode: None,
        directories: Some(vec!["/safe/path".to_string(), "/another/path".to_string()]),
        destination: Some(PermissionUpdateDestination::ProjectSettings),
    };

    assert_eq!(update.update_type, PermissionUpdateType::AddDirectories);
    assert!(update.directories.is_some());
    assert_eq!(update.directories.as_ref().unwrap().len(), 2);
}

#[test]
fn test_permission_update_serialization() {
    let update = PermissionUpdate {
        update_type: PermissionUpdateType::AddRules,
        rules: Some(vec![PermissionRuleValue {
            tool_name: "Bash".to_string(),
            rule_content: None,
        }]),
        behavior: Some(PermissionBehavior::Allow),
        mode: None,
        directories: None,
        destination: None,
    };

    let json = serde_json::to_string(&update).unwrap();
    assert!(json.contains(r#""type":"addRules""#));
    assert!(json.contains(r#""tool_name":"Bash""#));

    let deserialized: PermissionUpdate = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.update_type, update.update_type);
}

#[test]
fn test_tool_permission_context_creation() {
    let context = ToolPermissionContext {
        signal: None,
        suggestions: vec![],
    };

    assert!(context.signal.is_none());
    assert_eq!(context.suggestions.len(), 0);
}

#[test]
fn test_tool_permission_context_with_suggestions() {
    let context = ToolPermissionContext {
        signal: None,
        suggestions: vec![
            PermissionUpdate {
                update_type: PermissionUpdateType::AddRules,
                rules: Some(vec![PermissionRuleValue {
                    tool_name: "Bash".to_string(),
                    rule_content: None,
                }]),
                behavior: Some(PermissionBehavior::Allow),
                mode: None,
                directories: None,
                destination: None,
            },
        ],
    };

    assert_eq!(context.suggestions.len(), 1);
}

#[test]
fn test_permission_result_allow() {
    let result = PermissionResult::Allow(PermissionResultAllow {
        updated_input: None,
        updated_permissions: None,
    });

    match result {
        PermissionResult::Allow(allow) => {
            assert!(allow.updated_input.is_none());
            assert!(allow.updated_permissions.is_none());
        }
        _ => panic!("Expected Allow result"),
    }
}

#[test]
fn test_permission_result_allow_with_updates() {
    let result = PermissionResult::Allow(PermissionResultAllow {
        updated_input: Some(serde_json::json!({"modified": true})),
        updated_permissions: Some(vec![PermissionUpdate {
            update_type: PermissionUpdateType::AddRules,
            rules: None,
            behavior: None,
            mode: None,
            directories: None,
            destination: None,
        }]),
    });

    match result {
        PermissionResult::Allow(allow) => {
            assert!(allow.updated_input.is_some());
            assert!(allow.updated_permissions.is_some());
            assert_eq!(allow.updated_permissions.as_ref().unwrap().len(), 1);
        }
        _ => panic!("Expected Allow result"),
    }
}

#[test]
fn test_permission_result_deny() {
    let result = PermissionResult::Deny(PermissionResultDeny {
        message: "Access denied to /etc/passwd".to_string(),
        interrupt: false,
    });

    match result {
        PermissionResult::Deny(deny) => {
            assert_eq!(deny.message, "Access denied to /etc/passwd");
            assert!(!deny.interrupt);
        }
        _ => panic!("Expected Deny result"),
    }
}

#[test]
fn test_permission_result_deny_with_interrupt() {
    let result = PermissionResult::Deny(PermissionResultDeny {
        message: "Critical security violation".to_string(),
        interrupt: true,
    });

    match result {
        PermissionResult::Deny(deny) => {
            assert!(deny.interrupt);
            assert!(deny.message.contains("security"));
        }
        _ => panic!("Expected Deny result"),
    }
}

#[test]
fn test_permission_result_clone() {
    let original = PermissionResult::Allow(PermissionResultAllow {
        updated_input: None,
        updated_permissions: None,
    });

    let cloned = original.clone();

    match (original, cloned) {
        (PermissionResult::Allow(_), PermissionResult::Allow(_)) => {
            // Both are Allow
        }
        _ => panic!("Clone should be same variant"),
    }
}

#[test]
fn test_permission_mode_copy() {
    let mode1 = PermissionMode::AcceptEdits;
    let mode2 = mode1; // Should copy

    assert_eq!(mode1, mode2);
    assert_eq!(mode1, PermissionMode::AcceptEdits); // mode1 still usable
}

#[test]
fn test_permission_update_clone() {
    let update = PermissionUpdate {
        update_type: PermissionUpdateType::SetMode,
        rules: None,
        behavior: None,
        mode: Some(PermissionMode::Plan),
        directories: None,
        destination: None,
    };

    let cloned = update.clone();
    assert_eq!(cloned.update_type, update.update_type);
    assert_eq!(cloned.mode, update.mode);
}
