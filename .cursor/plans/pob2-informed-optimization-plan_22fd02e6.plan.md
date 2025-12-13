---
name: pob2-informed-optimization-plan
overview: 对比 POB2 架构与当前 TLI 计算引擎，提出可借鉴的优化迭代路径并按优先级落地
todos:
  - id: zone-breakdown
    content: 强化乘区明细与溯源输出
    status: pending
  - id: condition-layer
    content: 统一条件/标签/乘数求值接口
    status: pending
  - id: cache-key
    content: 完善缓存键含上下文并加统计
    status: pending
  - id: modifier-rules
    content: 规范 BASE/INC/MORE/FLAG 聚合
    status: pending
  - id: debug-trace
    content: 添加可选调试 trace 输出
    status: pending
  - id: data-alignment
    content: 对齐 schema/tags 命名规范
    status: pending
---

# POB2 对标优化迭代计划

## 目标

对比 POB2 的计算与缓存架构，补齐本项目在乘区可视化、修正聚合、条件系统、缓存与调试工具方面的不足，形成可落地的优先级路线。

## 优先级路线

1) **乘区与溯源可视化增强（高优）**

- 在 `tli-core/src/pipeline.rs` 强化 `MultiplierBreakdown`：对齐 POB2 的分区视图，补充标记来源分桶（技能/辅助/机制）与条件命中信息，输出到 `CalculatorOutput`。
- 在 `doc/开发日志.md` 记录区分 Inc/MORE/FLAG/OVERRIDE 的溯源规则，以便前端展示。

2) **条件/乘数评估层（高优）**

- 对标 POB2 的条件标签系统，在 `tli-core/src/stats.rs` 引入统一的条件求值接口：支持 per-stat、多标签、状态标志（context_flags）与阈值型条件，避免分散判断。
- 明确属性键约定与 tag 扩展策略，减少字符串匹配散落点。

3) **缓存健壮性与键设计（中高优）**

- 在 `tli-core/src/calculator_cache.rs` 重新纳入 `context_flags/context_values` 哈希，补充容量与命中率监控；对齐 POB2 的 `cacheSkillUUID` 思路，确保技能/配置/上下文唯一键。
- 增加缓存统计输出（miss/hit/size），便于悬停预览优化。

4) **Modifier 聚合一致性（中优）**

- 借鉴 POB2 的 BASE/INC/MORE/FLAG/OVERRIDE 分类，在 `tli-core/src/stats.rs` 与 `pipeline.rs` 做聚合规范：同类累加、不同 bucket 相乘、FLAG/OVERRIDE 单点决策，避免“额外”误分类。
- 针对 per_stack 与 gain-as-extra/conversion，补充重复计入的防护（历史标签去重规则文档化）。

5) **调试与快照工具（中优）**

- 参考 POB2 的 debug_trace，增加可选的 `debug_trace` 开关：输出标签匹配、Inc/MORE 分桶、Lucky/crit/cast-speed 关键中间值，便于回归。
- 提供轻量级 snapshot 对比接口（单输入 → 结构化 trace），方便 QA。

6) **数据/Schema 对齐（次优）**

- 复核 `supabase/seed.sql` 的技能/天赋/标签表，确保标签继承链与属性键命名遵循 dot.notation 与百分数小数化；补齐必要的 tag registry。

## 产出

- 架构对齐的改进清单（含代码触点与规则说明）。
- 可开关的乘区/溯源调试输出，支撑前端展示与回归。
- 健壮的缓存键与统计，提升悬停预览性能。