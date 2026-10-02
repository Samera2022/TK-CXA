//! Types for CCR integration

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableModel {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub provider: String,
    #[serde(rename = "contextTokens")]
    pub context_tokens: Option<u64>,
    pub capabilities: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    pub models: Vec<AvailableModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchModelRequest {
    pub model: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SwitchModelResponse {
    Success {
        success: bool,
        model: String,
        #[serde(rename = "previousModel")]
        previous_model: Option<String>,
    },
    Error {
        success: bool,
        error: String,
        message: String,
        #[serde(rename = "availableModels")]
        available_models: Vec<AvailableModel>,
    },
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
            base_url: "http://localhost:8788".to_string(),
            api_key: None,
            model_switching_enabled: true,
            orchestra_enabled: false,
        }
    }
}
