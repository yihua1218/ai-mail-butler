#!/usr/bin/env bash
set -euo pipefail

IMAGE_NAME="${IMAGE_NAME:-ai-mail-butler}"
IMAGE_TAG="${IMAGE_TAG:-latest}"
CONTAINER_CLI="${CONTAINER_CLI:-auto}"
SKIP_LOCAL_BUILD="${SKIP_LOCAL_BUILD:-false}"
BUILDKIT_PID=""

usage() {
    cat <<EOF
Usage: $0 [options]

Options:
  --runtime docker|nerdctl|auto   Container builder to use (default: auto)
  --image NAME                    Local image name (default: ai-mail-butler)
  --tag TAG                       Local image tag (default: latest)
  --skip-local-build              Skip host npm/cargo verification builds
  -h, --help                      Show this help

Environment:
  CONTAINER_CLI                   Same as --runtime
  IMAGE_NAME                      Same as --image
  IMAGE_TAG                       Same as --tag
  SKIP_LOCAL_BUILD=true           Same as --skip-local-build
EOF
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --runtime)
            CONTAINER_CLI="${2:?--runtime requires docker, nerdctl, or auto}"
            shift 2
            ;;
        --image)
            IMAGE_NAME="${2:?--image requires a value}"
            shift 2
            ;;
        --tag)
            IMAGE_TAG="${2:?--tag requires a value}"
            shift 2
            ;;
        --skip-local-build)
            SKIP_LOCAL_BUILD=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

cleanup() {
    if [ -n "${BUILDKIT_PID}" ]; then
        kill "${BUILDKIT_PID}" >/dev/null 2>&1 || true
        wait "${BUILDKIT_PID}" >/dev/null 2>&1 || true
    fi
}
trap cleanup EXIT

select_container_cli() {
    case "${CONTAINER_CLI}" in
        docker|nerdctl)
            if ! command -v "${CONTAINER_CLI}" >/dev/null 2>&1; then
                echo "${CONTAINER_CLI} was requested but is not installed." >&2
                exit 1
            fi
            ;;
        auto)
            if command -v docker >/dev/null 2>&1; then
                CONTAINER_CLI=docker
            elif command -v nerdctl >/dev/null 2>&1; then
                CONTAINER_CLI=nerdctl
            else
                echo "Neither docker nor nerdctl is installed." >&2
                exit 1
            fi
            ;;
        *)
            echo "Unsupported runtime '${CONTAINER_CLI}'. Use docker, nerdctl, or auto." >&2
            exit 2
            ;;
    esac
}

buildkit_is_running() {
    local socket
    for socket in \
        "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/buildkit-default/buildkitd.sock" \
        "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/buildkit/buildkitd.sock"
    do
        if [ -S "${socket}" ] && buildctl --addr "unix://${socket}" debug workers >/dev/null 2>&1; then
            return 0
        fi
    done
    return 1
}

ensure_nerdctl_buildkit() {
    if [ "${CONTAINER_CLI}" != "nerdctl" ]; then
        return 0
    fi

    if buildkit_is_running; then
        return 0
    fi

    if ! command -v buildkitd >/dev/null 2>&1 || ! command -v buildctl >/dev/null 2>&1; then
        echo "nerdctl build requires buildkitd and buildctl." >&2
        echo "Install BuildKit, then run: containerd-rootless-setuptool.sh install-buildkit" >&2
        exit 1
    fi

    local runtime_dir="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
    local socket_dir="${runtime_dir}/buildkit-default"
    mkdir -p "${socket_dir}"

    echo "Starting temporary rootless BuildKit for nerdctl..."
    if command -v rootlesskit >/dev/null 2>&1; then
        rootlesskit buildkitd --addr "unix://${socket_dir}/buildkitd.sock" >/tmp/ai-mail-butler-buildkit.log 2>&1 &
    else
        buildkitd --addr "unix://${socket_dir}/buildkitd.sock" >/tmp/ai-mail-butler-buildkit.log 2>&1 &
    fi
    BUILDKIT_PID="$!"

    for _ in $(seq 1 30); do
        if buildkit_is_running; then
            return 0
        fi
        sleep 1
    done

    echo "BuildKit did not become ready. Recent log output:" >&2
    tail -40 /tmp/ai-mail-butler-buildkit.log >&2 || true
    exit 1
}

run_local_builds() {
    if [ "${SKIP_LOCAL_BUILD}" = "true" ]; then
        echo "Skipping host npm/cargo verification builds."
        return 0
    fi

    echo "[1/3] Building frontend on host..."
    (
        cd frontend
        npm ci
        npm run build
    )

    echo "[2/3] Building Rust release binary on host..."
    cargo build --release --locked
}

build_container_image() {
    local image_ref="${IMAGE_NAME}:${IMAGE_TAG}"

    echo "[3/3] Building local container image with ${CONTAINER_CLI}: ${image_ref}"
    ensure_nerdctl_buildkit
    "${CONTAINER_CLI}" build -t "${image_ref}" .

    echo "Build complete."
    echo "Local image: ${image_ref}"
    echo "Try it with: ${CONTAINER_CLI} run --rm -p 3000:3000 ${image_ref}"
}

echo "=== AI Mail Butler: Build All ==="
select_container_cli
echo "Container runtime: ${CONTAINER_CLI}"
run_local_builds
build_container_image
