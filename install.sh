#!/bin/sh
# install.sh — Install the AgentLang `al` binary for the current platform.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/mohocp/Axon/main/install.sh | sh
#
# Environment variables:
#   AL_VERSION   — version tag to install (default: latest)
#   AL_INSTALL   — installation directory (default: /usr/local/bin)

set -eu

REPO="mohocp/Axon"
INSTALL_DIR="${AL_INSTALL:-/usr/local/bin}"
VERSION="${AL_VERSION:-}"

log()  { printf '  \033[1;32m%s\033[0m %s\n' "$1" "$2"; }
err()  { printf '  \033[1;31merror:\033[0m %s\n' "$1" >&2; exit 1; }

# --- Detect platform ---

detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Linux)  OS="unknown-linux-gnu" ;;
        Darwin) OS="apple-darwin" ;;
        *)      err "Unsupported OS: $OS. Only Linux and macOS are supported." ;;
    esac

    case "$ARCH" in
        x86_64|amd64)   ARCH="x86_64" ;;
        aarch64|arm64)   ARCH="aarch64" ;;
        *)               err "Unsupported architecture: $ARCH" ;;
    esac

    TARGET="${ARCH}-${OS}"
    log "detected" "platform $TARGET"
}

# --- Resolve version ---

resolve_version() {
    if [ -n "$VERSION" ]; then
        log "version" "$VERSION (pinned)"
        return
    fi

    log "resolving" "latest release..."
    VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')

    if [ -z "$VERSION" ]; then
        err "Could not determine latest release version. Set AL_VERSION manually."
    fi
    log "version" "$VERSION (latest)"
}

# --- Download and install ---

download_and_install() {
    ARCHIVE="al-${VERSION}-${TARGET}.tar.gz"
    URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARCHIVE}"
    CHECKSUM_URL="${URL}.sha256"

    TMPDIR=$(mktemp -d)
    trap 'rm -rf "$TMPDIR"' EXIT

    log "downloading" "$URL"
    curl -fSL "$URL" -o "${TMPDIR}/${ARCHIVE}" || err "Download failed. Check that ${VERSION} has a release for ${TARGET}."

    # Verify checksum if shasum is available
    if command -v shasum >/dev/null 2>&1; then
        log "verifying" "SHA-256 checksum..."
        curl -fsSL "$CHECKSUM_URL" -o "${TMPDIR}/expected.sha256" 2>/dev/null
        if [ -f "${TMPDIR}/expected.sha256" ]; then
            EXPECTED=$(awk '{print $1}' "${TMPDIR}/expected.sha256")
            ACTUAL=$(shasum -a 256 "${TMPDIR}/${ARCHIVE}" | awk '{print $1}')
            if [ "$EXPECTED" != "$ACTUAL" ]; then
                err "Checksum mismatch! Expected ${EXPECTED}, got ${ACTUAL}"
            fi
            log "verified" "checksum OK"
        fi
    fi

    log "extracting" "$ARCHIVE"
    tar xzf "${TMPDIR}/${ARCHIVE}" -C "$TMPDIR"

    # Find the binary in the extracted directory
    BIN=$(find "$TMPDIR" -name "al" -type f | head -1)
    if [ -z "$BIN" ]; then
        err "Binary 'al' not found in archive."
    fi

    log "installing" "${INSTALL_DIR}/al"
    mkdir -p "$INSTALL_DIR"
    cp "$BIN" "${INSTALL_DIR}/al"
    chmod +x "${INSTALL_DIR}/al"

    log "done" "al ${VERSION} installed to ${INSTALL_DIR}/al"
}

# --- Main ---

main() {
    printf '\n  \033[1mAgentLang Installer\033[0m\n\n'
    detect_platform
    resolve_version
    download_and_install
    printf '\n  Run \033[1mal --help\033[0m to get started.\n\n'
}

main
