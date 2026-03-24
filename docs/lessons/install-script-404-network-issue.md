---
title: "install.sh shows HTTP 404 due to user network issue blocking GitHub API"
doc_type: lesson
brief: "install.sh reports HTTP 404 from GitHub API when the user's network cannot reach api.github.com; the error message is misleading — it suggests missing releases rather than a connectivity problem."
confidence: verified
created: 2026-03-12
updated: 2026-03-12
revision: 2
---

# install.sh shows HTTP 404 due to user network issue blocking GitHub API

## Problem

Running the installer via `curl -fsSL .../install.sh | bash` produced:

```
:: Detected platform: aarch64-apple-darwin
Error: Failed to fetch latest release from GitHub API (HTTP 404).
Check that Xpectuer/cc_starter has published releases at
https://api.github.com/repos/Xpectuer/cc_starter/releases/latest
```

## Root Cause

The user's network could not reach `api.github.com`. The curl request to
`/repos/{owner}/{repo}/releases/latest` returned HTTP 404 due to the network
blocking or intercepting the connection, not because the repo lacked releases.

## Misleading Error Message

`install.sh` (`fetch_latest`, line 47) captures the HTTP status code and includes it
verbatim in the error message. A network-level 404 is indistinguishable from a genuine
"no releases published" 404, leading the error message to suggest the wrong fix
(publish a release) when the real problem is connectivity.

## Fix

Check network connectivity to `api.github.com` first:

```bash
curl -sI https://api.github.com/
```

If that fails, resolve the network issue (VPN, proxy, firewall, DNS) before retrying
the installer.

## Improvement Opportunity

The install script error message could be improved to hint at both causes:

```
Failed to fetch latest release (HTTP 404).
Possible causes:
  1. No releases have been published yet for Xpectuer/cc_starter
  2. Your network cannot reach api.github.com
```

## Affected Files

| File | Location | Note |
|------|----------|------|
| `install.sh` | `fetch_latest` function, line 47–54 | Error message does not distinguish network failure from missing release |
