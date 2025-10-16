#!/bin/bash
set -e

# Setup SSH authorized keys if provided
if [ -n "$SSH_AUTHORIZED_KEYS_FILE" ] && [ -f "$SSH_AUTHORIZED_KEYS_FILE" ]; then
    cp "$SSH_AUTHORIZED_KEYS_FILE" /home/testuser/.ssh/authorized_keys
    chown testuser:testuser /home/testuser/.ssh/authorized_keys
    chmod 600 /home/testuser/.ssh/authorized_keys
fi

# Generate host keys if they don't exist
if [ ! -f /etc/ssh/ssh_host_rsa_key ]; then
    ssh-keygen -A
fi

# Start SSH daemon
exec /usr/sbin/sshd -D -e