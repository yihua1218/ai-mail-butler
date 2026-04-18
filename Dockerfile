# Stage 1: Build (Builder)
FROM rust:slim-bookworm AS builder

# Install build dependencies (reqwest requires OpenSSL by default)
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy source code into the container
COPY . .

# Compile optimized release version
RUN cargo build --release

# Stage 2: Lightweight runtime environment (Runner)
FROM debian:bookworm-slim

# Install necessary HTTPS certificates and OpenSSL for runtime
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled standalone executable from the builder stage
COPY --from=builder /app/target/release/ai-mail-butler /usr/local/bin/ai-mail-butler

# Expose the defined port (Corresponds to your WEB_PORT/SMTP_PORT)
EXPOSE 3000
EXPOSE 25

# Start the application
CMD ["ai-mail-butler"]
