//! Simple test to verify Docker environment setup

mod integration;

use integration::*;
use anyhow::Result;

#[tokio::test]
async fn test_docker_environment_basic() -> Result<()> {
    println!("Testing basic Docker environment setup...");
    
    // Check prerequisites
    EnvUtils::setup_test_environment()?;
    
    // Create and start Docker environment
    let docker_env = DockerTestEnv::new();
    docker_env.start().await?;
    
    // Test basic SSH connectivity
    let ssh_helper = SshHelper::new();
    
    // Test ubuntu_min container
    let ubuntu_config = ssh_helper.config_for_container("ubuntu_min", 2223);
    let ubuntu_result = ssh_helper.test_connectivity(&ubuntu_config).await?;
    println!("Ubuntu container connectivity: {}", ubuntu_result);
    
    // Test bastion container
    let bastion_config = ssh_helper.config_for_container("bastion", 2224);
    let bastion_result = ssh_helper.test_connectivity(&bastion_config).await?;
    println!("Bastion container connectivity: {}", bastion_result);
    
    // Cleanup
    docker_env.stop().await?;
    
    assert!(ubuntu_result, "Ubuntu container should be accessible");
    assert!(bastion_result, "Bastion container should be accessible");
    
    println!("✅ Docker environment test passed");
    Ok(())
}