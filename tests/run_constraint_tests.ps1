# Run Mitoxide Constraint Tests

Write-Host "🧪 Running Mitoxide Constraint Tests" -ForegroundColor Blue
Write-Host "====================================" -ForegroundColor Blue

# Check prerequisites
Write-Host "Checking prerequisites..." -ForegroundColor Yellow

try {
    docker --version | Out-Null
    Write-Host "✅ Docker is available" -ForegroundColor Green
} catch {
    Write-Host "❌ Docker is not installed" -ForegroundColor Red
    exit 1
}

try {
    docker info | Out-Null
    Write-Host "✅ Docker daemon is running" -ForegroundColor Green
} catch {
    Write-Host "❌ Docker daemon is not running" -ForegroundColor Red
    exit 1
}

try {
    docker-compose --version | Out-Null
    Write-Host "✅ docker-compose is available" -ForegroundColor Green
} catch {
    Write-Host "❌ docker-compose is not available" -ForegroundColor Red
    exit 1
}

# Setup test environment
Write-Host ""
Write-Host "Setting up test environment..." -ForegroundColor Yellow
Set-Location (Split-Path $PSScriptRoot -Parent)

# Generate SSH keys if needed
if (-not (Test-Path "docker/ssh_keys/test_key")) {
    Write-Host "Generating SSH keys..." -ForegroundColor Yellow
    New-Item -ItemType Directory -Path "docker/ssh_keys" -Force | Out-Null
    ssh-keygen -t rsa -b 2048 -f docker/ssh_keys/test_key -N '""' -C "mitoxide-test-key"
}

# Build and start containers
Write-Host "Building and starting Docker containers..." -ForegroundColor Yellow
docker-compose build
docker-compose up -d

# Wait for containers to be ready
Write-Host "Waiting for containers to be ready..." -ForegroundColor Yellow
Start-Sleep -Seconds 10

# Run constraint tests
Write-Host ""
Write-Host "Running constraint tests..." -ForegroundColor Blue
Write-Host "==========================" -ForegroundColor Blue

# Test categories
$Tests = @(
    "test_readonly_filesystem_constraints",
    "test_memory_limit_constraints", 
    "test_network_isolation",
    "test_resource_exhaustion_recovery",
    "test_concurrent_connection_stress",
    "test_container_restart_recovery"
)

$Passed = 0
$Failed = 0
$FailedTests = @()

foreach ($test in $Tests) {
    Write-Host ""
    Write-Host "Running $test..." -ForegroundColor Cyan
    
    $result = cargo test --test constraint_tests "$test" -- --nocapture
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ $test PASSED" -ForegroundColor Green
        $Passed++
    } else {
        Write-Host "❌ $test FAILED" -ForegroundColor Red
        $Failed++
        $FailedTests += $test
    }
}

# Run integration tests as well
Write-Host ""
Write-Host "Running integration tests..." -ForegroundColor Cyan
$result = cargo test --test integration_tests -- --nocapture
if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ Integration tests PASSED" -ForegroundColor Green
    $Passed++
} else {
    Write-Host "❌ Integration tests FAILED" -ForegroundColor Red
    $Failed++
    $FailedTests += "integration_tests"
}

# Cleanup
Write-Host ""
Write-Host "Cleaning up..." -ForegroundColor Yellow
docker-compose down

# Summary
Write-Host ""
Write-Host "Test Summary" -ForegroundColor Blue
Write-Host "============" -ForegroundColor Blue
Write-Host "Passed: $Passed" -ForegroundColor Green
Write-Host "Failed: $Failed" -ForegroundColor Red
Write-Host "Total:  $($Passed + $Failed)" -ForegroundColor White

if ($Failed -gt 0) {
    Write-Host ""
    Write-Host "Failed tests:" -ForegroundColor Red
    foreach ($test in $FailedTests) {
        Write-Host "  - $test" -ForegroundColor Red
    }
    Write-Host ""
    Write-Host "❌ Some tests failed" -ForegroundColor Red
    exit 1
} else {
    Write-Host ""
    Write-Host "✅ All tests passed!" -ForegroundColor Green
    exit 0
}