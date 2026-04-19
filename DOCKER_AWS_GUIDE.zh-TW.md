# AI Mail Butler: AWS EC2 Docker 部署指南

本文件說明如何在 AWS EC2 的 Amazon Linux 2023 或 Ubuntu 實例上，安裝 Docker 並部署 **AI Mail Butler** 服務。

---

## 1. 安裝 Docker (AWS EC2)

### 針對 Amazon Linux 2023 (推薦)
```bash
# 更新系統
sudo dnf update -y

# 安裝 Docker
sudo dnf install -y docker

# 啟動 Docker 服務
sudo systemctl start docker
sudo systemctl enable docker

# 將當前使用者 (ec2-user) 加入 docker 群組，這樣之後就不需要 sudo
sudo usermod -aG docker ec2-user

# *** 重要：執行完上述指令後，請重新連線 SSH 以使群組設定生效 ***
```

### 針對 Ubuntu 22.04+
```bash
sudo apt update
sudo apt install -y docker.io
sudo systemctl start docker
sudo systemctl enable docker
sudo usermod -aG docker ubuntu
# 重新連線 SSH
```

---

## 2. 安裝 Docker Compose

Docker Compose 對於管理多容器應用（如 Web + 資料庫）非常有用。

```bash
# 下載 Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose

# 賦予執行權限
sudo chmod +x /usr/local/bin/docker-compose

# 驗證安裝
docker-compose --version
```

---

## 3. 部署 AI Mail Butler

### 步驟 A：取得專案
您可以直接將原始碼複製到 EC2，或是在本地編譯後上傳 Image。這裡推薦使用專案內的 `Dockerfile` 進行本地構建。

```bash
# 建立專案目錄
mkdir ~/ai-mail-butler && cd ~/ai-mail-butler

# (您可以透過 git clone 或上傳檔案到此目錄)
```

### 步驟 B：設定環境變數
建立 `.env` 檔案並填入您的設定：
```bash
cat <<EOF > .env
DATABASE_URL=sqlite:data/data.sqlite
PORT=3000
ADMIN_EMAIL=your-admin@example.com
ASSISTANT_EMAIL=assistant@yourdomain.com
AI_API_BASE_URL=http://localhost:1234/v1
AI_MODEL_NAME=your-model-name
SMTP_RELAY_HOST=smtp.gmail.com
SMTP_RELAY_PORT=587
SMTP_RELAY_USER=your-email@gmail.com
SMTP_RELAY_PASS=your-app-password
EOF
```

### 步驟 C：建立 Dockerfile
如果專案內還沒有 Dockerfile，請使用以下內容：
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

### 步驟 D：啟動服務
```bash
# 構建並啟動
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

## 4. AWS 安全組 (Security Group) 設定

為了讓服務正常運作，請務必在 AWS Console 的 Security Group 開放以下通訊埠：

| 類型 | 通訊埠 | 說明 |
| :--- | :--- | :--- |
| HTTP | 80 | 存取 Dashboard 網頁 |
| SMTP | 25 | 接收轉寄郵件 |
| SSH | 22 | 管理伺服器用 |

---

## 5. 常見問題與備註
- **資料持久化**：使用 `-v` 指令將資料庫掛載到宿主機，確保 Container 更新後資料不會遺失。
- **AI Backend**：如果您的 AI 模型 (LM Studio) 跑在其他機器，請將 `.env` 中的 `AI_API_BASE_URL` 改為對應的 IP 位址。
- **防火牆**：EC2 內部的 `iptables` 可能會擋住 25 port，請確保 AWS 安全組與系統防火牆皆已開放。
