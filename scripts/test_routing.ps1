#!/usr/bin/env pwsh
# Test script for routing functionality

Write-Host "🚀 Mitoxide Routing Tests" -ForegroundColor Green
Write-Host "=========================" -ForegroundColor Green

# Check prerequisites
Write-Host "Checking prerequisites..." -ForegroundColor Yellow

# Check Docker
try {
    docker --version | Out-Null
    Write-Host "✅ Docker is available" -ForegroundColor Green
} catch {
    Write-Host "❌ Docker is not available" -ForegroundColor Red
    exit 1
}

# Check docker-compose
try {
    docker-compose --version | Out-Null
    Write-Host "✅ Docker Compose is available" -ForegroundColor Green
} catch {
    Write-Host "❌ Docker Compose is not available" -ForegroundColor Red
    exit 1
}

# Check SSH keys
if (Test-Path "docker/ssh_keys/test_key") {
    Write-Host "✅ SSH keys are available" -ForegroundColor Green
} else {
    Write-Host "❌ SSH keys not found" -ForegroundColor Red
    Write-Host "Please run: docker/setup.ps1" -ForegroundColor Yellow
    exit 1
}

Write-Host ""
Write-Host "Running routing integration tests..." -ForegroundColor Yellow

# Run the routing tests
try {
    cargo test --package mitoxide --test routing_integration_tests -- --nocapture
    $exitCode = $LASTEXITCODE
    
    if ($exitCode -eq 0) {
        Write-Host ""
        Write-Host "🎉 All routing tests passed!" -ForegroundColor Green
    } else {
        Write-Host ""
        Write-Host "❌ Some routing tests failed" -ForegroundColor Red
        exit $exitCode
    }
} catch {
    Write-Host "❌ Failed to run routing tests: $_" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "Routing test summary:" -ForegroundColor Cyan
Write-Host "- Multi-hop SSH connections through bastion" -ForegroundColor White
Write-Host "- Connection routing and multiplexing" -ForegroundColor White
Write-Host "- Connection failure and recovery" -ForegroundColor White
Write-Host "- Load balancing and connection pooling" -ForegroundColor White
Write-Host "- Routing performance under load" -ForegroundColor White