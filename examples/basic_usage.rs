//! Basic usage example for Mitoxide
//! 
//! This example demonstrates the core functionality of Mitoxide including:
//! - Establishing SSH connections
//! - Executing remote commands
//! - File operations
//! - Error handling

use mitoxide::Session;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing for better debugging
    tracing_subscriber::init();

    println!("🚀 Mitoxide Basic Usage Example");
    println!("================================");

    // Create a session to a remote host
    // Replace with your actual SSH details
    let session = Session::ssh("user@remote-host")
        .timeout(std::time::Duration::from_secs(30))
        .await?;

    println!("📡 Connecting to remote host...");
    let context = session.connect().await?;
    println!("✅ Connected successfully!");

    // Execute a simple command
    println!("\n🔧 Executing remote command...");
    let output = context.proc_exec(&["uname", "-a"]).await?;
    
    if output.exit_code == 0 {
        println!("✅ Command executed successfully:");
        println!("   Output: {}", output.stdout.trim());
    } else {
        println!("❌ Command failed with exit code: {}", output.exit_code);
        println!("   Error: {}", output.stderr);
    }

    // File operations
    println!("\n📁 Testing file operations...");
    
    // Write a file
    let test_content = b"Hello from Mitoxide! 🦀";
    context.file_write("/tmp/mitoxide_test.txt", test_content).await?;
    println!("✅ File written to /tmp/mitoxide_test.txt");
    
    // Read the file back
    let read_content = context.file_read("/tmp/mitoxide_test.txt").await?;
    println!("✅ File read successfully:");
    println!("   Content: {}", String::from_utf8_lossy(&read_content));
    
    // Verify content matches
    if read_content == test_content {
        println!("✅ File content verification passed!");
    } else {
        println!("❌ File content verification failed!");
    }

    // Execute multiple commands concurrently
    println!("\n⚡ Testing concurrent execution...");
    
    let commands = vec![
        vec!["whoami"],
        vec!["pwd"],
        vec!["date"],
        vec!["uptime"],
    ];
    
    let mut handles = Vec::new();
    
    for command in commands {
        let ctx = context.clone();
        let cmd = command.clone();
        let handle = tokio::spawn(async move {
            let result = ctx.proc_exec(&cmd).await;
            (cmd, result)
        });
        handles.push(handle);
    }
    
    // Wait for all commands to complete
    for handle in handles {
        let (command, result) = handle.await?;
        match result {
            Ok(output) => {
                println!("✅ Command {:?}: {}", command, output.stdout.trim());
            }
            Err(e) => {
                println!("❌ Command {:?} failed: {}", command, e);
            }
        }
    }

    // Cleanup
    println!("\n🧹 Cleaning up...");
    let _ = context.proc_exec(&["rm", "/tmp/mitoxide_test.txt"]).await;
    println!("✅ Cleanup completed");

    println!("\n🎉 Example completed successfully!");
    
    Ok(())
}

// Example with error handling
async fn example_with_error_handling() -> Result<(), Box<dyn Error>> {
    let session = Session::ssh("user@remote-host").await?;
    let context = session.connect().await?;
    
    // This command might fail
    match context.proc_exec(&["nonexistent-command"]).await {
        Ok(output) => {
            println!("Command succeeded: {}", output.stdout);
        }
        Err(e) => {
            println!("Command failed as expected: {}", e);
            // Handle the error gracefully
        }
    }
    
    Ok(())
}

// Example with jump host
async fn example_with_jump_host() -> Result<(), Box<dyn Error>> {
    let session = Session::ssh("user@target-host")
        .jump_host("user@bastion-host")
        .await?;
    
    let context = session.connect().await?;
    let output = context.proc_exec(&["hostname"]).await?;
    
    println!("Connected through jump host to: {}", output.stdout.trim());
    
    Ok(())
}

// Example with privilege escalation
async fn example_with_sudo() -> Result<(), Box<dyn Error>> {
    let session = Session::ssh("user@remote-host")
        .sudo()
        .await?;
    
    let context = session.connect().await?;
    
    // This will run with sudo privileges
    let output = context.proc_exec(&["systemctl", "status", "nginx"]).await?;
    println!("Service status: {}", output.stdout);
    
    Ok(())
}