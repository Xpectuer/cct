# Self-Review: macOS default config path documentation fix

## Summary

Updated `README.md` and `CLAUDE.md` so the Quick Start and Project Overview/Config File Format sections explicitly document `~/Library/Application Support/cc-tui/profiles.toml` as the macOS default and `~/.config/cc-tui/profiles.toml` for Linux or other Unix-like systems. Proof-read both files, cross-checked the acceptance criteria, and confirmed the change remains documentation-only.

## Checklist

- [x] Implementation matches the plan steps and referenced files
- [x] Documentation-only change (only README, CLAUDE, progress/logs, and review files modified)
- [x] Verification commands (`rg` checks) completed successfully
- [x] No outstanding TODOs or inconsistent wording remain

## Findings

### Plan Conformance

| Plan Step | Status | Notes |
|-----------|--------|-------|
| Step 1: Update README default path guidance | Match | Quick Start now narrates both platform-specific defaults and manual edit instructions. |
| Step 2: Update CLAUDE.md platform-specific config path references | Match | Project Overview and Config File Format now describe the macOS path alongside the Linux path. |
| Step 3: Proof-read end-to-end | Match | Both modified files were read thoroughly to ensure formatting/punctuation were preserved. |
| Step 4: Cross-check acceptance criteria | Match | `rg` commands confirmed both files contain the desired path strings and no code files changed. |

### Issues

None.

## Verdict

**READY** – Documentation is consistent with the plan, all verification steps passed, and no blockers remain.
