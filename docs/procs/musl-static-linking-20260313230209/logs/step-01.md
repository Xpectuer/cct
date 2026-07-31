---
title: Step 01
doc_type: proc
brief: Step 01
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
## Step 1 — Replace linux-gnu target with two musl targets in release workflow

### Actions Taken

- Edited `.github/workflows/release.yml` matrix section.
- Replaced `x86_64-unknown-linux-gnu` entry with two new entries:
  - `x86_64-unknown-linux-musl` on `ubuntu-latest`
  - `aarch64-unknown-linux-musl` on `ubuntu-latest` with `use_cross: true`

### Verify Result

```
$ grep -c 'linux-musl' .github/workflows/release.yml
2
```

Result: **SUCCESS** — both musl targets present in the workflow matrix.
