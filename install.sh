#!/usr/bin/env bash
set -euo pipefail

# --- helpers ---

err() {
    echo "Error: $*" >&2
    exit 1
}

log() {
    echo ":: $*"
}

# --- detect platform ---

detect() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "${os}" in
        Darwin)
            case "${arch}" in
                arm64|aarch64) TARGET="aarch64-apple-darwin" ;;
                x86_64)        TARGET="x86_64-apple-darwin" ;;
                *)             err "Unsupported architecture on macOS: ${arch}" ;;
            esac
            ;;
        Linux)
            case "${arch}" in
                x86_64) TARGET="x86_64-unknown-linux-musl" ;;
                *)      err "Unsupported architecture on Linux: ${arch}" ;;
            esac
            ;;
        *) err "Unsupported OS: ${os}" ;;
    esac

    log "Detected platform: ${TARGET}"
}

# --- fetch latest release ---

# GitHub (default)
REPO="Xpectuer/cc_starter"

# GitLab (self-hosted) — set GITLAB_URL to use GitLab instead of GitHub
#   GITLAB_URL:     https://gitlab.example.com
#   GITLAB_PROJECT: group/project (default: ${REPO})
#   GITLAB_TOKEN:   optional, for private GitLab instances

fetch_latest() {
    if [ -n "${GITLAB_URL:-}" ]; then
        fetch_latest_gitlab
    else
        fetch_latest_github
    fi
    log "Latest release: ${VERSION}"
}

fetch_latest_github() {
    local api_url="https://api.github.com/repos/${REPO}/releases/latest"
    local response
    local http_code

    http_code="$(curl -sL -o /dev/null -w '%{http_code}' "${api_url}" 2>/dev/null)" || true

    response="$(curl -fsSL "${api_url}" 2>/dev/null)" \
        || err "Failed to fetch latest release from GitHub API (HTTP ${http_code}). Check that ${REPO} has published releases at ${api_url}"

    VERSION="$(echo "${response}" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)"

    [ -n "${VERSION}" ] || err "Could not parse release version from GitHub API response"
}

fetch_latest_gitlab() {
    local gitlab_project="${GITLAB_PROJECT:-${REPO}}"
    local encoded_project
    encoded_project="$(echo "${gitlab_project}" | sed 's|/|%2F|g')"

    local api_url="${GITLAB_URL}/api/v4/projects/${encoded_project}/releases?per_page=1&order_by=released_at"
    local response
    local http_code
    local auth_header=""

    if [ -n "${GITLAB_TOKEN:-}" ]; then
        auth_header="--header PRIVATE-TOKEN: ${GITLAB_TOKEN}"
    fi

    http_code="$(curl -sL -o /dev/null -w '%{http_code}' ${auth_header} "${api_url}" 2>/dev/null)" || true

    response="$(curl -fsSL ${auth_header} "${api_url}" 2>/dev/null)" \
        || err "Failed to fetch latest release from GitLab API (HTTP ${http_code}). Check that ${gitlab_project} has published releases at ${api_url}"

    # the response is a JSON array; take the tag_name of the first element
    VERSION="$(echo "${response}" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)"

    [ -n "${VERSION}" ] || err "Could not parse release version from GitLab API response"
}

# --- download with retry ---

MAX_RETRIES=3
RETRY_DELAY=2

download() {
    if [ -n "${GITLAB_URL:-}" ]; then
        download_gitlab
    else
        download_github
    fi
}

download_github() {
    local url="https://github.com/${REPO}/releases/download/${VERSION}/cct-${TARGET}.tar.gz"
    local attempt=1

    while [ "${attempt}" -le "${MAX_RETRIES}" ]; do
        log "Downloading cct-${TARGET}.tar.gz (attempt ${attempt}/${MAX_RETRIES})..."

        if curl -fSL "${url}" -o "${TMPDIR_INSTALL}/cct.tar.gz" 2>/dev/null \
            && tar -tzf "${TMPDIR_INSTALL}/cct.tar.gz" >/dev/null 2>&1; then
            log "Download verified."
            return 0
        fi

        rm -f "${TMPDIR_INSTALL}/cct.tar.gz"

        if [ "${attempt}" -lt "${MAX_RETRIES}" ]; then
            log "Download failed. Retrying in ${RETRY_DELAY}s..."
            sleep "${RETRY_DELAY}"
        fi

        attempt=$((attempt + 1))
    done

    err "Download failed after ${MAX_RETRIES} attempts"
}

download_gitlab() {
    local gitlab_project="${GITLAB_PROJECT:-${REPO}}"
    local encoded_project
    encoded_project="$(echo "${gitlab_project}" | sed 's|/|%2F|g')"

    local url="${GITLAB_URL}/api/v4/projects/${encoded_project}/packages/generic/cct/${VERSION}/cct-${TARGET}.tar.gz"
    local auth_header=""

    if [ -n "${GITLAB_TOKEN:-}" ]; then
        auth_header="--header PRIVATE-TOKEN: ${GITLAB_TOKEN}"
    fi

    local attempt=1

    while [ "${attempt}" -le "${MAX_RETRIES}" ]; do
        log "Downloading cct-${TARGET}.tar.gz from GitLab (attempt ${attempt}/${MAX_RETRIES})..."

        if curl -fSL ${auth_header} "${url}" -o "${TMPDIR_INSTALL}/cct.tar.gz" 2>/dev/null \
            && tar -tzf "${TMPDIR_INSTALL}/cct.tar.gz" >/dev/null 2>&1; then
            log "Download verified."
            return 0
        fi

        rm -f "${TMPDIR_INSTALL}/cct.tar.gz"

        if [ "${attempt}" -lt "${MAX_RETRIES}" ]; then
            log "Download failed. Retrying in ${RETRY_DELAY}s..."
            sleep "${RETRY_DELAY}"
        fi

        attempt=$((attempt + 1))
    done

    err "Download failed after ${MAX_RETRIES} attempts"
}

# --- install binary ---

INSTALL_DIR="${HOME}/.local/bin"

install_binary() {
    mkdir -p "${INSTALL_DIR}"

    tar -xzf "${TMPDIR_INSTALL}/cct.tar.gz" -C "${TMPDIR_INSTALL}/"

    install -m 755 "${TMPDIR_INSTALL}/cct" "${INSTALL_DIR}/cct" \
        || err "Failed to install cct to ${INSTALL_DIR}"

    log "Installed cct to ${INSTALL_DIR}/cct"
}

# --- PATH hint ---

path_hint() {
    case ":${PATH}:" in
        *":${INSTALL_DIR}:"*) ;;
        *)
            echo ""
            echo "Add ${INSTALL_DIR} to your PATH:"
            echo "  export PATH=\"\${HOME}/.local/bin:\$PATH\""
            echo ""
            echo "Add the line above to ~/.bashrc or ~/.zshrc to make it permanent."
            ;;
    esac
}

# --- main ---

main() {
    command -v curl >/dev/null 2>&1 || err "curl is required but not found"
    command -v tar  >/dev/null 2>&1 || err "tar is required but not found"

    TMPDIR_INSTALL="$(mktemp -d)"
    trap 'rm -rf "${TMPDIR_INSTALL}"' EXIT

    detect
    fetch_latest
    download
    install_binary
    path_hint

    echo ""
    log "cct ${VERSION} installed successfully!"
}

# Only run main when executed directly, not when sourced.
# When piped via curl|bash, BASH_SOURCE is empty — treat that as direct execution.
if [[ -z "${BASH_SOURCE[0]:-}" ]] || [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main
fi
