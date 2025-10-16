# Implementation Plan

- [x] 1. Project Structure and Foundation Setup





  - Create Cargo workspace with all crates (mitoxide, mitoxide-agent, mitoxide-proto, mitoxide-ssh, mitoxide-wasm)
  - Set up rust-toolchain.toml for Rust 1.78+
  - Configure workspace dependencies and feature flags
  - Create basic directory structure with src/lib.rs files
  - _Requirements: 1.1, 1.2, 1.3, 1.4_

- [x] 2. Protocol Foundation and Frame Codec

  - [x] 2.1 Implement basic frame structure and serialization


    - Create Frame struct with stream_id, sequence, flags, and payload fields
    - Implement MessagePack serialization/deserialization for frames
    - Write unit tests for frame codec round-trip properties
    - _Requirements: 4.1, 4.2_

  - [x] 2.2 Create message types and enums


    - Define Request and Response enums with all operation types
    - Implement Message wrapper enum for protocol messages
    - Create error taxonomy with MitoxideError and sub-error types
    - Write unit tests for message serialization
    - _Requirements: 4.1, 8.5_

  - [x] 2.3 Implement frame codec with async streams


    - Create FrameCodec for encoding/decoding frames over async streams
    - Implement length-prefixed framing with proper error handling
    - Add unit tests for codec with various frame sizes and error conditions
    - _Requirements: 4.2, 4.3_

- [x] 3. Stream Multiplexing and Flow Control

  - [x] 3.1 Create stream multiplexer


    - Implement StreamMultiplexer for managing multiple logical streams
    - Add stream creation, routing, and cleanup functionality
    - Write unit tests for stream lifecycle management
    - _Requirements: 4.2, 4.3_

  - [x] 3.2 Implement credit-based flow control


    - Add flow control with credit windows and back-pressure handling
    - Implement stream-level and connection-level flow control
    - Create unit tests for back-pressure scenarios and deadlock prevention
    - _Requirements: 4.3, 9.4_

  - [x] 3.3 Add property-based tests for multiplexer


    - Use proptest to verify multiplexer invariants
    - Test concurrent stream operations and state transitions
    - Verify flow control properties under various conditions
    - _Requirements: 9.4_

- [x] 4. SSH Transport Layer Implementation


  - [x] 4.1 Create transport abstraction


    - Define Transport trait with connect and bootstrap methods
    - Implement StdioTransport using SSH subprocess
    - Add connection lifecycle management and error handling
    - Write unit tests with mocked SSH processes
    - _Requirements: 2.1, 2.2, 2.4_

  - [x] 4.2 Implement agent bootstrap logic


    - Create platform detection (uname -m, uname -s)
    - Implement agent binary transfer over SSH
    - Add bootstrap sequence with proper error handling
    - Write unit tests for bootstrap with various platforms
    - _Requirements: 3.1, 3.2, 3.3, 3.4_

  - [x] 4.3 Add SSH connection management


    - Implement connection pooling and reuse
    - Add SSH configuration and authentication handling
    - Create connection health checking and recovery
    - Write integration tests with Docker SSH containers
    - _Requirements: 2.3, 5.4_

- [x] 5. Agent Binary Core Implementation





  - [x] 5.1 Create agent main loop and frame processing


    - Implement AgentLoop reading frames from stdin
    - Add frame dispatching to appropriate handlers
    - Create graceful shutdown and error recovery
    - Write unit tests for agent loop with mock stdin/stdout
    - _Requirements: 3.1, 4.4_

  - [x] 5.2 Implement basic request handlers


    - Create Handler trait and ProcessHandler implementation
    - Add FileHandler for file operations (get/put)
    - Implement basic error handling and response generation
    - Write unit tests for each handler with various inputs
    - _Requirements: 6.1, 6.3_

  - [x] 5.3 Add agent router and stream management


    - Implement agent-side router for multiplexed streams
    - Add stream correlation and response routing
    - Create handler registration and dispatch system
    - Write unit tests for routing with concurrent requests
    - _Requirements: 5.3, 4.4_

- [x] 6. Process Execution and System Operations





  - [x] 6.1 Implement process execution handler


    - Create ProcessHandler with command execution
    - Add environment variable passthrough and working directory support
    - Implement streaming stdout/stderr capture
    - Write unit tests for process execution with various commands
    - _Requirements: 6.1, 6.2_

  - [x] 6.2 Add file operations handler


    - Implement FileHandler with get/put operations
    - Add file metadata handling and permissions
    - Create directory operations and recursive transfers
    - Write unit tests for file operations with various scenarios
    - _Requirements: 6.3_

  - [x] 6.3 Implement PTY and privilege escalation


    - Create PTY handler for interactive commands
    - Add sudo/su/doas prompt detection with configurable patterns
    - Implement privilege escalation with credential handling
    - Write unit tests for PTY operations and privilege escalation
    - _Requirements: 6.4_

- [x] 7. WASM Runtime Integration





  - [x] 7.1 Create WASM module loading and validation


    - Implement WasmModule struct with metadata
    - Add module loading from bytes and file
    - Create module validation and capability checking
    - Write unit tests for module loading with valid/invalid modules
    - _Requirements: 7.1, 7.3_

  - [x] 7.2 Implement WASM execution runtime


    - Create WasmRuntime with wasmtime integration
    - Add WASI support with JSON stdin/stdout
    - Implement execution sandboxing and resource limits
    - Write unit tests for WASM execution with test modules
    - _Requirements: 7.2, 7.3_

  - [x] 7.3 Add WASM handler to agent


    - Create WasmHandler for agent-side WASM execution
    - Integrate WASM runtime with agent request processing
    - Add module caching and hash verification
    - Write integration tests for end-to-end WASM execution
    - _Requirements: 7.1, 7.2, 7.4_

- [x] 8. SDK Client Library Implementation





  - [x] 8.1 Create Session and connection management


    - Implement Session struct with SSH connection builder
    - Add connection establishment and agent bootstrapping
    - Create session lifecycle management and cleanup
    - Write unit tests for session creation and management
    - _Requirements: 8.1, 8.2_

  - [x] 8.2 Implement Context and operation methods


    - Create Context struct with RPC client functionality
    - Add proc_exec, put/get, call_json, call_wasm methods
    - Implement request/response correlation and error handling
    - Write unit tests for all Context operations
    - _Requirements: 8.3, 8.4_

  - [x] 8.3 Add Router and connection routing


    - Implement client-side Router for connection management
    - Add support for jump hosts and multi-hop connections
    - Create connection pooling and load balancing
    - Write unit tests for routing with various topologies
    - _Requirements: 5.1, 5.2_

- [x] 9. Docker Test Environment Setup





  - [x] 9.1 Create Docker test containers


    - Build alpine_ro container with read-only filesystem
    - Create ubuntu_min container with SSH server
    - Add bastion container for jump host testing
    - Configure SSH keys and known_hosts for container access
    - _Requirements: 10.1, 10.3_

  - [x] 9.2 Implement integration test framework


    - Create test harness for Docker container management
    - Add SSH connection helpers for test containers
    - Implement test utilities for file operations and assertions
    - Write basic connectivity tests for all container types
    - _Requirements: 10.4_

  - [x] 9.3 Add constraint testing scenarios


    - Create tests for read-only filesystem constraints
    - Add low memory limit testing scenarios
    - Implement network isolation and failure testing
    - Write tests for resource exhaustion and recovery
    - _Requirements: 10.2, 10.4_


- [x] 10. Comprehensive Integration Testing






  - [x] 10.1 Test agent bootstrap scenarios


    - Test memfd_create bootstrap on Linux containers
    - Verify /tmp fallback when memfd unavailable
    - Test bootstrap failure and recovery scenarios
    - Verify agent self-deletion and cleanup
    - _Requirements: 10.4, 3.1, 3.2_

  - [x] 10.2 Test process execution and I/O handling


    - Test large stdout/stderr streaming
    - Verify environment variable passthrough
    - Test binary data handling and encoding
    - Test process timeout and cancellation
    - _Requirements: 10.4, 6.1, 6.2_

  - [x] 10.3 Test WASM execution end-to-end


    - Create test WASM modules for various scenarios
    - Test JSON input/output serialization
    - Verify WASM sandboxing and resource limits
    - Test WASM error handling and recovery
    - _Requirements: 10.4, 7.2_

  - [x] 10.4 Test privilege escalation and PTY


    - Test sudo prompt detection and handling
    - Verify PTY operations with interactive commands
    - Test privilege escalation failure scenarios
    - Test credential handling and security
    - _Requirements: 10.4, 6.4_

  - [x] 10.5 Test jump host and routing



    - Test multi-hop SSH connections through bastion
    - Verify connection routing and multiplexing
    - Test connection failure and recovery
    - Test load balancing and connection pooling
    - _Requirements: 10.4, 5.2_

- [ ] 11. Performance Testing and Benchmarks

  - [ ] 11.1 Create RPC latency benchmarks
    - Implement Criterion benchmarks for request/response latency
    - Measure p50, p95, p99 latencies for various operations
    - Create baseline measurements and regression detection
    - _Requirements: 9.5, 14.1_

  - [ ] 11.2 Add throughput and scalability benchmarks
    - Test concurrent connection handling
    - Measure message throughput under load
    - Test memory usage patterns and leak detection
    - Create performance regression tests
    - _Requirements: 14.2, 14.3, 14.4_

- [ ] 12. Documentation and Examples

  - [ ] 12.1 Create comprehensive README
    - Write project introduction and quickstart guide
    - Add feature matrix and platform support documentation
    - Include troubleshooting and FAQ sections
    - Add architecture diagrams using Mermaid
    - _Requirements: 12.1, 12.2_

  - [ ] 12.2 Implement runnable examples
    - Create fanout_uname example for basic usage
    - Add WASM task execution example
    - Implement jump host traversal example
    - Add privilege escalation example with sudo
    - _Requirements: 12.1_

  - [ ] 12.3 Generate API documentation
    - Add comprehensive rustdoc comments to all public APIs
    - Create module-level documentation with examples
    - Add doctests for all public methods
    - Configure doc generation and publishing
    - _Requirements: 12.4_

- [ ] 13. CI/CD Pipeline Implementation

  - [ ] 13.1 Create GitHub Actions CI workflow
    - Set up matrix builds for Linux and macOS
    - Add cargo fmt, clippy, and cargo deny checks
    - Implement unit test execution with coverage
    - Add integration test execution with Docker
    - _Requirements: 11.1_

  - [ ] 13.2 Implement release automation
    - Create release workflow triggered by version tags
    - Add multi-platform binary builds for mitoxide-agent
    - Implement changelog generation using Conventional Commits
    - Add GitHub release creation with assets
    - _Requirements: 11.2, 11.3_

  - [ ] 13.3 Add security and quality checks
    - Integrate cargo-audit for vulnerability scanning
    - Add dependency license checking
    - Implement MSRV (Minimum Supported Rust Version) checking
    - Add unused dependency detection
    - _Requirements: 11.5, 13.3_

- [ ] 14. Security Implementation and Compliance

  - [ ] 14.1 Implement agent binary verification
    - Add hash verification for agent binaries
    - Implement optional signature checking
    - Create key management for signature verification
    - Write tests for verification success and failure cases
    - _Requirements: 3.5, 13.4_

  - [ ] 14.2 Add security documentation
    - Create SECURITY.md with vulnerability reporting process
    - Document security model and threat analysis
    - Add security best practices guide
    - Create compliance documentation for dual licensing
    - _Requirements: 13.1, 13.2_

- [ ] 15. Final Integration and Polish
  - [ ] 15.1 End-to-end system testing
    - Create comprehensive system tests covering all features
    - Test complete workflows from SDK to agent execution
    - Verify error handling and recovery across all components
    - Test performance under realistic load scenarios
    - _Requirements: 9.1, 9.2_

  - [ ] 15.2 Developer experience improvements
    - Create Makefile with common development tasks
    - Add pre-commit hooks for formatting and linting
    - Implement cargo feature documentation and examples
    - Add development setup and contribution guidelines
    - _Requirements: 12.1, 12.3_

  - [ ] 15.3 Production readiness verification
    - Verify all acceptance criteria are met
    - Test deployment scenarios and packaging
    - Validate documentation completeness and accuracy
    - Perform final security and performance review
    - _Requirements: All requirements verification_