//! CCR model switching tools for Codex

use crate::function_tool::FunctionCallError;
use crate::tools::context::{ToolInvocation, ToolOutput, ToolPayload, boxed_tool_output};
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::CoreToolRuntime;
use crate::tools::registry::ToolExecutor;
use codex_tools::{ToolName, ToolSpec, ToolExposure};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const CCR_SWITCH_MODEL_DESC: &str = 
    "Request CCR to switch the current model to a specific one. \
     If the model is not found, returns the list of available models. \
     Use this when you need capabilities of a different model \
     (e.g., larger context, better reasoning, vision support).";

const CCR_LIST_MODELS_DESC: &str =
    "List all available models from CCR (Claude Code Router). \
     Shows model IDs, providers, context sizes, and capabilities. \
     Use this to discover what models you can switch to.";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SwitchModelArgs {
    model: String,
    reason: String,
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

    async fn handle_call(&self, invocation: ToolInvocation) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let args: SwitchModelArgs = parse_arguments(&invocation.payload)?;

        // Simple HTTP client call to CCR
        let client = reqwest::Client::new();
        let url = format!("{}/v1/ccr/switch-model", self.ccr_url);
        
        let request_body = serde_json::json!({
            "model": args.model,
            "reason": args.reason,
        });

        let response = client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| FunctionCallError::RespondToModel(format!("Failed to contact CCR: {}", e)))?;

        let response_text = response
            .text()
            .await
            .map_err(|e| FunctionCallError::RespondToModel(format!("Failed to read CCR response: {}", e)))?;

        Ok(boxed_tool_output(response_text))
    }
}

impl ToolExecutor<ToolInvocation> for CcrSwitchModelHandler {
    fn tool_name(&self) -> ToolName {
        ToolName::plain("ccr_switch_model")
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::function(
            "ccr_switch_model",
            CCR_SWITCH_MODEL_DESC,
            serde_json::json!({
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
            }),
        )
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

    async fn handle_call(&self, _invocation: ToolInvocation) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let client = reqwest::Client::new();
        let url = format!("{}/v1/ccr/models", self.ccr_url);
        
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| FunctionCallError::RespondToModel(format!("Failed to contact CCR: {}", e)))?;

        let response_text = response
            .text()
            .await
            .map_err(|e| FunctionCallError::RespondToModel(format!("Failed to read CCR response: {}", e)))?;

        // Parse the JSON to format it nicely
        let models: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| FunctionCallError::RespondToModel(format!("Failed to parse CCR response: {}", e)))?;

        let models_array = models["models"].as_array()
            .ok_or_else(|| FunctionCallError::RespondToModel("Invalid response format".to_string()))?;

        if models_array.is_empty() {
            return Ok(boxed_tool_output("No models available from CCR."));
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
                    let caps_str: Vec<String> = caps.iter()
                        .filter_map(|c| c.as_str().map(String::from))
                        .collect();
                    output.push_str(&format!("  Capabilities: {}\n", caps_str.join(", ")));
                }
            }
            output.push('\n');
        }

        Ok(boxed_tool_output(output))
    }
}

impl ToolExecutor<ToolInvocation> for CcrListModelsHandler {
    fn tool_name(&self) -> ToolName {
        ToolName::plain("ccr_list_models")
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::function(
            "ccr_list_models",
            CCR_LIST_MODELS_DESC,
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        )
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
