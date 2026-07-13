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

If there are no uncommitted changes (clean working tree), tell the user there's
nothing to ship and stop.

### Step 2: Verify — Test + Clippy

Run both checks. They must pass with zero errors.

```bash
cargo test --verbose
cargo clippy -- -D warnings
```

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

Push the commits and the tag:

```bash
git push origin master
git push origin "vX.Y.Z"
```

### Step 8: Monitor CI/CD

After pushing, both GitHub Actions and GitLab CI will trigger on the new tag
(`v*`). Set up a cron job to monitor them every 5 minutes:

```bash
# Check GitHub Actions run for the tag
gh run list --branch "vX.Y.Z" --limit 5 --json status,conclusion,name,url

# Check GitLab pipeline for the tag
curl -sS --header "PRIVATE-TOKEN: ${GITLAB_TOKEN}" \
  "${CI_API_V4_URL}/projects/${PROJECT_ID}/pipelines?ref=vX.Y.Z&per_page=3"
```

Use a 5-minute cron (via `CronCreate`) to poll the CI status. Report to the
user when:
- **GitHub Actions**: the `Release` workflow completes successfully (release
  created with artifacts).
- **GitLab CI**: the `release` job completes successfully.

If a job fails, report the failure with a link to the logs immediately — don't
wait for the full pipeline.

The cron should auto-delete after both pipelines succeed (or after 30 minutes
if still pending — ask the user whether to keep waiting).

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
