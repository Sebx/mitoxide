#!/bin/bash
set -e

# Create SSH keys directory
mkdir -p docker/ssh_keys

# Generate SSH key pair for testing
if [ ! -f docker/ssh_keys/test_key ]; then
    ssh-keygen -t rsa -b 2048 -f docker/ssh_keys/test_key -N "" -C "mitoxide-test-key"
    echo "Generated SSH key pair for testing"
else
    echo "SSH key pair already exists"
fi

# Set proper permissions
chmod 600 docker/ssh_keys/test_key
chmod 644 docker/ssh_keys/test_key.pub

echo "SSH keys are ready in docker/ssh_keys/"
echo "Public key:"
cat docker/ssh_keys/test_key.pub