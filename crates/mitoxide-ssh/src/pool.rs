//! Connection pool and management

use crate::{Transport, Connection, TransportError, SshConfig, StdioTransport};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::{sleep, timeout};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of connections per host
    pub max_connections_per_host: usize,
    /// Maximum idle time before connection is closed
    pub max_idle_time: Duration,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Maximum number of connection retries
    pub max_retries: u32,
    /// Retry delay
    pub retry_delay: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections_per_host: 10,
            max_idle_time: Duration::from_secs(300), // 5 minutes
            connection_timeout: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(60), // 1 minute
            max_retries: 3,
            retry_delay: Duration::from_secs(1),
        }
    }
}

/// Connection pool entry
#[derive(Debug)]
struct PoolEntry {
    /// Connection instance
    connection: Connection,
    /// Last used timestamp
    last_used: Instant,
    /// Connection health status
    healthy: bool,
    /// Number of times this connection has been used
    use_count: u64,
}

/// Connection pool for managing SSH connections
pub struct ConnectionPool {
    /// Pool configuration
    config: PoolConfig,
    /// Active connections grouped by host
    connections: Arc<RwLock<HashMap<String, Vec<PoolEntry>>>>,
    /// Connection configurations
    ssh_configs: Arc<RwLock<HashMap<String, SshConfig>>>,
    /// Health check task handle
    health_check_handle: Option<tokio::task::JoinHandle<()>>,
}

/// A pooled connection wrapper
pub struct PooledConnection {
    /// Connection ID
    id: Uuid,
    /// Host key
    host_key: String,
    /// Underlying connection
    connection: Option<Connection>,
    /// Reference to the pool for returning the connection
    pool: Arc<ConnectionPool>,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(config: PoolConfig) -> Self {
        let pool = Self {
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            ssh_configs: Arc::new(RwLock::new(HashMap::new())),
            health_check_handle: None,
        };
        
        pool
    }
    
    /// Start the connection pool with health checking
    pub async fn start(&mut self) -> Result<(), TransportError> {
        info!("Starting connection pool");
        
        // Start health check task
        let connections = Arc::clone(&self.connections);
        let config = self.config.clone();
        
        let handle = tokio::spawn(async move {
            Self::health_check_loop(connections, config).await;
        });
        
        self.health_check_handle = Some(handle);
        Ok(())
    }
    
    /// Stop the connection pool
    pub async fn stop(&mut self) -> Result<(), TransportError> {
        info!("Stopping connection pool");
        
        // Stop health check task
        if let Some(handle) = self.health_check_handle.take() {
            handle.abort();
        }
        
        // Close all connections
        let mut connections = self.connections.write().await;
        for (host, entries) in connections.drain() {
            info!("Closing {} connections for host: {}", entries.len(), host);
            for mut entry in entries {
                if let Err(e) = entry.connection.close().await {
                    warn!("Error closing connection to {}: {}", host, e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Add SSH configuration for a host
    pub async fn add_host(&self, host: String, config: SshConfig) {
        let mut configs = self.ssh_configs.write().await;
        configs.insert(host.clone(), config);
        debug!("Added SSH configuration for host: {}", host);
    }
    
    /// Get a connection from the pool
    pub async fn get_connection(&self, host: &str) -> Result<PooledConnection, TransportError> {
        let host_key = host.to_string();
        
        // Try to get an existing connection
        if let Some(connection) = self.get_existing_connection(&host_key).await? {
            return Ok(connection);
        }
        
        // Create a new connection
        self.create_new_connection(&host_key).await
    }
    
    /// Get an existing connection from the pool
    async fn get_existing_connection(&self, host_key: &str) -> Result<Option<PooledConnection>, TransportError> {
        let mut connections = self.connections.write().await;
        
        if let Some(entries) = connections.get_mut(host_key) {
            // Find a healthy, idle connection
            for (i, entry) in entries.iter().enumerate() {
                if entry.healthy && entry.connection.is_connected() {
                    let mut entry = entries.remove(i);
                    entry.last_used = Instant::now();
                    entry.use_count += 1;
                    
                    debug!("Reusing existing connection to {}", host_key);
                    
                    return Ok(Some(PooledConnection {
                        id: Uuid::new_v4(),
                        host_key: host_key.to_string(),
                        connection: Some(entry.connection),
                        pool: Arc::new(self.clone()),
                    }));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Create a new connection
    async fn create_new_connection(&self, host_key: &str) -> Result<PooledConnection, TransportError> {
        // Check if we've reached the connection limit
        {
            let connections = self.connections.read().await;
            if let Some(entries) = connections.get(host_key) {
                if entries.len() >= self.config.max_connections_per_host {
                    return Err(TransportError::Configuration(
                        format!("Maximum connections reached for host: {}", host_key)
                    ));
                }
            }
        }
        
        // Get SSH configuration
        let ssh_config = {
            let configs = self.ssh_configs.read().await;
            configs.get(host_key).cloned()
                .ok_or_else(|| TransportError::Configuration(
                    format!("No SSH configuration found for host: {}", host_key)
                ))?
        };
        
        debug!("Creating new connection to {}", host_key);
        
        // Create transport and connect with retries
        let connection = self.connect_with_retries(ssh_config).await?;
        
        info!("Successfully created new connection to {}", host_key);
        
        Ok(PooledConnection {
            id: Uuid::new_v4(),
            host_key: host_key.to_string(),
            connection: Some(connection),
            pool: Arc::new(self.clone()),
        })
    }
    
    /// Connect with retries
    async fn connect_with_retries(&self, ssh_config: SshConfig) -> Result<Connection, TransportError> {
        let mut last_error = None;
        
        for attempt in 1..=self.config.max_retries {
            debug!("Connection attempt {} of {}", attempt, self.config.max_retries);
            
            let mut transport = StdioTransport::new(ssh_config.clone());
            
            match timeout(self.config.connection_timeout, transport.connect()).await {
                Ok(Ok(connection)) => {
                    debug!("Connection successful on attempt {}", attempt);
                    return Ok(connection);
                }
                Ok(Err(e)) => {
                    warn!("Connection attempt {} failed: {}", attempt, e);
                    last_error = Some(e);
                }
                Err(_) => {
                    let timeout_error = TransportError::Timeout;
                    warn!("Connection attempt {} timed out", attempt);
                    last_error = Some(timeout_error);
                }
            }
            
            if attempt < self.config.max_retries {
                sleep(self.config.retry_delay).await;
            }
        }
        
        Err(last_error.unwrap_or_else(|| {
            TransportError::Connection("All connection attempts failed".to_string())
        }))
    }
    
    /// Return a connection to the pool
    async fn return_connection(&self, host_key: String, connection: Connection) -> Result<(), TransportError> {
        if !connection.is_connected() {
            debug!("Not returning disconnected connection to pool");
            return Ok(());
        }
        
        let entry = PoolEntry {
            connection,
            last_used: Instant::now(),
            healthy: true,
            use_count: 1,
        };
        
        let mut connections = self.connections.write().await;
        let entries = connections.entry(host_key.clone()).or_insert_with(Vec::new);
        
        // Check if we're under the limit
        if entries.len() < self.config.max_connections_per_host {
            entries.push(entry);
            debug!("Returned connection to pool for host: {}", host_key);
        } else {
            debug!("Pool full, closing connection for host: {}", host_key);
            // Pool is full, close the connection
            drop(entry);
        }
        
        Ok(())
    }
    
    /// Health check loop
    async fn health_check_loop(
        connections: Arc<RwLock<HashMap<String, Vec<PoolEntry>>>>,
        config: PoolConfig,
    ) {
        let mut interval = tokio::time::interval(config.health_check_interval);
        
        loop {
            interval.tick().await;
            
            debug!("Running connection health check");
            
            let mut connections_guard = connections.write().await;
            let now = Instant::now();
            
            for (host, entries) in connections_guard.iter_mut() {
                entries.retain_mut(|entry| {
                    // Check if connection is too old
                    if now.duration_since(entry.last_used) > config.max_idle_time {
                        debug!("Closing idle connection to {}", host);
                        let _ = entry.connection.close();
                        return false;
                    }
                    
                    // Check if connection is still healthy
                    if !entry.connection.is_connected() {
                        debug!("Removing unhealthy connection to {}", host);
                        entry.healthy = false;
                        return false;
                    }
                    
                    true
                });
            }
            
            // Remove empty host entries
            connections_guard.retain(|_, entries| !entries.is_empty());
        }
    }
    
    /// Get pool statistics
    pub async fn stats(&self) -> PoolStats {
        let connections = self.connections.read().await;
        let mut total_connections = 0;
        let mut healthy_connections = 0;
        let mut hosts = 0;
        
        for (_, entries) in connections.iter() {
            hosts += 1;
            for entry in entries {
                total_connections += 1;
                if entry.healthy {
                    healthy_connections += 1;
                }
            }
        }
        
        PoolStats {
            total_connections,
            healthy_connections,
            hosts,
        }
    }
}

impl Clone for ConnectionPool {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            connections: Arc::clone(&self.connections),
            ssh_configs: Arc::clone(&self.ssh_configs),
            health_check_handle: None, // Don't clone the handle
        }
    }
}

impl Drop for ConnectionPool {
    fn drop(&mut self) {
        if let Some(handle) = self.health_check_handle.take() {
            handle.abort();
        }
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Total number of connections
    pub total_connections: usize,
    /// Number of healthy connections
    pub healthy_connections: usize,
    /// Number of hosts
    pub hosts: usize,
}

impl PooledConnection {
    /// Get the connection ID
    pub fn id(&self) -> Uuid {
        self.id
    }
    
    /// Get the host key
    pub fn host_key(&self) -> &str {
        &self.host_key
    }
    
    /// Get mutable reference to the underlying connection
    pub fn connection_mut(&mut self) -> Option<&mut Connection> {
        self.connection.as_mut()
    }
    
    /// Take ownership of the underlying connection
    pub fn take_connection(&mut self) -> Option<Connection> {
        self.connection.take()
    }
    
    /// Check if the connection is still active
    pub fn is_connected(&self) -> bool {
        self.connection.as_ref().map_or(false, |c| c.is_connected())
    }
}

impl Drop for PooledConnection {
    fn drop(&mut self) {
        if let Some(connection) = self.connection.take() {
            let pool = Arc::clone(&self.pool);
            let host_key = self.host_key.clone();
            
            // Return connection to pool in background
            tokio::spawn(async move {
                if let Err(e) = pool.return_connection(host_key, connection).await {
                    warn!("Failed to return connection to pool: {}", e);
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SshConfig;
    
    #[test]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.max_connections_per_host, 10);
        assert_eq!(config.max_idle_time, Duration::from_secs(300));
        assert_eq!(config.connection_timeout, Duration::from_secs(30));
    }
    
    #[tokio::test]
    async fn test_pool_creation() {
        let config = PoolConfig::default();
        let pool = ConnectionPool::new(config);
        
        let stats = pool.stats().await;
        assert_eq!(stats.total_connections, 0);
        assert_eq!(stats.healthy_connections, 0);
        assert_eq!(stats.hosts, 0);
    }
    
    #[tokio::test]
    async fn test_add_host() {
        let config = PoolConfig::default();
        let pool = ConnectionPool::new(config);
        
        let ssh_config = SshConfig::default();
        pool.add_host("test.example.com".to_string(), ssh_config).await;
        
        // Verify the configuration was added
        let configs = pool.ssh_configs.read().await;
        assert!(configs.contains_key("test.example.com"));
    }
    
    #[tokio::test]
    async fn test_pool_start_stop() {
        let config = PoolConfig::default();
        let mut pool = ConnectionPool::new(config);
        
        // Start the pool
        pool.start().await.unwrap();
        assert!(pool.health_check_handle.is_some());
        
        // Stop the pool
        pool.stop().await.unwrap();
        assert!(pool.health_check_handle.is_none());
    }
    
    #[tokio::test]
    async fn test_pooled_connection_properties() {
        let config = PoolConfig::default();
        let pool = Arc::new(ConnectionPool::new(config));
        
        let pooled_conn = PooledConnection {
            id: Uuid::new_v4(),
            host_key: "test.example.com".to_string(),
            connection: Some(Connection::new(None)),
            pool,
        };
        
        assert_eq!(pooled_conn.host_key(), "test.example.com");
        assert!(!pooled_conn.is_connected()); // No actual SSH process
    }
    
    #[test]
    fn test_pool_stats() {
        let stats = PoolStats {
            total_connections: 5,
            healthy_connections: 4,
            hosts: 2,
        };
        
        assert_eq!(stats.total_connections, 5);
        assert_eq!(stats.healthy_connections, 4);
        assert_eq!(stats.hosts, 2);
    }
}