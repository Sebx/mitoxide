#!/usr/bin/env pwsh
# Publishing script for Mitoxide crates

Write-Host "🚀 Mitoxide Publishing Script" -ForegroundColor Green
Write-Host "=============================" -ForegroundColor Green

# Check if logged in to crates.io
Write-Host "Checking crates.io authentication..." -ForegroundColor Yellow

try {
    $whoami = cargo whoami 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ Logged in to crates.io as: $whoami" -ForegroundColor Green
    } else {
        Write-Host "❌ Not logged in to crates.io" -ForegroundColor Red
        Write-Host "Please run: cargo login <your-token>" -ForegroundColor Yellow
        Write-Host "Get your token from: https://crates.io/me" -ForegroundColor Yellow
        exit 1
    }
} catch {
    Write-Host "❌ Error checking crates.io authentication: $_" -ForegroundColor Red
    exit 1
}

# Check if all crates build successfully
Write-Host ""
Write-Host "Building all crates..." -ForegroundColor Yellow

try {
    cargo build --workspace --release
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Build failed" -ForegroundColor Red
        exit 1
    }
    Write-Host "✅ All crates build successfully" -ForegroundColor Green
} catch {
    Write-Host "❌ Build error: $_" -ForegroundColor Red
    exit 1
}

# Run tests
Write-Host ""
Write-Host "Running tests..." -ForegroundColor Yellow

try {
    cargo test --workspace
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Tests failed" -ForegroundColor Red
        exit 1
    }
    Write-Host "✅ All tests pass" -ForegroundColor Green
} catch {
    Write-Host "❌ Test error: $_" -ForegroundColor Red
    exit 1
}

# Publish crates in dependency order
$crates = @(
    "mitoxide-proto",
    "mitoxide-wasm", 
    "mitoxide-ssh",
    "mitoxide-agent",
    "mitoxide"
)

Write-Host ""
Write-Host "Publishing crates in dependency order..." -ForegroundColor Yellow

foreach ($crate in $crates) {
    Write-Host ""
    Write-Host "📦 Publishing $crate..." -ForegroundColor Cyan
    
    try {
        # Dry run first
        Write-Host "  Running dry-run for $crate..." -ForegroundColor Gray
        cargo publish --dry-run --manifest-path "crates/$crate/Cargo.toml"
        
        if ($LASTEXITCODE -ne 0) {
            Write-Host "❌ Dry-run failed for $crate" -ForegroundColor Red
            exit 1
        }
        
        # Ask for confirmation
        $confirm = Read-Host "  Publish $crate to crates.io? (y/N)"
        if ($confirm -eq "y" -or $confirm -eq "Y") {
            # Actual publish
            cargo publish --manifest-path "crates/$crate/Cargo.toml"
            
            if ($LASTEXITCODE -eq 0) {
                Write-Host "✅ Successfully published $crate" -ForegroundColor Green
                
                # Wait a bit for crates.io to process
                Write-Host "  Waiting 30 seconds for crates.io to process..." -ForegroundColor Gray
                Start-Sleep -Seconds 30
            } else {
                Write-Host "❌ Failed to publish $crate" -ForegroundColor Red
                exit 1
            }
        } else {
            Write-Host "⏭️  Skipped $crate" -ForegroundColor Yellow
        }
    } catch {
        Write-Host "❌ Error publishing $crate: $_" -ForegroundColor Red
        exit 1
    }
}

Write-Host ""
Write-Host "🎉 Publishing process completed!" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "1. Check your crates on https://crates.io/users/yourusername" -ForegroundColor White
Write-Host "2. Verify documentation builds on https://docs.rs" -ForegroundColor White
Write-Host "3. Update your GitHub repository with the published versions" -ForegroundColor White
Write-Host "4. Create a GitHub release with release notes" -ForegroundColor White