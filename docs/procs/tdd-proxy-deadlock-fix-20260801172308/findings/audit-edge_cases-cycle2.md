---
doc_type: proc
brief: "Fidelity audit: 边界覆盖 (cycle 2)"
source_skill: execute
audit_phase: fidelity
audit_angle: edge_cases
audit_cycle: 2
confidence: verified
---

# 审查角度: 边界覆盖 (Cycle 2 — Fix 后复核)

**审查依据**: AC1/AC3/AC10 / Step 5/6/9/11/12
**审查周期**: 2/3
**Fix 依据**: logs/audit-fix-fidelity-cycle1.md（Fix B：新增 `stop_rejects_stale_socket`）

## 评分明细

| # | 检查项 | 评分 | 证据 | 严重程度 |
|---|--------|------|------|----------|
| 1 | AC1 时间界：死锁回归测试带超时断言（3s 界） | 10/10 | 同 cycle 1（tests/proxy_contract.rs:388-443，BUDGET=3s + recv_timeout 双路径有界）；本次全套复跑 12/12 含 concurrent_control_and_http ok | — |
| 2 | AC10 stop 超时三态：无文件/拒绝（stale）/无响应全覆盖 | **10/10** | **修复验证**：tests/proxy_contract.rs:687-725 `stop_rejects_stale_socket`（详见偏离 1 复核）——三态各自独立测试 + 时间界可区分（stale <1s vs 无响应 ≤2.5s）+ 无误报断言 + 反真空守卫；实测 2/2 通过、全套 12/12 | 已修复 |
| 3 | AC3 占端口三态：(a) lsof 报错 (b) 健康复用 (c) 控制死+TCP 被占报错 | 10/10 | 同 cycle 1（proxy_contract.rs:719-766 + launch_proxy_contract.rs:178-215/306-330 + install-script.md:143-153）；本次全套复跑 port_occupied_reports_error_keeps_occupant ok | — |
| 4 | EADDRINUSE 耗尽路径：3×500ms 收敛；先 TCP 后控制下是否仍可达/可测试 | 8/10 | 源证据复核无变化（src/proxy.rs:243-253 TCP 先行败者 exit(1)；263-281 控制段耗尽循环为防御性安全网，正常双启动不可达）；analysis.md:79-100 方向 A 论证复核充分（TCP 仲裁 + 20/20 + SIGSTOP 10/10 承担收敛语义）；耗尽分支本身仍无直接测试 | 次要 |
| 5 | lsof 降级全谱：缺失/失败/空/非 UTF-8 → None → 降级文本 | 9/10 | 源复核无变化：src/proxy.rs:472-484 四守卫齐备（`.output().ok()?` / `!status.success()` / `String::from_utf8().ok()?` / 非空 filter）；单测仅覆盖缺失态（proxy.rs:901-914），失败/空/非 UTF-8 三子路径仍无专门用例 | 次要 |
| 6 | 诊断端口来源：实际绑定端口非硬编码 19191 | 10/10 | 同 cycle 1（proxy.rs 用 `port` 参数；proxy_contract.rs:748 + launch_proxy_contract.rs:320 断言动态端口文本） | — |
| 7 | 双启动收敛界：败者 ≤2s、恰一存活、无 panic；20/20 证据；serial 隔离 | 10/10 | 同 cycle 1；本次全套复跑 double_start_race_one_wins ok | — |
| 8 | 僵尸恢复竞态窗口：双非 panic 出口；无遗漏 expect | 9/10 | 源复核无变化：子 TCP 失败 exit(1)（proxy.rs:245-253）+ 父就绪耗尽 bail（launch.rs:163-173）各自有契约测试；"父 drop 后端口被第三方抢走"完整交错仍无确定性测试 | 次要 |
| 9 | 就绪探测边界：3×500ms 耗尽 → bail，≤2s 不挂起 | 9/10 | 源复核无变化：launch.rs:163-173 耗尽 bail；测试仅覆盖 connect-拒绝快失败（elapsed≈1s）；静默应答者最坏 ~2.5s 形态未测 | 次要 |
| 10 | 并发关闭：shutdown 先响应后删文件；accept 错误 sleep 100ms | 9/10 | 源复核无变化：proxy.rs:525-528 控制侧 sleep 100ms；HTTP accept `Err(_) => continue`（proxy.rs:301）为前序代码；accept 错误分支仍无测试（难诱发） | 次要 |

## 偏离详情

### 偏离 1（Cycle 1 主导，已修复）: AC10 stop 三态中 stale 拒绝态缺直接测试

- **关联检查项**: #2
- **Cycle 1 评分**: 7/10 → **Cycle 2 评分**: 10/10
- **修复验证**（审计员实测，非仅读日志）:
  1. **测试真实存在且通过**: `cargo test --test proxy_contract stop` → `test stop_rejects_stale_socket ... ok` / `test stop_times_out_on_unresponsive_socket ... ok`（2/2）；全套 `cargo test --test proxy_contract` → 12/12（套件数 11→12，与 fix 日志一致）。
  2. **三态全覆盖成立**: ① 无响应（proxy_contract.rs:607-657：UnixListener accept 后 hold 不回包 → ≤2.5s 非 0 + stderr Error + 不误报）；② 无文件（659-679：<1s exit 0 + "Proxy is not running."）；③ stale（687-725：bind 后 drop 文件残留 → <1s 非 0 + stdout 无误报 + stderr Error）。
  3. **时间界可区分**: ③ 断言 `elapsed < 1s`（ECONNREFUSED 即时）与 ① 的 `≤2.5s`（STOP_TIMEOUT=2s+margin）形成对照——三态在时间维度两两可辨。**关键点**：③ 的 <1s 界同时 pin 住"快速拒绝"性质——若回归把 stale 当无响应走 2s 超时路径，exit 非 0 仍过但 elapsed 断言必红，三态不会坍缩。
  4. **可证伪性成立**（旧语义下必红）: 旧实现把 connect 错误吞掉打 "Proxy is not running." exit 0 → `!status.success()` 与 `!stdout.contains(...)` 两条断言必失败；若实现非 0 但静默退出 → `stderr.contains("Error")` 反真空守卫失败。四断言各主覆盖一个可观测面，无重叠死角。
  5. **断言表面与真实代码路径吻合**: main.rs:235-246 `socket_path.exists()` 为 true → `shutdown_proxy` → proxy.rs:121 `UnixStream::connect(socket_path)?` 对无监听路径立即 ECONNREFUSED → `?` 传播 → main 顶层错误处理 `eprintln!("Error: {err:#}")` + exit(1)（main.rs:112-115）。`stderr.contains("Error")` 命中的正是该错误处理器输出，非巧合文本。
  6. **前置条件 pin**: 测试在 drop 后断言 `stale.exists()`（687-699），钉死"文件残留无监听"这一真实 stale 形状；若未来实现删除文件，测试退化成的"无文件"态也会因非 0 断言失败——双保险。
  7. **隔离合规**: `#[serial]` + TempDir + CCT_PROXY_SOCKET env（spawn_stop 用 CARGO_BIN_EXE_cct 真实二进制），无用户实例干扰。
- **残余说明（不扣分）**: `stderr.contains("Error")` 依赖错误处理器 "Error:" 前缀格式，属契约级断言（错误通道携带 shutdown 错误），符合 assert-contracts 规则，可接受。

### 偏离 2: EADDRINUSE 耗尽分支不可达且无直接测试（维持 8/10）

- **关联检查项**: #4
- **复核结论**: 源代码与 analysis 论证均无变化。TCP 先行后控制段 3×500ms 耗尽循环（proxy.rs:263-281）在正常双启动下不可达，仅对僵尸文件/异例触发；收敛语义由 TCP 仲裁层等价承担（20/20 + SIGSTOP 10/10 证据复核仍在）。fix 未触及该分支，无新证据。
- **处置**: 维持 8/10。cycle 1 修复建议为"接受现状（analysis 论证充分）或补单测 pin 非 panic，低优先"——未执行，属可接受的残余低优先项。

### 偏离 3: lsof 降级三子路径无专门测试（维持 9/10）

- **关联检查项**: #5 — 源复核确认四守卫齐备，缺失态有测试；失败/空/非 UTF-8 三子路径仍仅代码审查覆盖。fix 未触及。维持 9/10，低优先。

### 偏离 4: 僵尸恢复竞态组合交错无确定性测试（维持 9/10）

- **关联检查项**: #8 — 两出口契约测试均在（proxy_contract.rs:719-766 + launch_proxy_contract.rs:268-298），组合窗口无确定性手段。fix 未触及。维持 9/10，低优先。

### 偏离 5: 静默应答者形态未测（维持 9/10）

- **关联检查项**: #9 — launch.rs:163-173 复核无变化；≤2s 断言以 connect-拒绝为口径，最坏 ~2.5s 有界。fix 未触及。维持 9/10，低优先。

### 偏离 6: accept 错误分支无测试；HTTP accept 无 sleep 为前序代码（维持 9/10）

- **关联检查项**: #10 — proxy.rs:525-528 控制侧 sleep 100ms 复核在案；HTTP 侧 `Err(_) => continue`（proxy.rs:301）为 HEAD 前序遗留。fix 未触及。维持 9/10，低优先。

## 角度总评

SCORE: 8

**总分**: 8/10（所有检查项最低分——最低为 #4 的 8/10）
**通过阈值**: ≥ 9

**判定**: ⚠️ CONDITIONAL — 主导偏离（#2 stop 三态 stale 拒绝态）已由 Fix B 真实修复并经审计员实测验证（2/2 定向 + 12/12 全套，四断言可证伪、时间界可区分、断言表面与代码路径吻合），#2 升 10/10。总分仍为 8 仅因残余低优先次要项 #4（EADDRINUSE 耗尽分支无直接测试）维持 8/10——该偏离已在 findings/double_start_race_one_wins-analysis.md:79-100 充分论证（方向 A 等价迁移 + 20/20/SIGSTOP 证据），cycle 1 亦判定"接受现状"为合规处置，无正确性风险。

**复核明细（Cycle 2 新增验证动作）**:
- `cargo test --test proxy_contract stop` → 2 passed（stop_rejects_stale_socket + stop_times_out_on_unresponsive_socket）
- `cargo test --test proxy_contract` → 12/12 全套通过（套件数 11→12 与 fix 日志一致）
- git status 确认 fix 仅改 tests/proxy_contract.rs（untracked 新增文件内：新测试 + 原测试文档注释更新），未触及 src/——次要项源证据均复核为无变化

**剩余建议（均低优先）**: #4 若追求闭环可在 proxy.rs 单测用"预占路径 + 并发 bind"构造耗尽分支非 panic 用例；#5 补假 lsof 脚本覆盖非 0 退出与非法 UTF-8 两态；#9 补静默应答者 fake 并把界放宽至 ≤3s。均不阻塞，接受现状亦合规。
