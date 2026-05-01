#!/bin/sh
set -eu

is_enabled() {
    case "$(printf '%s' "${1:-}" | tr '[:upper:]' '[:lower:]')" in
        1|true|yes|on) return 0 ;;
        *) return 1 ;;
    esac
}

mount_remote_debug_sshfs() {
    if ! is_enabled "${REMOTE_DEBUG_SSHFS_ENABLED:-false}"; then
        return 0
    fi

    if [ -z "${REMOTE_DEBUG_REMOTE:-}" ]; then
        echo "REMOTE_DEBUG_SSHFS_ENABLED is true, but REMOTE_DEBUG_REMOTE is empty." >&2
        exit 1
    fi

    if [ -z "${REMOTE_DEBUG_MOUNT_POINT:-}" ]; then
        echo "REMOTE_DEBUG_SSHFS_ENABLED is true, but REMOTE_DEBUG_MOUNT_POINT is empty." >&2
        exit 1
    fi

    case "${REMOTE_DEBUG_MOUNT_POINT}" in
        "~") REMOTE_DEBUG_MOUNT_POINT="${HOME:-/root}" ;;
        "~/"*) REMOTE_DEBUG_MOUNT_POINT="${HOME:-/root}/${REMOTE_DEBUG_MOUNT_POINT#\~/}" ;;
    esac
    export REMOTE_DEBUG_MOUNT_POINT

    mkdir -p "${REMOTE_DEBUG_MOUNT_POINT}"

    if mountpoint -q "${REMOTE_DEBUG_MOUNT_POINT}"; then
        echo "SSHFS remote debug mount already present at ${REMOTE_DEBUG_MOUNT_POINT}."
    else
        mode="$(printf '%s' "${REMOTE_DEBUG_MODE:-readonly}" | tr '[:upper:]' '[:lower:]')"
        default_options="reconnect,ServerAliveInterval=15,ServerAliveCountMax=3"
        if [ "${mode}" = "readonly" ]; then
            default_options="ro,${default_options}"
        fi
        sshfs_options="${REMOTE_DEBUG_SSHFS_OPTIONS:-${default_options}}"

        echo "Mounting ${REMOTE_DEBUG_REMOTE} at ${REMOTE_DEBUG_MOUNT_POINT} with SSHFS."
        sshfs "${REMOTE_DEBUG_REMOTE}" "${REMOTE_DEBUG_MOUNT_POINT}" -o "${sshfs_options}"
    fi

    if [ "$(printf '%s' "${REMOTE_DEBUG_MODE:-readonly}" | tr '[:upper:]' '[:lower:]')" = "overlay" ]; then
        export READONLY_MODE=true
        export READONLY_BASE="${READONLY_BASE:-${REMOTE_DEBUG_MOUNT_POINT}}"
        if [ -n "${REMOTE_DEBUG_OVERLAY_DIR:-}" ] && [ -z "${OVERLAY_DIR:-}" ]; then
            export OVERLAY_DIR="${REMOTE_DEBUG_OVERLAY_DIR}"
        fi
    fi
}

mount_remote_debug_sshfs

case "${1:-}" in
    -*) set -- ai-mail-butler "$@" ;;
esac

exec "$@"
