---
title: "Proxy Forwarding Base URL Must Not Carry the API Version Prefix"
doc_type: lesson
brief: "The proxy concatenates base_url + client path; OpenAI-compatible clients send paths that already start with /v1, so a base_url ending in /v1 produces /v1/v1/... and a 404 — verify prefix semantics against real client traffic"
confidence: verified
created: 2026-08-04
updated: 2026-08-04
revision: 1
---

# Lesson: Proxy Forwarding Base URL Must Not Carry the API Version Prefix

## Context

The `clauddy-codex` profile in `profiles.toml` had:

```toml
base_url = "https://clauddy.com/v1"
```

The cct proxy forwards by plain concatenation: `active.base_url + path_and_query`
(`src/proxy.rs` `handle_request`).

## The Bug

Real codex 0.146 traffic through the proxy (with the deadlock fix binary):

```
codex 请求:   POST /v1/responses
proxy 拼接:  https://clauddy.com/v1 + /v1/responses = https://clauddy.com/v1/v1/responses
上游响应:    404 (×6 retries, then codex gives up)
```

Direct verification of the two candidate URLs:

- `https://clauddy.com/v1/v1/responses` → **404**
- `https://clauddy.com/v1/responses` → 500 (path recognized; request body was truncated in the probe)

After switching the proxy to `base_url = "https://clauddy.com"` (no `/v1`):

```
codex 请求:   POST /v1/responses
proxy 拼接:  https://clauddy.com + /v1/responses = https://clauddy.com/v1/responses
上游响应:    200 (streaming) ✓
```

## Root Cause

- OpenAI-compatible clients (codex, and Claude with `ANTHROPIC_BASE_URL`) put the
  version prefix `/v1` in their request *path*, not in the configured base URL.
- The proxy forwards the client path verbatim, so the base URL must be the
  upstream *root* — carrying `/v1` in the base URL doubles the prefix.
- The project's own doc example was wrong: `docs/references/codex-backend-development-guide.md`
  showed `base_url = "https://api.openai.com/v1"`, which misleads users into
  the same mistake.

## The Fix

- Changed the profile: `base_url = "https://clauddy.com"` (drop `/v1`).
- Doc example in `codex-backend-development-guide.md` should be corrected to the
  no-prefix form.

## Rule Derived

> For a forwarding proxy, the base_url prefix semantics (concatenation vs
> replacement) must be verified against *real client traffic* — run the actual
> client once and read the upstream URL the proxy logs. Document examples must
> match the code's concatenation convention.

## Symptoms to Watch For

- Client requests 404/405 at the upstream even though the upstream is healthy
  when hit directly.
- The proxy log shows a doubled path segment (`/v1/v1/...`).
- A profile works for one client backend (Claude) but not another (codex)
  because of where the `/v1` lands.

## Related

- [[external-tool-config-schema-must-be-verified]] — sibling lesson: verify
  external contracts against working examples, never by analogy
- `src/proxy.rs` — `handle_request` concatenation logic
- `docs/references/codex-backend-development-guide.md` — the doc example that needs fixing
