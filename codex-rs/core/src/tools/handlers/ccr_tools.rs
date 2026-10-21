//! CCR model switching tools for Codex

use crate::function_tool::FunctionCallError;
use crate::tools::context::{FunctionToolOutput, ToolInvocation, ToolOutput, ToolPayload};
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::CoreToolRuntime;
use crate::tools::registry::ToolExecutor;
use codex_protocol::models::FunctionCallOutputContentItem;
use codex_tools::{ResponsesApiTool, ToolExposure, ToolName, ToolSpec};
use serde::{Deserialize, Serialize};

const CCR_SWITCH_MODEL_DESC: &str = "Request CCR to switch the current model to a specific one. \
     If the model is not found, returns the list of available models. \
     Use this when you need capabilities of a different model \
     (e.g., larger context, better reasoning, vision support).";

const CCR_LIST_MODELS_DESC: &str = "List all available models from CCR (Claude Code Router). \
     Shows model IDs, providers, context sizes, and capabilities. \
     Use this to discover what models you can switch to.";

const CCR_BUDGET_CONFIRM_DESC: &str =
    "Respond to a CCR budget prompt with continue, switch, or stop.";
const CCR_GET_BUDGET_DESC: &str = "Read the current CCR model budget state for this Codex session.";
const CCR_SELECT_ROLE_DESC: &str = "Ask CCR Orchestra to select the configured role for a task.";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SwitchModelArgs {
    model: String,
    reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BudgetConfirmArgs {
    action: String,
    model: String,
    target_model: Option<String>,
    reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SelectRoleArgs {
    task_description: String,
}

/// Handler for ccr_switch_model tool
pub struct CcrSwitchModelHandler {
    ccr_url: String,
}

impl CcrSwitchModelHandler {
    pub fn new(ccr_url: impl Into<String>) -> Self {
        Self {
            ccr_url: ccr_url.into(),
        }
    }

    async fn handle_call(
        &self,
        invocation: ToolInvocation,
    ) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let ToolPayload::Function { arguments } = &invocation.payload else {
            return Err(FunctionCallError::RespondToModel(
                "Expected function payload".to_string(),
            ));
        };
        let args: SwitchModelArgs = parse_arguments(arguments)?;

        let client = reqwest::Client::new();
        let url = format!("{}/__ccr/model-control/v1/switch", self.ccr_url);

        let request_body = serde_json::json!({
            "sessionKey": format!("codex:{}", invocation.session.session_id()),
            "targetModel": args.model,
            "reason": args.reason,
        });

        let response = client
            .post(&url)
            .header("x-cc-switch-client", "codex")
            .header(
                "x-ccr-session-id",
                invocation.session.session_id().to_string(),
            )
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                FunctionCallError::RespondToModel(format!("Failed to contact CCR: {}", e))
            })?;

        let response_text = response.text().await.map_err(|e| {
            FunctionCallError::RespondToModel(format!("Failed to read CCR response: {}", e))
        })?;

        Ok(Box::new(FunctionToolOutput {
            body: vec![FunctionCallOutputContentItem::InputText {
                text: response_text,
            }],
            success: Some(true),
            post_tool_use_response: None,
        }))
    }
}

impl ToolExecutor<ToolInvocation> for CcrSwitchModelHandler {
    fn tool_name(&self) -> ToolName {
        ToolName::plain("ccr_switch_model")
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::Function(ResponsesApiTool {
            name: "ccr_switch_model".to_string(),
            description: CCR_SWITCH_MODEL_DESC.to_string(),
            strict: false,
            defer_loading: None,
            parameters: serde_json::from_value(serde_json::json!({
                "type": "object",
                "properties": {
                    "model": {
                        "type": "string",
                        "description": "The model ID to switch to (e.g., 'openai/gpt-5.6-sol', 'anthropic/claude-opus-5')"
                    },
                    "reason": {
                        "type": "string",
                        "description": "Explanation of why this model switch is needed"
                    }
                },
                "required": ["model", "reason"]
            })).unwrap(),
            output_schema: None,
        })
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::Direct
    }

    fn handle<'a>(&'a self, invocation: ToolInvocation) -> codex_tools::ToolExecutorFuture<'a>
    where
        ToolInvocation: 'a,
    {
        Box::pin(self.handle_call(invocation))
    }
}

impl CoreToolRuntime for CcrSwitchModelHandler {}

/// Handler for ccr_list_models tool
pub struct CcrListModelsHandler {
    ccr_url: String,
}

impl CcrListModelsHandler {
    pub fn new(ccr_url: impl Into<String>) -> Self {
        Self {
            ccr_url: ccr_url.into(),
        }
    }

    async fn handle_call(
        &self,
        _invocation: ToolInvocation,
    ) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let client = reqwest::Client::new();
        let url = format!("{}/v1/ccr/models", self.ccr_url);

        let response = client
            .get(&url)
            .header("x-cc-switch-client", "codex")
            .header(
                "x-ccr-session-id",
                _invocation.session.session_id().to_string(),
            )
            .send()
            .await
            .map_err(|e| {
                FunctionCallError::RespondToModel(format!("Failed to contact CCR: {}", e))
            })?;

        let response_text = response.text().await.map_err(|e| {
            FunctionCallError::RespondToModel(format!("Failed to read CCR response: {}", e))
        })?;

        // Parse the JSON to format it nicely
        let models: serde_json::Value = serde_json::from_str(&response_text).map_err(|e| {
            FunctionCallError::RespondToModel(format!("Failed to parse CCR response: {}", e))
        })?;

        let models_array = models["models"].as_array().ok_or_else(|| {
            FunctionCallError::RespondToModel("Invalid response format".to_string())
        })?;

        if models_array.is_empty() {
            return Ok(Box::new(FunctionToolOutput {
                body: vec![FunctionCallOutputContentItem::InputText {
                    text: "No models available from CCR.".to_string(),
                }],
                success: Some(true),
                post_tool_use_response: None,
            }));
        }

        let mut output = String::from("Available models from CCR:\n\n");
        for model in models_array {
            let id = model["id"].as_str().unwrap_or("unknown");
            let name = model["displayName"].as_str().unwrap_or(id);
            let provider = model["provider"].as_str().unwrap_or("unknown");

            output.push_str(&format!("**{}** ({})\n", name, id));
            output.push_str(&format!("  Provider: {}\n", provider));

            if let Some(ctx) = model["contextTokens"].as_u64() {
                output.push_str(&format!("  Context: {} tokens\n", ctx));
            }

            if let Some(caps) = model["capabilities"].as_array() {
                if !caps.is_empty() {
                    let caps_str: Vec<String> = caps
                        .iter()
                        .filter_map(|c| c.as_str().map(String::from))
                        .collect();
                    output.push_str(&format!("  Capabilities: {}\n", caps_str.join(", ")));
                }
            }
            output.push('\n');
        }

        Ok(Box::new(FunctionToolOutput {
            body: vec![FunctionCallOutputContentItem::InputText { text: output }],
            success: Some(true),
            post_tool_use_response: None,
        }))
    }
}

impl ToolExecutor<ToolInvocation> for CcrListModelsHandler {
    fn tool_name(&self) -> ToolName {
        ToolName::plain("ccr_list_models")
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::Function(ResponsesApiTool {
            name: "ccr_list_models".to_string(),
            description: CCR_LIST_MODELS_DESC.to_string(),
            strict: false,
            defer_loading: None,
            parameters: serde_json::from_value(serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }))
            .unwrap(),
            output_schema: None,
        })
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::Direct
    }

    fn handle<'a>(&'a self, invocation: ToolInvocation) -> codex_tools::ToolExecutorFuture<'a>
    where
        ToolInvocation: 'a,
    {
        Box::pin(self.handle_call(invocation))
    }
}

impl CoreToolRuntime for CcrListModelsHandler {}

pub struct CcrGetBudgetHandler {
    ccr_url: String,
}

impl CcrGetBudgetHandler {
    pub fn new(ccr_url: impl Into<String>) -> Self {
        Self {
            ccr_url: ccr_url.into(),
        }
    }

    async fn handle_call(
        &self,
        invocation: ToolInvocation,
    ) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let session_key = format!("codex:{}", invocation.session.session_id());
        let url = format!(
            "{}/__ccr/model-control/v1/budget/{}",
            self.ccr_url, session_key
        );
        let response = reqwest::Client::new()
            .get(url)
            .header("x-cc-switch-client", "codex")
            .send()
            .await
            .map_err(|error| {
                FunctionCallError::RespondToModel(format!("Failed to contact CCR: {error}"))
            })?;
        let text = response.text().await.map_err(|error| {
            FunctionCallError::RespondToModel(format!("Failed to read CCR response: {error}"))
        })?;
        Ok(Box::new(FunctionToolOutput {
            body: vec![FunctionCallOutputContentItem::InputText { text }],
            success: Some(true),
            post_tool_use_response: None,
        }))
    }
}

impl ToolExecutor<ToolInvocation> for CcrGetBudgetHandler {
    fn tool_name(&self) -> ToolName {
        ToolName::plain("ccr_get_budget")
    }
    fn spec(&self) -> ToolSpec {
        ToolSpec::Function(ResponsesApiTool {
            name: "ccr_get_budget".to_string(),
            description: CCR_GET_BUDGET_DESC.to_string(),
            strict: false,
            defer_loading: None,
            parameters: serde_json::from_value(
                serde_json::json!({"type":"object","properties":{},"required":[]}),
            )
            .unwrap(),
            output_schema: None,
        })
    }
    fn exposure(&self) -> ToolExposure {
        ToolExposure::Direct
    }
    fn handle<'a>(&'a self, invocation: ToolInvocation) -> codex_tools::ToolExecutorFuture<'a>
    where
        ToolInvocation: 'a,
    {
        Box::pin(self.handle_call(invocation))
    }
}
impl CoreToolRuntime for CcrGetBudgetHandler {}

pub struct CcrBudgetConfirmHandler {
    ccr_url: String,
}
impl CcrBudgetConfirmHandler {
    pub fn new(ccr_url: impl Into<String>) -> Self {
        Self {
            ccr_url: ccr_url.into(),
        }
    }
    async fn handle_call(
        &self,
        invocation: ToolInvocation,
    ) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let ToolPayload::Function { arguments } = &invocation.payload else {
            return Err(FunctionCallError::RespondToModel(
                "Expected function payload".to_string(),
            ));
        };
        let args: BudgetConfirmArgs = parse_arguments(arguments)?;
        let session_key = format!("codex:{}", invocation.session.session_id());
        let response = reqwest::Client::new()
            .post(format!("{}/__ccr/model-control/v1/budget-confirm", self.ccr_url))
            .header("x-cc-switch-client", "codex")
            .json(&serde_json::json!({"action":args.action,"model":args.model,"sessionKey":session_key,"targetModel":args.target_model,"reason":args.reason}))
            .send().await
            .map_err(|error| FunctionCallError::RespondToModel(format!("Failed to contact CCR: {error}")))?;
        let text = response.text().await.map_err(|error| {
            FunctionCallError::RespondToModel(format!("Failed to read CCR response: {error}"))
        })?;
        Ok(Box::new(FunctionToolOutput {
            body: vec![FunctionCallOutputContentItem::InputText { text }],
            success: Some(true),
            post_tool_use_response: None,
        }))
    }
}
impl ToolExecutor<ToolInvocation> for CcrBudgetConfirmHandler {
    fn tool_name(&self) -> ToolName {
        ToolName::plain("ccr_budget_confirm")
    }
    fn spec(&self) -> ToolSpec {
        ToolSpec::Function(ResponsesApiTool {
            name: "ccr_budget_confirm".to_string(), description: CCR_BUDGET_CONFIRM_DESC.to_string(), strict: false,
            defer_loading: None,
            parameters: serde_json::from_value(serde_json::json!({"type":"object","properties":{"action":{"type":"string","enum":["continue","switch","stop"]},"model":{"type":"string"},"target_model":{"type":"string"},"reason":{"type":"string"}},"required":["action","model"]})).unwrap(), output_schema: None,
        })
    }
    fn exposure(&self) -> ToolExposure {
        ToolExposure::Direct
    }
    fn handle<'a>(&'a self, invocation: ToolInvocation) -> codex_tools::ToolExecutorFuture<'a>
    where
        ToolInvocation: 'a,
    {
        Box::pin(self.handle_call(invocation))
    }
}
impl CoreToolRuntime for CcrBudgetConfirmHandler {}

pub struct CcrSelectRoleHandler {
    ccr_url: String,
}

impl CcrSelectRoleHandler {
    pub fn new(ccr_url: impl Into<String>) -> Self {
        Self {
            ccr_url: ccr_url.into(),
        }
    }

    async fn handle_call(
        &self,
        invocation: ToolInvocation,
    ) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let ToolPayload::Function { arguments } = &invocation.payload else {
            return Err(FunctionCallError::RespondToModel(
                "Expected function payload".to_string(),
            ));
        };
        let args: SelectRoleArgs = parse_arguments(arguments)?;
        let response = reqwest::Client::new()
            .post(format!("{}/__ccr/orchestra/v1/select-role", self.ccr_url))
            .header("x-cc-switch-client", "codex")
            .header(
                "x-ccr-session-id",
                invocation.session.session_id().to_string(),
            )
            .json(&serde_json::json!({
                "sessionKey": format!("codex:{}", invocation.session.session_id()),
                "taskDescription": args.task_description,
            }))
            .send()
            .await
            .map_err(|error| {
                FunctionCallError::RespondToModel(format!("Failed to contact CCR: {error}"))
            })?;
        let text = response.text().await.map_err(|error| {
            FunctionCallError::RespondToModel(format!("Failed to read CCR response: {error}"))
        })?;
        Ok(Box::new(FunctionToolOutput {
            body: vec![FunctionCallOutputContentItem::InputText { text }],
            success: Some(true),
            post_tool_use_response: None,
        }))
    }
}

impl ToolExecutor<ToolInvocation> for CcrSelectRoleHandler {
    fn tool_name(&self) -> ToolName {
        ToolName::plain("ccr_select_role")
    }
    fn spec(&self) -> ToolSpec {
        ToolSpec::Function(ResponsesApiTool {
            name: "ccr_select_role".to_string(),
            description: CCR_SELECT_ROLE_DESC.to_string(),
            strict: false,
            defer_loading: None,
            parameters: serde_json::from_value(serde_json::json!({
                "type": "object",
                "properties": { "task_description": { "type": "string" } },
                "required": ["task_description"]
            }))
            .unwrap(),
            output_schema: None,
        })
    }
    fn exposure(&self) -> ToolExposure {
        ToolExposure::Direct
    }
    fn handle<'a>(&'a self, invocation: ToolInvocation) -> codex_tools::ToolExecutorFuture<'a>
    where
        ToolInvocation: 'a,
    {
        Box::pin(self.handle_call(invocation))
    }
}
impl CoreToolRuntime for CcrSelectRoleHandler {}
