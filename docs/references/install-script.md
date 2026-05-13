---
title: "Reference: cct install.sh"
doc_type: reference
brief: "curl|bash installer for cct — platform detection, GitHub release fetch, download with retry, binary install"
confidence: verified
created: 2026-03-10
updated: 2026-03-10
revision: 1
---

# Reference: install.sh

## Purpose

`install.sh` is a self-contained Bash installer that downloads and installs the latest `cct`
binary from GitHub Releases using a single `curl | bash` invocation.

## Usage

```bash
curl -fsSL https://raw.githubusercontent.com/zhengjy/cc_starter/master/install.sh | bash
```

## Prerequisites

| Tool | Required | Notes |
|------|----------|-------|
| `bash` | Yes | Shebang is `#!/usr/bin/env bash` |
| `curl` | Yes | Checked at runtime via `command -v` |
| `tar` | Yes | Checked at runtime via `command -v` |

## Supported Platforms

| OS | Architecture | Target Triple |
|----|-------------|---------------|
| Linux | x86_64 | `x86_64-unknown-linux-gnu` |
| macOS | arm64 / aarch64 | `aarch64-apple-darwin` |
| macOS | x86_64 | `x86_64-apple-darwin` |

Unsupported OS or architecture exits immediately with a descriptive error.

## Install Location

Binary is placed at `~/.local/bin/cct` (controlled by `INSTALL_DIR="${HOME}/.local/bin"`).
The directory is created with `mkdir -p` if it does not exist.

## Script Functions

### `detect()`
- Calls `uname -s` (OS) and `uname -m` (arch).
- Sets the global `TARGET` variable to the appropriate Rust target triple.
- Exits on unsupported OS or arch via `err()`.

### `fetch_latest()`
- Calls the GitHub API: `https://api.github.com/repos/zhengjy/cc_starter/releases/latest`
- Extracts `tag_name` from the JSON response using `sed`.
- Sets the global `VERSION` variable (e.g., `v0.3.1`).
- Exits if the API call fails or the JSON has no `tag_name`.

### `download()`
- Constructs the download URL: `https://github.com/zhengjy/cc_starter/releases/download/${VERSION}/cct-${TARGET}.tar.gz`
- Downloads to a `mktemp` scratch directory.
- Verifies tarball integrity with `tar -tzf`.
- Retries up to `MAX_RETRIES=3` times with `RETRY_DELAY=2` seconds between attempts.
- Exits with a clear error after exhausting retries.

### `install_binary()`
- Extracts the tarball to the scratch dir.
- Copies `cct` binary to `${INSTALL_DIR}/cct` with mode 755 using `install -m 755`.

### `path_hint()`
- Checks whether `INSTALL_DIR` is already in `$PATH` using a `case` pattern match.
- Prints shell export instructions if it is not found.

### `main()`
- Validates that `curl` and `tar` are available.
- Creates a `mktemp -d` scratch directory; registers `rm -rf` trap on `EXIT`.
- Calls: `detect` → `fetch_latest` → `download` → `install_binary` → `path_hint`.
- Prints success message with installed version.

## Key Behaviors

- **Idempotent**: Re-running overwrites the existing binary (using `install -m 755` which
  replaces in place).
- **Clean temp directory**: All intermediate files are created in a `mktemp -d` directory
  that is unconditionally removed on EXIT via `trap`.
- **No root required**: Installs to `~/.local/bin` — a user-writable directory.
- **Sourcing guard**: The `main` function is only called when the script is executed directly
  (`BASH_SOURCE[0] == 0`), not when sourced (required for BATS test compatibility).

## Test Coverage

Tests live in `tests/install.bats` and use [BATS](https://github.com/bats-core/bats-core).

| Test | Technique |
|------|-----------|
| `detect_linux_x86_64` | Override `uname` with `export -f` |
| `detect_macos_arm64` | Override `uname` with `export -f` |
| `detect_unsupported_os` | Override `uname` returning `FreeBSD` |
| `fetch_latest_parses_version` | Override `curl` with JSON fixture via `export -f` |
| `fetch_latest_fails_on_bad_response` | Override `curl` returning `{"error":"not found"}` |
| `download_retries_on_failure` | Override `curl` always returning exit 1; `sleep` stubbed to no-op |
| `install_binary_creates_dir_and_copies` | Create fake tarball in `mktemp -d`, redirect `INSTALL_DIR` |
| `path_hint_shown_when_not_in_path` | Set `INSTALL_DIR` to a path not in `$PATH` |
| `path_hint_silent_when_in_path` | Set `INSTALL_DIR` to `/usr/bin` (always in `$PATH`) |

Run tests with:
```bash
bats tests/install.bats
```

## GitLab (Self-Hosted) Support

When `GITLAB_URL` is set, the script uses the GitLab API and Generic Package Registry instead of GitHub:

```bash
GITLAB_URL=https://gitlab.example.com \
GITLAB_PROJECT=group/project \
GITLAB_TOKEN=glpat-xxxx \
bash install.sh
```

| Variable | Default | Notes |
|----------|---------|-------|
| `GITLAB_URL` | (unset) | Set to use GitLab instead of GitHub |
| `GITLAB_PROJECT` | `${REPO}` | GitLab project path (e.g., `group/project`) |
| `GITLAB_TOKEN` | (unset) | Optional, for private GitLab instances |

**Note:** GitLab-hosted releases only provide Linux musl binaries (no macOS). On macOS, leave `GITLAB_URL` unset to use GitHub.

## Configuration Variables

| Variable | Default | Notes |
|----------|---------|-------|
| `REPO` | `Xpectuer/cc_starter` | GitHub repo identifier |
| `INSTALL_DIR` | `${HOME}/.local/bin` | Destination for the binary |
| `MAX_RETRIES` | `3` | Number of download attempts |
| `RETRY_DELAY` | `2` | Seconds between retry attempts |
| `GITLAB_URL` | (unset) | Self-hosted GitLab base URL |
| `GITLAB_PROJECT` | (unset) | GitLab project path |
| `GITLAB_TOKEN` | (unset) | GitLab personal access token |
