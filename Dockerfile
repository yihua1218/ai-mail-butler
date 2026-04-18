# Lightweight runtime environment
FROM debian:bookworm-slim

# Install necessary HTTPS certificates and OpenSSL for runtime
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# TARGETARCH is automatically populated by Docker Buildx (e.g. amd64, arm64)
ARG TARGETARCH

# Copy the pre-compiled frontend assets
COPY frontend/dist ./frontend/dist

# Copy the pre-compiled Rust executable based on architecture
COPY bin/${TARGETARCH}/ai-mail-butler /usr/local/bin/ai-mail-butler
RUN chmod +x /usr/local/bin/ai-mail-butler

# Expose the defined ports
EXPOSE 3000
EXPOSE 25

# Start the application
CMD ["ai-mail-butler"]
