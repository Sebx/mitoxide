//! Mitoxide Agent Binary
//!
//! The remote agent that executes operations on behalf of the client.

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, error};

mod agent;
mod handlers;
mod router;
mod bootstrap;

use agent::AgentLoop;
use handlers::{ProcessHandler, FileHandler, PtyHandler, PingHandler, WasmHandler};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting Mitoxide agent");

    // Create and run the agent loop
    let mut agent = AgentLoop::new();
    
    // Register handlers
    agent.register_handler("process_exec".to_string(), Arc::new(ProcessHandler)).await;
    agent.register_handler("file_get".to_string(), Arc::new(FileHandler)).await;
    agent.register_handler("file_put".to_string(), Arc::new(FileHandler)).await;
    agent.register_handler("dir_list".to_string(), Arc::new(FileHandler)).await;
    agent.register_handler("pty_exec".to_string(), Arc::new(PtyHandler)).await;
    agent.register_handler("ping".to_string(), Arc::new(PingHandler)).await;
    
    // Register WASM handler
    match WasmHandler::new() {
        Ok(wasm_handler) => {
            agent.register_handler("wasm_exec".to_string(), Arc::new(wasm_handler)).await;
            info!("WASM handler registered successfully");
        }
        Err(e) => {
            error!("Failed to create WASM handler: {}", e);
            // Continue without WASM support
        }
    }
    
    info!("All handlers registered, starting agent loop");
    
    if let Err(e) = agent.run().await {
        error!("Agent error: {}", e);
        std::process::exit(1);
    }

    info!("Agent shutting down");
    Ok(())
}