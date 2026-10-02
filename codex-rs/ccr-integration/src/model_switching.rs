//! Model switching client for CCR

use crate::types::*;
use anyhow::{Context, Result};
use reqwest::Client;

pub struct CcrModelSwitcher {
    config: CcrConfig,
    client: Client,
    client_id: String,
    session_id: Option<String>,
}

impl CcrModelSwitcher {
    pub fn new(config: CcrConfig, client_id: String, session_id: Option<String>) -> Self {
        Self {
            config,
            client: Client::new(),
            client_id,
            session_id,
        }
    }

    /// Get all available models from CCR
    pub async fn get_models(&self) -> Result<Vec<AvailableModel>> {
        let url = format!("{}/v1/ccr/models", self.config.base_url);
        
        let mut request = self.client.get(&url);
        
        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }
        
        let response = request
            .send()
            .await
            .context("Failed to send request to CCR")?;
        
        let models_response: ModelsResponse = response
            .json()
            .await
            .context("Failed to parse models response")?;
        
        Ok(models_response.models)
    }

    /// Switch to a specific model
    pub async fn switch_model(&self, model: &str, reason: &str) -> Result<SwitchModelResponse> {
        let url = format!("{}/v1/ccr/switch-model", self.config.base_url);
        
        let request_body = SwitchModelRequest {
            model: model.to_string(),
            reason: reason.to_string(),
        };
        
        let mut request = self.client.post(&url).json(&request_body);
        
        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }
        
        request = request.header("x-cc-switch-client", &self.client_id);
        
        if let Some(session_id) = &self.session_id {
            request = request.header("x-ccr-session-id", session_id);
        }
        
        let response = request
            .send()
            .await
            .context("Failed to send switch model request")?;
        
        let switch_response: SwitchModelResponse = response
            .json()
            .await
            .context("Failed to parse switch model response")?;
        
        Ok(switch_response)
    }
}

/// Tool definition for ccr_switch_model
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definition() {
        let def = tool_definition();
        assert_eq!(def["name"], "ccr_switch_model");
        assert!(def["description"].as_str().is_some());
        assert!(def["input_schema"]["properties"]["model"].is_object());
        assert!(def["input_schema"]["properties"]["reason"].is_object());
    }
}
