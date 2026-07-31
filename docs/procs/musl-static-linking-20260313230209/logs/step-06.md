---
title: Step 06
doc_type: proc
brief: Step 06
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
## Step 6 — Acceptance Criteria Cross-Check

### Criteria Table

| Criterion | Expected | Found in release.yml | Lines | Status |
|-----------|----------|---------------------|-------|--------|
| `file` command confirms Linux binaries are "statically linked" | A step with `file ... \| grep -i "statically linked"` | "Verify static linking" step (L58-60): `file target/${{ matrix.target }}/release/cct \| grep -i "statically linked"` — runs for all `linux-musl` targets | 58-60 | PASS |
| cct binary runs in Ubuntu 20.04 Docker container (`cct --help`) | A step with `docker run ... ubuntu:20.04 cct --help` | "Smoke test on Ubuntu 20.04" step (L62-67): `docker run --rm -v .../cct:/usr/local/bin/cct ubuntu:20.04 cct --help` — runs for `x86_64-unknown-linux-musl` only | 62-67 | PASS |
| CI includes a smoke test step that validates musl binary functionality | The Ubuntu 20.04 step above | Same step as above — exercises the binary on a vanilla glibc distro to confirm no dynamic deps | 62-67 | PASS |

### Notes

- The `Verify static linking` step uses `if: contains(matrix.target, 'linux-musl')`, so it covers both `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl`.
- The Docker smoke test is restricted to `x86_64-unknown-linux-musl` (`if: matrix.target == 'x86_64-unknown-linux-musl'`), which is correct because the CI runner is x86_64 and cannot natively execute aarch64 binaries.
- The `--rm` flag ensures no leftover containers.

### Verdict

**SUCCESS** — All three acceptance criteria are addressed with correct conditions and commands.
