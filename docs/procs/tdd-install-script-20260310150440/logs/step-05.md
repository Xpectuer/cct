---
title: Step 05
doc_type: proc
brief: Step 05
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
## Case 5 — fetch_latest_fails_on_bad_response (MANUAL → stub)

### RED
- Added test that stubs `curl` to return JSON without tag_name field
- Test asserts non-zero exit and "Could not parse release version" in output

### GREEN
- fetch_latest() already handles this — `[ -n "${VERSION}" ]` check triggers err()
- Test passes

### REFACTOR
- No changes needed

### Result
SUCCESS — error path already covered by fetch_latest() implementation
