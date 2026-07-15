---
name: ship
description: >
  Ship a new release of cct. Use this skill whenever the user says "ship",
  "release", "publish", "deploy a new version", "cut a release", or wants to
  push a tagged version. Handles the full pipeline: verify (test + clippy),
  commit, version bump, tag, push, and CI/CD monitoring.
compatibility: cargo, git, gh
---

# Ship a cct Release

Ship a new tagged release of cct through the full pipeline: verify, commit,
bump version, tag, push, and monitor CI/CD.

## Prerequisites

- Working directory must be the cct repo root.
- `cargo` must be available.
- `gh` CLI should be available (for GitHub Actions monitoring).
- Git remote `origin` points to `github.com:Xpectuer/cct.git`.

## Procedure

Execute each step in order. Stop and report if any step fails — do not continue
past a failure.

### Step 1: Check for Changes

```bash
git status --porcelain
```

- If there are **staged or modified** files (first/second column non-empty):
  proceed to Step 2.
- If there are **only untracked** files (`??`):
  list them and ask the user whether to include them. If yes, `git add` the
  ones they want and proceed. If no, stop — nothing to ship.
- If the working tree is **completely clean**: tell the user there's nothing
  to ship and stop.

### Step 2: Verify — Format + Clippy + Test

Run all three checks in order. They must pass with zero errors.

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test --verbose
```

If `cargo fmt --check` fails, run `cargo fmt` first, then re-check, then commit the formatting fix before continuing. Do not proceed past a failing clippy or test suite.

If either fails, report the failure output and stop. Do not proceed with a
failing suite.

### Step 3: Commit All Changes

Stage everything and commit. Use a descriptive commit message that summarizes
the changes since the last tag — look at `git diff --stat` and `git log` to
write a meaningful conventional-commit message.

```bash
git add -A
git commit -m "<type>: <summary>"
```

Commit message format: `type: summary` where `type` is one of `feat`, `fix`,
`refactor`, `chore`, `docs`, `test`. Keep the summary concise (under 72 chars).
Emoji prefix (e.g. `📝 docs: ...`) is optional and should match the repo's
existing style.

### Step 4: Determine the Next Version

Find the latest tag:

```bash
git tag --sort=-v:refname | head -1
```

Analyze the commits since that tag to decide the bump level:

- **Patch** (0.0.X → 0.0.X+1): only `fix`, `docs`, `chore`, `refactor`, `test` commits.
- **Minor** (0.X.0 → 0.X+1.0): any `feat` commit present.
- **Major** (X.0.0 → X+1.0.0): breaking changes (a commit with `!` after the type, e.g. `feat!: ...`, or a `BREAKING CHANGE:` footer).

Since cct is pre-1.0, most releases will be patch or minor bumps. When in
doubt, prefer the smaller bump.

### Step 5: Bump Cargo.toml Version

Update `Cargo.toml` to match the new version (without the `v` prefix):

```toml
version = "X.Y.Z"
```

Use `Edit` to change only the `version` line. The tag version and Cargo.toml
version must be identical (tag: `v0.0.31`, Cargo.toml: `0.0.31`).

Commit the version bump:

```bash
git add Cargo.toml
git commit -m "chore: bump version to vX.Y.Z"
```

### Step 6: Tag

Create an annotated tag on the latest commit:

```bash
git tag -a "vX.Y.Z" -m "vX.Y.Z"
```

### Step 7: Push

Push the commits and the tag to `origin` (GitHub):

```bash
git push origin master
git push origin "vX.Y.Z"
```

Then, if a `gitlab` remote exists, push there too (best-effort — failure is a
warning, not a blocker):

```bash
git push gitlab master
git push gitlab "vX.Y.Z"
```

If the gitlab push fails (e.g., remote unreachable, authentication error),
warn the user but continue — the GitHub release is the primary artifact.

### Step 8: Monitor CI/CD

After pushing, both GitHub Actions and GitLab CI will trigger on the new tag
(`v*`).

**GitHub Actions** — use `gh`:

```bash
gh run list --branch "vX.Y.Z" --limit 5 --json status,conclusion,name,url
```

**GitLab CI** — only if `GITLAB_TOKEN` is available. Source `~/.env` (and
`./.env` if present) to load it. Derive `CI_API_V4_URL` and `PROJECT_ID` from
the gitlab remote URL:

```bash
# Source env files to get GITLAB_TOKEN
source ~/.env 2>/dev/null; source ./.env 2>/dev/null

# Derive GitLab API URL from remote (e.g. git@gitlab.clounix.com:zhengjy/cc_starter.git)
CI_API_V4_URL="https://<gitlab-host>/api/v4"
# Project ID is the URL-encoded path after the colon: zhengjy%2Fcc_starter
PROJECT_ID="<url-encoded-path>"

curl -sS --header "PRIVATE-TOKEN: ${GITLAB_TOKEN}" \
  "${CI_API_V4_URL}/projects/${PROJECT_ID}/pipelines?ref=vX.Y.Z&per_page=3"
```

If `GITLAB_TOKEN` is not found in either file, skip GitLab CI monitoring
entirely — just monitor GitHub Actions.

After pushing, set up a cron job (via `CronCreate`, durable: false) to poll
CI status every 5 minutes. Report to the user when:
- **GitHub Actions**: the `Release` workflow completes (release created with
  artifacts).
- **GitLab CI** (if token available): the pipeline completes.

If any job fails, report the failure with a link to the logs immediately.
The cron should auto-delete after all tracked pipelines succeed, or after 30
minutes — ask the user whether to keep waiting if jobs are still running.

## What Gets Built

| Platform | CI Provider |
|----------|-------------|
| aarch64-apple-darwin | GitHub Actions |
| x86_64-apple-darwin | GitHub Actions |
| x86_64-unknown-linux-musl | GitHub Actions + GitLab CI |
| aarch64-unknown-linux-musl | GitHub Actions + GitLab CI |

## Failure Recovery

If any step before the push fails:
- Fix the issue, then resume from the failed step.
- The commits haven't been pushed yet, so `git reset` is safe.

If CI fails after the push:
- Fix the issue in a new commit, bump the version again, and re-ship.
- Never force-push a tag that has already been pushed.
