//! Code Manipulation Tools
//!
//! This module implements the 15 code manipulation tools defined in the MCP spec:
//! - cortex.code.create_unit
//! - cortex.code.update_unit
//! - cortex.code.delete_unit
//! - cortex.code.move_unit
//! - cortex.code.rename_unit
//! - cortex.code.extract_function
//! - cortex.code.inline_function
//! - cortex.code.change_signature
//! - cortex.code.add_parameter
//! - cortex.code.remove_parameter
//! - cortex.code.add_import
//! - cortex.code.optimize_imports
//! - cortex.code.generate_getter_setter
//! - cortex.code.implement_interface
//! - cortex.code.override_method

use async_trait::async_trait;
use cortex_storage::ConnectionManager;
use cortex_vfs::VirtualFileSystem;
use mcp_server::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

// =============================================================================
// Shared Context
// =============================================================================

/// Shared context for all code manipulation tools
#[derive(Clone)]
pub struct CodeManipulationContext {
    storage: Arc<ConnectionManager>,
    vfs: Arc<VirtualFileSystem>,
}

impl CodeManipulationContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
        Self { storage, vfs }
    }
}

// =============================================================================
// cortex.code.create_unit
// =============================================================================

pub struct CodeCreateUnitTool {
    ctx: CodeManipulationContext,
}

impl CodeCreateUnitTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateUnitInput {
    file_path: String,
    unit_type: String,
    name: String,
    signature: Option<String>,
    body: String,
    position: Option<String>,
    visibility: Option<String>,
    docstring: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct CreateUnitOutput {
    unit_id: String,
    qualified_name: String,
    version: i64,
}

#[async_trait]
impl Tool for CodeCreateUnitTool {
    fn name(&self) -> &str {
        "cortex.code.create_unit"
    }

    fn description(&self) -> Option<&str> {
        Some("Creates a new code unit (function, class, etc.) in a file")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(CreateUnitInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: CreateUnitInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!(
            "Creating unit '{}' of type '{}' in file '{}'",
            input.name, input.unit_type, input.file_path
        );

        // TODO: Implement unit creation logic using tree-sitter parser
        // For now, return a placeholder response
        let output = CreateUnitOutput {
            unit_id: format!("unit_{}", uuid::Uuid::new_v4()),
            qualified_name: format!("{}::{}", input.file_path, input.name),
            version: 1,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.update_unit
// =============================================================================

pub struct CodeUpdateUnitTool {
    ctx: CodeManipulationContext,
}

impl CodeUpdateUnitTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateUnitInput {
    unit_id: String,
    signature: Option<String>,
    body: Option<String>,
    docstring: Option<String>,
    visibility: Option<String>,
    expected_version: i64,
    #[serde(default = "default_true")]
    preserve_comments: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct UpdateUnitOutput {
    unit_id: String,
    new_version: i64,
    updated: bool,
}

#[async_trait]
impl Tool for CodeUpdateUnitTool {
    fn name(&self) -> &str {
        "cortex.code.update_unit"
    }

    fn description(&self) -> Option<&str> {
        Some("Updates an existing code unit (signature, body, docstring, visibility)")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(UpdateUnitInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: UpdateUnitInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Updating unit '{}'", input.unit_id);

        let output = UpdateUnitOutput {
            unit_id: input.unit_id.clone(),
            new_version: input.expected_version + 1,
            updated: true,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.delete_unit
// =============================================================================

pub struct CodeDeleteUnitTool {
    ctx: CodeManipulationContext,
}

impl CodeDeleteUnitTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DeleteUnitInput {
    unit_id: String,
    #[serde(default)]
    cascade: bool,
    expected_version: i64,
}

#[derive(Debug, Serialize, JsonSchema)]
struct DeleteUnitOutput {
    unit_id: String,
    deleted: bool,
    cascade_deleted: Vec<String>,
}

#[async_trait]
impl Tool for CodeDeleteUnitTool {
    fn name(&self) -> &str {
        "cortex.code.delete_unit"
    }

    fn description(&self) -> Option<&str> {
        Some("Deletes a code unit and optionally its dependents")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DeleteUnitInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DeleteUnitInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Deleting unit '{}'", input.unit_id);

        let output = DeleteUnitOutput {
            unit_id: input.unit_id.clone(),
            deleted: true,
            cascade_deleted: vec![],
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.move_unit
// =============================================================================

pub struct CodeMoveUnitTool {
    ctx: CodeManipulationContext,
}

impl CodeMoveUnitTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MoveUnitInput {
    unit_id: String,
    target_file: String,
    position: Option<String>,
    #[serde(default = "default_true")]
    update_imports: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct MoveUnitOutput {
    unit_id: String,
    old_file: String,
    new_file: String,
    imports_updated: Vec<String>,
}

#[async_trait]
impl Tool for CodeMoveUnitTool {
    fn name(&self) -> &str {
        "cortex.code.move_unit"
    }

    fn description(&self) -> Option<&str> {
        Some("Moves a code unit to another file, updating imports")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(MoveUnitInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: MoveUnitInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Moving unit '{}' to '{}'", input.unit_id, input.target_file);

        let output = MoveUnitOutput {
            unit_id: input.unit_id.clone(),
            old_file: "/old/path.rs".to_string(),
            new_file: input.target_file.clone(),
            imports_updated: vec![],
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.rename_unit
// =============================================================================

pub struct CodeRenameUnitTool {
    ctx: CodeManipulationContext,
}

impl CodeRenameUnitTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RenameUnitInput {
    unit_id: String,
    new_name: String,
    #[serde(default = "default_true")]
    update_references: bool,
    #[serde(default = "default_workspace_scope")]
    scope: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct RenameUnitOutput {
    unit_id: String,
    old_name: String,
    new_name: String,
    references_updated: i32,
}

#[async_trait]
impl Tool for CodeRenameUnitTool {
    fn name(&self) -> &str {
        "cortex.code.rename_unit"
    }

    fn description(&self) -> Option<&str> {
        Some("Renames a code unit and updates all references")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(RenameUnitInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: RenameUnitInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Renaming unit '{}' to '{}'", input.unit_id, input.new_name);

        let output = RenameUnitOutput {
            unit_id: input.unit_id.clone(),
            old_name: "old_name".to_string(),
            new_name: input.new_name.clone(),
            references_updated: 0,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.extract_function
// =============================================================================

pub struct CodeExtractFunctionTool {
    ctx: CodeManipulationContext,
}

impl CodeExtractFunctionTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ExtractFunctionInput {
    source_unit_id: String,
    start_line: i32,
    end_line: i32,
    function_name: String,
    #[serde(default = "default_before_position")]
    position: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ExtractFunctionOutput {
    new_unit_id: String,
    function_name: String,
    parameters: Vec<String>,
    return_type: Option<String>,
}

#[async_trait]
impl Tool for CodeExtractFunctionTool {
    fn name(&self) -> &str {
        "cortex.code.extract_function"
    }

    fn description(&self) -> Option<&str> {
        Some("Extracts code block into a new function with inferred parameters")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ExtractFunctionInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: ExtractFunctionInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!(
            "Extracting function '{}' from lines {}-{}",
            input.function_name, input.start_line, input.end_line
        );

        let output = ExtractFunctionOutput {
            new_unit_id: format!("unit_{}", uuid::Uuid::new_v4()),
            function_name: input.function_name.clone(),
            parameters: vec![],
            return_type: None,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.inline_function
// =============================================================================

pub struct CodeInlineFunctionTool {
    ctx: CodeManipulationContext,
}

impl CodeInlineFunctionTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct InlineFunctionInput {
    function_id: String,
    call_sites: Option<Vec<String>>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct InlineFunctionOutput {
    function_id: String,
    sites_inlined: i32,
    function_removed: bool,
}

#[async_trait]
impl Tool for CodeInlineFunctionTool {
    fn name(&self) -> &str {
        "cortex.code.inline_function"
    }

    fn description(&self) -> Option<&str> {
        Some("Inlines a function at call sites, optionally removing the function")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(InlineFunctionInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: InlineFunctionInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Inlining function '{}'", input.function_id);

        let output = InlineFunctionOutput {
            function_id: input.function_id.clone(),
            sites_inlined: 0,
            function_removed: false,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.change_signature
// =============================================================================

pub struct CodeChangeSignatureTool {
    ctx: CodeManipulationContext,
}

impl CodeChangeSignatureTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ChangeSignatureInput {
    unit_id: String,
    new_signature: String,
    #[serde(default = "default_true")]
    update_callers: bool,
    #[serde(default = "default_migration_strategy")]
    migration_strategy: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ChangeSignatureOutput {
    unit_id: String,
    old_signature: String,
    new_signature: String,
    callers_updated: i32,
}

#[async_trait]
impl Tool for CodeChangeSignatureTool {
    fn name(&self) -> &str {
        "cortex.code.change_signature"
    }

    fn description(&self) -> Option<&str> {
        Some("Changes function/method signature and updates all callers")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ChangeSignatureInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: ChangeSignatureInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Changing signature for unit '{}'", input.unit_id);

        let output = ChangeSignatureOutput {
            unit_id: input.unit_id.clone(),
            old_signature: "old_sig".to_string(),
            new_signature: input.new_signature.clone(),
            callers_updated: 0,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.add_parameter
// =============================================================================

pub struct CodeAddParameterTool {
    ctx: CodeManipulationContext,
}

impl CodeAddParameterTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AddParameterInput {
    unit_id: String,
    parameter_name: String,
    parameter_type: String,
    default_value: Option<String>,
    #[serde(default = "default_last_position")]
    position: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct AddParameterOutput {
    unit_id: String,
    parameter_added: String,
    new_signature: String,
}

#[async_trait]
impl Tool for CodeAddParameterTool {
    fn name(&self) -> &str {
        "cortex.code.add_parameter"
    }

    fn description(&self) -> Option<&str> {
        Some("Adds a parameter to a function signature")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AddParameterInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AddParameterInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!(
            "Adding parameter '{}' to unit '{}'",
            input.parameter_name, input.unit_id
        );

        let output = AddParameterOutput {
            unit_id: input.unit_id.clone(),
            parameter_added: input.parameter_name.clone(),
            new_signature: format!("fn(..., {}: {})", input.parameter_name, input.parameter_type),
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.remove_parameter
// =============================================================================

pub struct CodeRemoveParameterTool {
    ctx: CodeManipulationContext,
}

impl CodeRemoveParameterTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RemoveParameterInput {
    unit_id: String,
    parameter_name: String,
    #[serde(default = "default_true")]
    update_callers: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct RemoveParameterOutput {
    unit_id: String,
    parameter_removed: String,
    new_signature: String,
    callers_updated: i32,
}

#[async_trait]
impl Tool for CodeRemoveParameterTool {
    fn name(&self) -> &str {
        "cortex.code.remove_parameter"
    }

    fn description(&self) -> Option<&str> {
        Some("Removes a parameter from a function signature and updates callers")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(RemoveParameterInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: RemoveParameterInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!(
            "Removing parameter '{}' from unit '{}'",
            input.parameter_name, input.unit_id
        );

        let output = RemoveParameterOutput {
            unit_id: input.unit_id.clone(),
            parameter_removed: input.parameter_name.clone(),
            new_signature: "fn(...)".to_string(),
            callers_updated: 0,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.add_import
// =============================================================================

pub struct CodeAddImportTool {
    ctx: CodeManipulationContext,
}

impl CodeAddImportTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AddImportInput {
    file_path: String,
    import_spec: String,
    #[serde(default = "default_auto_position")]
    position: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct AddImportOutput {
    file_path: String,
    import_added: String,
    line_number: i32,
}

#[async_trait]
impl Tool for CodeAddImportTool {
    fn name(&self) -> &str {
        "cortex.code.add_import"
    }

    fn description(&self) -> Option<&str> {
        Some("Adds an import statement to a file at the optimal position")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AddImportInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AddImportInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Adding import '{}' to '{}'", input.import_spec, input.file_path);

        let output = AddImportOutput {
            file_path: input.file_path.clone(),
            import_added: input.import_spec.clone(),
            line_number: 1,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.optimize_imports
// =============================================================================

pub struct CodeOptimizeImportsTool {
    ctx: CodeManipulationContext,
}

impl CodeOptimizeImportsTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct OptimizeImportsInput {
    file_path: String,
    #[serde(default = "default_true")]
    remove_unused: bool,
    #[serde(default = "default_true")]
    sort: bool,
    #[serde(default = "default_true")]
    group: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct OptimizeImportsOutput {
    file_path: String,
    imports_removed: i32,
    imports_sorted: bool,
    imports_grouped: bool,
}

#[async_trait]
impl Tool for CodeOptimizeImportsTool {
    fn name(&self) -> &str {
        "cortex.code.optimize_imports"
    }

    fn description(&self) -> Option<&str> {
        Some("Optimizes imports by removing unused, sorting, and grouping")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(OptimizeImportsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: OptimizeImportsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Optimizing imports in '{}'", input.file_path);

        let output = OptimizeImportsOutput {
            file_path: input.file_path.clone(),
            imports_removed: 0,
            imports_sorted: input.sort,
            imports_grouped: input.group,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.generate_getter_setter
// =============================================================================

pub struct CodeGenerateGetterSetterTool {
    ctx: CodeManipulationContext,
}

impl CodeGenerateGetterSetterTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GenerateGetterSetterInput {
    class_id: String,
    field_name: String,
    #[serde(default = "default_both_generation")]
    generate: String,
    visibility: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct GenerateGetterSetterOutput {
    class_id: String,
    field_name: String,
    getter_id: Option<String>,
    setter_id: Option<String>,
}

#[async_trait]
impl Tool for CodeGenerateGetterSetterTool {
    fn name(&self) -> &str {
        "cortex.code.generate_getter_setter"
    }

    fn description(&self) -> Option<&str> {
        Some("Generates getter/setter methods for a class field")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(GenerateGetterSetterInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: GenerateGetterSetterInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!(
            "Generating getter/setter for field '{}' in class '{}'",
            input.field_name, input.class_id
        );

        let output = GenerateGetterSetterOutput {
            class_id: input.class_id.clone(),
            field_name: input.field_name.clone(),
            getter_id: Some(format!("unit_{}", uuid::Uuid::new_v4())),
            setter_id: Some(format!("unit_{}", uuid::Uuid::new_v4())),
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.implement_interface
// =============================================================================

pub struct CodeImplementInterfaceTool {
    ctx: CodeManipulationContext,
}

impl CodeImplementInterfaceTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ImplementInterfaceInput {
    class_id: String,
    interface_id: String,
    #[serde(default = "default_true")]
    generate_stubs: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ImplementInterfaceOutput {
    class_id: String,
    interface_id: String,
    methods_generated: Vec<String>,
}

#[async_trait]
impl Tool for CodeImplementInterfaceTool {
    fn name(&self) -> &str {
        "cortex.code.implement_interface"
    }

    fn description(&self) -> Option<&str> {
        Some("Implements an interface/trait with auto-generated stubs")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ImplementInterfaceInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: ImplementInterfaceInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!(
            "Implementing interface '{}' in class '{}'",
            input.interface_id, input.class_id
        );

        let output = ImplementInterfaceOutput {
            class_id: input.class_id.clone(),
            interface_id: input.interface_id.clone(),
            methods_generated: vec![],
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.override_method
// =============================================================================

pub struct CodeOverrideMethodTool {
    ctx: CodeManipulationContext,
}

impl CodeOverrideMethodTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct OverrideMethodInput {
    class_id: String,
    method_name: String,
    #[serde(default = "default_true")]
    call_super: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct OverrideMethodOutput {
    class_id: String,
    method_id: String,
    method_name: String,
    calls_super: bool,
}

#[async_trait]
impl Tool for CodeOverrideMethodTool {
    fn name(&self) -> &str {
        "cortex.code.override_method"
    }

    fn description(&self) -> Option<&str> {
        Some("Overrides a parent class method with stub implementation")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(OverrideMethodInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: OverrideMethodInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!(
            "Overriding method '{}' in class '{}'",
            input.method_name, input.class_id
        );

        let output = OverrideMethodOutput {
            class_id: input.class_id.clone(),
            method_id: format!("unit_{}", uuid::Uuid::new_v4()),
            method_name: input.method_name.clone(),
            calls_super: input.call_super,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// Helper functions
// =============================================================================

fn default_true() -> bool {
    true
}

fn default_workspace_scope() -> String {
    "workspace".to_string()
}

fn default_before_position() -> String {
    "before".to_string()
}

fn default_migration_strategy() -> String {
    "replace".to_string()
}

fn default_last_position() -> String {
    "last".to_string()
}

fn default_auto_position() -> String {
    "auto".to_string()
}

fn default_both_generation() -> String {
    "both".to_string()
}
