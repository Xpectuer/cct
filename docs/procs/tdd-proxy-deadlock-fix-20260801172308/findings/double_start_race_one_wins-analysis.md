---
title: "double_start_race_one_wins — Failure Analysis"
brief: "双启动竞态根因分析"
doc_type: finding
created: 2026-08-01T12:55:00Z
step: 12
---

# double_start_race_one_wins — Failure Analysis

关联失败记录: `findings/double_start_race_one_wins-failure-attempt-1.md`
（status_a=Some(256) + status_b=None + check_proxy_running false 间歇 flake，本分析已复现并定位根因）

## Root Cause

**探测误判（500ms 应用层探测作为 liveness oracle）+ 输家退出不清理自己刚绑的控制 socket 文件**。精确竞态路径：

```
时间线（A=先 bind 控制 socket 的胜者，B=后到的败者；负载下调度任意交错）：
1. A: TokioUnixListener::bind(sock) OK → spawn 控制 accept loop → TcpListener::bind(port) OK
   （A 已持有 TCP + 控制通道，但 accept loop 可能尚未被 current_thread runtime 调度，
     或正被 CPU 饥饿）
2. B: 控制 bind → EEXIST（A 的文件）→ 分支 is_bind_conflict：
   B 调用 check_proxy_running(sock) —— 单次 500ms 应用层探测（约束 #2 写死）
3. 若 A 在 500ms 窗口内未应答（启动中未调度 / CPU 饥饿 / SIGSTOP）→ B 误判 A 为僵尸
4. B: remove_file(A 的活 socket 文件) → 重绑控制 OK（B 的文件）   ← delete-on-conflict 的 TOCTOU 残差
5. B: TCP bind → EADDRINUSE（A 持有 TCP）→ exit(1)
   —— TCP bind 失败路径（src/proxy.rs:283-292）不删除自己刚绑的控制 socket 文件
6. 终态：A 存活（TCP 正常），但其控制 inode 已被 B unlink（孤儿监听）；
   路径上的文件是 B 的死 socket（B 退出未清理）→ check_proxy_running → ECONNREFUSED → false
7. 断言 tests/proxy_contract.rs:775 失败：status_a=Some(256)、status_b=None、check false
   —— 与 failure-attempt-1 记录的形态逐字段一致
```

**机制分类结论**：

- **(a) TCP/控制双锁交叉：排除，不可能发生。** 控制 bind 位于 TCP bind 之前，构成单门（single gate）：
  B 的 TCP bind 被 B 的控制 bind 门控——B 处在控制冲突分支时**尚未**持有 TCP；A 的控制 bind 要么成功
  （A 是胜者，B 探测它），要么失败（A 退出）。任意时刻至多一个进程持有 TCP，且持 TCP 者一定是
  控制 bind 成功者。~500 次实验 + 40 次契约测试零观测"双双死亡"。
- **(b) 探测误判 delete-on-conflict：根因。** 上述步骤 3-6。探测对"已绑定但未应答"的启动中/饥饿实例
  超时误判为死 socket → 删除活 socket → 重绑 → TCP 冲突退出 → 死文件遮蔽幸存者控制通道。
  与 refactor 日志"手动压力竞态实测 2/12 双绑"同机制；本次已确定性复现（见 Evidence）。
- **(c) 纯饥饿探测超时（无删除）：同根因的第二形态。** 一次重载观测（8 burners + 8 并发脚本）：
  胜者 B 正常应答了败者的探测（`ctl << status`），2.5s 后最终探测对存活的 B 超时失败
  （B 的 accept loop 被饥饿 >500ms，连接被 accept 后读到 EOF —— "ctl << empty command"）。
  同样源于"单次 500ms 探测在重载下不可靠"。

**放大器**：步骤 5 输家退出路径不清理自己已绑的控制 socket 文件（shutdown 分支删文件，
但 TCP-bind 失败 exit(1) 分支不删）——把瞬时误判变成**永久性**遮蔽：即使胜者之后恢复调度，
其控制通道已 unlink，路径上是败者的死文件。

## Evidence

实验均在 worktree `target/debug/cct` 真实二进制 + 临时 socket + 动态端口（与契约测试同 env
CCT_PROXY_SOCKET/CCT_PROXY_PORT/CCT_PROXY_LOG=1），脚本在 /tmp（未改动任何仓库文件）。

| 实验 | 样本 | 结果 |
|------|------|------|
| 空闲机双启动循环（back-to-back） | 60 | 60/60 干净收敛（败者 exit(1) + 胜者健康） |
| 并行负载 4 脚本 × 50 | 200 | 200/200 干净收敛 |
| 并行负载 4 脚本 + 4 burners × 40 | 160 | 160/160 干净收敛（饥饿随机，非负载单调） |
| 重载 8 脚本 + 8 burners × 40 | 40 (h1) | **1/40 真实失败**：败者 exit(1)（"another live proxy owns"），胜者存活且曾正常应答，最终探测 BrokenPipe；胜者 stderr 见 "ctl << empty command"（形态 c） |
| 契约测试 `double_start_race_one_wins` × 20 两轮 | 40 | **2/40 真实失败**（每轮第 1 次即红），两次均为 line 775 断言，完整消息：`status_a=Some(ExitStatus(unix_wait_status(256))), status_b=None` —— 与记录 flake 逐字段一致 |
| **确定性复现（SIGSTOP 冻结胜者）** | D=100ms × 10 | **10/10 复现记录形态**：胜者存活、败者 exit(1)（stderr：`control socket bound` + `TCP bind 127...failed`）、socket 文件存在、探测 ECONNREFUSED |
| SIGSTOP 扫参 D∈{5,30,100,200}ms | 24 | **24/24 复现**（冻结任意 ≥5ms 即触发——窗口即"500ms 探测内未应答"） |

失败形态分布（全部真实失败样本）：100% 为"恰一个 exit(1) + 胜者存活 + 探测 false"；
零"双双退出"、零"预算内双活"。与 failure-attempt-1 的记录一致。

确定性复现与随机复现的一致性：SIGSTOP 把"CPU 饥饿/启动未调度"强制为"进程冻结"，令
check_proxy_running 的 500ms 探测必然超时——实验 24/24 全部走步骤 3-6 路径并产生
与记录 flake 完全相同的终态（败者 exit(1) 且留下死文件、胜者控制通道 unlink、探测 ECONNREFUSED）。

## Suggested Fix Direction

### 推荐：方向 A —— 调换 bind 顺序（先 TCP 后控制）

`run_proxy` 中把 `TcpListener::bind` 移到控制 socket bind **之前**；控制段 delete-on-conflict
逻辑原样保留（EEXIST → 探测 → 死才删 → 重绑；重绑冲突重探测耗尽报错）。

理由：
1. **彻底消除本失败类**：TCP bind 成为唯一仲裁者。败者在 TCP EADDRINUSE 处直接 exit(1)，
   **根本走不到控制 bind**——不重绑、不删除、不留下任何 socket 文件。活 proxy 必然持有 TCP，
   因此"对活实例探测误判→删活 socket"路径在双启动场景下不可达；控制 EEXIST 分支只会在
   **僵尸文件**（探测确认无应答）时触发——探测-删除只对真僵尸执行，正是约束 #3 的意图。
2. **与既有契约完全兼容**：测试只 pin 了 "port {port} already in use"（AC3 及
   launch_proxy_contract.rs:320），无任何断言 "another live proxy owns"。败者消息变为
   约束 #4 的端口占用诊断（含 lsof PID，信息更强）。AC3 占端口行为不变；僵尸自愈
   （TCP 空闲 + 僵尸文件 → 探测失败 → 删 + 重绑）不变；AC9"不删活 proxy socket"从"探测
   确认后不删"升级为"根本不触碰"。
3. **KISS**：纯重排两个代码块，不新增状态/文件/机制；移除的是整个"探测-删除活实例"失败类
   而非打补丁；EEXIST 重探测重试循环保留作僵尸安全网。"another live proxy owns" 分支
   保留作防御（非 proxy 进程占 socket 路径等异例），可顺带简化或原样保留。
4. 与约束 #5 意图一致（双启动收敛 + 不破坏活 proxy 控制通道），实现比文字更严格；
   与约束 #3 父进程"试探 bind 判端口空闲"的先端口后 spawn 顺序同构。
5. 残差：形态 (c) 纯饥饿——测试的**单次** 500ms 探测在极端负载下对健康胜者仍可能超时。
   这是测试环境产物而非正确性缺陷（约束 #2 写死 500ms，产品侧无改动空间）；方向 A 后其
   发生概率大幅下降（不再有 unlink 永久破坏，只剩探测快照本身超时），且胜者控制通道完好，
   系统语义健康。如需更稳可考虑测试侧有界重探测（测试语义调整，属独立决策，本分析不推荐）。

### 备选：方向 B（若控制-first 顺序是 plan 硬性要求）——输家清理 + 胜者自愈

复合修复：(E) TCP bind 失败 exit(1) 前删除自己刚绑的控制 socket 文件（消除死文件遮蔽）；
(F) 胜者侧自愈——控制 accept loop 周期比对路径 inode 与自身 listener inode（stat vs fstat），
检测到被 unlink 则重绑（refactor 日志已提过的"胜者侧控制 socket 自愈（按 inode 比对 + 重绑）"）。
代价：inode 簿记 + 重绑循环，机制更多、新 bug 面更大；且 (E)+(F) 只修复 (b) 的永久化，
不修复 (c) 形态的探测超时本身。复杂度高于 A，KISS 评分低。

### 方向 C：不适用

当前控制-first 顺序下败者探测时**从不持有 TCP**（单门性质），"loser 释放 TCP"在现有顺序中
是空操作；其思想正是方向 A（败者在 TCP 处先退）。无独立价值。

### 方向 D（测试/契约容忍）：拒绝

削弱 AC9/AC10 收敛语义，与约束 #5/#10 冲突。

### 补充（可选硬化，仅在选 B 时）

删除前二次探测（间隔 ~100ms 再探测一次再删）可收窄误判窗口，但饥饿下多次探测仍可能全超时，
不能消除；方向 A 下不需要。

**结论**：推荐方向 A（先 TCP 后控制）——最小改动、确定性消除本失败类、与约束 #3/#4/#5 意图
及全部契约断言兼容。这是 plan 级顺序决策（constraints.md #5 文字描述的是控制 socket 收敛策略，
未固定 bind 顺序），实施前建议在 plan 中确认，但按意图判读完全合规。
