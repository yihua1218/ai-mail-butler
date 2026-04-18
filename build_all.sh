#!/bin/bash
set -e

echo "=== AI Mail Butler: Multi-Platform Build Script ==="

echo "[1/4] Building Frontend..."
cd frontend
npm install
npm run build
cd ..

echo "[2/4] Building for native macOS (Apple Silicon)..."
cargo build --release

echo "[3/4] Building for Linux x86_64 (amd64) via cross..."
if ! command -v cross &> /dev/null; then
    echo "cross not found. Installing cross..."
    cargo install cross --git https://github.com/cross-rs/cross
fi
CROSS_CUSTOM_TOOLCHAIN=1 cross build --release --target x86_64-unknown-linux-gnu

echo "[4/4] Building for Linux aarch64 (arm64) via cross..."
CROSS_CUSTOM_TOOLCHAIN=1 cross build --release --target aarch64-unknown-linux-gnu

echo "=== Organizing Binaries for Docker ==="
mkdir -p bin/amd64 bin/arm64
cp target/x86_64-unknown-linux-gnu/release/ai-mail-butler bin/amd64/
cp target/aarch64-unknown-linux-gnu/release/ai-mail-butler bin/arm64/

echo "Build complete! You can now run:"
echo "docker buildx build --platform linux/amd64,linux/arm64 -t ai-mail-butler:latest ."
