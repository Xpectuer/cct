---
title: "Plan Review: Codex history shared across profiles"
doc_type: proc
brief: "Self-review of plan/ against spec acceptance criteria"
confidence: verified
created: 2026-08-01
updated: 2026-08-01
revision: 3
---

# Plan Review

Reviewed: `./plan/`
Spec: `./spec.md`

## Checklist Results

| Check | Status | Notes |
|-------|--------|-------|
| All acceptance criteria covered | PASS | 7 条验收标准映射至 Step 1-14（code-spec.md Step 16 表格） |
| File paths verified | PASS | src/launch.rs、config.rs、app.rs、ui.rs、main.rs 全文/相关段已读；docs 为文档更新步骤（Step 14 按现状编辑，无锚点） |
| Old anchors are unique | PASS | 20 个 old anchor 无重复（脚本校验）；Step 2 锚定 Step 1 产物尾部 |
| Verify steps are executable | PASS | 全部为 cargo test / rg / cargo build 命令，无人工判断词 |
| Execution order valid | PASS | validate-dag.sh 退出 0；无前向依赖 |
| YAML DAG block valid | PASS | validate-dag.sh 退出 0（WARN 均为同文件自动串行化） |
| Files declared per step | PASS | YAML `files` 与实际编辑一致 |
| Commit message valid | PASS | 主题 60 字符；type `feat`；scope `codex` 符合仓库惯例 |
| Terminal steps present | PASS | Step 15-18（proof-read/cross-check/review/commit） |
| Index complete | PASS | index.md 含全部 5 个 aspect 文件 + Description/When to Use |
| Domain knowledge present | PASS | 实体/术语/业务规则三节齐备 |

## Gaps Found

自审修复 1 项 + 两轮交叉验证修复 11 项（详见下方修订记录）：

1. Step 2 old anchor 与 Step 1 重复 → 锚定 Step 1 产物尾部
2. Step 8 使用 `KeyDiff` 但 Step 9 才定义（编译断裂）→ 交换：Step 8 = config.rs `KeyDiff`+`apply_overlay_winner`，Step 9 = launch.rs diff（依赖 [4,8]）
3. Step 5 删除 `write_codex_auth` 时 exec_codex 与契约测试仍引用 → 重排：Step 5 = exec_codex 重写（保留 auth 函数），Step 7 = 删除函数 + 全部引用测试（含契约测试，Step 13 重建）
4. 旧 plan.md（rev 1 符号链接方案）与 spec rev 2 矛盾 → 替换为指向 plan/ 的指针文档
5. architecture.md：diff 位置（config.rs→launch.rs）、契约测试文件（新文件→launch.rs tests 模块）
6. constraints.md：HC5 允许 Esc、HC7 实际函数名
7. Step 13 契约测试用真实 `resolve_codex_layout` 会污染共享 home → tempdir 构造 `CodexLayout`
8. Step 14 补 ARCHITECTURE.md/README.md + AC7 grep
9. verification.md 行 5/8/9 未同步重排 → 已更新
10. Step 13 锚定 Step 7 已删测试 → 改为 INSERT 完整测试（锚定存活文本）
11. Step 12 `d` 分支用 stale 内存 profile 重生成 overlay（静默覆盖用户选择）→ 新增 `launch::apply_on_disk_winner`（回写→重载→重生成），`d` 分支调用它
12. Step 7 删除描述"整段"会误删交错的 `generate_codex_config_*` → 按名删除 + 保留名单
13. Step 10 Verify 与 imports、Step 3 整函数替换、Step 16 AC 映射、review.md 计数

重新校验：DAG RC=0；20 个 old anchor 无重复。两轮 yield-verifier 复核后 BLOCKER 清零。

## Third-round verification（revision 3）

第三轮 yield-verifier：**VERDICT: DONE**，无 BLOCKER。3 项 ADVISORY 已修：
- Step 13 第三个编辑补显式 Old 锚（`update_codex_model_reaches_config` 函数体尾部）
- verification.md 行 12 的 `rg launch_and_exit` 措辞修正（3 处调用 + 1 处定义）
- spec.md Testing 节注明 main.rs dispatch 不可单测的等效替代（apply_on_disk_winner / 回写闭环 / footer 断言）

## Verdict

READY（revision 3 — 第三轮 yield-verifier DONE 后定稿）
