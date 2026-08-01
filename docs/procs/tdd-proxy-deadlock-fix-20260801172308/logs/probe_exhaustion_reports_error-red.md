---
title: "probe_exhaustion_reports_error — Red Phase"
brief: "probe_exhaustion_reports_error — Red: exit 0"
doc_type: proc
created: 2026-08-01T10:48:19Z
case: "probe_exhaustion_reports_error"
phase: red
---
Exit code: 0
Full output: `rtk proxy cargo test --test launch_proxy_contract probe_exhaustion_reports_error`（工作树根目录执行，rtk proxy 取完整输出）

```
    Blocking waiting for file lock on artifact directory
    Finished `test` profile [unoptimized + debuginfo] target(s) in 8.41s
     Running tests/launch_proxy_contract.rs (target/debug/deps/launch_proxy_contract-11aa5be2c88374b7)

running 1 test
test probe_exhaustion_reports_error ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 1.53s
```

Red 确认：**本用例无法在 Red 阶段演示失败——就绪耗尽语义已被现有实现交付**（vacuous Red，与 TC-16/TC-17 同因）。`src/launch.rs:134` 的 `ensure_proxy_running` 流程为：应用层探测 → 失败 → 试探 bind 端口 → spawn（CCT_PROXY_BIN）→ 就绪探测（PROBE_TIMEOUT 500ms × PROBE_RETRIES 3）→ 耗尽后 `bail!("proxy did not become healthy after {} probes")`。本用例的 CCT_PROXY_BIN 是 `#!/bin/bash\nexit 0` 的立即退出脚本（不监听、不写 socket），因此 spawn 后每次 `check_proxy_running` 均 connect ENOENT 快速失败 → 循环仅靠 sleep 耗尽 → Err。就绪探测循环本身已由 TC-15 Green（Step 4 重写）实现，本测试是就绪耗尽语义（Err + 信息 + ≤2s 不挂起）的回归守卫。

测试断言全部非空洞（若耗尽路径被破坏，任一断言都会失败），覆盖完整就绪耗尽语义链：
1. **Err 契约**：`result.is_err()` —— 立即退出的 CCT_PROXY_BIN 目标必须令 `ensure_proxy_running` 返回 Err（若实现无限等待或假 Ok，此处红）；
2. **错误信息**：`msg.contains("did not become healthy")` —— 错误必须指明就绪探测耗尽（当前实现消息 "proxy did not become healthy after 3 probes"，若实现返回无信息量的裸错误/端口错误，此处红）；
3. **≤2s 不挂起**：`elapsed <= 2s`（Instant 测量）—— PROBE_RETRIES 3 × PROBE_TIMEOUT 500ms ≈ 1.5s 上下，若实现探测无限重试或每轮等待超时累积（如无 socket 文件时仍等满 500ms 的 connect 超时），此处红。

实际运行结果：1.53s 完成（与 3 × 500ms 耗尽耗时吻合），无挂起——对比 Step 4 "Old" 的原始死锁场景（旧实现死锁于 kill 等待），新实现的耗尽路径有界返回。

结论：本用例作为"就绪永不健康"类实现的回归防护网有效（对"无限等待"或"耗尽后返回 Ok"类实现会红），但当前实现已满足该契约——Green 阶段将无需改动 src/ 即为全绿（已实现行为 + 回归测试落位）。
