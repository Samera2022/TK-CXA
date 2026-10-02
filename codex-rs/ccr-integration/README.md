# Codex CCR Integration

This crate provides integration between Codex and Claude Code Router (CCR) for model switching and orchestration.

## Features

- **Model Switching**: Allows agents to dynamically switch between models via CCR
- **Model Discovery**: Query available models from CCR
- **Session Management**: Track model preferences per session

## Usage

```rust
use codex_ccr_integration::{CcrModelSwitcher, CcrConfig};

// Create a model switcher
let config = CcrConfig {
    base_url: "http://localhost:8788".to_string(),
    api_key: Some("your-api-key".to_string()),
    model_switching_enabled: true,
    orchestra_enabled: false,
};

let switcher = CcrModelSwitcher::new(
    config,
    "codex".to_string(),
    Some("session-123".to_string())
);

// Get available models
let models = switcher.get_models().await?;

// Switch to a specific model
let response = switcher.switch_model(
    "openai/gpt-5.6-sol",
    "Need larger context window for codebase analysis"
).await?;
```

## Tool Integration

To integrate as a Codex tool, use the `tool_definition()` function:

```rust
use codex_ccr_integration::model_switching::tool_definition;

let tool_def = tool_definition();
// Register this tool definition with Codex's tool system
```

## TODO

- [ ] Integration with Codex's tool executor system
- [ ] Add budget tracking UI
- [ ] Add orchestra status display in TUI
- [ ] Integration tests with CCR
- [ ] Configuration management
- [ ] Error handling improvements
