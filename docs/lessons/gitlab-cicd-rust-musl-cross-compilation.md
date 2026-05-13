---
title: "GitLab CI/CD Rust musl Cross-Compilation on Self-Hosted Runners"
doc_type: lesson
brief: "How to cross-compile Rust musl binaries on self-hosted GitLab Docker runners — zigbuild over cross, YAML multiline traps, and package registry uploads"
confidence: verified
created: 2026-05-13
updated: 2026-05-13
revision: 1
---

# GitLab CI/CD Rust musl Cross-Compilation on Self-Hosted Runners

## Problem

Cross-compiling a Rust project for `aarch64-unknown-linux-musl` on a self-hosted GitLab Docker executor (x86_64) using `cross` fails because `cross` requires Docker inside the build container — Docker-in-Docker nesting.

## Solution: cargo-zigbuild

`cargo-zigbuild` uses Zig as a static cross-linker — no containers needed. Just install the `zig` binary and `cargo-zigbuild`.

```yaml
build-aarch64-linux-musl:
  before_script:
    - apt-get update && apt-get install -y musl-tools curl xz-utils
    - curl -fsSL https://ziglang.org/download/0.13.0/zig-linux-x86_64-0.13.0.tar.xz | tar xJ -C /usr/local --strip-components=1
    - ln -sf /usr/local/zig /usr/local/bin/zig  # zig extracts to /usr/local, not /usr/local/bin
    - rustup target add aarch64-unknown-linux-musl
  script:
    - cargo install cargo-zigbuild
    - cargo zigbuild --release --target aarch64-unknown-linux-musl
```

## Traps

### Trap 1: `cross` needs Docker in Docker executor containers

`cross build` spawns a Docker container with the aarch64 toolchain. In a Docker executor, the build container has no Docker daemon. Mounting `/var/run/docker.sock` plus installing `docker.io` works but adds significant complexity vs zigbuild.

### Trap 2: `pip3 install ziglang` fails on Debian Trixie (PEP 668)

`rust:1.94` is Debian Trixie. `pip3 install` without `--break-system-packages` fails with "externally-managed-environment". Downloading the Zig static tarball directly avoids Python entirely.

### Trap 3: `zig` extract path not on PATH

The Zig tarball extracts `zig-linux-x86_64-0.13.0/zig` to `/usr/local/zig` with `--strip-components=1`. `/usr/local/bin/` is on PATH, not `/usr/local/`. A symlink fixes it: `ln -sf /usr/local/zig /usr/local/bin/zig`.

### Trap 4: YAML `>` folds shell newlines to spaces

```yaml
# BROKEN — > folds the for-loop into one line, echo eats curl args
after_script:
  - >
    for f in dist/*.tar.gz; do
      echo "Uploading..."
      curl ...
    done

# CORRECT — | preserves newlines
after_script:
  - |
    for f in dist/*.tar.gz; do
      echo "Uploading..."
      curl ...
    done
```

### Trap 5: GitLab API `sort=desc` breaks on some self-hosted instances

The parameter `sort=desc&order_by=released_at` returned a JSON schema instead of data on gitlab.clounix.com. Just using `order_by=released_at` (which defaults to desc) works correctly.

## Upload: GitLab Generic Package Registry

Upload build artifacts via CI job token:

```yaml
after_script:
  - |
    for f in dist/*.tar.gz; do
      curl -sS --fail --header "JOB-TOKEN: ${CI_JOB_TOKEN}" \
           --upload-file "$f" \
           "${CI_API_V4_URL}/projects/${CI_PROJECT_ID}/packages/generic/cct/${CI_COMMIT_TAG}/$(basename $f)"
    done
```

Download URL pattern:
```
${GITLAB_URL}/api/v4/projects/${encoded_project}/packages/generic/cct/${VERSION}/cct-${TARGET}.tar.gz
```

## Working Config

See `.gitlab-ci.yml` — the build stage uses zigbuild for aarch64 and native cargo for x86_64, both uploading to the Generic Package Registry.

## Related

- [[gitlab-runner-ssh-cloning-self-hosted]]
- install.sh now supports GitLab-hosted releases via `GITLAB_URL` / `GITLAB_PROJECT` env vars
