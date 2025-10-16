#!/bin/bash
set -e

# Setup SSH authorized keys if provided
if [ -n "$SSH_AUTHORIZED_KEYS_FILE" ] && [ -f "$SSH_AUTHORIZED_KEYS_FILE" ]; then
    cp "$SSH_AUTHORIZED_KEYS_FILE" /home/testuser/.ssh/authorized_keys
    chown testuser:testuser /home/testuser/.ssh/authorized_keys
    chmod 600 /home/testuser/.ssh/authorized_keys
    
    # Also copy the private key for jump host functionality
    if [ -f "/etc/ssh/keys/test_key" ]; then
        cp /etc/ssh/keys/test_key /home/testuser/.ssh/id_rsa
        chown testuser:testuser /home/testuser/.ssh/id_rsa
        chmod 600 /home/testuser/.ssh/id_rsa
    fi
fi

# Generate host keys if they don't exist
if [ ! -f /etc/ssh/ssh_host_rsa_key ]; then
    ssh-keygen -A
fi

# Create /var/run/sshd if it doesn't exist
mkdir -p /var/run/sshd

# Start SSH daemon
exec /usr/sbin/sshd -D