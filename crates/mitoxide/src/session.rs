//! Session management and connection handling

use crate::{Result, MitoxideError, Context, Router};
// use mitoxide_proto::{Message, Request, Response};
use mitoxide_ssh::{Transport, StdioTransport, SshConfig, ConnectionInfo};

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Session configuration
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// SSH configuration
    pub ssh_config: SshConfig,
    /// Agent configuration
    pub agent_config: AgentConfig,
    /// Connection timeout
    pub timeout: Duration,
    /// Maximum number of concurrent streams
    pub max_streams: u32,
    /// Enable agent bootstrapping
    pub bootstrap_agent: bool,
}

/// Agent configuration
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Agent binary path (if None, uses embedded binary)
    pub binary_path: Option<PathBuf>,
    /// Agent execution timeout
    pub execution_timeout: Duration,
    /// Enable hash verification
    pub verify_hash: bool,
    /// Enable signature verification
    pub verify_signature: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            binary_path: None,
            execution_timeout: Duration::from_secs(300),
            verify_hash: false,
            verify_signature: false,
        }
    }
}

/// Session status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionStatus {
    /// Session is being created
    Connecting,
    /// Agent is being bootstrapped
    Bootstrapping,
    /// Session is active and ready
    Active,
    /// Session is disconnected
    Disconnected,
    /// Session encountered an error
    Error(String),
}

/// Session state information
#[derive(Debug, Clone)]
pub struct SessionState {
    /// Unique session ID
    pub id: Uuid,
    /// Target connection string
    pub target: String,
    /// Current session status
    pub status: SessionStatus,
    /// Agent version (if available)
    pub agent_version: Option<String>,
    /// Available capabilities
    pub capabilities: Vec<String>,
    /// Connection information
    pub connection_info: Option<ConnectionInfo>,
}

/// Session builder for configuring connections
pub struct SessionBuilder {
    /// Target connection string
    target: String,
    /// SSH configuration
    ssh_config: SshConfig,
    /// Agent configuration
    agent_config: AgentConfig,
    /// Connection timeout
    timeout: Duration,
    /// Maximum streams
    max_streams: u32,
    /// Bootstrap agent flag
    bootstrap_agent: bool,
}

impl SessionBuilder {
    /// Create a new session builder
    pub fn new(target: String) -> Self {
        // Parse target string (user@host:port)
        let (username, host, port) = Self::parse_target(&target);
        
        let ssh_config = SshConfig {
            host,
            port,
            username,
            ..Default::default()
        };
        
        Self {
            target,
            ssh_config,
            agent_config: AgentConfig::default(),
            timeout: Duration::from_secs(30),
            max_streams: 100,
            bootstrap_agent: true,
        }
    }
    
    /// Parse target string into components
    fn parse_target(target: &str) -> (String, String, u16) {
        // Format: [user@]host[:port]
        let mut username = "root".to_string();
        let mut host = target.to_string();
        let mut port = 22;
        
        // Extract username if present
        if let Some(at_pos) = target.find('@') {
            username = target[..at_pos].to_string();
            host = target[at_pos + 1..].to_string();
        }
        
        // Extract port if present
        if let Some(colon_pos) = host.rfind(':') {
            if let Ok(parsed_port) = host[colon_pos + 1..].parse::<u16>() {
                port = parsed_port;
                host = host[..colon_pos].to_string();
            }
        }
        
        (username, host, port)
    }
    
    /// Set SSH key path
    pub fn with_key(mut self, key_path: PathBuf) -> Self {
        self.ssh_config.key_path = Some(key_path);
        self
    }
    
    /// Set connection timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self.ssh_config.connect_timeout = timeout.as_secs();
        self
    }
    
    /// Set SSH options
    pub fn with_ssh_option(mut self, key: String, value: String) -> Self {
        self.ssh_config.options.insert(key, value);
        self
    }
    
    /// Set agent binary path
    pub fn with_agent_binary(mut self, path: PathBuf) -> Self {
        self.agent_config.binary_path = Some(path);
        self
    }
    
    /// Enable/disable agent bootstrapping
    pub fn with_bootstrap(mut self, bootstrap: bool) -> Self {
        self.bootstrap_agent = bootstrap;
        self
    }
    
    /// Set maximum concurrent streams
    pub fn with_max_streams(mut self, max_streams: u32) -> Self {
        self.max_streams = max_streams;
        self
    }
    
    /// Enable hash verification
    pub fn with_hash_verification(mut self, verify: bool) -> Self {
        self.agent_config.verify_hash = verify;
        self
    }
    
    /// Build the session configuration
    pub fn build_config(self) -> SessionConfig {
        SessionConfig {
            ssh_config: self.ssh_config,
            agent_config: self.agent_config,
            timeout: self.timeout,
            max_streams: self.max_streams,
            bootstrap_agent: self.bootstrap_agent,
        }
    }
    
    /// Connect and create the session
    pub async fn connect(self) -> Result<ConnectedSession> {
        let target = self.target.clone();
        let config = self.build_config();
        let session = Session::new(target, config);
        session.connect().await
    }
}

/// Active session with established connection
pub struct ConnectedSession {
    /// Session state
    state: Arc<RwLock<SessionState>>,
    /// Connection router
    router: Arc<Router>,
    /// Session configuration
    config: SessionConfig,
    /// Shutdown sender
    shutdown_tx: mpsc::Sender<()>,
}

impl ConnectedSession {
    /// Create a new connected session
    pub(crate) fn new(
        state: SessionState,
        router: Router,
        config: SessionConfig,
        shutdown_tx: mpsc::Sender<()>,
    ) -> Self {
        Self {
            state: Arc::new(RwLock::new(state)),
            router: Arc::new(router),
            config,
            shutdown_tx,
        }
    }
    
    /// Get session state
    pub async fn state(&self) -> SessionState {
        self.state.read().await.clone()
    }
    
    /// Get session ID
    pub async fn id(&self) -> Uuid {
        self.state.read().await.id
    }
    
    /// Create a new execution context
    pub async fn context(&self) -> Result<Context> {
        let state = self.state.read().await;
        if state.status != SessionStatus::Active {
            return Err(MitoxideError::Protocol(
                format!("Session not active: {:?}", state.status)
            ));
        }
        
        Context::new(state.id, self.router.clone())
    }
    
    /// Test connection health
    pub async fn ping(&self) -> Result<Duration> {
        let context = self.context().await?;
        context.ping().await
    }
    
    /// Get connection information
    pub async fn connection_info(&self) -> Option<ConnectionInfo> {
        self.state.read().await.connection_info.clone()
    }
    
    /// Gracefully disconnect the session
    pub async fn disconnect(self) -> Result<()> {
        info!("Disconnecting session {}", self.id().await);
        
        // Update session status
        {
            let mut state = self.state.write().await;
            state.status = SessionStatus::Disconnected;
        }
        
        // Send shutdown signal
        if let Err(e) = self.shutdown_tx.send(()).await {
            warn!("Failed to send shutdown signal: {}", e);
        }
        
        // Wait for router to clean up
        self.router.shutdown().await?;
        
        info!("Session disconnected successfully");
        Ok(())
    }
}

impl Drop for ConnectedSession {
    fn drop(&mut self) {
        // Try to send shutdown signal on drop
        let _ = self.shutdown_tx.try_send(());
    }
}

/// Main session type for managing SSH connections
pub struct Session {
    /// Target connection string
    target: String,
    /// Session configuration
    config: SessionConfig,
}

impl Session {
    /// Create a new SSH session builder
    pub async fn ssh(target: &str) -> Result<SessionBuilder> {
        debug!("Creating SSH session builder for target: {}", target);
        Ok(SessionBuilder::new(target.to_string()))
    }
    
    /// Create a new session with configuration
    pub fn new(target: String, config: SessionConfig) -> Self {
        Self { target, config }
    }
    
    /// Connect to the remote host and establish session
    pub async fn connect(self) -> Result<ConnectedSession> {
        info!("Connecting to target: {}", self.target);
        
        let session_id = Uuid::new_v4();
        let mut state = SessionState {
            id: session_id,
            target: self.target.clone(),
            status: SessionStatus::Connecting,
            agent_version: None,
            capabilities: Vec::new(),
            connection_info: None,
        };
        
        // Create transport
        let mut transport = StdioTransport::new(self.config.ssh_config.clone());
        
        // Test connection first
        transport.test_connection().await
            .map_err(|e| MitoxideError::Transport(format!("Connection test failed: {}", e)))?;
        
        // Establish connection
        let connection = transport.connect().await
            .map_err(|e| MitoxideError::Transport(format!("Failed to connect: {}", e)))?;
        
        state.connection_info = Some(transport.connection_info());
        
        // Bootstrap agent if enabled
        if self.config.bootstrap_agent {
            state.status = SessionStatus::Bootstrapping;
            
            let agent_binary = self.get_agent_binary().await?;
            transport.bootstrap_agent(&agent_binary).await
                .map_err(|e| MitoxideError::Agent(format!("Agent bootstrap failed: {}", e)))?;
            
            info!("Agent bootstrapped successfully");
        }
        
        // Create router and start communication
        let (router, shutdown_tx) = Router::new(
            connection,
            self.config.max_streams,
            self.config.timeout,
        ).await?;
        
        // Update state to active
        state.status = SessionStatus::Active;
        state.capabilities = vec![
            "process_exec".to_string(),
            "file_ops".to_string(),
        ];
        
        // Add WASM capability if enabled
        #[cfg(feature = "wasm")]
        {
            state.capabilities.push("wasm_exec".to_string());
        }
        
        info!("Session {} established successfully", session_id);
        
        Ok(ConnectedSession::new(state, router, self.config, shutdown_tx))
    }
    
    /// Get agent binary (embedded or from file)
    async fn get_agent_binary(&self) -> Result<Vec<u8>> {
        if let Some(binary_path) = &self.config.agent_config.binary_path {
            // Load from file
            tokio::fs::read(binary_path).await
                .map_err(|e| MitoxideError::Agent(format!("Failed to read agent binary: {}", e)))
        } else {
            // Use embedded binary (placeholder for now)
            // In a real implementation, this would be the compiled mitoxide-agent binary
            // embedded using include_bytes! or similar
            Ok(b"#!/bin/bash\necho 'Mock agent binary'\n".to_vec())
        }
    }
}

#[cfg(test)]
mod tests;