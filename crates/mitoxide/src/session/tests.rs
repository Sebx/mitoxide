//! Unit tests for session management

use super::*;
// use crate::MitoxideError;
use std::time::Duration;
// use tokio_test;

#[tokio::test]
async fn test_session_builder_creation() {
    let builder = SessionBuilder::new("user@example.com:2222".to_string());
    
    assert_eq!(builder.target, "user@example.com:2222");
    assert_eq!(builder.ssh_config.username, "user");
    assert_eq!(builder.ssh_config.host, "example.com");
    assert_eq!(builder.ssh_config.port, 2222);
}

#[tokio::test]
async fn test_session_builder_parse_target() {
    // Test various target formats
    let test_cases = vec![
        ("localhost", ("root", "localhost", 22)),
        ("user@host", ("user", "host", 22)),
        ("host:2222", ("root", "host", 2222)),
        ("user@host:2222", ("user", "host", 2222)),
        ("192.168.1.1", ("root", "192.168.1.1", 22)),
        ("user@192.168.1.1:2222", ("user", "192.168.1.1", 2222)),
    ];
    
    for (target, expected) in test_cases {
        let (username, host, port) = SessionBuilder::parse_target(target);
        assert_eq!((username.as_str(), host.as_str(), port), expected, "Failed for target: {}", target);
    }
}

#[tokio::test]
async fn test_session_builder_configuration() {
    let builder = SessionBuilder::new("test@example.com".to_string())
        .with_timeout(Duration::from_secs(60))
        .with_key(PathBuf::from("/path/to/key"))
        .with_ssh_option("ServerAliveInterval".to_string(), "30".to_string())
        .with_max_streams(50)
        .with_bootstrap(false)
        .with_hash_verification(true);
    
    let config = builder.build_config();
    
    assert_eq!(config.timeout, Duration::from_secs(60));
    assert_eq!(config.ssh_config.key_path, Some(PathBuf::from("/path/to/key")));
    assert_eq!(config.ssh_config.options.get("ServerAliveInterval"), Some(&"30".to_string()));
    assert_eq!(config.max_streams, 50);
    assert_eq!(config.bootstrap_agent, false);
    assert_eq!(config.agent_config.verify_hash, true);
}

#[tokio::test]
async fn test_session_ssh_builder() {
    let result = Session::ssh("test@example.com").await;
    assert!(result.is_ok());
    
    let builder = result.unwrap();
    assert_eq!(builder.target, "test@example.com");
    assert_eq!(builder.ssh_config.username, "test");
    assert_eq!(builder.ssh_config.host, "example.com");
    assert_eq!(builder.ssh_config.port, 22);
}

#[tokio::test]
async fn test_agent_config_default() {
    let config = AgentConfig::default();
    
    assert_eq!(config.binary_path, None);
    assert_eq!(config.execution_timeout, Duration::from_secs(300));
    assert_eq!(config.verify_hash, false);
    assert_eq!(config.verify_signature, false);
}

#[tokio::test]
async fn test_session_state_creation() {
    let session_id = Uuid::new_v4();
    let state = SessionState {
        id: session_id,
        target: "test@example.com".to_string(),
        status: SessionStatus::Connecting,
        agent_version: None,
        capabilities: Vec::new(),
        connection_info: None,
    };
    
    assert_eq!(state.id, session_id);
    assert_eq!(state.target, "test@example.com");
    assert_eq!(state.status, SessionStatus::Connecting);
    assert!(state.capabilities.is_empty());
}

#[tokio::test]
async fn test_session_status_equality() {
    assert_eq!(SessionStatus::Connecting, SessionStatus::Connecting);
    assert_eq!(SessionStatus::Active, SessionStatus::Active);
    assert_eq!(SessionStatus::Disconnected, SessionStatus::Disconnected);
    
    let error1 = SessionStatus::Error("test".to_string());
    let error2 = SessionStatus::Error("test".to_string());
    let error3 = SessionStatus::Error("different".to_string());
    
    assert_eq!(error1, error2);
    assert_ne!(error1, error3);
}

// Mock tests would require actual SSH connections, so we'll test the configuration
// and builder logic here. Integration tests with Docker containers will test
// the actual connection functionality.

#[test]
fn test_session_config_clone() {
    let config = SessionConfig {
        ssh_config: SshConfig::default(),
        agent_config: AgentConfig::default(),
        timeout: Duration::from_secs(30),
        max_streams: 100,
        bootstrap_agent: true,
    };
    
    let cloned = config.clone();
    assert_eq!(config.timeout, cloned.timeout);
    assert_eq!(config.max_streams, cloned.max_streams);
    assert_eq!(config.bootstrap_agent, cloned.bootstrap_agent);
}

#[test]
fn test_session_state_clone() {
    let state = SessionState {
        id: Uuid::new_v4(),
        target: "test".to_string(),
        status: SessionStatus::Active,
        agent_version: Some("1.0.0".to_string()),
        capabilities: vec!["test".to_string()],
        connection_info: None,
    };
    
    let cloned = state.clone();
    assert_eq!(state.id, cloned.id);
    assert_eq!(state.target, cloned.target);
    assert_eq!(state.status, cloned.status);
    assert_eq!(state.agent_version, cloned.agent_version);
    assert_eq!(state.capabilities, cloned.capabilities);
}