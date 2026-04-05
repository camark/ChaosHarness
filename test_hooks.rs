// Test script for hooks functionality
use rust_harness::hooks::events::HookEvent;
use rust_harness::hooks::executor::HookExecutor;
use rust_harness::hooks::registry::HookRegistry;
use rust_harness::hooks::schemas::HookDefinition;
use rust_harness::hooks::types::HookContext;
use serde_json::json;

#[tokio::main]
async fn main() {
    println!("Testing hooks functionality...");
    
    // Create registry and add a test hook
    let registry = HookRegistry::new();
    
    // Add a simple logging hook
    let hook = HookDefinition::new(
        "test_logger".to_string(),
        "echo 'Hook triggered: %HOOK_NAME%'".to_string(),
        "post_tool_use".to_string(),
    );
    
    registry.register_blocking(hook);
    
    // Create executor
    let executor = HookExecutor::new(registry);
    
    // Test post_tool_use event
    let context = HookContext::post_tool_use("bash", &json!({"command": "ls"}), "output", false);
    let result = executor.execute(&HookEvent::PostToolUse, context).await;
    
    println!("Hook result: {:?}", result);
    println!("Test complete!");
}
