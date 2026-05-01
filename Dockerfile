FROM node:22-bookworm-slim AS frontend-builder

WORKDIR /app/frontend

COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci

COPY frontend/ ./
RUN npm run build


FROM rust:1-bookworm AS rust-builder

WORKDIR /app

RUN apt-get update && \
    apt-get install -y --no-install-recommends pkg-config libsqlite3-dev && \
    rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock build.rs ./
COPY src ./src

RUN cargo build --release --locked


FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates dnsutils libsqlite3-0 openssh-client sshfs && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=frontend-builder /app/frontend/dist ./frontend/dist
COPY --from=rust-builder /app/target/release/ai-mail-butler /usr/local/bin/ai-mail-butler
COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh

RUN chmod +x /usr/local/bin/ai-mail-butler && \
    chmod +x /usr/local/bin/docker-entrypoint.sh && \
    mkdir -p /app/data

EXPOSE 3000
EXPOSE 25

ENTRYPOINT ["docker-entrypoint.sh"]
CMD ["ai-mail-butler"]
