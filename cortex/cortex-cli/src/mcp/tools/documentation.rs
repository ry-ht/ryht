//! Documentation Tools (8 tools)

use async_trait::async_trait;
use cortex_storage::ConnectionManager;
use mcp_sdk::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

#[derive(Clone)]
pub struct DocumentationContext {
    storage: Arc<ConnectionManager>,
}

impl DocumentationContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self { storage }
    }
}

macro_rules! impl_doc_tool {
    ($name:ident, $tool_name:expr, $desc:expr, $input:ty, $output:ty) => {
        pub struct $name {
            ctx: DocumentationContext,
        }

        impl $name {
            pub fn new(ctx: DocumentationContext) -> Self {
                Self { ctx }
            }
        }

        #[async_trait]
        impl Tool for $name {
            fn name(&self) -> &str {
                $tool_name
            }

            fn description(&self) -> Option<&str> {
                Some($desc)
            }

            fn input_schema(&self) -> Value {
                serde_json::to_value(schemars::schema_for!($input)).unwrap()
            }

            async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
                let _input: $input = serde_json::from_value(input)
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
                debug!("{} executed", $tool_name);
                let output = <$output>::default();
                Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
            }
        }
    };
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocGenerateInput {
    unit_id: String,
    #[serde(default = "default_api_type")]
    doc_type: String,
    #[serde(default = "default_markdown")]
    format: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct DocGenerateOutput {
    documentation: String,
    format: String,
}

impl_doc_tool!(DocGenerateTool, "cortex.doc.generate", "Generate documentation", DocGenerateInput, DocGenerateOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocUpdateInput {
    unit_id: String,
    doc_content: String,
    #[serde(default = "default_docstring")]
    doc_type: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct DocUpdateOutput {
    unit_id: String,
    updated: bool,
}

impl_doc_tool!(DocUpdateTool, "cortex.doc.update", "Update existing documentation", DocUpdateInput, DocUpdateOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocExtractInput {
    scope_path: String,
    #[serde(default)]
    include_private: bool,
    #[serde(default = "default_markdown")]
    format: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct DocExtractOutput {
    documentation: String,
    units_documented: i32,
}

impl_doc_tool!(DocExtractTool, "cortex.doc.extract", "Extract documentation from code", DocExtractInput, DocExtractOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocFindUndocumentedInput {
    scope_path: String,
    #[serde(default = "default_public")]
    visibility: String,
    #[serde(default = "default_complexity_one")]
    min_complexity: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct DocFindUndocumentedOutput {
    undocumented_units: Vec<UndocumentedUnit>,
    total_count: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct UndocumentedUnit {
    unit_id: String,
    name: String,
    unit_type: String,
}

impl_doc_tool!(DocFindUndocumentedTool, "cortex.doc.find_undocumented", "Find undocumented code", DocFindUndocumentedInput, DocFindUndocumentedOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocCheckConsistencyInput {
    scope_path: String,
    #[serde(default = "default_true")]
    check_parameters: bool,
    #[serde(default = "default_true")]
    check_returns: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct DocCheckConsistencyOutput {
    inconsistencies: Vec<DocInconsistency>,
    total_count: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct DocInconsistency {
    unit_id: String,
    inconsistency_type: String,
    description: String,
}

impl_doc_tool!(DocCheckConsistencyTool, "cortex.doc.check_consistency", "Check doc-code consistency", DocCheckConsistencyInput, DocCheckConsistencyOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocLinkToCodeInput {
    doc_id: String,
    unit_id: String,
    #[serde(default = "default_describes")]
    link_type: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct DocLinkToCodeOutput {
    link_id: String,
    created: bool,
}

impl_doc_tool!(DocLinkToCodeTool, "cortex.doc.link_to_code", "Link documentation to code", DocLinkToCodeInput, DocLinkToCodeOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocGenerateReadmeInput {
    scope_path: String,
    sections: Option<Vec<String>>,
    #[serde(default = "default_true")]
    include_api: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct DocGenerateReadmeOutput {
    readme_content: String,
    sections_included: Vec<String>,
}

impl_doc_tool!(DocGenerateReadmeTool, "cortex.doc.generate_readme", "Generate README file", DocGenerateReadmeInput, DocGenerateReadmeOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocGenerateChangelogInput {
    from_version: Option<String>,
    to_version: Option<String>,
    #[serde(default = "default_keepachangelog")]
    format: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct DocGenerateChangelogOutput {
    changelog_content: String,
    format: String,
}

impl_doc_tool!(DocGenerateChangelogTool, "cortex.doc.generate_changelog", "Generate CHANGELOG", DocGenerateChangelogInput, DocGenerateChangelogOutput);

fn default_api_type() -> String { "api".to_string() }
fn default_markdown() -> String { "markdown".to_string() }
fn default_docstring() -> String { "docstring".to_string() }
fn default_public() -> String { "public".to_string() }
fn default_complexity_one() -> i32 { 1 }
fn default_true() -> bool { true }
fn default_describes() -> String { "describes".to_string() }
fn default_keepachangelog() -> String { "keepachangelog".to_string() }
