# Verifier Report: Implementability (Angle 3/7) — Round 3

## Score: 9/10
## Verdict: PASS

## Findings

### ADVISORY: "旧版死锁遗留 → 明确报错退出" 的 AC 与实际探测语义存在偏差（复用而非报错）
- Location: spec.md:Solution Summary + 占端口 AC + 迁移 AC
- Evidence: 死锁的旧版实例对 status 探测**照常响应**（run_control_socket accept 新连接后经 spawn_blocking 正常应答，阻塞的只是 HTTP 调度）→ 探测成功 → ensure_proxy_running 复用返回 Ok，走不到"探测失败且端口被占→报错"分支；HTTP 挂起症状持续
- Fix: 迁移 AC 补精确语义："旧版死锁实例若控制 socket 仍响应，新版本探测会将其视为健康并复用（此时 codex 请求仍挂起）；唯一修复路径是用户手动终止旧进程"；占端口报错路径实际覆盖"控制 socket 不可达 + TCP 被占"

### ADVISORY: 子进程 EADDRINUSE → 重新探测后的收敛条件未指定
- Location: spec.md:Solution Summary
- Fix: 补"探测未响应则重试 3 次、每次 500ms，耗尽后报错退出"

## Round-2 闭合核查（4/4 + 1 INTERVIEW_NEEDED 全部闭合）
1. INTERVIEW_NEEDED（PID vs 无 lsof）→ 只读 lsof + 降级 ✓
2. CCT_PROXY_SOCKET → 与 config_path/kimi_config_path 先例逐行一致，父/子/stop 三端一致生效 ✓
3. 端口空闲判定落父进程 → ensure_proxy_running 已携带 _port 参数 ✓
4. extra_args 嵌入 exec → argv 构造合法（exec 位于所有全局旗标之后）✓

## 已验证通过的关键点（round-3 复验）
1. tokio full features ✓ 2. CCT_PROXY_BIN 先例 ✓ 3. 脱敏唯一泄漏点 proxy.rs:420 ✓ 4. shutdown 清 socket 机械改动 ✓ 5. set_read_timeout 实现 stop 超时 ✓ 6. TCP bind panic 改报错 ✓ 7. 测试隔离 ✓ 8. generate_codex_config 已不在 src 中 ✓

无 INTERVIEW_NEEDED。
