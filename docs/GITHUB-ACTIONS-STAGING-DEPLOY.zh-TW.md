# GitHub Actions Staging 部署設定說明（docker-publish）

本文件說明如何設定 GitHub Actions，讓 [`.github/workflows/docker-publish.yml`](../.github/workflows/docker-publish.yml) 成功 publish 的 image，可以在 staging server 被拉取並透過 Docker Compose 重新啟動。

內容分成兩部分：

1. GitHub 端的變數與秘密資料設定
2. Staging 伺服器端的檔案與資料目錄準備

## 1. 目前 Workflow 的 Deploy 行為

[`.github/workflows/docker-publish.yml`](../.github/workflows/docker-publish.yml) 的 `deploy` job 會：

1. 只在 branch push 時執行（不在 PR、不在 tag）
2. 將 image 推送到 `ghcr.io`
3. 用 SSH 連到 staging server
4. 在遠端執行：
   - `docker login`（有 token 時）
   - `docker pull <image-with-sha-tag>`
   - `docker compose down`
   - `docker compose up -d`

部署使用的 image tag 固定為：

- `sha-${GITHUB_SHA}`

因此每次成功 pipeline 都會部署不可變的 SHA 版本。當 staging server 成功 pull 新 image，且 Docker Compose `down` / `up -d` 成功後，workflow 會對 Cloudflare zone 執行 `purge_everything` 清除整站快取。

## 2. GitHub Actions 必須設定的資料

進入 repository：

- `Settings` -> `Secrets and variables` -> `Actions`

建立以下 **Repository secrets**（或改用 Environment secrets，前提是 workflow 綁定 environment）：

| Secret 名稱                | 必填              | 說明                                           |
| -------------------------- | ----------------- | ---------------------------------------------- |
| `DEPLOY_HOST`              | 是                | Staging 伺服器主機名稱或 IP                    |
| `DEPLOY_USER`              | 是                | Action 使用的 SSH 帳號                         |
| `DEPLOY_SSH_KEY`           | 是                | SSH 私鑰內容                                   |
| `DEPLOY_PORT`              | 否                | SSH 連接埠（未填預設 `22`）                    |
| `DEPLOY_PATH`              | 是                | 遠端伺服器上的部署目錄絕對路徑                 |
| `DEPLOY_REGISTRY_USERNAME` | 私有 image 時必填 | GHCR 登入帳號                                  |
| `DEPLOY_REGISTRY_TOKEN`    | 私有 image 時必填 | 提供 `docker login` 使用的 GHCR token/password |
| `CLOUDFLARE_ZONE_ID`       | 是                | 部署成功後要 purge 的 Cloudflare zone ID       |
| `CLOUDFLARE_API_TOKEN`     | 是                | 只能清除快取的 Cloudflare API Token            |

### 產生給 GitHub Actions 部署用的 SSH 金鑰

workflow 會透過 `DEPLOY_SSH_KEY` 登入 staging server。建議為 CI 部署建立一組專用 SSH key pair。

1. 在本機產生新的 key pair：

```bash
ssh-keygen -t ed25519 -C "github-actions-staging-deploy" -f ~/.ssh/gha_staging_deploy
```

2. 將 private key 內容放入 GitHub secret `DEPLOY_SSH_KEY`：

```bash
cat ~/.ssh/gha_staging_deploy
```

3. 將 public key 加到 staging server 的 SSH 授權檔：

```bash
cat ~/.ssh/gha_staging_deploy.pub
```

把 `.pub` 的整行內容追加到 staging server：

- `~/.ssh/authorized_keys`（預設）
- 或你自訂的 `~/.ssh/authxxx`（如果 SSHD 的 `AuthorizedKeysFile` 指向此檔）

在 staging server 的操作範例：

```bash
mkdir -p ~/.ssh
chmod 700 ~/.ssh
cat >> ~/.ssh/authorized_keys
# 貼上 gha_staging_deploy.pub 的完整一行後按 Ctrl-D
chmod 600 ~/.ssh/authorized_keys
```

若你使用自訂 `authxxx` 檔案，也要套用嚴格權限（`chmod 600`）。

### GHCR token 權限建議

若套件為 private，`DEPLOY_REGISTRY_TOKEN` 建議至少具備：

- `read:packages`

若你們的 private repository/package 政策有要求，另外加上：

- `repo`

## 3. 可選：使用 GitHub Environment（staging）

若你需要 environment 級別審核與隔離 secrets，可建立 `staging` 環境：

- `Settings` -> `Environments` -> `New environment` -> `staging`

可在此設定：

- Required reviewers（可選）
- Wait timer（可選）
- 與上面相同名稱的 Environment secrets

若要讓 workflow 使用這個環境，請在 `deploy` job 加上：

```yaml
deploy:
  environment: staging
```

若未設定 `environment`，workflow 只會讀取 repository-level secrets。

## 4. Staging 伺服器前置需求

首次部署前，請先準備：

1. 已安裝 Docker Engine
2. 已安裝 Docker Compose plugin（可用 `docker compose`）
3. `DEPLOY_USER` 可執行 Docker 指令
4. `DEPLOY_PATH` 指向的目錄已存在

範例：

```bash
sudo mkdir -p /opt/ai-mail-butler
sudo chown -R <deploy-user>:<deploy-user> /opt/ai-mail-butler
```

然後把 GitHub secret `DEPLOY_PATH` 設成 `/opt/ai-mail-butler`。

## 5. DEPLOY_PATH 目錄下必備檔案

在 `DEPLOY_PATH` 至少要有：

1. `docker-compose.yml`
2. `.env`
3. 持久化資料目錄（例如 `ai-mail-butler-data/`）

### 5.1 docker-compose.yml 範例

建議使用 workflow 預期的 image 變數：

```yaml
version: '3.8'

services:
  ai-mail-butler:
    image: ${IMAGE_NAME}:${IMAGE_TAG}
    container_name: ai-mail-butler
    ports:
      - "3000:3000"
      - "25:25"
    env_file:
      - .env
    volumes:
      - ./ai-mail-butler-data:/app/data
    restart: unless-stopped
```

workflow 在 `docker compose up -d` 前，會先 export：

- `IMAGE_NAME=ghcr.io/<owner>/<repo>`
- `IMAGE_TAG=sha-<commit>`

因此 Compose 會啟動剛剛 pull 下來的那個 SHA 版本。

### 5.2 .env 範例

建立 `.env`，放入執行期設定（SMTP、AI 供應商、管理員信箱等）。範例：

```env
DATABASE_URL=sqlite:/app/data/data.sqlite
PORT=3000
HOST=0.0.0.0
PUBLIC_URL=https://staging.example.com
ADMIN_EMAIL=admin@example.com

SMTP_RELAY_HOST=smtp.example.com
SMTP_RELAY_PORT=587
SMTP_RELAY_USER=your-user
SMTP_RELAY_PASS=your-password
ASSISTANT_EMAIL=assistant@example.com

AI_API_BASE_URL=http://your-ai-endpoint/v1
AI_API_KEY=your-key
AI_MODEL_NAME=your-model

CLOUDFLARE_ZONE_ID=your-cloudflare-zone-id
CLOUDFLARE_API_TOKEN=your-cache-purge-only-token
```

Cloudflare API Token 權限限制與部分清除快取範例，請參考 [Cloudflare Cache Purge Token 與快取清除操作](CLOUDFLARE-CACHE-PURGE.zh-TW.md)。

## 6. 首次部署檢查清單

1. GitHub 必要 secrets 都已設定
2. `DEPLOY_PATH` 已建立，且含 `docker-compose.yml` 與 `.env`
3. Staging 使用者可執行 `docker pull` 與 `docker compose up -d`
4. `DEPLOY_REGISTRY_TOKEN` 具備 GHCR pull 權限
5. Push 到 `main` 或 `master` 觸發部署

## 7. 在 Server 驗證部署結果

當 Actions 完成後，SSH 到 staging server 執行：

```bash
cd <DEPLOY_PATH>
docker compose ps
docker compose logs --tail=100
docker image ls | grep ai-mail-butler
```

若 container 正常 running 且 log 正常，即代表部署成功。
