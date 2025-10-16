# Ubuntu minimal container with SSH server
FROM ubuntu:22.04

# Prevent interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Install minimal SSH server and required packages
RUN apt-get update && apt-get install -y \
    openssh-server \
    openssh-client \
    sudo \
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

# Configure SSH server
RUN mkdir -p /var/run/sshd \
    && echo "Port 22" >> /etc/ssh/sshd_config \
    && echo "PermitRootLogin no" >> /etc/ssh/sshd_config \
    && echo "PasswordAuthentication no" >> /etc/ssh/sshd_config \
    && echo "PubkeyAuthentication yes" >> /etc/ssh/sshd_config \
    && echo "AuthorizedKeysFile /home/testuser/.ssh/authorized_keys" >> /etc/ssh/sshd_config \
    && echo "Subsystem sftp /usr/lib/openssh/sftp-server" >> /etc/ssh/sshd_config

# Create startup script
COPY docker/scripts/ubuntu_entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

EXPOSE 22

ENTRYPOINT ["/entrypoint.sh"]