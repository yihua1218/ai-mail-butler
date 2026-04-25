# GitHub Actions Staging Deploy Setup (docker-publish)

This guide explains how to configure GitHub Actions so a successfully published image from [`.github/workflows/docker-publish.yml`](../.github/workflows/docker-publish.yml) can be pulled on your staging server and restarted with Docker Compose.

It focuses on two parts:

1. GitHub-side variables and secrets
2. Staging server files and data layout

## 1. How Deploy Works in Current Workflow

The `deploy` job in [`.github/workflows/docker-publish.yml`](../.github/workflows/docker-publish.yml):

1. Runs only on branch pushes (not PRs, not tags).
2. Builds and pushes image tags to `ghcr.io`.
3. SSHes into your staging server.
4. Executes:
   - `docker login` (if token exists)
   - `docker pull <image-with-sha-tag>`
   - `docker compose down`
   - `docker compose up -d`

The deploy image tag is fixed to:

- `sha-${GITHUB_SHA}`

So every successful pipeline deploys an immutable SHA tag. After the staging server successfully pulls the new image and restarts Docker Compose, the workflow purges the entire Cloudflare zone cache with `purge_everything`.

## 2. GitHub Actions Settings You Must Configure

Go to your repository:

- `Settings` -> `Secrets and variables` -> `Actions`

Create the following **Repository secrets** (or Environment secrets if you bind this job to an environment):

| Secret name                | Required                | Description                                      |
| -------------------------- | ----------------------- | ------------------------------------------------ |
| `DEPLOY_HOST`              | Yes                     | Staging server hostname or IP                    |
| `DEPLOY_USER`              | Yes                     | SSH username used by the action                  |
| `DEPLOY_SSH_KEY`           | Yes                     | Private key content for SSH login                |
| `DEPLOY_PORT`              | No                      | SSH port (defaults to `22` if omitted)           |
| `DEPLOY_PATH`              | Yes                     | Absolute path on server containing compose files |
| `DEPLOY_REGISTRY_USERNAME` | Yes (for private image) | GHCR login username                              |
| `DEPLOY_REGISTRY_TOKEN`    | Yes (for private image) | GHCR token/password used by `docker login`       |
| `CLOUDFLARE_ZONE_ID`       | Yes                     | Cloudflare zone ID to purge after deploy         |
| `CLOUDFLARE_API_TOKEN`     | Yes                     | Cache-purge-only Cloudflare API Token            |

### Generate SSH key pair for GitHub Actions deploy

The workflow logs in to the server with `DEPLOY_SSH_KEY`. A common setup is to create a dedicated key pair only for CI deploy.

1. Generate a new key pair on your local machine:

```bash
ssh-keygen -t ed25519 -C "github-actions-staging-deploy" -f ~/.ssh/gha_staging_deploy
```

2. Add the private key content to GitHub secret `DEPLOY_SSH_KEY`:

```bash
cat ~/.ssh/gha_staging_deploy
```

3. Add the public key to your staging server SSH authorized keys file:

```bash
cat ~/.ssh/gha_staging_deploy.pub
```

Then append that `.pub` content on staging server into:

- `~/.ssh/authorized_keys` (default)
- or your custom `~/.ssh/authxxx` file if SSHD is configured with `AuthorizedKeysFile` pointing there

Example on staging server:

```bash
mkdir -p ~/.ssh
chmod 700 ~/.ssh
cat >> ~/.ssh/authorized_keys
# paste one full line from gha_staging_deploy.pub, then Ctrl-D
chmod 600 ~/.ssh/authorized_keys
```

If you use a custom `authxxx` file, apply strict permissions too (`chmod 600`).

### Token scope for GHCR pull

If your package is private, `DEPLOY_REGISTRY_TOKEN` should be a PAT (or equivalent token) with at least:

- `read:packages`

If repository/package visibility policy requires it, also include:

- `repo` (for private repository contexts)

## 3. Optional: Use a Dedicated GitHub Environment (staging)

If you want environment-level approvals and scoped secrets, create a GitHub Environment named `staging`:

- `Settings` -> `Environments` -> `New environment` -> `staging`

Then add:

- Required reviewers (optional)
- Wait timer (optional)
- Environment secrets with the same names above

To make the workflow consume that environment, add this in the `deploy` job:

```yaml
deploy:
  environment: staging
```

Without this field, the workflow reads repository-level secrets only.

## 4. Staging Server Requirements

Before first deployment, prepare your server:

1. Docker Engine installed
2. Docker Compose plugin installed (`docker compose` command available)
3. `DEPLOY_USER` can run Docker commands
4. Directory from `DEPLOY_PATH` already exists

Example:

```bash
sudo mkdir -p /opt/ai-mail-butler
sudo chown -R <deploy-user>:<deploy-user> /opt/ai-mail-butler
```

Set `DEPLOY_PATH=/opt/ai-mail-butler` in GitHub secret.

## 5. Files Required Under DEPLOY_PATH

Inside `DEPLOY_PATH`, prepare at least:

1. `docker-compose.yml`
2. `.env`
3. persistent data directory (for example `ai-mail-butler-data/`)

### 5.1 Example docker-compose.yml

Use the image variables expected by workflow:

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

The workflow exports these before `docker compose up -d`:

- `IMAGE_NAME=ghcr.io/<owner>/<repo>`
- `IMAGE_TAG=sha-<commit>`

So Compose will start exactly the tag just pulled.

### 5.2 Example .env

Create `.env` with your runtime configuration (SMTP, AI provider, admin email, etc.). Example keys:

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

For Cloudflare API Token permissions and partial cache purge examples, see [Cloudflare Cache Purge Token and Operations](CLOUDFLARE-CACHE-PURGE.md).

## 6. First Deployment Checklist

1. All required GitHub secrets are set
2. `DEPLOY_PATH` exists and contains `docker-compose.yml` and `.env`
3. Staging user can run `docker pull` and `docker compose up -d`
4. `DEPLOY_REGISTRY_TOKEN` can pull from GHCR
5. Push to `main` or `master` to trigger deploy

## 7. Verify Deployment on Server

After Actions completes, SSH into staging server and run:

```bash
cd <DEPLOY_PATH>
docker compose ps
docker compose logs --tail=100
docker image ls | grep ai-mail-butler
```

If container is up and logs are healthy, deploy is successful.
