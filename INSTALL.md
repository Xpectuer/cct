# INSTALL.md — Agent Installation Guide for cct

This document provides automated installation instructions for AI coding agents. Each method is self-contained with prerequisite checks, the install command, and a verification step.

## Prerequisites

Before attempting any install method, verify these are available:

```bash
command -v curl >/dev/null 2>&1 || echo "MISSING: curl"
command -v tar >/dev/null 2>&1 || echo "MISSING: tar"
```

Both are required for the curl|bash method. For the cargo method, only `cargo` is needed.

## Platform Detection

```bash
uname -s   # Darwin (macOS) or Linux
uname -m   # arm64, aarch64, x86_64
```

Supported targets:
| OS | Arch | Target Triple |
|----|------|---------------|
| Darwin | arm64 / aarch64 | `aarch64-apple-darwin` |
| Darwin | x86_64 | `x86_64-apple-darwin` |
| Linux | x86_64 | `x86_64-unknown-linux-musl` |
| Linux | aarch64 | `aarch64-unknown-linux-musl` |

If `uname -m` returns an arch not in this table, the curl|bash method will fail — fall back to the cargo method.

## Method 1: curl|bash from GitHub (recommended)

Single command, no root required. Installs the latest release binary to `~/.local/bin/cct`.

```bash
curl -fsSL https://raw.githubusercontent.com/Xpectuer/cc_starter/refs/heads/master/install.sh | bash
```

**What it does:** detects platform → fetches latest release tag from GitHub API → downloads `cct-<target>.tar.gz` with up to 3 retries → extracts and installs to `~/.local/bin/cct` → prints a PATH hint if `~/.local/bin` is not on PATH.

**Error handling:**
- If the script exits non-zero, read stderr for the specific error (unsupported platform, curl failure, download exhausted retries).
- GitHub API rate limits: if you hit `HTTP 403`, wait 60 seconds and retry.
- Network issues: the script retries downloads 3 times with 2-second delays. If all fail, check connectivity to `api.github.com` and `github.com`.

## Method 1b: curl|bash from self-hosted GitLab

For environments where GitHub is unreachable. Requires a GitLab instance with releases published to the Generic Package Registry.

```bash
curl -fsSL https://gitlab.clounix.com/zhengjy/cc_starter/-/raw/master/install.sh | \
  GITLAB_URL=https://gitlab.clounix.com \
  GITLAB_PROJECT=zhengjy/cc_starter \
  bash
```

Add `GITLAB_TOKEN=<token>` for private GitLab instances.

**Note:** GitLab-hosted releases only provide Linux binaries. On macOS, use Method 1 (GitHub) or Method 2 (cargo).

## Method 2: cargo install (from source)

Requires Rust 1.70+ and a Unix-like OS. Builds from the local checkout.

```bash
# Verify Rust is installed
command -v cargo >/dev/null 2>&1 || echo "MISSING: cargo (install via https://rustup.rs)"

# Build and install
cargo install --path .
```

The binary ends up in `~/.cargo/bin/cct`. Ensure `~/.cargo/bin` is on PATH.

## Method 3: Manual download from GitHub Releases

Use when you need a specific version or the install script is unavailable.

```bash
# Set version and target
VERSION="v0.3.1"                          # replace with desired version
TARGET="aarch64-apple-darwin"             # replace with detected target
INSTALL_DIR="${HOME}/.local/bin"

# Download, extract, install
curl -fsSLO "https://github.com/Xpectuer/cc_starter/releases/download/${VERSION}/cct-${TARGET}.tar.gz"
tar -xzf "cct-${TARGET}.tar.gz"
mkdir -p "${INSTALL_DIR}"
install -m 755 cct "${INSTALL_DIR}/cct"
rm -f cct "cct-${TARGET}.tar.gz"
```

## Verification

After any install method, verify the binary works:

```bash
# Check it's on PATH and executable
which cct

# Run it (exits immediately if no profiles exist, generating default config)
cct --help

# Expected: prints help text with subcommands (add, edit, run, env)
```

If `cct` is not found, add the install directory to PATH:

```bash
export PATH="${HOME}/.local/bin:${HOME}/.cargo/bin:$PATH"
```

To make this permanent, append the same line to `~/.bashrc` or `~/.zshrc`.

## Post-Install

On first run, `cct` generates a default config at:
- **macOS:** `~/Library/Application Support/cc-tui/profiles.toml`
- **Linux:** `~/.config/cc-tui/profiles.toml`

No further setup is required. Profiles can be added via `cct add` (CLI) or by pressing `a` in the TUI.
