//! CCR Integration for Codex
//! 
//! Provides integration with Claude Code Router for model switching and orchestration.

pub mod model_switching;
pub mod tools;
pub mod types;

pub use model_switching::CcrModelSwitcher;
pub use tools::{CcrListModelsTool, CcrSwitchModelTool, SwitchModelArgs};
pub use types::*;
