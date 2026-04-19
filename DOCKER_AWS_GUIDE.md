# AI Mail Butler: AWS EC2 Docker Deployment Guide

This document explains how to install Docker and deploy the **AI Mail Butler** service on an AWS EC2 instance (Amazon Linux 2023 or Ubuntu).

---

## 1. Install Docker (AWS EC2)

### For Amazon Linux 2023 (Recommended)
```bash
# Update the system
sudo dnf update -y

# Install Docker
sudo dnf install -y docker

# Start Docker service
sudo systemctl start docker
sudo systemctl enable docker

# Add the current user (ec2-user) to the docker group to run commands without sudo
sudo usermod -aG docker ec2-user

# *** IMPORTANT: After running the above, reconnect your SSH session for group changes to take effect ***
```

### For Ubuntu 22.04+
```bash
sudo apt update
sudo apt install -y docker.io
sudo systemctl start docker
sudo systemctl enable docker
sudo usermod -aG docker ubuntu
# Reconnect your SSH session
```

---

## 2. Install Docker Compose

Docker Compose is useful for managing multi-container applications.

```bash
# Download Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose

# Grant execution permissions
sudo chmod +x /usr/local/bin/docker-compose

# Verify installation
docker-compose --version
```

---

## 3. Deploy AI Mail Butler

### Step A: Obtain the Project
You can clone the source code to EC2 or build the image locally and push it. Using the provided `Dockerfile` for local building on EC2 is recommended.

```bash
# Create project directory
mkdir ~/ai-mail-butler && cd ~/ai-mail-butler

# (Clone via git or upload files to this directory)
```

### Step B: Configure Environment Variables
Create a `.env` file with your settings:
```bash
cat <<EOF > .env
DATABASE_URL=sqlite:data/data.sqlite
PORT=3000
ADMIN_EMAIL=your-admin@example.com
ASSISTANT_EMAIL=assistant@yourdomain.com
AI_API_BASE_URL=http://your-ai-server-ip:1234/v1
AI_MODEL_NAME=your-model-name
SMTP_RELAY_HOST=smtp.gmail.com
SMTP_RELAY_PORT=587
SMTP_RELAY_USER=your-email@gmail.com
SMTP_RELAY_PASS=your-app-password
EOF
```

### Step C: Create Dockerfile
If the project doesn't have one yet, use the following content:
```dockerfile
FROM rust:1.81-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/ai-mail-butler /app/
COPY --from=builder /app/frontend/dist /app/frontend/dist
EXPOSE 3000 25
CMD ["./ai-mail-butler"]
```

### Step D: Start the Service
```bash
# Build and run
docker build -t ai-mail-butler .
docker run -d \
  --name ai-mail-butler \
  -p 80:3000 \
  -p 25:25 \
  --env-file .env \
  -v $(pwd)/data:/app/data \
  ai-mail-butler
```

---

## 4. AWS Security Group Configuration

Ensure the following ports are open in your AWS EC2 Security Group:

| Type | Port | Description |
| :--- | :--- | :--- |
| HTTP | 80 | Access the Dashboard web UI |
| SMTP | 25 | Receive forwarded emails |
| SSH | 22 | Server management |

---

## 5. Notes & Troubleshooting
- **Data Persistence**: Always use the `-v` flag to mount the database directory to the host to ensure data isn't lost when the container is updated.
- **AI Backend**: If your AI model (e.g., LM Studio) is running on a different machine, ensure `AI_API_BASE_URL` points to the correct reachable IP address.
- **Firewall**: Ensure both AWS Security Groups and the OS-level firewall (like `iptables` or `ufw`) allow traffic on port 25.
