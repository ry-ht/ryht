#!/usr/bin/env bash
# Script to remove workspace_type from cortex codebase
set -e

echo "Removing workspace_type from cortex codebase..."

# Remove WorkspaceType import from commands.rs
sed -i '' 's/WorkspaceType, //' cortex/cortex/src/commands.rs
sed -i '' 's/, WorkspaceType//' cortex/cortex/src/commands.rs

# Remove from main.rs CLI
sed -i '' '/WorkspaceTypeArg/d' cortex/cortex/src/main.rs
sed -i '' '/workspace_type:/d' cortex/cortex/src/main.rs
sed -i '' 's/workspace_type.into()//' cortex/cortex/src/main.rs
sed -i '' 's/config.workspace_type//' cortex/cortex/src/main.rs

# Remove from interactive.rs
sed -i '' '/workspace_type/d' cortex/cortex/src/interactive.rs

# Remove from API types
sed -i '' '/workspace_type:/d' cortex/cortex/src/api/types.rs

# Remove from services
sed -i '' 's/workspace_type: Option<String>,\?//' cortex/cortex/src/services/workspace.rs
sed -i '' '/workspace_type/d' cortex/cortex/src/services/workspace.rs

# Remove from conversions
sed -i '' '/workspace_type/d' cortex/cortex/src/conversions/mod.rs

echo "✓ Basic removals complete"
echo "Note: Tests and detailed fixes need manual review"
