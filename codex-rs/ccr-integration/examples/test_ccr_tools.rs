//! Test CCR tools manually

use codex_ccr_integration::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = CcrConfig::default();

    println!("CCR Config:");
    println!("  Base URL: {}", config.base_url);
    println!(
        "  API Key: {}",
        config.api_key.as_deref().unwrap_or("<not set>")
    );
    println!();

    let switcher = Arc::new(CcrModelSwitcher::new(
        config,
        "codex".to_string(),
        Some("test-session".to_string()),
    ));

    // Test list models
    println!("=== Testing list_models ===");
    let list_tool = CcrListModelsTool::new(switcher.clone());
    match list_tool.execute().await {
        Ok(output) => println!("{}", output),
        Err(e) => println!("Error: {}", e),
    }
    println!();

    // Test switch model
    println!("=== Testing switch_model ===");
    let switch_tool = CcrSwitchModelTool::new(switcher.clone());
    let args = SwitchModelArgs {
        model: "gpt-5.6-sol".to_string(),
        reason: "Testing model switch".to_string(),
    };
    match switch_tool.execute(args).await {
        Ok(output) => println!("{}", output),
        Err(e) => println!("Error: {}", e),
    }
    println!();

    // Test query budget
    println!("=== Testing query_budget ===");
    let budget_tool = CcrQueryBudgetTool::new(switcher.clone());
    match budget_tool.execute().await {
        Ok(output) => println!("{}", output),
        Err(e) => println!("Error: {}", e),
    }

    Ok(())
}
