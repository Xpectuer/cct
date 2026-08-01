# Verifier Report: Simplicity/KISS (Angle 5/7)

## Score: 9/10
## Verdict: PASS

## Findings

### ADVISORY: AC2 的"死 proxy 自动重启"范围未澄清——对"进程存活但死锁"的旧 proxy 实际无法重启
- Location: spec.md:60 (Smoke AC 2)
- Evidence: 真实死 proxy（PID 29182）是进程存活 + TCP 19191 被占的死锁进程；`remove_file(proxy.sock)`（proxy.rs:185）只清 socket 文件，`TcpListener::bind`（proxy.rs:198-200）仍 `Address already in use` panic；`cct proxy stop`（send_control shutdown）对死锁进程无效。按 AC2 字面自动重启在真实死锁场景不成立。
- Fix: AC2 收窄为"死 proxy = 进程已退出的僵尸（socket 残留、端口空闲）→ 自动重启（现有 spawn-if-needed + remove_file 路径即可）"；"进程存活但死锁"的旧版遗留实例为一次性手动清理（session-log 已记录 PID 29182），不新增进程终止机制。

### ADVISORY: 并发启动防护的"报错/复用退出"表述二义
- Location: spec.md:64 (AC 6)
- Evidence: AC 写 "报错/复用退出"，decisions.md 只接受"报错退出"
- Fix: 删去"/复用"，与决策记录对齐为单一语义

## 核查通过项
- 无被否决设计复活（--profile overlay / 冲突对话框 / 迁移工具均未出现）✓
- Part A 最小化：仅换 listener + accept loop；handle_control 的 spawn_blocking 已是现状；无新依赖 ✓
- Part B 零功能改动 ✓
- 测试设计匹配 bug 规模（4 契约场景 vs 3 缺陷面）✓
- Unix 哲学：改动收敛 proxy.rs 内 + launch.rs 薄 seam ✓
