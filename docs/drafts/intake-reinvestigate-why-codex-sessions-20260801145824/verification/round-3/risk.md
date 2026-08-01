# Verifier Report: Risk Coverage (Angle 4/7) — Round 3

## Score: 9/10
## Verdict: PASS

## Round-2 复核：4 条 ADVISORY 全部闭合
1. 三处实现级空白 ✓（端口空闲判定/控制 socket EADDRINUSE/重试耗尽终态）
2. lsof 与无 lsof 机制冲突 ✓
3. 超时常量 ✓（500ms×3 / stop 2s）
4. 迁移落点 ✓（install-script.md + 顺序无关核实安全）

## Findings

### ADVISORY: lsof 诊断命令硬编码端口 19191（与 simplicity 重复，已修复）
- Location: spec.md:19
- Fix: 已改为 `proxy_port()` 实际端口插值 ✓

### ADVISORY: lsof 降级路径仅覆盖"缺失"，未覆盖"存在但调用失败"
- Location: spec.md:19
- Fix: 降级条件写为"lsof 不可用或调用失败时降级为定位命令文本"

### ADVISORY: 子进程 TCP bind 竞态失败路径的展示与就绪探测重试预算未显式绑定常量
- Location: spec.md:19
- Fix: 补"TCP bind 失败同样输出 lsof 诊断文本；就绪探测复用 500ms×3 常量"

## 新机制风险分析（已核实，有界可接受）
- 试探 bind 竞态窗口：socket bind 先于 TCP bind 决出胜负；残余微窗口后果仅为当次控制通道失效、下次启动自愈，KISS 可接受 ✓
- extra_args 嵌入 exec：与 launch.rs:182-200 现状一致，非产品行为变更 ✓
- CCT_PROXY_SOCKET / 只读 lsof / 6 旗标零 drift：均核实 ✓

无 INTERVIEW_NEEDED。
