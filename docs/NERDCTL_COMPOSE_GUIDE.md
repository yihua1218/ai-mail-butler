# Running the Application with nerdctl and Compose

This guide explains how to run the AI Mail Butler application using a pre-built Docker image from GitHub Container Registry (ghcr.io) with `nerdctl compose` or `docker compose`.

This approach is ideal for production or staging environments where you want to deploy a specific version of the application without building it from the source code on the host machine.

## Prerequisites

- **`nerdctl`**: Ensure you have `nerdctl` and `nerdctl-compose` installed. Installation instructions can be found on the [containerd website](https://containerd.io/).
- **Docker (Alternative)**: If you prefer to use Docker, the same commands apply. Just replace `nerdctl` with `docker`.
- **GitHub Repository**: The image will be pulled from `ghcr.io/yihua1218/ai-mail-butler:latest`.

## Step 1: Create Configuration File

The application requires environment variables for its configuration. These are managed in a `.env` file.

1.  Copy the example configuration file to a new `.env` file:
    ```bash
    cp .env.example .env
    ```

2.  **Edit the `.env` file** with your specific settings. You must at least configure your SMTP relay, AI provider details, and administrator email.
    ```ini
    # Server Configuration
    PORT=3000
    HOST=0.0.0.0 # Use 0.0.0.0 to be accessible from outside the container
    PUBLIC_URL=https://your-domain.com # Public base URL used in magic login links sent by email
    RUST_LOG=info,ai_mail_butler=debug
    ADMIN_EMAIL=your-admin-email@example.com

    # Database Configuration
    DATABASE_URL=sqlite:/app/data/data.sqlite # Path inside the container

    # Mail Configuration
    SMTP_RELAY_HOST=smtp.your-provider.com
    SMTP_RELAY_PORT=587
    SMTP_RELAY_USER=your-smtp-username
    SMTP_RELAY_PASS=your-smtp-password
    ASSISTANT_EMAIL=assistant@your-domain.com

    # AI Configuration (e.g., OpenAI, LM Studio)
    AI_API_BASE_URL=http://your-ai-provider-host:1234/v1
    AI_API_KEY=your-api-key
    AI_MODEL_NAME=your-model-name

    # ... other settings ...
    ```
    **Important**:
    - Set `HOST` to `0.0.0.0` to allow the server to accept connections from outside the container.
    - Set `PUBLIC_URL` to your public domain (e.g. `https://your-domain.com`) so magic login links in emails point to the correct address instead of `localhost`.
    - The `DATABASE_URL` should point to the path inside the container, which is `/app/data/data.sqlite` as defined by the volume mount.

## Step 2: Modify the Compose File

The default `docker-compose.yml` is configured to build the image from the source. You need to modify it to pull the pre-built image from `ghcr.io`.

1.  Open `docker-compose.yml`.
2.  Find the `ai-mail-butler` service definition.
3.  **Replace `build: .` with `image: ghcr.io/yihua1218/ai-mail-butler:latest`**.

The updated service should look like this:

```yaml
version: '3.8'

services:
  ai-mail-butler:
    # build: .  <-- Comment out or remove this line
    image: ghcr.io/yihua1218/ai-mail-butler:latest # <-- Add this line
    container_name: ai-mail-butler
    ports:
      - "3000:3000" # Web UI
      - "2525:25"   # SMTP Server (using 2525 on host to avoid requiring root)
    env_file:
      - .env
    volumes:
      # Mounts a local directory for persistent data (database, mail spool, etc.)
      - ./ai-mail-butler-data:/app/data
    restart: unless-stopped
```
**Note on Port 25**: We map host port `2525` to container port `25`. This is because binding to ports below 1024 on the host typically requires root privileges. You can change `2525` to `25` if you are running `nerdctl` or `docker` as root.

## Step 3: Run the Application

Now you can start the application using `nerdctl compose`.

1.  **Pull the latest image**:
    ```bash
    nerdctl pull ghcr.io/yihua1218/ai-mail-butler:latest:latest
    ```
    *(For Docker users: `docker pull ghcr.io/yihua1218/ai-mail-butler:latest`)*

2.  **Start the service in detached mode**:
    ```bash
    nerdctl compose up -d
    ```
    *(For Docker users: `docker compose up -d`)*

## Step 4: Verify the Application

1.  **Check if the container is running**:
    ```bash
    nerdctl compose ps
    ```
    You should see the `ai-mail-butler` container in the `running` state.

2.  **Check the logs** to ensure everything started correctly:
    ```bash
    nerdctl compose logs -f
    ```
    *(For Docker users: `docker compose logs -f`)*

    You should see log output from the Rust application, indicating that the server is listening on port 3000.

3.  **Access the Web UI**: Open your web browser and navigate to `http://localhost:3000`.

## Step 5: Stopping the Application

To stop the application and remove the container, run:
```bash
nerdctl compose down
```
*(For Docker users: `docker compose down`)*

This command will stop and remove the container, but the data in the `./ai-mail-butler-data` volume will be preserved.
