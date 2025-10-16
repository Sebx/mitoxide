//! Dedicated integration tests for jump host and routing functionality
//! 
//! This test suite focuses specifically on multi-hop SSH connections,
//! connection routing, multiplexing, failure recovery, and load balancing.

mod integration;

use integration::routing_tests::RoutingTests;
use anyhow::Result;

/// Run all routing tests
#[tokio::test]
async fn test_all_routing_functionality() -> Result<()> {
    let routing_tests = RoutingTests::new();
    routing_tests.run_all_tests().await
}

/// Test multi-hop SSH connections through bastion
#[tokio::test]
async fn test_multi_hop_connections() -> Result<()> {
    let routing_tests = RoutingTests::new();
    routing_tests.setup().await?;
    
    let result = routing_tests.test_multi_hop_connections().await;
    
    routing_tests.cleanup().await?;
    result
}

/// Test connection routing and multiplexing
#[tokio::test]
async fn test_connection_routing_multiplexing() -> Result<()> {
    let routing_tests = RoutingTests::new();
    routing_tests.setup().await?;
    
    let result = routing_tests.test_connection_routing_multiplexing().await;
    
    routing_tests.cleanup().await?;
    result
}

/// Test connection failure and recovery
#[tokio::test]
async fn test_connection_failure_recovery() -> Result<()> {
    let routing_tests = RoutingTests::new();
    routing_tests.setup().await?;
    
    let result = routing_tests.test_connection_failure_recovery().await;
    
    routing_tests.cleanup().await?;
    result
}

/// Test load balancing and connection pooling
#[tokio::test]
async fn test_load_balancing_connection_pooling() -> Result<()> {
    let routing_tests = RoutingTests::new();
    routing_tests.setup().await?;
    
    let result = routing_tests.test_load_balancing_connection_pooling().await;
    
    routing_tests.cleanup().await?;
    result
}

/// Test routing performance under load
#[tokio::test]
async fn test_routing_performance() -> Result<()> {
    let routing_tests = RoutingTests::new();
    routing_tests.setup().await?;
    
    let result = routing_tests.test_routing_performance().await;
    
    routing_tests.cleanup().await?;
    result
}