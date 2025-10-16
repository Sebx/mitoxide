# Setup Mitoxide Docker Test Environment

Write-Host "🐳 Setting up Mitoxide Docker Test Environment" -ForegroundColor Blue
Write-Host "==============================================" -ForegroundColor Blue

# Check Docker availability
& "docker/check_docker.ps1"

# Generate SSH keys if they don't exist
if (-not (Test-Path "docker/ssh_keys/test_key")) {
    Write-Host "🔑 Generating SSH keys..." -ForegroundColor Yellow
    ssh-keygen -t rsa -b 2048 -f docker/ssh_keys/test_key -N '""' -C "mitoxide-test-key"
} else {
    Write-Host "✅ SSH keys already exist" -ForegroundColor Green
}

# Build Docker images
Write-Host "🏗️  Building Docker images..." -ForegroundColor Yellow
docker-compose build

# Start containers
Write-Host "🚀 Starting containers..." -ForegroundColor Yellow
docker-compose up -d

# Wait for containers to be ready
Write-Host "⏳ Waiting for containers to be ready..." -ForegroundColor Yellow
Start-Sleep -Seconds 10

# Test connectivity
Write-Host "🔍 Testing SSH connectivity..." -ForegroundColor Yellow

Write-Host "Testing Alpine RO (port 2222):" -ForegroundColor Cyan
try {
    ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -i docker/ssh_keys/test_key -p 2222 testuser@localhost "uname -a"
    Write-Host "✅ Alpine RO connection successful" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to connect to alpine_ro" -ForegroundColor Red
}

Write-Host "Testing Ubuntu Min (port 2223):" -ForegroundColor Cyan
try {
    ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -i docker/ssh_keys/test_key -p 2223 testuser@localhost "uname -a"
    Write-Host "✅ Ubuntu Min connection successful" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to connect to ubuntu_min" -ForegroundColor Red
}

Write-Host "Testing Bastion (port 2224):" -ForegroundColor Cyan
try {
    ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -i docker/ssh_keys/test_key -p 2224 testuser@localhost "uname -a"
    Write-Host "✅ Bastion connection successful" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to connect to bastion" -ForegroundColor Red
}

Write-Host ""
Write-Host "✅ Docker test environment is ready!" -ForegroundColor Green
Write-Host ""
Write-Host "Available containers:" -ForegroundColor Blue
Write-Host "  - alpine_ro:    localhost:2222 (read-only filesystem, memory constrained)" -ForegroundColor White
Write-Host "  - ubuntu_min:   localhost:2223 (standard Ubuntu environment)" -ForegroundColor White
Write-Host "  - bastion:      localhost:2224 (jump host for backend access)" -ForegroundColor White
Write-Host "  - backend_target: (accessible only through bastion)" -ForegroundColor White
Write-Host ""
Write-Host "SSH key: docker/ssh_keys/test_key" -ForegroundColor Blue
Write-Host "SSH user: testuser" -ForegroundColor Blue
Write-Host ""
Write-Host "Usage examples:" -ForegroundColor Blue
Write-Host "  ssh -i docker/ssh_keys/test_key -p 2222 testuser@localhost  # Alpine RO" -ForegroundColor Gray
Write-Host "  ssh -i docker/ssh_keys/test_key -p 2223 testuser@localhost  # Ubuntu Min" -ForegroundColor Gray
Write-Host "  ssh -i docker/ssh_keys/test_key -p 2224 testuser@localhost  # Bastion" -ForegroundColor Gray
Write-Host ""
Write-Host "Management commands:" -ForegroundColor Blue
Write-Host "  docker-compose ps           # Check container status" -ForegroundColor Gray
Write-Host "  docker-compose logs         # View all logs" -ForegroundColor Gray
Write-Host "  docker-compose down         # Stop containers" -ForegroundColor Gray
Write-Host "  docker-compose down -v --remove-orphans && docker system prune -f  # Clean up everything" -ForegroundColor Gray