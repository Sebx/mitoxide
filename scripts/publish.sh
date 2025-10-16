#!/bin/bash
# Publishing script for Mitoxide crates

echo "🚀 Mitoxide Publishing Script"
echo "============================="

# Check if logged in to crates.io
echo "Checking crates.io authentication..."

if cargo whoami > /dev/null 2>&1; then
    whoami_result=$(cargo whoami)
    echo "✅ Logged in to crates.io as: $whoami_result"
else
    echo "❌ Not logged in to crates.io"
    echo "Please run: cargo login <your-token>"
    echo "Get your token from: https://crates.io/me"
    exit 1
fi

# Check if all crates build successfully
echo ""
echo "Building all crates..."

if cargo build --workspace --release; then
    echo "✅ All crates build successfully"
else
    echo "❌ Build failed"
    exit 1
fi

# Run tests
echo ""
echo "Running tests..."

if cargo test --workspace; then
    echo "✅ All tests pass"
else
    echo "❌ Tests failed"
    exit 1
fi

# Publish crates in dependency order
crates=(
    "mitoxide-proto"
    "mitoxide-wasm"
    "mitoxide-ssh"
    "mitoxide-agent"
    "mitoxide"
)

echo ""
echo "Publishing crates in dependency order..."

for crate in "${crates[@]}"; do
    echo ""
    echo "📦 Publishing $crate..."
    
    # Dry run first
    echo "  Running dry-run for $crate..."
    if ! cargo publish --dry-run --manifest-path "crates/$crate/Cargo.toml"; then
        echo "❌ Dry-run failed for $crate"
        exit 1
    fi
    
    # Ask for confirmation
    read -p "  Publish $crate to crates.io? (y/N): " confirm
    if [[ $confirm == [yY] ]]; then
        # Actual publish
        if cargo publish --manifest-path "crates/$crate/Cargo.toml"; then
            echo "✅ Successfully published $crate"
            
            # Wait a bit for crates.io to process
            echo "  Waiting 30 seconds for crates.io to process..."
            sleep 30
        else
            echo "❌ Failed to publish $crate"
            exit 1
        fi
    else
        echo "⏭️  Skipped $crate"
    fi
done

echo ""
echo "🎉 Publishing process completed!"
echo ""
echo "Next steps:"
echo "1. Check your crates on https://crates.io/users/yourusername"
echo "2. Verify documentation builds on https://docs.rs"
echo "3. Update your GitHub repository with the published versions"
echo "4. Create a GitHub release with release notes"