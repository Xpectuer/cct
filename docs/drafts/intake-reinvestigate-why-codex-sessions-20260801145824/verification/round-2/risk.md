# Verifier Report: Risk Coverage (Angle 4/7) — Round 2

## Score: 8/10
## Verdict: PASS

## Findings

### ADVISORY: 自愈序列三处实现级空白——Unix 控制 socket bind 竞态仍 panic、端口空闲判定机制未定义、就绪探测重试耗尽终态未定义
- Location: spec.md:19 / 56
- Evidence: UnixListener::bind expect panic（proxy.rs:187）未被 TCP bind 修复覆盖；"端口空闲"判定未写明（bind 探测需先 drop）；重试耗尽行为未定义
- Fix: ① 控制 socket bind 失败与 TCP bind 同等处理（EADDRINUSE → 重新探测）；② 端口空闲判定 = 一次 bind 探测先 drop 再 spawn；③ 重试耗尽 → 报错退出；契约测试补双启动竞态场景

### ADVISORY: 占端口报错中的"占用者 PID"获取方式与"不引入 lsof/pidfile"冲突
- Location: spec.md:66 / 56
- Evidence: macOS 无 /proc，端口占用 PID 常用手段即 lsof
- Fix: 明确"错误路径单次 lsof 仅用于报错信息展示，不用于管理决策/自动 kill"，或降级为不含 PID

### ADVISORY: 探测/就绪/stop 的超时与重试具体参数未落纸
- Location: spec.md:19 / 73
- Evidence: 全文无具体数值；"僵尸 vs 占端口"判定边界依赖超时值，过短会误删活 proxy 的 socket
- Fix: 写死常量（如探测 500ms × 3 次、stop 2s），契约测试固化同一常量

### ADVISORY: 迁移步骤缺"写入哪个文档"与操作顺序说明
- Location: spec.md:74
- Fix: 指明落点（docs/references/install-script.md 或发布说明）；明确顺序无关（kill 释放端口，新版本启动自行清理死 socket，手动删除仅兜底）

## Round-1 修复复核（全部闭合）
- BLOCKER 1（占端口无法重启）→ 闭合（用户决策覆盖）✓
- BLOCKER 2（api_key 日志泄漏）→ 闭合（脱敏声明 + AC 断言）✓
- ADVISORY 1-6 → 全部闭合 ✓

## 新方案未引入数据丢失风险
- proxy 内存态无持久化；临时 CODEX_HOME 隔离；配置快照断言；实测不符有回退条款 ✓

无 INTERVIEW_NEEDED。
