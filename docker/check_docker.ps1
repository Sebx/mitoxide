# Check Docker availability on Windows

Write-Host "Checking Docker availability..." -ForegroundColor Blue

# Check if Docker is installed
try {
    $dockerVersion = docker --version
    Write-Host "✅ Docker is installed: $dockerVersion" -ForegroundColor Green
} catch {
    Write-Host "❌ Docker is not installed" -ForegroundColor Red
    Write-Host "Please install Docker Desktop from: https://www.docker.com/products/docker-desktop" -ForegroundColor Yellow
    exit 1
}

# Check if Docker daemon is running
try {
    docker info | Out-Null
    Write-Host "✅ Docker daemon is running" -ForegroundColor Green
} catch {
    Write-Host "❌ Docker daemon is not running" -ForegroundColor Red
    Write-Host "Please start Docker Desktop and try again" -ForegroundColor Yellow
    exit 1
}

# Check if docker-compose is available
try {
    $composeVersion = docker-compose --version
    Write-Host "✅ Docker Compose is available: $composeVersion" -ForegroundColor Green
} catch {
    Write-Host "❌ docker-compose is not available" -ForegroundColor Red
    Write-Host "Please install docker-compose or use 'docker compose' (newer versions)" -ForegroundColor Yellow
    exit 1
}

# Test basic Docker functionality
Write-Host "Testing Docker functionality..." -ForegroundColor Blue
try {
    docker run --rm hello-world | Out-Null
    Write-Host "✅ Docker is working correctly" -ForegroundColor Green
} catch {
    Write-Host "❌ Docker test failed" -ForegroundColor Red
    exit 1
}

Write-Host "🚀 Ready to build and run test containers!" -ForegroundColor Green