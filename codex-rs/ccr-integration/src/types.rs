//! Types for CCR integration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(default)]
    pub provider: Option<String>,
    pub capabilities: Option<HashMap<String, serde_json::Value>>,
    pub limits: Option<ModelLimits>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLimits {
    #[serde(rename = "contextTokens")]
    pub context_tokens: Option<u64>,
    #[serde(rename = "inputTokens")]
    pub input_tokens: Option<u64>,
    #[serde(rename = "outputTokens")]
    pub output_tokens: Option<u64>,
    #[serde(rename = "maxTokens")]
    pub max_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchModelRequest {
    #[serde(rename = "sessionKey")]
    pub session_key: String,
    #[serde(rename = "targetModel")]
    pub target_model: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetState {
    pub model: String,
    pub budget: u64,
    pub used: u64,
    pub remaining: u64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SwitchModelResponse {
    Success {
        success: bool,
        #[serde(rename = "previousModel")]
        previous_model: Option<String>,
        #[serde(rename = "currentModel")]
        current_model: String,
        #[serde(rename = "budgetState")]
        budget_state: BudgetState,
    },
    Error {
        success: bool,
        error: String,
        #[serde(rename = "availableModels")]
        available_models: Vec<ModelInfo>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetQueryResponse {
    #[serde(rename = "sessionKey")]
    pub session_key: String,
    pub model: String,
    pub budget: u64,
    pub used: u64,
    pub remaining: u64,
    pub percentage: f64,
}

#[derive(Debug, Clone)]
pub struct CcrConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub model_switching_enabled: bool,
    pub orchestra_enabled: bool,
}

impl Default for CcrConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("CCR_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            api_key: std::env::var("CCR_API_KEY").ok(),
            model_switching_enabled: true,
            orchestra_enabled: false,
        }
    }
}
