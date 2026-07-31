---
title: "Self-Review: musl Static Linking"
doc_type: proc
brief: Reviewed .github/workflows/release.yml against the plan in ref/plan.md. The workflow replaces the single x86_64-unk
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
# Self-Review: musl Static Linking

## Summary

Reviewed `.github/workflows/release.yml` against the plan in `ref/plan.md`. The workflow replaces the single `x86_64-unknown-linux-gnu` target with two musl targets (`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`), adds musl toolchain and cross-compilation support, updates the binary path for target-specific output directories, and adds static linking verification plus an Ubuntu 20.04 smoke test.

## Checklist

- [x] Implementation matches plan
- [x] YAML syntax valid
- [x] No security concerns
- [x] Edge cases considered

## Findings

### Plan Conformance

Every step in the plan (Steps 1-4) is faithfully reflected in the workflow:

| Plan Step | Status | Notes |
|-----------|--------|-------|
| Step 1: Replace gnu target with two musl targets + `use_cross` flag | Match | Lines 21-25 |
| Step 2: Add musl toolchain install, cross install, conditional build | Match | Lines 33-50 |
| Step 3: Update package step path to `target/${{ matrix.target }}/release` | Match | Lines 52-56 |
| Step 4: Add static linking verification and Ubuntu 20.04 smoke test | Match | Lines 58-67 |

Grep verification counts all pass:
- `linux-musl`: 5 (2 target names, 2 `contains()` conditions, 1 in `rustup target add` run block)
- `musl-tools`: 1
- `cross build`: 1
- `statically linked`: 1
- `ubuntu:20.04`: 1

### YAML Quality

- Indentation is consistent (2-space)
- `if:` conditions use correct GitHub Actions expression syntax
- `contains()` and `==` comparisons are idiomatic
- Matrix `include` structure is standard
- Multi-line `run:` blocks use `|` correctly

### Security

- `softprops/action-gh-release@v2` is pinned to major version (acceptable for most projects; SHA pinning would be stricter but is not required by plan)
- `permissions: contents: write` is correctly scoped to the release job only, not the build job
- Docker smoke test uses `--rm` to clean up containers
- No secrets are exposed in logs

### Edge Cases and Potential Failure Modes

1. **`file` command output format**: The `grep -i "statically linked"` check depends on `file` outputting that exact phrase for musl-linked ELF binaries. On Ubuntu runners with standard `file` versions this is reliable, but worth noting.
2. **Cross installation from git**: `cargo install cross --git https://github.com/cross-rs/cross` builds from HEAD of the default branch. If cross introduces a breaking change, CI could fail. Pinning to a tag (e.g., `--tag v0.2.5`) would be more deterministic, but the plan does not require it.
3. **aarch64 smoke test is absent**: The smoke test only runs for `x86_64-unknown-linux-musl`. The aarch64 binary cannot easily be smoke-tested on an x86_64 runner without QEMU. This is an intentional omission per the plan.
4. **Docker image pull**: The Ubuntu 20.04 smoke test requires pulling `ubuntu:20.04` from Docker Hub. Rate limits could theoretically cause flaky failures on shared runners, though this is rare.
5. **macOS Intel runner**: `macos-15-intel` is used for `x86_64-apple-darwin`. This is a newer runner label; if GitHub deprecates or renames it, the workflow would need updating. This is pre-existing (not introduced by this change).

### No Issues Found

No bugs, typos, or deviations from the plan were identified.

## Verdict

**PASS** -- The implementation faithfully matches the plan across all four steps. YAML is well-formed and idiomatic. No security concerns. The noted edge cases (cross version pinning, Docker rate limits) are minor and do not warrant blocking.
