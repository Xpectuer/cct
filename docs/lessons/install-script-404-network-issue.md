---
title: "An intercepted GitHub API request can look like a genuine release 404"
doc_type: lesson
brief: "A proxy or intermediary can return HTTP 404 for GitHub API traffic, which the installer cannot distinguish from a repository with no published release"
confidence: verified
created: 2026-03-12
updated: 2026-08-12
revision: 3
---

# An Intercepted GitHub API Request Can Look Like a Genuine Release 404

## Problem

Running the installer via `curl -fsSL .../install.sh | bash` produced:

```
:: Detected platform: aarch64-apple-darwin
Error: Failed to fetch latest release from GitHub API (HTTP 404).
Check that Xpectuer/cc_starter has published releases at
https://api.github.com/repos/Xpectuer/cc_starter/releases/latest
```

## Root Cause

The request path to `api.github.com` was blocked or intercepted. An intermediary returned
HTTP 404 for `/repos/{owner}/{repo}/releases/latest`; the response did not prove that the
repository lacked releases.

## Misleading Error Message

`install.sh` (`fetch_latest_github`) captures the HTTP status code and includes it
verbatim in the error message. A network-level 404 is indistinguishable from a genuine
"no releases published" 404, leading the error message to suggest the wrong fix
(publish a release) when the real problem is connectivity.

## Fix

Check network connectivity to `api.github.com` first:

```bash
curl -sI https://api.github.com/
```

Also inspect the response headers and body. A normal GitHub API response identifies itself
as GitHub and returns JSON; a proxy, firewall, captive portal, or mirror may return its own
404 page. Resolve that network path before retrying the installer.

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
| `install.sh` | `fetch_latest_github` | Error message does not distinguish an intermediary 404 from a genuine missing-release 404 |
