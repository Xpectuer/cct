---
title: INSTALL_PROMPT.md — Copy-Paste Install Prompt for AI Agents
doc_type: readme
brief: A self-contained prompt that instructs any LLM or AI coding agent to install cct.
confidence: verified
created: 2026-08-05
updated: 2026-08-05
revision: 1
---
# INSTALL_PROMPT.md — Copy-Paste Install Prompt for AI Agents

Copy everything below the divider and paste it into any LLM or AI coding agent (Claude Code, Codex CLI, Cursor, Copilot, etc.). The prompt is self-contained — the agent does not need to read any other file to complete the install.

---

You are an AI coding agent. Install `cct` (a terminal UI launcher for Claude Code and OpenAI Codex) on this machine. The user has asked you to read and follow these installation instructions.

1. **Detect the platform.** Run `uname -s` and `uname -m` to determine the OS and architecture.

2. **Check prerequisites.** `curl` and `tar` must be available. If either is missing, install it with the system package manager.

3. **Install using the first method that applies:**
   - **Public internet (GitHub, recommended):**
     ```bash
     curl -fsSL https://raw.githubusercontent.com/Xpectuer/cc_starter/refs/heads/master/install.sh | bash
     ```
   - **Internal network (self-hosted GitLab)** — Linux only; on macOS use the GitHub or cargo method instead:
     ```bash
     curl -fsSL https://gitlab.clounix.com/zhengjy/cc_starter/-/raw/master/install.sh | \
       GITLAB_URL=https://gitlab.clounix.com GITLAB_PROJECT=zhengjy/cc_starter bash
     ```
   - **Fallback** (no network access to GitHub or GitLab, or architecture unsupported by the release binaries): install Rust 1.70+ (https://rustup.rs), then build from source:
     ```bash
     cargo install --path .
     ```

4. **Verify the install.** The binary must be on PATH and respond to `--help`:
   ```bash
   command -v cct
   cct --help
   ```
   If `cct` is not found, add the install directory to PATH (`~/.local/bin` for curl|bash, `~/.cargo/bin` for cargo) and export it in the shell profile (`~/.bashrc` or `~/.zshrc`).

5. **Report back.** Tell the user which method you used and where the binary was installed.

Notes:
- Installs to `~/.local/bin/cct` (curl|bash) or `~/.cargo/bin/cct` (cargo). No root or sudo required.
- On first run, `cct` generates a default config at `~/Library/Application Support/cc-tui/profiles.toml` (macOS) or `~/.config/cc-tui/profiles.toml` (Linux).
- If a step fails, read the error message, diagnose (network, platform support, permissions), and retry.
