//! Agent bootstrap and platform detection

use crate::{Transport, TransportError};
use tracing::{debug, info};

/// Platform information detected from remote host
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformInfo {
    /// Architecture (e.g., "x86_64", "aarch64")
    pub arch: String,
    /// Operating system (e.g., "Linux", "Darwin")
    pub os: String,
    /// OS version/distribution info
    pub version: Option<String>,
    /// Available bootstrap methods
    pub bootstrap_methods: Vec<BootstrapMethod>,
}

/// Available bootstrap methods
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootstrapMethod {
    /// Use memfd_create syscall (Linux only)
    MemfdCreate,
    /// Use temporary file in /tmp
    TempFile,
    /// Use temporary file in /dev/shm
    DevShm,
    /// Use Python for in-memory execution
    Python,
    /// Use shell script execution
    Shell,
}

/// Bootstrap functionality for SSH transport
pub struct Bootstrap {
    /// Detected platform information
    platform_info: Option<PlatformInfo>,
    /// Custom bootstrap script template
    custom_script: Option<String>,
}

impl Bootstrap {
    /// Create a new bootstrap instance
    pub fn new() -> Self {
        Self {
            platform_info: None,
            custom_script: None,
        }
    }
    
    /// Set a custom bootstrap script template
    pub fn with_custom_script(mut self, script: String) -> Self {
        self.custom_script = Some(script);
        self
    }
    
    /// Detect platform information from the remote host
    pub async fn detect_platform<T: Transport>(&mut self, transport: &mut T) -> Result<&PlatformInfo, TransportError> {
        info!("Detecting remote platform");
        
        // Get basic platform info
        let platform_cmd = "uname -m && uname -s && (lsb_release -d 2>/dev/null || cat /etc/os-release 2>/dev/null | head -1 || echo 'Unknown')";
        let platform_output = self.execute_command(transport, platform_cmd).await?;
        
        let lines: Vec<&str> = platform_output.trim().split('\n').collect();
        if lines.len() < 2 {
            return Err(TransportError::Bootstrap("Failed to detect platform".to_string()));
        }
        
        let arch = lines[0].trim().to_string();
        let os = lines[1].trim().to_string();
        let version = if lines.len() > 2 {
            Some(lines[2].trim().to_string())
        } else {
            None
        };
        
        debug!("Detected platform: {} {} {:?}", arch, os, version);
        
        // Detect available bootstrap methods
        let bootstrap_methods = self.detect_bootstrap_methods(transport, &os).await?;
        
        let platform_info = PlatformInfo {
            arch,
            os,
            version,
            bootstrap_methods,
        };
        
        self.platform_info = Some(platform_info);
        Ok(self.platform_info.as_ref().unwrap())
    }
    
    /// Detect available bootstrap methods
    async fn detect_bootstrap_methods<T: Transport>(
        &self, 
        transport: &mut T, 
        os: &str
    ) -> Result<Vec<BootstrapMethod>, TransportError> {
        let mut methods = Vec::new();
        
        // Check for memfd_create (Linux only)
        if os == "Linux" {
            let memfd_check = "python3 -c 'import ctypes; libc = ctypes.CDLL(\"libc.so.6\"); print(libc.syscall(319, b\"test\", 1) >= 0)' 2>/dev/null || echo 'False'";
            if let Ok(output) = self.execute_command(transport, memfd_check).await {
                if output.trim() == "True" {
                    methods.push(BootstrapMethod::MemfdCreate);
                    debug!("memfd_create available");
                }
            }
        }
        
        // Check for Python
        let python_check = "python3 --version 2>/dev/null || python --version 2>/dev/null";
        if self.execute_command(transport, python_check).await.is_ok() {
            methods.push(BootstrapMethod::Python);
            debug!("Python available");
        }
        
        // Check for /dev/shm
        let devshm_check = "[ -d /dev/shm ] && [ -w /dev/shm ] && echo 'available'";
        if let Ok(output) = self.execute_command(transport, devshm_check).await {
            if output.trim() == "available" {
                methods.push(BootstrapMethod::DevShm);
                debug!("/dev/shm available");
            }
        }
        
        // Check for /tmp
        let tmp_check = "[ -d /tmp ] && [ -w /tmp ] && echo 'available'";
        if let Ok(output) = self.execute_command(transport, tmp_check).await {
            if output.trim() == "available" {
                methods.push(BootstrapMethod::TempFile);
                debug!("/tmp available");
            }
        }
        
        // Shell is always available as fallback
        methods.push(BootstrapMethod::Shell);
        
        Ok(methods)
    }
    
    /// Generate bootstrap script for the detected platform
    pub fn generate_bootstrap_script(&self, _agent_binary: &[u8]) -> Result<String, TransportError> {
        let platform_info = self.platform_info.as_ref()
            .ok_or_else(|| TransportError::Bootstrap("Platform not detected".to_string()))?;
        
        if let Some(custom_script) = &self.custom_script {
            return Ok(custom_script.clone());
        }
        
        // Choose the best available bootstrap method
        let method = platform_info.bootstrap_methods.first()
            .ok_or_else(|| TransportError::Bootstrap("No bootstrap methods available".to_string()))?;
        
        let script = match method {
            BootstrapMethod::MemfdCreate => self.generate_memfd_script(),
            BootstrapMethod::Python => self.generate_python_script(),
            BootstrapMethod::DevShm => self.generate_devshm_script(),
            BootstrapMethod::TempFile => self.generate_tempfile_script(),
            BootstrapMethod::Shell => self.generate_shell_script(),
        };
        
        debug!("Generated bootstrap script using method: {:?}", method);
        Ok(script)
    }
    
    /// Generate memfd_create bootstrap script
    fn generate_memfd_script(&self) -> String {
        r#"
set -e
python3 -c "
import os, sys, ctypes
try:
    libc = ctypes.CDLL('libc.so.6')
    fd = libc.syscall(319, b'mitoxide-agent', 1)  # memfd_create
    if fd >= 0:
        agent_data = sys.stdin.buffer.read()
        os.write(fd, agent_data)
        os.fexecve(fd, ['/proc/self/fd/%d' % fd], os.environ)
    else:
        raise Exception('memfd_create failed')
except Exception as e:
    print(f'memfd_create failed: {e}', file=sys.stderr)
    sys.exit(1)
"
        "#.trim().to_string()
    }
    
    /// Generate Python bootstrap script
    fn generate_python_script(&self) -> String {
        r#"
set -e
python3 -c "
import os, sys, tempfile, stat
try:
    with tempfile.NamedTemporaryFile(delete=False, mode='wb') as f:
        agent_data = sys.stdin.buffer.read()
        f.write(agent_data)
        f.flush()
        os.chmod(f.name, stat.S_IRWXU)
        os.execv(f.name, [f.name])
except Exception as e:
    print(f'Python bootstrap failed: {e}', file=sys.stderr)
    sys.exit(1)
"
        "#.trim().to_string()
    }
    
    /// Generate /dev/shm bootstrap script
    fn generate_devshm_script(&self) -> String {
        r#"
set -e
AGENT_PATH="/dev/shm/mitoxide-agent-$$-$(date +%s)"
cat > "$AGENT_PATH"
chmod +x "$AGENT_PATH"
exec "$AGENT_PATH"
        "#.trim().to_string()
    }
    
    /// Generate /tmp bootstrap script
    fn generate_tempfile_script(&self) -> String {
        r#"
set -e
AGENT_PATH="/tmp/mitoxide-agent-$$-$(date +%s)"
cat > "$AGENT_PATH"
chmod +x "$AGENT_PATH"
trap 'rm -f "$AGENT_PATH" 2>/dev/null || true' EXIT
exec "$AGENT_PATH"
        "#.trim().to_string()
    }
    
    /// Generate shell bootstrap script (fallback)
    fn generate_shell_script(&self) -> String {
        r#"
set -e
# Try to find a writable directory
for dir in /dev/shm /tmp /var/tmp; do
    if [ -d "$dir" ] && [ -w "$dir" ]; then
        AGENT_PATH="$dir/mitoxide-agent-$$-$(date +%s)"
        cat > "$AGENT_PATH"
        chmod +x "$AGENT_PATH"
        trap 'rm -f "$AGENT_PATH" 2>/dev/null || true' EXIT
        exec "$AGENT_PATH"
        break
    fi
done
echo "No writable directory found for agent bootstrap" >&2
exit 1
        "#.trim().to_string()
    }
    
    /// Execute bootstrap on the remote host
    pub async fn execute_bootstrap<T: Transport>(
        &self, 
        transport: &mut T, 
        agent_binary: &[u8]
    ) -> Result<(), TransportError> {
        let script = self.generate_bootstrap_script(agent_binary)?;
        
        info!("Executing agent bootstrap");
        debug!("Bootstrap script: {}", script);
        
        // This would be implemented by the specific transport
        transport.bootstrap_agent(agent_binary).await
    }
    
    /// Get platform information
    pub fn platform_info(&self) -> Option<&PlatformInfo> {
        self.platform_info.as_ref()
    }
    
    /// Helper method to execute commands (would be implemented by transport)
    async fn execute_command<T: Transport>(&self, _transport: &mut T, command: &str) -> Result<String, TransportError> {
        // This is a placeholder - in reality, we'd need a way to execute commands
        // through the transport without bootstrapping the agent
        debug!("Would execute command: {}", command);
        
        // For now, return mock responses based on command
        if command.contains("uname -m") {
            Ok("x86_64\nLinux\nUbuntu 20.04.3 LTS".to_string())
        } else if command.contains("python3 --version") {
            Ok("Python 3.8.10".to_string())
        } else if command.contains("memfd_create") {
            Ok("True".to_string())
        } else if command.contains("/dev/shm") || command.contains("/tmp") {
            Ok("available".to_string())
        } else {
            Err(TransportError::CommandFailed { 
                code: 1, 
                message: "Command not found".to_string() 
            })
        }
    }
}

impl Default for Bootstrap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Transport, TransportError, ConnectionInfo, TransportType};
    use async_trait::async_trait;
    
    // Mock transport for testing
    struct MockTransport {
        should_fail: bool,
    }
    
    impl MockTransport {
        fn new(should_fail: bool) -> Self {
            Self { should_fail }
        }
    }
    
    #[async_trait]
    impl Transport for MockTransport {
        async fn connect(&mut self) -> Result<crate::Connection, TransportError> {
            if self.should_fail {
                Err(TransportError::Connection("Mock connection failed".to_string()))
            } else {
                Ok(crate::Connection::new(None))
            }
        }
        
        async fn bootstrap_agent(&mut self, _agent_binary: &[u8]) -> Result<(), TransportError> {
            if self.should_fail {
                Err(TransportError::Bootstrap("Mock bootstrap failed".to_string()))
            } else {
                Ok(())
            }
        }
        
        fn connection_info(&self) -> ConnectionInfo {
            ConnectionInfo {
                host: "mock.example.com".to_string(),
                port: 22,
                username: "mockuser".to_string(),
                transport_type: TransportType::Local,
            }
        }
        
        async fn test_connection(&mut self) -> Result<(), TransportError> {
            if self.should_fail {
                Err(TransportError::Connection("Mock test failed".to_string()))
            } else {
                Ok(())
            }
        }
    }
    
    #[tokio::test]
    async fn test_bootstrap_creation() {
        let bootstrap = Bootstrap::new();
        assert!(bootstrap.platform_info.is_none());
        assert!(bootstrap.custom_script.is_none());
    }
    
    #[tokio::test]
    async fn test_custom_script() {
        let custom_script = "echo 'custom bootstrap'".to_string();
        let bootstrap = Bootstrap::new().with_custom_script(custom_script.clone());
        assert_eq!(bootstrap.custom_script, Some(custom_script));
    }
    
    #[tokio::test]
    async fn test_platform_detection() {
        let mut transport = MockTransport::new(false);
        let mut bootstrap = Bootstrap::new();
        
        let platform_info = bootstrap.detect_platform(&mut transport).await.unwrap();
        
        assert_eq!(platform_info.arch, "x86_64");
        assert_eq!(platform_info.os, "Linux");
        assert!(!platform_info.bootstrap_methods.is_empty());
    }
    
    #[test]
    fn test_bootstrap_method_detection() {
        let methods = vec![
            BootstrapMethod::MemfdCreate,
            BootstrapMethod::Python,
            BootstrapMethod::DevShm,
            BootstrapMethod::TempFile,
            BootstrapMethod::Shell,
        ];
        
        // Test that all methods are distinct
        for (i, method1) in methods.iter().enumerate() {
            for (j, method2) in methods.iter().enumerate() {
                if i != j {
                    assert_ne!(method1, method2);
                }
            }
        }
    }
    
    #[tokio::test]
    async fn test_script_generation() {
        let mut transport = MockTransport::new(false);
        let mut bootstrap = Bootstrap::new();
        
        // Detect platform first
        bootstrap.detect_platform(&mut transport).await.unwrap();
        
        // Generate bootstrap script
        let agent_binary = b"fake agent binary";
        let script = bootstrap.generate_bootstrap_script(agent_binary).unwrap();
        
        assert!(!script.is_empty());
        assert!(script.contains("set -e")); // Should have error handling
    }
    
    #[test]
    fn test_memfd_script_generation() {
        let bootstrap = Bootstrap::new();
        let script = bootstrap.generate_memfd_script();
        
        assert!(script.contains("memfd_create"));
        assert!(script.contains("python3"));
        assert!(script.contains("syscall(319"));
    }
    
    #[test]
    fn test_python_script_generation() {
        let bootstrap = Bootstrap::new();
        let script = bootstrap.generate_python_script();
        
        assert!(script.contains("tempfile"));
        assert!(script.contains("python3"));
        assert!(script.contains("os.execv"));
    }
    
    #[test]
    fn test_tempfile_script_generation() {
        let bootstrap = Bootstrap::new();
        let script = bootstrap.generate_tempfile_script();
        
        assert!(script.contains("/tmp"));
        assert!(script.contains("chmod +x"));
        assert!(script.contains("exec"));
    }
    
    #[test]
    fn test_shell_script_generation() {
        let bootstrap = Bootstrap::new();
        let script = bootstrap.generate_shell_script();
        
        assert!(script.contains("/dev/shm"));
        assert!(script.contains("/tmp"));
        assert!(script.contains("chmod +x"));
        assert!(script.contains("exec"));
    }
}