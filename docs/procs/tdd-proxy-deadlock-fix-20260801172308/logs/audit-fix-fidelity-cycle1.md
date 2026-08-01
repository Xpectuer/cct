---
title: "audit-fix-fidelity — Cycle 1"
brief: "计划忠实度审计 cycle 1 修复：TC-9 case ③ stale socket 快错误路径回归测试"
doc_type: proc
created: 2026-08-02T00:00:00+0800
case: audit-fix-fidelity
phase: audit-fix
---

## 修复日志: 计划忠实度 — Cycle 1

依据: `findings/audit-fidelity-cycle1.md` 偏离 1（TC-9 case ③ stale socket 快错误路径
无自动化断言）。修复范围: 仅 `tests/proxy_contract.rs`（audit 任务规则 1：不动 src/、
poc/、其它 docs）。

### 偏离 B1: TC-9 case ③ stale socket 回归测试

- **修复文件**: `tests/proxy_contract.rs`
- **修复内容**:
  - **before**: `stop_times_out_on_unresponsive_socket` 仅覆盖两态——① socket 存在但
    无响应（`UnixListener::bind` + accept 后 hold 住不回包 → ≤2.5s 非 0 + stderr 含
    Error + 不误报 "Proxy is not running."）；② socket 不存在（→ 快速 <1s exit 0 +
    "Proxy is not running."）。tdd.md:50 承诺的 case ③（socket 文件存在但 connect
    立即拒绝——旧版遗留死 socket → 快速非 0）无任何自动化断言；代码路径存在
    （main.rs `stop_proxy` 因 `socket_path.exists()` 不走 not-running 分支 →
    `shutdown_proxy` → `send_control_timeout` 的 `UnixStream::connect` ECONNREFUSED
    `?` 传播 → 快速非 0），但全仓无测试锁定该语义。
  - **after**: 新增独立测试函数 `stop_rejects_stale_socket`（`#[test] #[serial]`），
    置于 `stop_times_out_on_unresponsive_socket` 之后：
    1. `UnixListener::bind` 临时路径后立即 `drop` —— socket 文件残留、无人 accept
       （旧版遗留死 socket 的真实形状），并加 `stale.exists()` 前置断言；
    2. `spawn_stop(&stale)` + `wait_with_output()`（复用既有 helper）——connect 立即
       ECONNREFUSED，不存在挂起风险，无需 wait_child_exit 轮询；
    3. 断言四件套：
       - `!output.status.success()` —— 快速**非 0** 退出；
       - `elapsed < Duration::from_secs(1)` —— 快速（ECONNREFUSED 即时返回，远小于
         ① 的 2.5s 超时预算）；
       - `!stdout.contains("Proxy is not running.")` —— 不得误报 not running；
       - `stderr.contains("Error")` —— stderr 必须携带 shutdown connect 错误
         （反真空守卫，防该测试路径静默退化）。
    同时将原测试的文档注释更新为三态全覆盖说明（① ② 本测试 + ③ 新测试），与
    tdd.md TC-9 case 1-3 对齐。
- **验证**: `cargo test --test proxy_contract`（raw `rtk proxy` 输出）：
  ```
  running 12 tests
  test stop_rejects_stale_socket ... ok
  test stop_times_out_on_unresponsive_socket ... ok
  test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.06s
  ```
  复跑捕获退出码：`EXIT_CODE=0`（12/12 全绿，含新增的 case ③ 测试，套件数由 11 增至
  12——与 verify 命令"12 个测试——含新增的"一致）。
- **可证伪性**: 该测试在"错误实现（误报 not running exit 0）"下必然 FAIL，理由：
  1. 旧实现（或任何回退为"把 connect 错误当 not running 吞掉"的实现）在 `stop_proxy`
     中对 `shutdown_proxy` 错误不传播、改打 "Proxy is not running." 并返回 Ok →
     `!output.status.success()` 断言直接失败（实际得到 exit 0）；
  2. 同一实现必然向 stdout 打印 "Proxy is not running."（main.rs 的唯一成功消息路径）
     → `!stdout.contains("Proxy is not running.")` 断言失败；
  3. 吞掉错误后 stderr 无任何错误行 → `stderr.contains("Error")` 反真空守卫失败，
     即使前两条被放宽也会红。
  即三条断言各自主覆盖错误实现的"退出码错误"与"误报文本"两个可观测面，修复前
  （错误语义）与修复后（正确语义）输出可区分；任何一处回退（例如有人在
  `shutdown_proxy` 把 connect 错误转 Ok、或在 `stop_proxy` 加 catch-all 吞错）都会
  让本测试变红。另注：此路径为即时 ECONNREFUSED，无挂起风险，`<1s` 预算对正常
  机器余量充足，不存在 flake 面。
