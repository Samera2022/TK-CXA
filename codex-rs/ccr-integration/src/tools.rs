//! CCR tool executors for Codex

use crate::model_switching::CcrModelSwitcher;
use crate::types::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Arguments for switching to a specific model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchModelArgs {
    pub model: String,
    pub reason: String,
}

/// Tool handler for CCR model switching
pub struct CcrSwitchModelTool {
    switcher: Arc<CcrModelSwitcher>,
}

impl CcrSwitchModelTool {
    pub fn new(switcher: Arc<CcrModelSwitcher>) -> Self {
        Self { switcher }
    }

    pub async fn execute(&self, args: SwitchModelArgs) -> Result<String> {
        let response = self.switcher.switch_model(&args.model, &args.reason).await?;

        match response {
            SwitchModelResponse::Success {
                model,
                previous_model,
                ..
            } => {
                let prev = previous_model
                    .map(|m| format!(" (was: {})", m))
                    .unwrap_or_default();
                Ok(format!("Successfully switched to model: {}{}", model, prev))
            }
            SwitchModelResponse::Error {
                message,
                available_models,
                ..
            } => {
                let models_list = available_models
                    .iter()
                    .map(|m| format!("  - {} ({})", m.id, m.display_name))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(format!(
                    "Model switch failed: {}\n\nAvailable models:\n{}",
                    message, models_list
                ))
            }
        }
    }

    pub fn tool_definition() -> serde_json::Value {
        serde_json::json!({
            "name": "ccr_switch_model",
            "description": "Request CCR to switch the current model to a specific one. \
                           If the model is not found, returns the list of available models. \
                           Use this when you need capabilities of a different model \
                           (e.g., larger context, better reasoning, vision support).",
            "input_schema": {
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
            }
        })
    }
}

/// Tool handler for listing available CCR models
pub struct CcrListModelsTool {
    switcher: Arc<CcrModelSwitcher>,
}

impl CcrListModelsTool {
    pub fn new(switcher: Arc<CcrModelSwitcher>) -> Self {
        Self { switcher }
    }

    pub async fn execute(&self) -> Result<String> {
        let models = self.switcher.get_models().await?;

        if models.is_empty() {
            return Ok("No models available from CCR.".to_string());
        }

        let mut output = String::from("Available models from CCR:\n\n");
        for model in models {
            output.push_str(&format!("**{}** ({})\n", model.display_name, model.id));
            output.push_str(&format!("  Provider: {}\n", model.provider));
            if let Some(ctx) = model.context_tokens {
                output.push_str(&format!("  Context: {} tokens\n", ctx));
            }
            if let Some(caps) = &model.capabilities {
                if !caps.is_empty() {
                    output.push_str(&format!("  Capabilities: {}\n", caps.join(", ")));
                }
            }
            output.push('\n');
        }

        Ok(output)
    }

    pub fn tool_definition() -> serde_json::Value {
        serde_json::json!({
            "name": "ccr_list_models",
            "description": "List all available models from CCR (Claude Code Router). \
                           Shows model IDs, providers, context sizes, and capabilities. \
                           Use this to discover what models you can switch to.",
            "input_schema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_switch_model_tool_definition() {
        let def = CcrSwitchModelTool::tool_definition();
        assert_eq!(def["name"], "ccr_switch_model");
        assert!(def["description"].as_str().is_some());
        assert!(def["input_schema"]["properties"]["model"].is_object());
        assert!(def["input_schema"]["properties"]["reason"].is_object());
    }

    #[test]
    fn test_list_models_tool_definition() {
        let def = CcrListModelsTool::tool_definition();
        assert_eq!(def["name"], "ccr_list_models");
        assert!(def["description"].as_str().is_some());
    }
}
