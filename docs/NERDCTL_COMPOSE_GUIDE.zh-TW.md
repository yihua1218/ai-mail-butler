# 使用 nerdctl 與 Compose 執行應用程式

本指南說明如何使用 `nerdctl compose` 或 `docker compose`，從 GitHub Container Registry (ghcr.io) 拉取預先建置的 Docker 映像來執行 AI Mail Butler 應用程式。

這種方法非常適合生產或預備環境，因為您可以在不安裝原始碼編譯環境的主機上，直接部署特定版本的應用程式。

## 先決條件

- **`nerdctl`**: 請確保您已安裝 `nerdctl` 和 `nerdctl-compose`。安裝說明可以在 [containerd 網站](https://containerd.io/) 找到。
- **Docker (替代方案)**: 如果您偏好使用 Docker，本文件的指令同樣適用，只需將 `nerdctl` 替換為 `docker` 即可。
- **GitHub 儲存庫**: 映像將從 `ghcr.io/yihua1218/ai-mail-butler:latest` 拉取。

## 步驟 1：建立設定檔

應用程式需要環境變數來進行設定，這些變數由 `.env` 檔案管理。

1.  將範例設定檔複製為一個新的 `.env` 檔案：
    ```bash
    cp .env.example .env
    ```

2.  **編輯 `.env` 檔案**，填入您的具體設定。您至少需要設定您的 SMTP 中繼、AI 供應商詳細資訊和管理員電子郵件。
    ```ini
    # 伺服器設定
    PORT=3000
    HOST=0.0.0.0 # 使用 0.0.0.0 以便從容器外部存取
    RUST_LOG=info,ai_mail_butler=debug
    ADMIN_EMAIL=your-admin-email@example.com

    # 資料庫設定
    DATABASE_URL=sqlite:/app/data/data.sqlite # 指向容器內的路徑

    # 郵件設定
    SMTP_RELAY_HOST=smtp.your-provider.com
    SMTP_RELAY_PORT=587
    SMTP_RELAY_USER=your-smtp-username
    SMTP_RELAY_PASS=your-smtp-password
    ASSISTANT_EMAIL=assistant@your-domain.com

    # AI 設定 (例如 OpenAI, LM Studio)
    AI_API_BASE_URL=http://your-ai-provider-host:1234/v1
    AI_API_KEY=your-api-key
    AI_MODEL_NAME=your-model-name

    # ... 其他設定 ...
    ```
    **重要提示**:
    - 將 `HOST` 設定為 `0.0.0.0` 以允許伺服器接受來自容器外部的連線。
    - `DATABASE_URL` 應指向容器內的路徑，根據 volume 掛載的定義，這裡是 `/app/data/data.sqlite`。

## 步驟 2：修改 Compose 檔案

預設的 `docker-compose.yml` 設定為從原始碼建置映像。您需要修改它，以便從 `ghcr.io` 拉取預先建置的映像。

1.  開啟 `docker-compose.yml` 檔案。
2.  找到 `ai-mail-butler` 服務的定義。
3.  **將 `build: .` 替換為 `image: ghcr.io/yihua1218/ai-mail-butler:latest`**。

更新後的服務應如下所示：

```yaml
version: '3.8'

services:
  ai-mail-butler:
    # build: .  <-- 註解或刪除此行
    image: ghcr.io/yihua1218/ai-mail-butler:latest # <-- 新增此行
    container_name: ai-mail-butler
    ports:
      - "3000:3000" # Web UI
      - "2525:25"   # SMTP 伺服器 (在主機上使用 2525 以避免需要 root 權限)
    env_file:
      - .env
    volumes:
      # 掛載本地目錄以持久化儲存資料 (資料庫、郵件池等)
      - ./ai-mail-butler-data:/app/data
    restart: unless-stopped
```
**關於 Port 25 的說明**: 我們將主機的 `2525` 連接埠對應到容器的 `25` 連接埠。這是因為在主機上綁定低於 1024 的連接埠通常需要 root 權限。如果您以 root 身分執行 `nerdctl` 或 `docker`，可以將 `2525` 改回 `25`。

## 步驟 3：執行應用程式

現在您可以使用 `nerdctl compose` 來啟動應用程式。

1.  **拉取最新的映像**：
    ```bash
    nerdctl pull ghcr.io/yihua1218/ai-mail-butler:latest:latest
    ```
    *(對於 Docker 用戶: `docker pull ghcr.io/yihua1218/ai-mail-butler:latest:latest`)*

2.  **以分離模式啟動服務**：
    ```bash
    nerdctl compose up -d
    ```
    *(對於 Docker 用戶: `docker compose up -d`)*

## 步驟 4：驗證應用程式

1.  **檢查容器是否正在執行**：
    ```bash
    nerdctl compose ps
    ```
    您應該會看到 `ai-mail-butler` 容器處於 `running` 狀態。

2.  **檢查日誌**以確保一切正常啟動：
    ```bash
    nerdctl compose logs -f
    ```
    *(對於 Docker 用戶: `docker compose logs -f`)*

    您應該會看到 Rust 應用程式的日誌輸出，顯示伺服器正在監聽 3000 連接埠。

3.  **存取 Web UI**: 打開您的網頁瀏覽器，並前往 `http://localhost:3000`。

## 步驟 5：停止應用程式

若要停止應用程式並移除容器，請執行：
```bash
nerdctl compose down
```
*(對於 Docker 用戶: `docker compose down`)*

此命令將停止並移除容器，但 `./ai-mail-butler-data` 卷宗中的資料將被保留。
