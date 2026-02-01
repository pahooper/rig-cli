//! Built-in tools for the Rig MCP server with declarative configuration and Rig patterns.

use jsonschema::Validator;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use schemars::{JsonSchema, schema_for};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::marker::PhantomData;
use std::sync::Arc;
use thiserror::Error;

/// Error type for Rig tools.
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum ToolError {
    /// Error during tool execution.
    #[error("Execution error: {0}")]
    Execution(String),
    /// Error during validation.
    #[error("Validation error: {0}")]
    Validation(String),
}

/// Type alias for the submission callback.
type SubmitCallback<T> = Arc<dyn Fn(T) -> String + Send + Sync>;

/// A declarative toolkit for JSON-based workflows using Rig's `schemars` pattern.
///
/// Consolidates a schema (derived from `T`), an example, and success behavior into
/// a single source of truth for the `submit`, `validate`, and `example` tools.
pub struct JsonSchemaToolkit<T>
where
    T: JsonSchema + Serialize + DeserializeOwned + Send + Sync + 'static,
{
    schema: Arc<Value>,
    example: String,
    on_submit: Option<SubmitCallback<T>>,
    success_message: String,
    submit_tool_name: String,
    submit_tool_description: String,
    validate_tool_name: String,
    validate_tool_description: String,
    example_tool_name: String,
    example_tool_description: String,
    _marker: PhantomData<T>,
}

impl<T> JsonSchemaToolkit<T>
where
    T: JsonSchema + Serialize + DeserializeOwned + Send + Sync + 'static,
{
    /// Returns a new builder for configuring the toolkit for the given type.
    #[must_use]
    pub fn builder() -> JsonSchemaToolkitBuilder<T> {
        JsonSchemaToolkitBuilder::default()
    }

    /// Consumes the toolkit and returns the trio of configured tools.
    #[must_use]
    pub fn build_tools(self) -> (SubmitTool<T>, ValidateJsonTool, JsonExampleTool) {
        let example = Arc::new(self.example);
        let success_message = Arc::new(self.success_message);

        (
            SubmitTool {
                name: self.submit_tool_name,
                description: self.submit_tool_description,
                schema: self.schema.clone(),
                on_submit: self.on_submit,
                success_message,
                _marker: PhantomData,
            },
            ValidateJsonTool {
                name: self.validate_tool_name,
                description: self.validate_tool_description,
                schema: self.schema.clone(),
            },
            JsonExampleTool {
                name: self.example_tool_name,
                description: self.example_tool_description,
                example,
            },
        )
    }
}

/// Builder for `JsonSchemaToolkit`.
pub struct JsonSchemaToolkitBuilder<T>
where
    T: JsonSchema + Serialize + DeserializeOwned + Send + Sync + 'static,
{
    example: Option<Value>,
    on_submit: Option<SubmitCallback<T>>,
    success_message: Option<String>,
    submit_tool_name: Option<String>,
    submit_tool_description: Option<String>,
    validate_tool_name: Option<String>,
    validate_tool_description: Option<String>,
    example_tool_name: Option<String>,
    example_tool_description: Option<String>,
    _marker: PhantomData<T>,
}

impl<T> Default for JsonSchemaToolkitBuilder<T>
where
    T: JsonSchema + Serialize + DeserializeOwned + Send + Sync + 'static,
{
    fn default() -> Self {
        Self {
            example: None,
            on_submit: None,
            success_message: None,
            submit_tool_name: None,
            submit_tool_description: None,
            validate_tool_name: None,
            validate_tool_description: None,
            example_tool_name: None,
            example_tool_description: None,
            _marker: PhantomData,
        }
    }
}

impl<T> JsonSchemaToolkitBuilder<T>
where
    T: JsonSchema + Serialize + DeserializeOwned + Send + Sync + 'static,
{
    /// Sets the example JSON structure.
    #[must_use]
    pub fn example(mut self, example: T) -> Self {
        self.example = Some(serde_json::to_value(example).unwrap_or_else(|_| json!({})));
        self
    }

    /// Sets the message returned upon successful validation and submission.
    ///
    /// Note: If `on_submit` is provided, its return value will be used instead.
    #[must_use]
    pub fn on_success(mut self, message: impl Into<String>) -> Self {
        self.success_message = Some(message.into());
        self
    }

    /// Sets a callback to be executed when the `submit` tool is called.
    /// The callback receives the deserialized data and returns a success message.
    #[must_use]
    pub fn on_submit<F>(mut self, callback: F) -> Self
    where
        F: Fn(T) -> String + Send + Sync + 'static,
    {
        self.on_submit = Some(Arc::new(callback));
        self
    }

    /// Customizes the submit tool name and description.
    #[must_use]
    pub fn customize_submit(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        self.submit_tool_name = Some(name.into());
        self.submit_tool_description = Some(description.into());
        self
    }

    /// Customizes the validate tool name and description.
    #[must_use]
    pub fn customize_validate(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        self.validate_tool_name = Some(name.into());
        self.validate_tool_description = Some(description.into());
        self
    }

    /// Customizes the example tool name and description.
    #[must_use]
    pub fn customize_example(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        self.example_tool_name = Some(name.into());
        self.example_tool_description = Some(description.into());
        self
    }

    /// Builds the toolkit.
    #[must_use]
    pub fn build(self) -> JsonSchemaToolkit<T> {
        JsonSchemaToolkit {
            schema: Arc::new(json!(schema_for!(T))),
            example: self
                .example
                .as_ref()
                .map_or_else(|| "{}".to_string(), std::string::ToString::to_string),
            on_submit: self.on_submit,
            success_message: self
                .success_message
                .unwrap_or_else(|| "Successfully submitted.".to_string()),
            submit_tool_name: self.submit_tool_name.unwrap_or_else(|| "submit".to_string()),
            submit_tool_description: self.submit_tool_description.unwrap_or_else(|| {
                "Submit the structured data. This will perform final validation and processing."
                    .to_string()
            }),
            validate_tool_name: self
                .validate_tool_name
                .unwrap_or_else(|| "validate_json".to_string()),
            validate_tool_description: self.validate_tool_description.unwrap_or_else(|| {
                "Validate JSON against the configured schema. Use this to check your format before submitting."
                    .to_string()
            }),
            example_tool_name: self
                .example_tool_name
                .unwrap_or_else(|| "json_example".to_string()),
            example_tool_description: self.example_tool_description.unwrap_or_else(|| {
                "Get an example of the expected JSON format.".to_string()
            }),
            _marker: PhantomData,
        }
    }
}

/// Tool for submitting work in JSON format with built-in validation.
///
/// This tool is generic over `T`, allowing for "Direct Struct Mapping"
/// where the LLM arguments are deserialized directly into your Rust struct.
pub struct SubmitTool<T>
where
    T: JsonSchema + Serialize + DeserializeOwned + Send + Sync + 'static,
{
    pub(crate) name: String,
    pub(crate) description: String,
    schema: Arc<Value>,
    on_submit: Option<SubmitCallback<T>>,
    success_message: Arc<String>,
    _marker: PhantomData<T>,
}

impl<T> Tool for SubmitTool<T>
where
    T: JsonSchema + Serialize + DeserializeOwned + Send + Sync + 'static,
{
    const NAME: &'static str = "submit";
    type Error = ToolError;
    type Args = T;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: (*self.schema).clone(),
        }
    }

    async fn call(&self, args: T) -> Result<String, ToolError> {
        // Since we are using T as Args, Rig already handles deserialization.
        // However, we still have the JSON schema for the MCP definition.

        self.on_submit.as_ref().map_or_else(
            || Ok(self.success_message.to_string()),
            |callback| Ok(callback(args)),
        )
    }
}

/// Arguments for the `ValidateJsonTool`.
#[derive(Deserialize, Serialize)]
pub struct ValidateJsonArgs {
    /// The JSON data to validate.
    pub json: Value,
}

/// Tool for validating JSON against a schema and providing feedback to the Agent.
pub struct ValidateJsonTool {
    pub(crate) name: String,
    pub(crate) description: String,
    schema: Arc<Value>,
}

impl Tool for ValidateJsonTool {
    const NAME: &'static str = "validate_json";
    type Error = ToolError;
    type Args = ValidateJsonArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "json": {
                        "type": "object",
                        "description": "The JSON data to validate"
                    }
                },
                "required": ["json"]
            }),
        }
    }

    async fn call(&self, args: ValidateJsonArgs) -> Result<String, ToolError> {
        let validator =
            Validator::new(&self.schema).map_err(|e| ToolError::Validation(e.to_string()))?;

        let errors: Vec<_> = validator.iter_errors(&args.json).collect();
        if errors.is_empty() {
            Ok("JSON is valid. You may now call the submit tool.".to_string())
        } else {
            let msgs: Vec<String> = errors.iter().map(ToString::to_string).collect();
            Ok(format!(
                "JSON is invalid. Please fix the following errors before submitting:\n{}",
                msgs.join("\n")
            ))
        }
    }
}

/// Arguments for the `JsonExampleTool`.
#[derive(Deserialize, Serialize)]
pub struct JsonExampleArgs {}

/// Tool for providing JSON examples derived from the toolkit.
pub struct JsonExampleTool {
    pub(crate) name: String,
    pub(crate) description: String,
    example: Arc<String>,
}

impl Tool for JsonExampleTool {
    const NAME: &'static str = "json_example";
    type Error = ToolError;
    type Args = JsonExampleArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        }
    }

    async fn call(&self, _args: JsonExampleArgs) -> Result<String, ToolError> {
        Ok(self.example.to_string())
    }
}
