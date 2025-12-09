# TLI BD 决策辅助系统

火炬之光：无限 (Torchlight Infinite) Build Decision Assistant System

## 项目结构

```
yex/
├── supabase/           # 数据库层 (Agent A)
│   ├── schema.sql      # 数据库 Schema
│   └── seed.sql        # 基础数据
├── data/               # 配置数据
│   └── tags_registry.json  # 标签注册表
├── tli-core/           # 计算引擎 (Agent B)
│   ├── src/
│   │   ├── lib.rs      # 库入口
│   │   ├── types.rs    # 类型定义
│   │   ├── tags.rs     # 标签系统
│   │   ├── stats.rs    # 属性聚合
│   │   ├── conversion.rs  # 伤害转化
│   │   ├── pipeline.rs # 计算管线
│   │   └── utils.rs    # 工具函数
│   ├── Cargo.toml
│   └── build.sh        # 构建脚本
├── bindings/           # TypeScript 类型绑定
│   └── index.ts
└── pkg/                # WASM 输出 (构建后生成)
```

## 核心功能

### Agent A: 数据库层 (Supabase)

- **标签注册表** (`tags_registry`): 支持层级继承的标签系统
- **装备系统**: 基底 (`items_meta`), 词缀 (`affixes`), 传奇 (`unique_items`)
- **技能系统**: 主动技能、辅助技能、等级成长
- **转化规则**: DAG 顺序的伤害转化

### Agent B: 计算引擎 (Rust/WASM)

- **标签整数化 (Tag Interning)**: String → u32 映射，BitSet 高效集合运算
- **UTAS 系统**: 通用标签与属性系统，支持继承展开
- **计算管线**:
  1. Sanitization & Slot Conflict
  2. Stat Pool Aggregation
  3. Base Calculation
  4. Extra & Conversion (Tag Retention)
  5. Modification (Inc/More)
  6. Speed Layer
  7. Crit & Luck
  8. Mitigation & Output

### Tag Retention (标签记忆)

核心特性：转化后的伤害同时享受原标签和新标签的加成

```
示例：100 物理 → 50% 转火焰
结果：50 物理 + 50 火焰（带物理历史标签）
加成：物理 Inc 10% + 火焰 Inc 10%
     = 50 × 1.1 + 50 × 1.1 × 1.1 = 55 + 60.5 = 115.5
```

## 构建

### 数据库

```bash
# 使用 Supabase CLI
supabase db reset  # 重置并应用 schema.sql + seed.sql
```

### WASM

```bash
cd tli-core

# 安装 wasm-pack
cargo install wasm-pack

# 构建
./build.sh
# 或手动
wasm-pack build --target web --out-dir ../pkg --release
```

## 使用

```typescript
import init, { calculate, version } from './pkg/tli_core.js';
import type { CalculatorInput, CalculatorOutput } from './bindings';

await init();

const input: CalculatorInput = {
  context_flags: { is_moving: true },
  context_values: {},
  target_config: { level: 100, resistances: {}, generic_dr: 0, armor: 0, evasion: 0, defense_constant: 0 },
  items: [],
  active_skill: {
    id: 'fireball',
    skill_type: 'active',
    is_attack: false,
    level: 20,
    base_damage: { 'dmg.fire.min': 100, 'dmg.fire.max': 150 },
    base_time: 0.8,
    mana_cost: 15,
    effectiveness: 1.0,
    tags: ['Tag_Spell', 'Tag_Fire', 'Tag_AOE'],
    stats: {},
    injected_tags: [],
    mana_multiplier: 1.0,
  },
  support_skills: [],
  global_overrides: {
    'mod.inc.dmg.fire': 2.0,  // +200% 火焰伤害
    'crit.chance': 0.4,        // 40% 暴击率
  },
};

const result: CalculatorOutput = JSON.parse(calculate(JSON.stringify(input)));
console.log(`DPS: ${result.dps_theoretical.toFixed(0)}`);
console.log(`Hit Damage: ${result.hit_damage.toFixed(0)}`);
```

## 属性命名规范

- 格式: `dot.notation` (全小写)
- 百分比存为小数 (0.15 = 15%)

| 前缀 | 说明 | 示例 |
|------|------|------|
| `dmg.` | 基础伤害 | `dmg.phys.min`, `dmg.fire.max` |
| `mod.inc.` | Increased 修正 | `mod.inc.dmg.fire` |
| `mod.more.` | More 修正 | `mod.more.dmg.all` |
| `crit.` | 暴击相关 | `crit.chance`, `crit.dmg` |
| `speed.` | 速度 | `speed.attack`, `speed.cast` |
| `pen.` | 穿透 | `pen.fire`, `pen.elemental` |
| `conv.` | 转化 | `conv.phys_to_fire` |
| `extra.` | 额外获得 | `extra.phys_as_fire` |
| `res.` | 抗性 | `res.fire`, `res.cold` |
| `def.` | 防御 | `def.armor`, `def.block` |

## 许可证

MIT

