# TLI BD 决策辅助系统

> 火炬之光：无限 (Torchlight Infinite) Build Decision Assistant System

一款极简配置、高直观性的 BD 决策辅助工具。通过 **上下文即时对比** 和 **无状态模拟**，解决装备更替的数值决策难题。

## ✨ 核心特性

- **Contextual Insight**: 鼠标悬停即显示 DPS/EHP Diff（净收益）
- **Stateless Simulation**: Rust/WASM 纯函数计算，前端仅负责状态映射
- **Modular Agentic**: 数据库、算法、前端三层分离，通过 Schema 契约协作
- **Tag Retention**: 转化后伤害保留原标签，同时享受新旧标签加成
- **LRU Cache**: 两级缓存（结果缓存 + PreparedContext 缓存）优化悬停预览性能

## 📁 项目结构

```
tld/
├── tli-core/                   # 计算引擎 (Rust/WASM)
│   └── src/
│       ├── lib.rs              # WASM 入口点
│       ├── types.rs            # 类型定义
│       ├── tags.rs             # 标签系统 (UTAS)
│       ├── stats.rs            # 属性聚合 (StatAggregator)
│       ├── modifiers.rs        # 修正系统 (ModDB/ModList)     [P1]
│       ├── condition_ast.rs    # 条件 AST 系统                [P1]
│       ├── mechanics.rs        # 机制系统 (祝福/球类/资源)
│       ├── conversion.rs       # 伤害转化 & 标签记忆
│       ├── pipeline.rs         # 9 阶段计算管线
│       ├── calculator_cache.rs # LRU 缓存 & 增量计算           [P2]
│       ├── utils.rs            # 工具函数
│       └── data/
│           └── tags_registry.json  # 标签注册表
├── bindings/                   # TypeScript 类型绑定 (ts-rs 导出)
├── pkg/                        # WASM 构建产物
├── supabase/                   # 数据库层
│   ├── schema.sql              # 表结构定义
│   └── seed.sql                # 基础数据
├── doc/                        # 项目文档
│   ├── 开发日志.md
│   ├── 新增机制实施指南.md      # Agent 协作指南
│   └── 火炬之光：无限 BD 决策辅助系统 - 开发规划 (V2.6).md
└── archive/                    # 归档（废弃/参考文件）
```

## 🔧 架构设计

### 三层 Agent 协作

| Agent | 职责 | 产出 |
|-------|------|------|
| **Agent A: Data** (Supabase) | 技能/装备/词缀/标签数据管理 | `schema.sql`, `seed.sql` |
| **Agent B: Core** (Rust/WASM) | 计算引擎，完全不关心 UI | `.wasm` pkg, `bindings.ts` |
| **Agent C: UI** (Next.js) | 交互、状态管理、Worker 调度 | UI Components |

### 9 阶段计算管线

```
1. Sanitization        → 输入校验 & 槽位冲突检测
2. Stat Aggregation    → 属性聚合 (StatPool + ModDB)
3. Base Calculation    → 基础点伤计算
4. Extra As            → "额外获得" 处理
5. Conversion          → 伤害转化 (Tag Retention)
6. Modification        → Inc/More 乘区应用
7. Speed Layer         → 攻速/施法速度计算
8. Crit & Luck         → 暴击 & 幸运伤害
9. Mitigation & Output → 命中/减伤/最终输出
```

### 修正系统 (P1 重构)

```rust
// ModifierStore 统一管理修正
ModDB::new()
    .add(Modifier { key: "dmg.fire", kind: More, value: 0.5, source: "天赋" })
    .add(Modifier { key: "dmg.fire", kind: Inc, value: 1.0, condition: "is_moving" });

// 带条件查询
let fire_more = mod_db.product_more_with_ctx("dmg.fire", &eval_ctx);
```

### 条件 AST 系统 (P1 重构)

```rust
// 支持的条件类型
Flag("is_moving")                      // 布尔标志
Compare(Gte, Value("life_percent"), Literal(0.35))  // 数值比较
HasTag(22)                             // 标签检查
MechanicStacks("focus_blessing", Gte, 5) // 机制层数
And(Flag("is_moving"), HasTag(110))    // 复合条件
```

### 增量计算 (P2 重构)

```
┌─────────────────────────────────────────────────────────────┐
│  calculate_diff_incremental                                  │
├─────────────────────────────────────────────────────────────┤
│  1. 获取 base_input 的 PreparedContext (从缓存)              │
│  2. 为 preview_input 生成增量 modifiers                     │
│  3. 合并增量到 ModDB                                         │
│  4. 重新计算后续阶段                                         │
│  5. 返回 CalculationDiff                                     │
└─────────────────────────────────────────────────────────────┘
```

## 🚀 快速开始

### 构建 WASM

```bash
cd tli-core

# 安装 wasm-pack
cargo install wasm-pack

# 构建
./build.sh
# 或手动
wasm-pack build --target web --out-dir ../pkg --release
```

### 数据库

```bash
# 使用 Supabase CLI
supabase db reset  # 重置并应用 schema.sql + seed.sql
```

### 使用示例

```typescript
import init, { calculate, calculate_diff, get_cache_stats } from './pkg/tli_core.js';
import type { CalculatorInput, CalculatorOutput } from './bindings';

await init();

const input: CalculatorInput = {
  context_flags: { is_moving: true },
  context_values: { life_percent: 0.8 },
  target_config: { level: 100, resistances: {}, generic_dr: 0, armor: 0, evasion: 0, defense_constant: 0 },
  items: [],
  active_skill: {
    id: 'chain_lightning',
    skill_type: 'active',
    is_attack: false,
    level: 21,
    base_damage: { 'dmg.lightning.min': 113, 'dmg.lightning.max': 2147 },
    base_time: 0.65,
    mana_cost: 8,
    effectiveness: 1.77,
    tags: ['Tag_Spell', 'Tag_Lightning', 'Tag_Chain'],
    stats: {},
    injected_tags: [],
    mana_multiplier: 1.0,
  },
  support_skills: [],
  global_overrides: {},
  mechanic_definitions: [],
  mechanic_states: [],
  preview_slot: null,
};

const result: CalculatorOutput = JSON.parse(calculate(JSON.stringify(input)));
console.log(`DPS: ${result.dps_theoretical.toFixed(0)}`);
console.log(`Hit Damage: ${result.hit_damage.toFixed(0)}`);
console.log(`Cache Stats:`, get_cache_stats());
```

## 📋 属性命名规范

| 前缀 | 说明 | 示例 |
|------|------|------|
| `dmg.` | 基础伤害 | `dmg.phys.min`, `dmg.fire.max` |
| `mod.inc.` | Increased 修正 | `mod.inc.dmg.fire` |
| `mod.more.` | More 修正 | `mod.more.dmg.all` |
| `mod.more.*.per_*` | 每层 More | `mod.more.dmg.cold.per_focus_blessing` |
| `crit.` | 暴击相关 | `crit.chance`, `crit.dmg` |
| `speed.` | 速度 | `speed.attack`, `speed.cast` |
| `pen.` | 穿透 | `pen.fire`, `pen.elemental` |
| `conv.` | 转化 | `conv.phys_to_fire` |
| `extra.` | 额外获得 | `extra.phys_as_fire` |
| `flag.` | 布尔开关 | `flag.is_lucky`, `flag.cannot_crit` |
| `mechanic.*.max_stacks` | 机制上限 | `mechanic.focus_blessing.max_stacks` |

## 📖 文档

- [开发规划 V2.6](doc/火炬之光：无限%20BD%20决策辅助系统%20-%20开发规划%20(V2.6).md) - 完整架构设计
- [开发日志](doc/开发日志.md) - 开发历程与技术决策
- [新增机制实施指南](doc/新增机制实施指南.md) - Agent 协作 Prompt 指南

## 🔖 版本历史

### v0.4.0 (2025-12)
- **P0**: StatKey 规范统一、CacheKey 扩展、TagRegistry 数据化
- **P1**: 引入 ModifierStore 抽象、条件 AST 系统
- **P2**: PreparedContext 缓存、增量 Diff 计算
- **P3**: 可解释性输出增强、乘区来源追溯

### v0.3.0 (2025-12)
- 实现核心天赋系统（苦寒、积聚、世事无常、奇妙角度）
- 完善机制系统（祝福、战意、per_xxx 联动）
- 伤害转化与标签记忆
- LRU 缓存优化

## 📄 许可证

MIT
