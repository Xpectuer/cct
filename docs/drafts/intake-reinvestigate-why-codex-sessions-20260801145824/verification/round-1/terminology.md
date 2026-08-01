# Verifier Report: Terminology (Angle 7/7)

## Score: 4/10
## Verdict: FAIL

## Findings

### BLOCKER: AC7 术语定义与 spec 自身 AC8 的文档范围列表冲突（同一术语两种范围）
- Location: spec.md:43（Terminology AC7 行）vs spec.md:66（AC 8）
- Evidence: 表内含 README 缺 ARCHITECTURE.md；AC8 反之。session-log.md:90 访谈权威清单无 README；实仓 README 0 处 CODEX_HOME
- Fix: 表内 AC7 定义去掉 README、补入 ARCHITECTURE.md（与 completeness/consistency/risk ADVISORY 相同发现）

### BLOCKER: "cct proxy" 实体（Part A 的全部主题）无任何定义、零关系声明
- Location: spec.md Terminology 表；domain-model.md 实体表与关系表
- Evidence: Part A 标题即 "cct proxy 死锁修复"，AC1/2/3/6/7 均以 proxy 为行为主体，但 Terminology 表无 "cct proxy" 条目；domain-model.md 7 实体 8 关系完全不包含 proxy；"cct profile(auth_type=proxy) → cct proxy → upstream" 数据流仅由 AC3 暗示，未显式声明方向与基数
- Fix: Terminology 新增 "cct proxy" 条目（127.0.0.1:19191/v1 本地 HTTP 转发代理 + Unix 控制 socket + 生命周期由 cct 拥有）；domain-model.md 补两条关系（cct profile(auth_type=proxy) → cct proxy → upstream，1:N 共用单实例；cct proxy → Codex 进程 HTTP 转发）

### ADVISORY: 裸用 "provider" 作为 model_provider 的未声明简称（8 处）
- Location: spec.md:21、49-50、62-63、66
- Fix: model_provider 条目补 "provider：本文为 model_provider 简称" 或全文统一

### ADVISORY: 缩写 cwd 未展开；"控制 socket / 死 socket / socket 文件" 复合词无定义
- Location: spec.md:37、19、53、60
- Fix: 首次出现写 "cwd（当前工作目录）"；附一句 "死 socket：指向已退出/无响应 proxy 的 Unix socket 文件"

### ADVISORY: "proxy" 一词承载三种含义（cct proxy 组件 / auth_type 值 / Clash 系统代理）
- Location: spec.md:62、74、19
- Fix: 新增 cct proxy 条目时注明与后两者的区分

### ADVISORY: Source 列 "Step 4" 在 session-log 中无法精确定位
- Location: spec.md:33-43
- Fix: 引用具体条目（[debate] [coverage] [ux] 等）

## 核对通过项
- 未误用 _Avoid_ 词（会话丢失/provider 隔离会话/resume 按 profile 过滤均未出现）✓
- 7 个核心术语在 spec/terminology.md/domain-model.md/session-log 四处定义一致 ✓
- 溯源：codex exec 链路、stub 上游可精确定位 ✓
- cct profile 与 Codex profile 区分正确维持 ✓
