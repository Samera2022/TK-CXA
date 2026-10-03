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
                current_model,
                previous_model,
                budget_state,
                ..
            } => {
                let prev = previous_model
                    .map(|m| format!(" (was: {})", m))
                    .unwrap_or_default();
                Ok(format!(
                    "Successfully switched to model: {}{}\n\nBudget state:\n  Model: {}\n  Budget: {} tokens\n  Used: {} tokens ({:.1}%)\n  Remaining: {} tokens",
                    current_model, prev,
                    budget_state.model,
                    budget_state.budget,
                    budget_state.used,
                    budget_state.percentage,
                    budget_state.remaining
                ))
            }
            SwitchModelResponse::Error {
                error,
                available_models,
                ..
            } => {
                let models_list = available_models
                    .iter()
                    .map(|m| {
                        let provider = m.provider.as_deref().unwrap_or("unknown");
                        let ctx = m
                            .limits
                            .as_ref()
                            .and_then(|l| l.context_tokens)
                            .map(|c| format!(", {} tokens", c))
                            .unwrap_or_default();
                        format!("  - {} ({}: {}{})", m.display_name, m.id, provider, ctx)
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(format!(
                    "Model switch failed: {}\n\nAvailable models:\n{}",
                    error, models_list
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
                           (e.g., larger context, better reasoning, vision support). \
                           IMPORTANT: Model IDs should be simple names like 'gpt-5.6-sol' or 'claude-opus-5', \
                           NOT prefixed with provider (do not use 'openai/' or 'anthropic/' prefixes).",
            "input_schema": {
                "type": "object",
                "properties": {
                    "model": {
                        "type": "string",
                        "description": "The model ID to switch to (e.g., 'gpt-5.6-sol', 'claude-opus-5', 'deepseek-v4')"
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
            let provider = model.provider.as_deref().unwrap_or("unknown");
            output.push_str(&format!("**{}** ({})\n", model.display_name, model.id));
            output.push_str(&format!("  Provider: {}\n", provider));
            
            if let Some(limits) = &model.limits {
                if let Some(ctx) = limits.context_tokens {
                    output.push_str(&format!("  Context: {} tokens\n", ctx));
                }
                if let Some(max_out) = limits.max_tokens {
                    output.push_str(&format!("  Max output: {} tokens\n", max_out));
                }
            }
            
            if let Some(caps) = &model.capabilities {
                if !caps.is_empty() {
                    let cap_names: Vec<String> = caps
                        .iter()
                        .filter_map(|(k, v)| {
                            if v.as_bool() == Some(true) {
                                Some(k.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    if !cap_names.is_empty() {
                        output.push_str(&format!("  Capabilities: {}\n", cap_names.join(", ")));
                    }
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

/// Tool handler for querying budget state
pub struct CcrQueryBudgetTool {
    switcher: Arc<CcrModelSwitcher>,
}

impl CcrQueryBudgetTool {
    pub fn new(switcher: Arc<CcrModelSwitcher>) -> Self {
        Self { switcher }
    }

    pub async fn execute(&self) -> Result<String> {
        let budget = self.switcher.query_budget().await?;

        Ok(format!(
            "Budget state for session '{}':\n\n\
             Model: {}\n\
             Budget: {} tokens\n\
             Used: {} tokens ({:.1}%)\n\
             Remaining: {} tokens\n\n\
             {}",
            budget.session_key,
            budget.model,
            budget.budget,
            budget.used,
            budget.percentage,
            budget.remaining,
            if budget.percentage > 80.0 {
                "⚠️  Warning: Budget usage is high!"
            } else {
                "✓ Budget is within normal range"
            }
        ))
    }

    pub fn tool_definition() -> serde_json::Value {
        serde_json::json!({
            "name": "ccr_query_budget",
            "description": "Query the current token budget state for this session. \
                           Shows how many tokens have been used, remaining budget, and usage percentage. \
                           Use this to check if you're approaching budget limits.",
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

    #[test]
    fn test_query_budget_tool_definition() {
        let def = CcrQueryBudgetTool::tool_definition();
        assert_eq!(def["name"], "ccr_query_budget");
        assert!(def["description"].as_str().is_some());
    }
}

/// Arguments for budget confirmation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfirmArgs {
    pub action: String,    // "continue", "switch", or "stop"
    pub model: String,
    pub target_model: Option<String>,
}

/// Tool handler for CCR budget confirmation
pub struct CcrBudgetConfirmTool {
    switcher: Arc<CcrModelSwitcher>,
}

impl CcrBudgetConfirmTool {
    pub fn new(switcher: Arc<CcrModelSwitcher>) -> Self {
        Self { switcher }
    }

    pub async fn execute(&self, args: BudgetConfirmArgs) -> Result<String> {
        let target = args.target_model.as_deref();
        let response = self
            .switcher
            .confirm_budget(&args.action, &args.model, target)
            .await?;

        if response.stopped.unwrap_or(false) {
            Ok(format!(
                "⛔ Task execution stopped.\n\n{}",
                response.message
            ))
        } else if let Some(new_model) = response.new_model {
            Ok(format!(
                "✓ Model switched successfully.\n\n\
                 Previous model: {}\n\
                 New model: {}\n\n\
                 {}",
                args.model, new_model, response.message
            ))
        } else {
            Ok(format!("✓ Continuing with current model.\n\n{}", response.message))
        }
    }

    pub fn tool_definition() -> serde_json::Value {
        serde_json::json!({
            "name": "ccr_budget_confirm",
            "description": "Respond to a budget limit warning from CCR. \
                           When CCR indicates that token budget is running low, use this tool to decide whether to: \
                           (1) continue with the current model, (2) switch to a more cost-effective model, or (3) stop the task. \
                           This is typically called automatically when CCR sends a budget warning.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["continue", "switch", "stop"],
                        "description": "'continue' to keep using the current model, 'switch' to change models, 'stop' to halt execution"
                    },
                    "model": {
                        "type": "string",
                        "description": "The current model that triggered the budget warning"
                    },
                    "target_model": {
                        "type": "string",
                        "description": "Required if action is 'switch'. The model ID to switch to (e.g., 'gpt-5.6-sol', 'deepseek-v4-flash')"
                    }
                },
                "required": ["action", "model"]
            }
        })
    }
}
