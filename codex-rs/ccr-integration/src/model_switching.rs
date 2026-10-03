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

    /// Get session key for CCR requests
    fn session_key(&self) -> String {
        format!(
            "{}:{}",
            self.client_id,
            self.session_id.as_deref().unwrap_or("default")
        )
    }

    /// Get all available models from CCR
    pub async fn get_models(&self) -> Result<Vec<ModelInfo>> {
        let url = format!("{}/__ccr/model-control/v1/models", self.config.base_url);

        let mut request = self.client.get(&url);

        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .send()
            .await
            .context("Failed to send request to CCR")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "CCR returned error status {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            );
        }

        let models_response: ModelsResponse = response
            .json()
            .await
            .context("Failed to parse models response")?;

        Ok(models_response.models)
    }

    /// Switch to a specific model
    pub async fn switch_model(&self, model: &str, reason: &str) -> Result<SwitchModelResponse> {
        let url = format!("{}/__ccr/model-control/v1/switch", self.config.base_url);

        let request_body = SwitchModelRequest {
            session_key: self.session_key(),
            target_model: model.to_string(),
            reason: reason.to_string(),
        };

        let mut request = self.client.post(&url).json(&request_body);

        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
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

    /// Query budget state for the current session
    pub async fn query_budget(&self) -> Result<BudgetQueryResponse> {
        let session_key = self.session_key();
        let url = format!(
            "{}/__ccr/model-control/v1/budget/{}",
            self.config.base_url, session_key
        );

        let mut request = self.client.get(&url);

        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .send()
            .await
            .context("Failed to send budget query request")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "CCR returned error status {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            );
        }

        let budget_response: BudgetQueryResponse = response
            .json()
            .await
            .context("Failed to parse budget response")?;

        Ok(budget_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_key_generation() {
        let config = CcrConfig::default();
        let switcher = CcrModelSwitcher::new(config, "codex".to_string(), Some("test-session".to_string()));
        assert_eq!(switcher.session_key(), "codex:test-session");

        let config2 = CcrConfig::default();
        let switcher2 = CcrModelSwitcher::new(config2, "codex".to_string(), None);
        assert_eq!(switcher2.session_key(), "codex:default");
    }
}
