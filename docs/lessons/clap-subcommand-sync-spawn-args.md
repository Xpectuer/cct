---
title: "Spawn Args Must Stay in Sync With Clap Subcommand Changes"
doc_type: lesson
brief: "When a clap subcommand gains a required child enum, every code path that spawns the binary with that subcommand must be updated"
confidence: verified
created: 2026-07-15
updated: 2026-07-15
revision: 1
---

# Lesson: Spawn Args Must Stay in Sync With Clap Subcommand Changes

## Context

`cct proxy stop` 功能被添加时（commit `0c3ec46`），引入了 `ProxyCommand` 枚举：

```rust
enum ProxyCommand {
    Start,
    Stop,
}

enum Commands {
    Proxy(ProxyCommand),  // 之前 Proxy 是独立的叶子子命令
}
```

这意味着 `proxy` 从独立子命令变成需要子命令的父命令 —— `cct proxy` 不再有效，
必须是 `cct proxy start` 或 `cct proxy stop`。

## Failure

`ensure_proxy_running`（`src/launch.rs:91`）中 spawn proxy 守护进程的代码未被更新：

```rust
// 错误：仍然只传 "proxy"
cmd.arg("proxy")
    .stdin(std::process::Stdio::null())
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null());
```

由于 stderr 被设为 `Stdio::null()`，clap 的解析错误被静默丢弃。进程立即退出，
Unix socket 从未创建。`ensure_proxy_running` 轮询 socket 5 秒后超时，返回：

> Error: failed to start proxy: proxy did not start within 5 seconds

这个错误信息完全掩盖了真正的失败原因。

## Root Cause

**三层信息丢失**：
1. spawn 时将 stderr 丢弃 → clap 错误不可见
2. 只检查 socket 是否存在（success/failure 信号是间接的）
3. 超时错误消息没有包含子进程退出状态

## Fix

```rust
cmd.arg("proxy")
    .arg("start")  // ← 加上必需的子命令
    .stdin(std::process::Stdio::null())
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null());
```

## Prevention

1. **当修改 clap 子命令结构时**，全文搜索 spawn 或 `Command::new` 中引用该子命令的位置：
   ```bash
   rg '"proxy"' src/
   rg '\.arg\("proxy"\)' src/
   ```
2. **spawn 后的健康检查应该优先检查进程是否还活着**，而不只是轮询副作用（socket）：
   ```rust
   // 在等待 socket 的同时周期性检查子进程是否已退出
   if let Ok(Some(status)) = child.try_wait() {
       anyhow::bail!("proxy exited early with status: {status}");
   }
   ```
3. **对于 effectful 边界代码，至少在 spawn 阶段做 mock/stub 测试**，验证传递的 args 正确。

## See Also

- [docs/failures/launch-failure-20260715144129.md](../failures/launch-failure-20260715144129.md) — 结构化失败记录
- [test-boundaries-with-stubs-before-manual-verification](../rules/test-boundaries-with-stubs-before-manual-verification.md) — 边界测试规范
