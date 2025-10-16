# Alpine container with read-only filesystem for constraint testing
FROM alpine:3.18

# Install minimal SSH server and required packages
RUN apk add --no-cache \
    openssh-server \
    openssh-client \
    bash \
    coreutils \
    procps \
    && rm -rf /var/cache/apk/*

# Create SSH host keys
RUN ssh-keygen -A

# Create test user
RUN adduser -D -s /bin/bash testuser

# Create SSH directory for testuser
RUN mkdir -p /home/testuser/.ssh \
    && chown testuser:testuser /home/testuser/.ssh \
    && chmod 700 /home/testuser/.ssh

# Configure SSH server
RUN echo "Port 22" >> /etc/ssh/sshd_config \
    && echo "PermitRootLogin no" >> /etc/ssh/sshd_config \
    && echo "PasswordAuthentication no" >> /etc/ssh/sshd_config \
    && echo "PubkeyAuthentication yes" >> /etc/ssh/sshd_config \
    && echo "AuthorizedKeysFile /home/testuser/.ssh/authorized_keys" >> /etc/ssh/sshd_config \
    && echo "Subsystem sftp /usr/lib/openssh/sftp-server" >> /etc/ssh/sshd_config

# Create startup script
COPY docker/scripts/alpine_entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Create directories that will be writable in tmpfs
RUN mkdir -p /tmp /run /var/run /var/log \
    && chmod 1777 /tmp

EXPOSE 22

USER root
ENTRYPOINT ["/entrypoint.sh"]