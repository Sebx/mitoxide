# Bastion host for jump host testing
FROM ubuntu:22.04

# Prevent interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Install SSH server and client packages
RUN apt-get update && apt-get install -y \
    openssh-server \
    openssh-client \
    sudo \
    netcat-openbsd \
    curl \
    wget \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create SSH host keys
RUN ssh-keygen -A

# Create test user with sudo privileges
RUN useradd -m -s /bin/bash -G sudo testuser \
    && echo "testuser ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Create SSH directory for testuser
RUN mkdir -p /home/testuser/.ssh \
    && chown testuser:testuser /home/testuser/.ssh \
    && chmod 700 /home/testuser/.ssh

# Configure SSH server for bastion functionality
RUN mkdir -p /var/run/sshd \
    && echo "Port 22" >> /etc/ssh/sshd_config \
    && echo "PermitRootLogin no" >> /etc/ssh/sshd_config \
    && echo "PasswordAuthentication no" >> /etc/ssh/sshd_config \
    && echo "PubkeyAuthentication yes" >> /etc/ssh/sshd_config \
    && echo "AuthorizedKeysFile /home/testuser/.ssh/authorized_keys" >> /etc/ssh/sshd_config \
    && echo "Subsystem sftp /usr/lib/openssh/sftp-server" >> /etc/ssh/sshd_config \
    && echo "AllowTcpForwarding yes" >> /etc/ssh/sshd_config \
    && echo "GatewayPorts no" >> /etc/ssh/sshd_config \
    && echo "X11Forwarding no" >> /etc/ssh/sshd_config

# Configure SSH client for jump host functionality
RUN echo "Host backend_target" >> /etc/ssh/ssh_config \
    && echo "    HostName mitoxide_backend_target" >> /etc/ssh/ssh_config \
    && echo "    User testuser" >> /etc/ssh/ssh_config \
    && echo "    IdentityFile /etc/ssh/keys/test_key" >> /etc/ssh/ssh_config \
    && echo "    StrictHostKeyChecking no" >> /etc/ssh/ssh_config \
    && echo "    UserKnownHostsFile /dev/null" >> /etc/ssh/ssh_config

# Create startup script
COPY docker/scripts/bastion_entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

EXPOSE 22

ENTRYPOINT ["/entrypoint.sh"]