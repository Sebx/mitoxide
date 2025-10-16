//! Docker container management for integration tests

use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;
use anyhow::{Context, Result};

/// Docker container configuration
#[derive(Debug, Clone)]
pub struct ContainerConfig {
    pub name: String,
    pub service: String,
    pub port: u16,
    pub constraints: ContainerConstraints,
}

/// Container resource constraints for testing
#[derive(Debug, Clone)]
pub struct ContainerConstraints {
    pub memory_limit: Option<String>,
    pub cpu_limit: Option<f32>,
    pub read_only: bool,
    pub tmpfs_mounts: Vec<String>,
}

/// Docker test environment manager
pub struct DockerTestEnv {
    containers: HashMap<String, ContainerConfig>,
    compose_file: String,
}

impl DockerTestEnv {
    /// Create a new Docker test environment
    pub fn new() -> Self {
        let mut containers = HashMap::new();
        
        // Alpine RO container with constraints
        containers.insert("alpine_ro".to_string(), ContainerConfig {
            name: "mitoxide_alpine_ro".to_string(),
            service: "alpine_ro".to_string(),
            port: 2222,
            constraints: ContainerConstraints {
                memory_limit: Some("64m".to_string()),
                cpu_limit: Some(0.5),
                read_only: true,
                tmpfs_mounts: vec!["/tmp:size=64m".to_string()],
            },
        });
        
        // Ubuntu minimal container
        containers.insert("ubuntu_min".to_string(), ContainerConfig {
            name: "mitoxide_ubuntu_min".to_string(),
            service: "ubuntu_min".to_string(),
            port: 2223,
            constraints: ContainerConstraints {
                memory_limit: None,
                cpu_limit: None,
                read_only: false,
                tmpfs_mounts: vec![],
            },
        });
        
        // Bastion host
        containers.insert("bastion".to_string(), ContainerConfig {
            name: "mitoxide_bastion".to_string(),
            service: "bastion".to_string(),
            port: 2224,
            constraints: ContainerConstraints {
                memory_limit: None,
                cpu_limit: None,
                read_only: false,
                tmpfs_mounts: vec![],
            },
        });
        
        // Backend target (no external port)
        containers.insert("backend_target".to_string(), ContainerConfig {
            name: "mitoxide_backend_target".to_string(),
            service: "backend_target".to_string(),
            port: 22, // Internal port only
            constraints: ContainerConstraints {
                memory_limit: None,
                cpu_limit: None,
                read_only: false,
                tmpfs_mounts: vec![],
            },
        });
        
        Self {
            containers,
            compose_file: "docker-compose.yml".to_string(),
        }
    }
    
    /// Start all containers
    pub async fn start(&self) -> Result<()> {
        println!("Starting Docker test environment...");
        
        // Build containers if needed
        self.build().await?;
        
        // Start containers
        let output = Command::new("docker-compose")
            .args(&["up", "-d"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to start Docker containers")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to start containers: {}", stderr);
        }
        
        // Wait for containers to be ready
        self.wait_for_ready().await?;
        
        println!("Docker test environment is ready");
        Ok(())
    }
    
    /// Stop all containers
    pub async fn stop(&self) -> Result<()> {
        println!("Stopping Docker test environment...");
        
        let output = Command::new("docker-compose")
            .args(&["down"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to stop Docker containers")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to stop containers: {}", stderr);
        }
        
        println!("Docker test environment stopped");
        Ok(())
    }
    
    /// Build all container images
    pub async fn build(&self) -> Result<()> {
        println!("Building Docker images...");
        
        let output = Command::new("docker-compose")
            .args(&["build"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to build Docker images")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to build images: {}", stderr);
        }
        
        println!("Docker images built successfully");
        Ok(())
    }
    
    /// Clean up all containers and resources
    pub async fn cleanup(&self) -> Result<()> {
        println!("Cleaning up Docker test environment...");
        
        let output = Command::new("docker-compose")
            .args(&["down", "-v", "--remove-orphans"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to cleanup Docker containers")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to cleanup containers: {}", stderr);
        }
        
        // Prune system resources
        let _ = Command::new("docker")
            .args(&["system", "prune", "-f"])
            .output();
        
        println!("Docker test environment cleaned up");
        Ok(())
    }
    
    /// Wait for all containers to be ready
    async fn wait_for_ready(&self) -> Result<()> {
        println!("Waiting for containers to be ready...");
        
        let max_attempts = 30;
        let delay = Duration::from_secs(2);
        
        for attempt in 1..=max_attempts {
            let mut all_ready = true;
            
            for (name, config) in &self.containers {
                if !self.is_container_ready(&config.name).await? {
                    all_ready = false;
                    break;
                }
            }
            
            if all_ready {
                println!("All containers are ready");
                return Ok(());
            }
            
            if attempt < max_attempts {
                println!("Attempt {}/{}: Waiting for containers...", attempt, max_attempts);
                sleep(delay).await;
            }
        }
        
        anyhow::bail!("Containers failed to become ready within timeout");
    }
    
    /// Check if a specific container is ready
    async fn is_container_ready(&self, container_name: &str) -> Result<bool> {
        let output = Command::new("docker")
            .args(&["exec", container_name, "echo", "ready"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to check container readiness")?;
        
        Ok(output.status.success())
    }
    
    /// Get container configuration by name
    pub fn get_container(&self, name: &str) -> Option<&ContainerConfig> {
        self.containers.get(name)
    }
    
    /// List all available containers
    pub fn list_containers(&self) -> Vec<&str> {
        self.containers.keys().map(|s| s.as_str()).collect()
    }
    
    /// Get container status
    pub async fn get_status(&self) -> Result<HashMap<String, ContainerStatus>> {
        let output = Command::new("docker-compose")
            .args(&["ps", "--format", "json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to get container status")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to get status: {}", stderr);
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut status_map = HashMap::new();
        
        // Parse docker-compose ps output
        for line in stdout.lines() {
            if let Ok(status) = serde_json::from_str::<serde_json::Value>(line) {
                if let (Some(name), Some(state)) = (
                    status.get("Name").and_then(|v| v.as_str()),
                    status.get("State").and_then(|v| v.as_str())
                ) {
                    status_map.insert(name.to_string(), ContainerStatus {
                        name: name.to_string(),
                        state: state.to_string(),
                        running: state == "running",
                    });
                }
            }
        }
        
        Ok(status_map)
    }
    
    /// Execute command in container
    pub async fn exec_command(&self, container: &str, command: &[&str]) -> Result<CommandOutput> {
        let container_config = self.get_container(container)
            .ok_or_else(|| anyhow::anyhow!("Unknown container: {}", container))?;
        
        let mut cmd = Command::new("docker");
        cmd.args(&["exec", &container_config.name]);
        cmd.args(command);
        
        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to execute command in container")?;
        
        Ok(CommandOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

/// Container status information
#[derive(Debug, Clone)]
pub struct ContainerStatus {
    pub name: String,
    pub state: String,
    pub running: bool,
}

/// Command execution output
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CommandOutput {
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}