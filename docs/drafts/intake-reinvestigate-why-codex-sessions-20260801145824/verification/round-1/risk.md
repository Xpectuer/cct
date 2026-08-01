# Verifier Report: Risk Coverage (Angle 4/7)

## Score: 4/10
## Verdict: FAIL

## Findings

### BLOCKER: 自动重启恢复路径在 spec 自己定义的"死 proxy"场景下必然失败——死进程仍占 TCP 端口时无法 bind，且恢复失败无可观察行为
- Location: spec.md §Solution Summary（Part A）/ Decisions 3 / AC 2
- Evidence: 死锁 proxy（PID 29182）是"进程存活且 LISTEN 19191"；清理 socket 后重新 spawn，TcpListener::bind EADDRINUSE → 现行代码 panic（proxy.rs:200）。且：现行 shutdown 走 process::exit(0)（proxy.rs:480），run_control_socket 退出后的 socket 清理（proxy.rs:194）永不执行——**每次干净 `cct proxy stop` 都留下死 socket 文件**，死 socket 是稳态
- Fix: 定义完整恢复序列：(1) 探测失败 → 验证端口占用者 → 终止死进程后再 bind；(2) spawn 后循环探测就绪；(3) 恢复失败明确报错（区分端口被占 vs spawn 失败）非 panic；(4) 重启后重发 switch 恢复 active profile。用户本机死锁实例（PID 29182）+ 遗留 proxy.sock 清理纳入迁移步骤

### BLOCKER: CCT_PROXY_LOG 开启时控制 socket 日志原样打印 switch 命令 JSON，api_key 明文进入 stderr/日志
- Location: src/proxy.rs:420 `log_proxy!("ctl << {}", line.trim())`
- Evidence: switch 命令原始 JSON 含 `"api_key":"sk-..."`（ControlCommand 定义 proxy.rs:38-41）；CCT_PROXY_LOG 开启时明文落盘。直接违反 mask-secrets-on-every-display-path 规则
- Fix: 控制命令日志打印前脱敏 api_key（复用 mask 策略）；契约测试断言 CCT_PROXY_LOG 开启时日志不含 api_key

### ADVISORY: bind 失败路径行为未定义（端口被非 cct 进程占用、双启动 TOCTOU 竞争），Decision 5"报错退出"与 AC"报错/复用退出"歧义
- Location: spec.md Decisions 5 / AC 6
- Fix: 定义 bind 失败为清晰错误退出（非 panic），契约测试断言退出码 + 消息；统一"报错 vs 复用"语义

### ADVISORY: 探测超时与 `cct proxy stop` 均无超时语义，死 proxy 会让 stop 永久挂起
- Location: spec.md Terminology "应用层探测（带超时）"
- Evidence: send_control（proxy.rs:96-114）read_line 无超时
- Fix: 指定探测超时/重试参数（可写死），send_control 加超时（probe 与 stop 共用），契约测试覆盖"无响应 proxy 上 stop 超时返回错误"

### ADVISORY: 自动重启后 active profile 为空，会话中请求会收到误导性 502
- Location: spec.md AC 2
- Fix: 明确恢复范围（仅 launch 时刻，重启后 switch 由既有流程保证——ensure_proxy_running → switch_profile → exec）

### ADVISORY: 回滚/迁移策略未声明
- Location: spec.md 全文
- Fix: 一行回退声明（revert 到上一发布；proxy 状态内存态无迁移负担）+ 迁移清理步骤（PID 29182、遗留 proxy.sock）

### ADVISORY: AC7 文档清单不一致（README vs ARCHITECTURE.md）
- Location: spec.md Terminology "AC7" vs AC 7
- Fix: 统一两份清单（与 completeness ADVISORY 3 相同）

### ADVISORY: pre-mortem"分层诊断"缓解未落到可执行 AC
- Location: spec.md Decisions 6
- Fix: 分层步骤写进 AC（curl --noproxy '*' 验证 proxy 层 → codex 对话验证上游层），与 Open Question 4 呼应

无 INTERVIEW_NEEDED——两个 BLOCKER 均可由设计决定闭合。
