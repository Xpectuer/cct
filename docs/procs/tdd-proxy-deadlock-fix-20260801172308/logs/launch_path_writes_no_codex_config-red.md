---
title: "launch_path_writes_no_codex_config — Red Phase"
brief: "launch_path_writes_no_codex_config — Red: exit 0（vacuous Red，回归守卫）"
doc_type: proc
created: 2026-08-01T13:06:44Z
case: "launch_path_writes_no_codex_config"
phase: red
---
Exit code: 0（vacuous Red：实现已不写 Codex 配置文件，测试通过，作为 AC14 回归守卫）

Full output: `cargo test --test proxy_contract launch_path_writes_no_codex_config`（工作树根目录执行，rtk proxy 原始输出）

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 1 test
test launch_path_writes_no_codex_config ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.51s
```

## 判定：vacuous Red（回归守卫），exit 0

首轮执行曾以 exit 101 失败（非约束违反，见下方 flake 说明）；其后连续 11 次执行全部通过（exit 0）。当前实现下该测试不产生真实 Red——`generate_codex_config` 已从 `src/` 移除（grep 全库 0 匹配），`exec_codex_proxy` 经 `build_codex_proxy_config_args` 以 `--config` CLI 旗标内联传参、CODEX_HOME 留在默认 `~/.codex`；`src/proxy.rs` 对文件系统仅有 socket 清理 `remove_file`，无任何 CODEX_HOME 写入路径。测试即 AC14 的配置快照回归守卫：若未来实现回退为写 config.toml / auth.json / profile-*.config.toml，本测试转红。

## 测试内容（tests/proxy_contract.rs 追加）

- 临时 CODEX_HOME + 空 profiles.toml（CCT_CONFIG 覆盖）+ 临时 CCT_PROXY_SOCKET/PORT（RestartEnvGuard 注入 CCT_PROXY_BIN=真实二进制）
- 启动链路前置两步等价路径（exec_codex_proxy 的 1-2 步，第 3 步 exec-replace 不可达）：`ensure_proxy_running` + `switch_profile`
- 断言：写入前后 codex_home 递归文件集合一致（snapshot）+ 无 config.toml / auth.json / profile-*.config.toml
- 附加实测佐证：手动复现（临时 CODEX_HOME + 空 profiles.toml 下直接 `cct proxy start` → status → switch）后 codex_home 保持为空

## 首轮 flake 记录（exit 101，非约束违反）

首轮（新编译二进制首次执行，伴随 "Blocking waiting for file lock on artifact directory"）在测试自身的 switch_profile 管道步骤失败：

```
thread 'launch_path_writes_no_codex_config' (7632015) panicked at tests/proxy_contract.rs:907:10:
switch_profile must be answered by the proxy: Os { code: 57, kind: NotConnected, message: "Socket is not connected" }
```

ensure_proxy_running 的就绪探测已成功（proxy 应答过 status），随后 switch 连接即获 ENOTCONN——macOS unix socket 紧连竞态（测试环境问题，非实现缺陷：同一场景手动复现稳定通过，后续 11/11 次执行通过）。快照断言（约束 #14 本体）未触发，失败点位于测试自身管道。若 Green 阶段复现此 flake，可考虑 switch 前增加一次探测重试。
