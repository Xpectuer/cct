---
title: "audit-fix — Cycle 2"
brief: "审计 cycle 3 前置修复：控制 socket 重探测耗尽测试 + code-spec Step 6 注记 + guide footer 文案对齐"
doc_type: proc
created: 2026-08-02T00:00:00+0800
case: audit-fix-cycle2
phase: audit-fix
---

## 修复日志: Cycle 2（审计 cycle 3 前置）

依据: `findings/audit-edge_cases-cycle2.md` Item 4（8/10）与 `findings/audit-fidelity-cycle2.md`
偏差 1 残差 + 新发现。修复范围（audit 任务规则）：仅 `tests/proxy_contract.rs`、
`docs/procs/tdd-proxy-deadlock-fix-20260801172308/ref/plan/code-spec.md`、
`docs/references/codex-backend-development-guide.md` 三文件。

### 偏离 D1: 控制 socket EADDRINUSE 重探测耗尽分支无测试

- **修复文件**: `tests/proxy_contract.rs`
- **修复内容**:
  - **before**: 控制段 EADDRINUSE/EEXIST 防御路径（src/proxy.rs:261-286：EEXIST →
    探测 → 死才删 → rebind；rebind 仍冲突 → 重探测 PROBE_RETRIES×PROBE_TIMEOUT 耗尽 →
    exit_bind_failed exit(1)）在 TCP-first 顺序下正常双启动场景不可达，全仓无直接测试。
  - **after**: 新增集成测试 `control_socket_rebind_exhaustion_exits`（`#[test] #[serial]`，
    置于 `double_start_race_one_wins` 之后）：
    1. 占用者形态为**空目录**而非审计建议的"循环 bind→sleep→drop 抢绑线程"——
       设计决策（测试注释与下方说明详述）：目录不可被 `remove_file` 删除（macOS
       EPERM / Linux EISDIR，proxy 侧 `let _ =` 吞掉），重绑瞬间路径必然仍被占，
       确定性命中耗尽分支；循环抢绑线程与 proxy 的 remove→rebind 存在 µs 级竞态
       （抢绑者下一次 bind 必须恰落在此窗口内才赢，proxy 重绑几乎总是先到 →
       "偶发 proxy 恰好 rebind 成功"实为主流时序 → flake）。本机实测 macOS 行为：
       bind 目录 → EADDRINUSE（os error 48，`is_bind_conflict` 匹配）；unlink 目录 →
       EPERM；connect 目录 → ENOTSOCK 立即失败（探测快速失败）；Linux 同为
       EADDRINUSE（dentry 存在即冲突）——即 proxy.rs 注释定义的"僵尸文件/异例
       （非 proxy 进程占路径）"场景，正是 TCP 先行后该分支仅剩的触发形态。
    2. 流程: `create_dir` 占住 `CCT_PROXY_SOCKET` 路径 → `spawn_proxy`（TCP 动态
       端口空闲 → TCP bind 成功）→ 控制 bind EADDRINUSE → 探测 false（connect
       立即失败）→ remove 失败（忽略）→ rebind 仍冲突 → 3×（探测 false + sleep
       500ms）耗尽 → exit_bind_failed → exit(1)。
    3. 断言四件套（审计要求逐项）：
       - `!status.success()` —— 非 0 退出（耗尽路径 exit(1)，非 panic 101）；
       - `stderr.contains("control socket bind")` —— exit_bind_failed 诊断文本；
       - `!stderr.contains("panic")` —— 无 panic；
       - `elapsed <= 3s` —— 有界收敛不挂起（算法最坏 ~1.5s：3×500ms sleep +
         探测均瞬时失败，实测 1.58-1.63s）。
  - **验证**:
    - 全量 `cargo test --test proxy_contract` 连跑 3 次，exit 0，13 passed（12+1）：
      run1 `13 passed (1 suite, 7.63s)` / run2 `13 passed (1 suite, 7.79s)` /
      run3 `test result: ok. 13 passed; 0 failed ... 7.68s`
    - 新测试单独连跑 3 次，全部 ok（1.63s / 1.62s / 1.58s）——非 flake，
      耗时与 3×500ms 重探测 sleep 的预期一致。

### 偏离 D2: code-spec.md Step 6 未记录 TCP-first 执行修订

- **修复文件**: `docs/procs/tdd-proxy-deadlock-fix-20260801172308/ref/plan/code-spec.md`
- **修复内容**:
  - **before**: Step 6（`## Step 6 — run_proxy 启动：先探测再删 + bind 失败报错`）的
    New 代码块仍为控制 socket 先 bind 的旧顺序（先探测→删→bind），未记录 TC-12
    竞态修复将 bind 顺序改为 TCP 先行（findings/double_start_race_one_wins-analysis.md
    的论证），与落地代码（src/proxy.rs:239-243 注释）存在计划-实现保真度缺口。
  - **after**: 在 Step 6 小节末尾（**Verify** 行之后、`## Step 7` 之前）surgical 追加
    一行执行修订注记（原文照抄，未改其它内容）：
    `（执行修订：bind 顺序为 TCP 先行——双启动竞态由 TCP 仲裁收敛（findings/double_start_race_one_wins-analysis.md）；控制段 delete-on-conflict + EEXIST 重探测耗尽保留作僵尸/抢绑防御。）`
- **验证**: `git diff` 仅新增该行（surgical 追加）；findings 相对路径目标文件
  `findings/double_start_race_one_wins-analysis.md` 存在。

### 偏离 D3: guide footer 文案与 ui.rs 标签不符

- **修复文件**: `docs/references/codex-backend-development-guide.md`
- **修复内容**:
  - **before**: 第 146 行 "the footer hint changes to `s: Full-auto` on the Codex tab"
    —— 与代码不符（ui.rs 的 `[s] Approval`，且 ui.rs:468 测试断言
    `codex_footer.contains("[s] Approval")`；`Full-auto` 为旧版文案残留）。
  - **after**: 改为与代码一致：
    "the footer hint changes to `s: Approval` on the Codex tab"（surgical 单行替换）。
- **验证**: grep 确认 ui.rs 实际标签（`src/ui.rs:90` Codex footer `[s] Approval`、
  `src/ui.rs:468` 测试断言同一文本）；guide:146 现与代码一致。

## 汇总验证（全部修复后）

1. `cargo test --test proxy_contract` ×3：exit 0，13 passed / 13 passed / 13 passed
   （含新增 `control_socket_rebind_exhaustion_exits`）。
2. 新测试单独 ×3：ok（1.63s / 1.62s / 1.58s），稳定非 flake。
3. `bash docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/scripts/verify-B013-doc-cleanup.sh`：
   `[PASS] B013: 5 份文档无陈旧叙述, resume 过滤语义已说明`，exit 0。
