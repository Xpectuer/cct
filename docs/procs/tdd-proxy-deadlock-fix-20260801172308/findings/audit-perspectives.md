# Audit Perspectives

Generated from: docs/procs/tdd-proxy-deadlock-fix-20260801172308/ref/spec.md, docs/procs/tdd-proxy-deadlock-fix-20260801172308/ref/plan/code-spec.md
Proc type: tdd（23 用例全 RGR；回归门 193/193；run-all 15/15/0/0）
Changed files: 27（25 git + 2 untracked tests）

| # | Key | Name | Focus | Spec Sections | Plan Sections |
|---|-----|------|-------|---------------|---------------|
| 1 | completeness | 需求完整性 | 每条 AC 是否有实现证据，证据是否真能证明该 AC | AC1-AC15、Decisions 1-9 | Step 23 Cross-Check、Step 22 |
| 2 | fidelity | 计划忠实度 | 23 用例的 files/approach/verify 是否照 plan 落地；4 个已知偏差的决策合理性 | — | All steps + Execution Order |
| 3 | honesty | 代码诚实度 | vacuous Red 记录、空断言、TODO/stub、Refactor 加行为、fake 真实性、PASS 证据自证性 | Decisions 5/6/7 | Step 10-15、Step 22 |
| 4 | edge_cases | 边界覆盖 | 死锁时间界、stop 2s 超时边界、EADDRINUSE 耗尽、lsof 降级、占端口三态、并发关闭 | AC1/AC3/AC10 | Step 11/12、Step 5/6/9 |

## Angle 1: 需求完整性 (completeness)
| # | Check Item |
|---|------------|
| 1 | AC-1 死锁回归：concurrent_control_and_http 真实运行 proxy 二进制，≥20 次并发 status + HTTP GET 同一窗口完成，断言含 3s 时间界 |
| 2 | AC-2 僵尸自愈：三层证据（launch ensure_proxy_running 探测→bind→spawn→就绪 + proxy 先探测再删 + zombie_recovery_restarts_proxy/zombie_socket_triggers_restart/B002）；SIGKILL 后 socket 残留再触发；耗尽报错文本存在 |
| 3 | AC-3 占端口报错：port_occupied_reports_error_keeps_occupant 断言非 0 + stderr 含 lsof PID 或降级文本 + 无 panic + 占用者存活；port_occupied_bails_with_diagnosis 断言 Err 未 spawn；端口取实际绑定值非硬编码 19191 |
| 4 | AC-4 stub 转发：stub_forwarding_with_bearer 断言 (method,path,Authorization)=Bearer sk-contract-key + DELTA；stub 是 SSE 契约实现（responses-API SSE）非固定字符串 |
| 5 | AC-5 日志脱敏：log_masks_api_key 捕获 stderr 断言无明文；mask_ctl_line 覆盖 sk- 前缀与 custom-token 两种形态；switch 与 HTTP 请求两条日志路径都过 mask |
| 6 | AC-6 同 provider 可见：B006 输出 session-id 对比；**6 个 --config 旗标由真实函数生成，无手工复刻** |
| 7 | AC-7 跨 provider 不可见 + 显式恢复：B007 断言另一 provider id 不出现（id 级核对，非仅计数）+ 显式 resume <id> 成功 |
| 8 | AC-8 cwd 过滤 + --all：B008 切换真实 cwd/仓库目录验证 |
| 9 | AC-9 活 proxy 报错 + 复用：double_start_race_one_wins + reuses_live_proxy（进程数不变）；不删活 proxy socket 有断言 |
| 10 | AC-10 契约覆盖 7 场景 + 隔离：7 行为契约逐一存在；CCT_PROXY_SOCKET 临时路径 + 动态端口 + serial + tempfile |
| 11 | AC-11 迁移说明：install-script.md 三要素（旧实例健康复用→手动 kill；删遗留 socket 兜底；新版本不再死锁）；**偏差 3：文档路径 ~/.config  vs macOS 实际 ~/Library/Application Support——判定忠实 plan vs 误导用户** |
| 12 | AC-12 L2 前置：B012 预检 + 基线补录行存在 + kill 前 ps -p 确认记录 |
| 13 | AC-13 五文档清理：grep 零命中陈旧叙述 + resume 语义段落（provider ∩ cwd、--all 关不掉 provider 过滤、显式 id 绕过）；历史快照零改动 |
| 14 | AC-14 不写配置 + 接口冻结：launch_path_writes_no_codex_config 快照对比 + B014 接口未变 |
| 15 | AC-15 分层诊断：B015 curl --noproxy '*' 先行；poc.md Results Log 当日记录行 |

## Angle 2: 计划忠实度 (fidelity)
| # | Check Item |
|---|------------|
| 1 | TC-1（Step 1）：CCT_PROXY_SOCKET env 优先；单测 set_var/remove_var；Red 真失败 |
| 2 | TC-2（Step 3）：check_proxy_running 应用层探测；三常量存在；send_control 签名保持（接口冻结 #9） |
| 3 | TC-3（Step 5）：tcp_port_owner 单次 lsof + 失败/空/非 UTF-8 → None；降级文本含 "lsof -iTCP"；单测真隔离 lsof |
| 4 | TC-4（Step 8）：两 helper 集中 + 两应用点；outbound 泄漏备注在 TC-8 是否真覆盖 |
| 5 | TC-5（Step 9）：shutdown_proxy STOP_TIMEOUT 传播错误；stop_proxy 区分两态；Refactor status_to_result 只移代码 |
| 6 | TC-6（Step 11）：与 plan 骨架一致（≥20 status + GET + 3s 界）；依赖链无跳步 |
| 7 | TC-7（Step 11）：vacuous Red 判定理由可信；Green 断言真实验证转发/Bearer/SSE |
| 8 | TC-8（Step 11）：stderr piped 捕获；覆盖 ctl + 请求两路径 |
| 9 | TC-9（Step 11）：①无响应 ≤2.5s 非 0 ②无 socket 快速 exit 0 ③stale connect-refused 快速非 0——三 case 全实现 |
| 10 | TC-10（Step 11）：SIGKILL→残留→ensure_proxy_running Ok；CCT_PROXY_BIN 注入正确 |
| 11 | TC-11（Step 11）：占用者存活断言 |
| 12 | TC-12（Step 11）：**偏差 1**——bind 顺序改为先 TCP 后控制；(a) analysis 证据充分性；(b) 收敛契约保持（20/20 无 flake）；(c) EADDRINUSE 控制分支是否成死代码、AC10 契约是否被迁移到 TCP 层 |
| 13 | TC-13（Step 7）：shutdown 清理 socket；handle_control 签名变更存在 |
| 14 | TC-14（Step 13）：临时 CODEX_HOME + 快照对比断言 |
| 15 | TC-15..19（Step 12）：fake 脚本真实应答；5 个契约断言（fake 启动/进程数不变/≤2s Err/Err 含 port 未 spawn） |
| 16 | TC-20（Step 15）：**偏差 2 harness 部分**——wait/trap/SSE 修复真实；15/15 来自真实输出；基线补录存在 |
| 17 | TC-21（Step 16）：B006/B007/B008 全 PASS 判定成立；B007/B008 断言重写仍断言原契约（id 级） |
| 18 | TC-22（Step 17）：B015 [PASS] + Results Log 当日行 |
| 19 | TC-23（Step 19-21）：迁移小节 + 五文档清理 + 三脚本 PASS；**偏差 3 判定** |
| 20 | TC-23 范围核查（**偏差 4**）：full_auto 布尔叙述未动——(a) 不在 AC13 范围；(b) 预存 drift 非本次引入；(c) 与代码语义冲突风险记录 |
| 21 | 终端步骤（22-25）：Proof-Read 记录、Cross-Check 表、review.md、提交信息 |
| 22 | 执行顺序：DAG 与 tdd.md 依赖列一致，无跳步 |

## Angle 3: 代码诚实度 (honesty)
| # | Check Item |
|---|------------|
| 1 | vacuous Red 记录完整性：TC-7/9/10/16/17/18/19 逐一核实 red 日志注明 vacuous + 理由；Exit code 口径与事实一致 |
| 2 | vacuous 用例的 Green 断言强度：断言非空转（stub 记录/DELTA/退出码/stderr/耗时/进程存在性变化） |
| 3 | 空断言扫描：assert!(true)、let _ =、未使用结果、脚本固定输出 PASS |
| 4 | TC-14 快照守卫真实性：glob 指向真实路径；前后对比真做；CODEX_HOME env 真设（否则落在真实 ~/.codex） |
| 5 | Refactor 不添加行为：日志核对断言数不变；TC-12 竞态修复可追溯到 failure-dispatch |
| 6 | fake 脚本真实性：真 rm + accept + 应答 status；probe_exhaustion fake 立即退出 |
| 7 | TODO/stub 扫描：零命中；无残留 bind panic 路径 |
| 8 | run-all 15/15 自证性：汇总来自真实退出码聚合；无 Skip 掩盖、无 || echo PASS |
| 9 | 脱敏无死角：outbound 泄漏覆盖；mask_ctl_line 用解析后 cmd.api_key |
| 10 | 基线证据引用：refs/proxy-deadlock-diagnosis.md 真实存在且支持声明 |

## Angle 4: 边界覆盖 (edge_cases)
| # | Check Item |
|---|------------|
| 1 | AC1 时间界：死锁回归测试带超时断言（3s 界），否则挂死 CI |
| 2 | AC10 stop 超时三态：无文件（快速 exit 0）/拒绝（stale，快速非 0）/无响应（[2s,2.5s] 非 0）全覆盖 |
| 3 | AC3 占端口三态：(a) TCP 被占+无控制 socket→lsof 报错；(b) 旧实例控制仍响应+TCP 被占→健康复用（AC11）；(c) 控制死+TCP 被占→报错——(b) 有测试或文档明示 |
| 4 | EADDRINUSE 耗尽路径：3×500ms 收敛；**结合偏差 1** 核查先 TCP 后控制下是否仍可达/可测试 |
| 5 | lsof 降级全谱：缺失/失败/空/非 UTF-8 → None → 降级文本 |
| 6 | 诊断端口来源：实际绑定端口非硬编码 19191 |
| 7 | 双启动收敛界：败者 ≤2s、恰一存活、无 panic；20/20 复跑证据记录；serial 隔离 |
| 8 | 僵尸恢复竞态窗口：父进程 bind 后 spawn 窗口被抢 → 子进程 TCP 失败报错 + 父就绪耗尽报错——双非 panic 出口；无遗漏 expect |
| 9 | 就绪探测边界：3×500ms 耗尽 → bail 明确报错，≤2s 不挂起 |
| 10 | 并发关闭：shutdown 删文件不删 in-flight 连接；accept 错误 sleep 100ms 防忙循环 |

**审计执行建议**：completeness/edge_cases 以 git diff + 测试源码核对；fidelity 以 logs RGR 记录 + findings 对照；honesty 优先抽查 vacuous 组 + TC-14 + run-all 原始输出。偏差 1（bind 顺序）与偏差 3（文档路径）需给出明确判定结论；偏差 2/4 为记录+残余风险型。
