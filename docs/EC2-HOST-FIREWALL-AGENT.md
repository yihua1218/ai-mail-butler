# EC2 Host Firewall Agent

AI Mail Butler can detect malicious SMTP behavior inside the application, but Docker deployments should enforce IP blocking on the EC2 host before Docker forwards traffic to the container.

Traffic path:

```text
Internet -> AWS Security Group -> EC2 Host Firewall -> Docker NAT/Forwarding -> AI Mail Butler Container
```

## Recommended EC2 Setup

Run the SMTP/Web app in Docker without `NET_ADMIN`. Run the firewall agent as a host-level systemd service.

Current iptables-only EC2 hosts:

```yaml
backend:
  preferred: iptables-docker-user
  fallback: iptables-input
```

Future nftables-capable EC2 hosts:

```yaml
backend:
  preferred: nftables
  fallback: iptables-docker-user
```

The SMTP detector should use the host agent socket:

```env
SMTP_SECURITY_BLOCKING_BACKEND=host-agent
SMTP_SECURITY_TEMP_BLOCK_ENABLED=true
SMTP_FIREWALL_AGENT_SOCKET=/run/ai-mail-butler/firewall-agent.sock
```

## Host Agent

Default config:

```text
config/firewall-agent.yaml
```

Production path:

```text
/etc/ai-mail-butler/firewall-agent.yaml
```

Start manually:

```bash
ai-mail-butler --mode firewall-agent --firewall-config /etc/ai-mail-butler/firewall-agent.yaml
```

The agent listens on:

```text
/run/ai-mail-butler/firewall-agent.sock
```

## systemd Unit

```ini
[Unit]
Description=AI Mail Butler Firewall Agent
After=network.target docker.service
Wants=docker.service

[Service]
Type=simple
ExecStart=/usr/local/bin/ai-mail-butler --mode firewall-agent --firewall-config /etc/ai-mail-butler/firewall-agent.yaml
Restart=always
RestartSec=3
User=root
Group=root

RuntimeDirectory=ai-mail-butler
RuntimeDirectoryMode=0770
StateDirectory=ai-mail-butler
LogsDirectory=ai-mail-butler

[Install]
WantedBy=multi-user.target
```

## Admin CLI

The same binary can talk to the Unix socket:

```bash
ai-mail-butler --mode fw --fw-action health
ai-mail-butler --mode fw --fw-action list
ai-mail-butler --mode fw --fw-action block --ip 198.51.100.42 --duration 1h --reason "AUTH probing"
ai-mail-butler --mode fw --fw-action unblock --ip 198.51.100.42 --reason "manual review"
```

## Docker Notes

Mount only the socket directory into the app container:

```yaml
volumes:
  - /run/ai-mail-butler:/run/ai-mail-butler
```

Do not run the SMTP container with:

```text
--privileged
--cap-add=NET_ADMIN
-v /var/run/docker.sock:/var/run/docker.sock
```

The firewall agent, not the SMTP container, owns host firewall changes.
