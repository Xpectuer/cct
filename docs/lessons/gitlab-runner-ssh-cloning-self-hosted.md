---
title: "Lesson: GitLab Runner SSH Cloning on Self-Hosted Instances"
doc_type: lesson
brief: "How to fix HTTPS clone 403 errors on self-hosted GitLab by switching to SSH — covering clone_url, pre_get_sources_script, helper image quirks, volume permission traps, and tag matching"
confidence: verified
created: 2026-05-13
updated: 2026-05-13
revision: 1
---

# GitLab Runner SSH Cloning on Self-Hosted Instances

## Problem

Self-hosted GitLab instances often block HTTPS git access (403). The runner must clone via SSH instead. Setting this up correctly hits four non-obvious traps.

## Trap 1: Tagged runners ignore untagged jobs

If a runner has tags (e.g., `cc_starter`), it will **only** pick up jobs that specify matching tags. A `.gitlab-ci.yml` without `tags:` produces untagged jobs → runner ignores them → pipeline stays "paused".

**Fix:** Add `default: tags: [cc_starter]` to `.gitlab-ci.yml`, or use job-level `tags:`.

## Trap 2: `clone_url` alone doesn't work — the helper image has no SSH

Setting `clone_url = "ssh://git@gitlab.clounix.com"` makes the runner rewrite clone URLs to SSH. But the gitlab-runner **helper image** (Alpine-based) that executes `git clone` doesn't include an SSH client. Git fails with `cannot run ssh: No such file or directory`.

**Fix:** Add `pre_get_sources_script = "apk add --no-cache openssh"` to install the SSH client before cloning. Note: `pre_get_sources_script` runs in the **helper image**, not the build image. Use `apk`, not `apt-get`.

## Trap 3: `pre_get_sources_script` runs in the helper, not the build image

The helper image is `registry.gitlab.com/gitlab-org/gitlab-runner/gitlab-runner-helper` (Alpine Linux). `apt-get` does not exist there. Always use `apk`.

## Trap 4: Volume-mounted `.ssh` has wrong ownership

Mounting the host's `~/.ssh` directly to `/root/.ssh` in the container preserves host file ownership (e.g., UID 1000). SSH rejects files not owned by the current user (root = UID 0). Error: `Bad owner or permissions on /root/.ssh/config`.

**Fix:** Mount to a staging path, then copy only the needed files in `pre_get_sources_script` with correct permissions:

```toml
volumes = ["/cache", "/home/zhengjy/.ssh:/mnt/ssh:ro"]

pre_get_sources_script = """
apk add --no-cache openssh
mkdir -p /root/.ssh && chmod 700 /root/.ssh
cp /mnt/ssh/id_rsa /root/.ssh/ && chmod 600 /root/.ssh/id_rsa
cp /mnt/ssh/known_hosts /root/.ssh/ && chmod 600 /root/.ssh/known_hosts
"""
```

Only copy `id_rsa` and `known_hosts` — skip `config`, `authorized_keys`, etc. to avoid permission issues.

## Working config

```toml
[[runners]]
  name = "cc_starter"
  executor = "docker"
  clone_url = "ssh://git@gitlab.clounix.com"
  pre_get_sources_script = """
    apk add --no-cache openssh
    mkdir -p /root/.ssh && chmod 700 /root/.ssh
    cp /mnt/ssh/id_rsa /root/.ssh/ && chmod 600 /root/.ssh/id_rsa
    cp /mnt/ssh/known_hosts /root/.ssh/ && chmod 600 /root/.ssh/known_hosts
  """
  [runners.docker]
    image = "rust:1.94"
    volumes = ["/cache", "/home/zhengjy/.ssh:/mnt/ssh:ro"]
```

## Diagnosis checklist

1. Runner shows "online" but pipeline is "paused" → check tag match
2. `fatal: unable to access 'https://...': 403` → HTTPS blocked, switch to SSH
3. `cannot run ssh: No such file or directory` → install openssh in helper via `pre_get_sources_script`
4. `apt-get: command not found` in pre_get_sources_script → use `apk`, not `apt-get`
5. `Bad owner or permissions on /root/.ssh/...` → don't mount `.ssh` directly; copy files with correct ownership
