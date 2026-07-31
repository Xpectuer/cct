---
title: Step 03
doc_type: proc
brief: Step 03
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
## Step 3 — Fix binary path in Package step

### Actions Taken

- Edited `.github/workflows/release.yml` line 54: changed `cd target/release` to `cd target/${{ matrix.target }}/release`.
- Updated the tar output path from `../../` to `../../../` and the `cd` back from `../..` to `../../..` to account for the extra directory level.

### Verify Result

```
grep 'matrix.target.*release' .github/workflows/release.yml | grep -c 'cd target'
```

Output: `1` — PASS.
