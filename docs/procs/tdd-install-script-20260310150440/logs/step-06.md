---
title: Step 06
doc_type: proc
brief: Step 06
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
## Case 6 — download_retries_on_failure (MANUAL → stub)

### RED
- Added test that stubs `curl` to always return 1 (failure) and `sleep` to no-op
- Sets MAX_RETRIES=2 for fast test, creates temp TMPDIR_INSTALL
- Asserts non-zero exit and "Download failed after" in output

### GREEN
- Implemented download() in install.sh with retry loop, tar integrity check
- Test passes — retries exhaust and err() fires

### REFACTOR
- No changes needed

### Result
SUCCESS — retry logic verified via curl stub
