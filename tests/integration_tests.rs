//! Integration tests for Mitoxide Docker test environment
//! 
//! These tests verify that the Docker containers are properly configured
//! and that SSH connectivity works as expected.

mod integration;

use integration::*;
use anyhow::Result;
use std::time::Duration;

/// Test Docker environment setup and basic functionality
#[tokio::test]
async fn test_docker_environment_setup() -> Result<()> {
    // Check prerequisites
    EnvUtils::setup_test_environment()?;
    
    // Create Docker test environment
    let docker_env = DockerTestEnv::new();
    
    // Start containers
    docker_env.start().await?;
    
    // Verify containers are running
    let status = docker_env.get_status().await?;
    
    for container_name in docker_env.list_containers() {
        let container_config = docker_env.get_container(container_name).unwrap();
        let status_info = status.get(&container_config.name);
        
        assert!(
            status_info.is_some(),
            "Container {} not found in status",
            container_name
        );
        
        let status_info = status_info.unwrap();
        assert!(
            status_info.running,
            "Container {} is not running: {}",
            container_name,
            status_info.state
        );
    }
    
    // Cleanup
    docker_env.stop().await?;
    
    Ok(())
}

/// Test SSH connectivity to all containers
#[tokio::test]
async fn test_ssh_connectivity() -> Result<()> {
    // Setup environment
    EnvUtils::setup_test_environment()?;
    let docker_env = DockerTestEnv::new();
    docker_env.start().await?;
    
    // Test SSH connections
    let ssh_helper = SshHelper::new();
    let results = ssh_helper.test_all_containers().await?;
    
    // Verify all connections succeeded
    assert!(
        results.all_successful(),
        "Some SSH connections failed: {:?}",
        results.failed_tests()
    );
    
    println!(
        "✅ All SSH connections successful ({}/{})",
        results.success_count(),
        results.total_count()
    );
    
    // Cleanup
    docker_env.stop().await?;
    
    Ok(())
}

/// Test basic command execution over SSH
#[tokio::test]
async fn test_basic_command_execution() -> Result<()> {
    // Setup environment
    EnvUtils::setup_test_environment()?;
    let docker_env = DockerTestEnv::new();
    docker_env.start().await?;
    
    let ssh_helper = SshHelper::new();
    
    // Test commands on different containers
    let test_cases = vec![
        ("alpine_ro", 2222, "uname -a"),
        ("ubuntu_min", 2223, "whoami"),
        ("bastion", 2224, "echo 'hello world'"),
    ];
    
    for (container, port, command) in test_cases {
        let config = ssh_helper.config_for_container(container, port);
        let output = ssh_helper.execute_command(&config, &command.split_whitespace().collect::<Vec<_>>()).await?;
        
        TestAssertions::assert_ssh_success(&output, &format!("Command '{}' on {}", command, container))?;
        
        // Verify expected outputs
        match command {
            cmd if cmd.contains("uname") => {
                TestAssertions::assert_output_contains(&output.stdout, "Linux", "uname output")?;
            }
            cmd if cmd.contains("whoami") => {
                TestAssertions::assert_output_contains(&output.stdout, "testuser", "whoami output")?;
            }
            cmd if cmd.contains("echo") => {
                TestAssertions::assert_output_contains(&output.stdout, "hello world", "echo output")?;
            }
            _ => {}
        }
        
        println!("✅ Command '{}' on {} executed successfully", command, container);
    }
    
    // Cleanup
    docker_env.stop().await?;
    
    Ok(())
}

/// Test file operations over SSH
#[tokio::test]
async fn test_file_operations() -> Result<()> {
    // Setup environment
    EnvUtils::setup_test_environment()?;
    let docker_env = DockerTestEnv::new();
    docker_env.start().await?;
    
    let ssh_helper = SshHelper::new();
    
    // Create test file
    let test_content = "Hello, Mitoxide integration test!";
    let (_temp_dir, test_file) = FileTestUtils::create_test_file(test_content)?;
    
    // Test file upload and download on ubuntu_min (writable filesystem)
    let config = ssh_helper.config_for_container("ubuntu_min", 2223);
    
    // Upload file
    ssh_helper.copy_file_to(&config, &test_file, "/tmp/test_upload.txt").await?;
    
    // Verify file exists on remote
    let output = ssh_helper.execute_command(&config, &["cat", "/tmp/test_upload.txt"]).await?;
    TestAssertions::assert_ssh_success(&output, "Reading uploaded file")?;
    TestAssertions::assert_output_contains(&output.stdout, test_content, "Uploaded file content")?;
    
    // Download file back
    let (_temp_dir2, download_path) = FileTestUtils::create_test_file("")?; // Empty file
    ssh_helper.copy_file_from(&config, "/tmp/test_upload.txt", &download_path).await?;
    
    // Verify downloaded content
    assert!(
        FileTestUtils::verify_file_content(&download_path, test_content)?,
        "Downloaded file content doesn't match"
    );
    
    println!("✅ File operations test completed successfully");
    
    // Cleanup
    docker_env.stop().await?;
    
    Ok(())
}

/// Test jump host connectivity
#[tokio::test]
async fn test_jump_host_connectivity() -> Result<()> {
    // Setup environment
    EnvUtils::setup_test_environment()?;
    let docker_env = DockerTestEnv::new();
    docker_env.start().await?;
    
    let ssh_helper = SshHelper::new();
    
    // Test connection through jump host
    let jump_config = ssh_helper.config_for_jump_host("mitoxide_backend_target", 2224);
    
    // Test basic connectivity
    let connectivity_result = ssh_helper.test_connectivity(&jump_config).await?;
    assert!(connectivity_result, "Jump host connectivity test failed");
    
    // Test command execution through jump host
    let output = ssh_helper.execute_command(&jump_config, &["hostname"]).await?;
    TestAssertions::assert_ssh_success(&output, "Command execution through jump host")?;
    
    // Verify we're connected to the backend target
    TestAssertions::assert_output_contains(&output.stdout, "mitoxide", "Backend target hostname")?;
    
    println!("✅ Jump host connectivity test completed successfully");
    
    // Cleanup
    docker_env.stop().await?;
    
    Ok(())
}

/// Test performance characteristics
#[tokio::test]
async fn test_performance_characteristics() -> Result<()> {
    // Setup environment
    EnvUtils::setup_test_environment()?;
    let docker_env = DockerTestEnv::new();
    docker_env.start().await?;
    
    let ssh_helper = SshHelper::new();
    let config = ssh_helper.config_for_container("ubuntu_min", 2223);
    
    // Test connection establishment time
    let (connectivity_result, connection_time) = PerformanceUtils::measure_async(
        ssh_helper.test_connectivity(&config)
    ).await;
    
    assert!(connectivity_result?, "Performance test connectivity failed");
    
    // Assert reasonable connection time (should be under 5 seconds)
    TestAssertions::assert_performance_threshold(
        connection_time,
        Duration::from_secs(5),
        "SSH connection establishment"
    )?;
    
    // Test command execution latency
    let (output, execution_time) = PerformanceUtils::measure_async(
        ssh_helper.execute_command(&config, &["echo", "latency_test"])
    ).await;
    
    TestAssertions::assert_ssh_success(&output, "Latency test command")?;
    
    // Assert reasonable execution time (should be under 2 seconds)
    TestAssertions::assert_performance_threshold(
        execution_time,
        Duration::from_secs(2),
        "SSH command execution"
    )?;
    
    println!(
        "✅ Performance test completed - Connection: {}, Execution: {}",
        PerformanceUtils::format_duration(connection_time),
        PerformanceUtils::format_duration(execution_time)
    );
    
    // Cleanup
    docker_env.stop().await?;
    
    Ok(())
}

/// Test container resource constraints
#[tokio::test]
async fn test_container_constraints() -> Result<()> {
    // Setup environment
    EnvUtils::setup_test_environment()?;
    let docker_env = DockerTestEnv::new();
    docker_env.start().await?;
    
    let ssh_helper = SshHelper::new();
    
    // Test alpine_ro read-only filesystem constraint
    let alpine_config = ssh_helper.config_for_container("alpine_ro", 2222);
    
    // Try to write to read-only filesystem (should fail)
    let output = ssh_helper.execute_command(
        &alpine_config,
        &["touch", "/root/should_fail.txt"]
    ).await?;
    
    // This should fail due to read-only filesystem
    assert!(
        !output.success(),
        "Write to read-only filesystem should fail, but succeeded"
    );
    
    // Verify we can write to tmpfs
    let output = ssh_helper.execute_command(
        &alpine_config,
        &["touch", "/tmp/should_succeed.txt"]
    ).await?;
    
    TestAssertions::assert_ssh_success(&output, "Write to tmpfs")?;
    
    // Test memory constraints by checking available memory
    let output = ssh_helper.execute_command(
        &alpine_config,
        &["cat", "/proc/meminfo"]
    ).await?;
    
    TestAssertions::assert_ssh_success(&output, "Reading memory info")?;
    
    // Parse memory info to verify constraints
    if let Some(line) = output.stdout.lines().find(|line| line.starts_with("MemTotal:")) {
        let mem_kb: u64 = line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        
        // Should be around 64MB (65536 KB), allow some overhead
        assert!(
            mem_kb < 80_000,
            "Memory limit not properly enforced: {} KB",
            mem_kb
        );
        
        println!("✅ Memory constraint verified: {} KB", mem_kb);
    }
    
    println!("✅ Container constraints test completed successfully");
    
    // Cleanup
    docker_env.stop().await?;
    
    Ok(())
}

/// Test comprehensive jump host and routing functionality
#[tokio::test]
async fn test_comprehensive_routing() -> Result<()> {
    use integration::routing_tests::RoutingTests;
    
    let routing_tests = RoutingTests::new();
    routing_tests.run_all_tests().await?;
    
    Ok(())
}