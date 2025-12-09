## 1. 项目愿景 (Product Vision)
**目标**：开发一款极简配置、高直观性的 BD 决策辅助工具。  
**核心价值**：通过 **上下文即时对比 (Contextual Comparison)** 和 **无状态模拟 (Stateless Simulation)**，解决装备更替的数值决策难题。

**核心理念：**

1. **Contextual Insight**：鼠标悬停即显示 Diff（DPS/EHP 净收益）。
2. **Stateless Simulation**：Rust/WASM 纯函数计算，前端仅负责状态映射。
3. **Modular Agentic**：数据库、算法、前端三层分离，通过 Schema 契约协作。

---

## 2. 架构设计与契约 (Architecture & Contracts)
| 层级 (Agent) | 核心职责 | 关键产出 | 交互契约 (Contract) |
| :--- | :--- | :--- | :--- |
| **Agent A: Data**   (Supabase) | 负责技能数据、道具数据、打造词缀模版、标签注册表、英雄追忆等。   重点：Schema 规范与基础数据填充。 | `schema.sql`   `seed.sql`   `tag.json` | `database.types.ts`   (Auto-generated) |
| **Agent B: Core**   (Rust/WASM) | 负责 AST 解析、UTAS 引擎、标签整数化 (Interning)、伤害管线、防御结算等。   **完全不关心 UI**。 | `.wasm` pkg | `bindings.ts`   (via `ts-rs` export) |
| **Agent C: UI**   (Next.js) | 负责交互、状态管理 (Zustand)、标签溯源可视化、Worker 调度。   负责将 UI 状态组装为 Input JSON。 | UI Components | Worker Wrapper |


---

## 3. 核心计算引擎详解 (Core Specifications - Agent B)
### 3.1 输入结构契约 (Input Schema)
使用 `ts-rs` 导出以下结构：

```rust
#[derive(TS)]
struct CalculatorInput {
    // 1. 动态上下文 (解决 Flag 和 Slider)
    // e.g. "is_moving": true, "enemy_status_shocked": true, "enemy_range": "near"
    context_flags: HashMap<String, bool>,
    context_values: HashMap<String, f64>,

    // 2. 目标配置 (影响减伤公式)
    target_config: TargetConfig, // { level, defense_constant, res_map, generic_dr }

    // 3. 装备与特性数据 (含物品、石板、追忆)
    items: Vec<ItemData>, 

    // 4. 技能组 (主技能 + 辅助技能)
    active_skill: SkillData,
    support_skills: Vec<SkillData>, // 辅助技能提供 More 和 Mana Multiplier

    // 5. 全局注入 (天赋盘/手动输入)
    global_overrides: HashMap<String, f64>,

    // 6. 预览槽位 (用于 Diff 计算，若有值则替换 items 中对应 slot)
    preview_slot: Option<(SlotType, ItemData)>, 
}
```

### 3.2 计算管线 (Pipeline)
1. **Sanitization & Slot Conflict (预处理)**:
    - **2H vs Dual Wield**: 检查 `items` 或 `preview_slot`。若主手为双手武器 (Two-Handed)，强制移除/忽略副手 (Off-hand) 槽位的数据。
    - **Unique Limit**: 检查唯一装备限制。
2. **Stat Pool Aggregation**:
    - 遍历 Items, Skills, Overrides。
    - 解析 Conditional AST (e.g., `if is_moving then damage_inc += 20`).
    - **Local vs Global**: 优先计算装备的局部属性（Local，如武器物理点伤、武器攻速），修正 Item Base Stats 后再汇总全局属性。
3. **Base Calculation**:
    - 确定技能基础点伤。
    - 应用 **Damage Effectiveness**。
4. **Extra & Conversion (Tag Retention)**:
    - **Phase A: Gain as Extra**: 计算“额外获得”逻辑（如：物理的 20% 额外获得为火焰）。此步骤不扣除原伤害，且产出的新伤害保留原伤害标签。
    - **Phase B: Conversion**: 执行转化矩阵（Phys -> Cold -> Fire）。此步骤扣除原伤害，产出新伤害保留历史标签。
    - _Note_: 必须建立 DAG 确保顺序正确 (Phys -> Lightning -> Cold -> Fire -> Chaos)。
5. **Modification (Inc/More)**:
    - **Increased**: 同标签累加。
    - **More**: 按 `bucket` 归并后相乘。
6. **Speed Layer (速度乘区)**:
    - 区分 `Attack Speed` 与 `Cast Speed`。

公式：

    - $$Rate = \frac{1}{\text{BaseTime}} \times (1 + \sum \text{Inc}) \times \prod \text{More}$$
    - 处理 `Cooldown Recovery` 对由 CD 限制 DPS 的技能的影响。
7. **Crit & Luck**:
    - 计算暴击率与爆伤。
    - 处理 `Lucky` 逻辑。
8. **Mitigation & Output**:
    - 计算命中、抗性穿透、护甲减伤。
    - 输出 `dps_theoretical` (Hit Dmg * Rate), `dps_effective`, `ehp_series`.

### 3.3 通用标签与属性系统 (UTAS)
摒弃硬编码，采用全标签匹配逻辑来实现节点联动与自动判定。

#### **3.3.1 标签四分类 (Taxonomy)**
+ **Identity (身份)**: `Tag_Spell`, `Tag_Aura`。用于筛选增伤词缀是否生效。
+ **Mechanic (机制)**: `Mech_Blessing`。携带层数、充能等动态资源数值，需支持参数化。
+ **Rule (规则)**: `Rule_Conversion`。改变计算公式的开关（如：坚韧祝福转化为增伤）。
+ **State (状态)**: `Tag_State_Low_Life`。用于条件增伤判定。

#### **3.3.2 性能优化：标签整数化 (Tag Interning)**
+ **原理**：WASM 内部严禁使用 String 进行匹配。
+ **实现**：
    - **Startup**: 加载 `tags_registry`，建立全局映射 `Map<String, u32>`。
    - **Runtime**: 所有标签转化为 `u32` 或 `BitSet` 进行集合运算（如 `contains`, `is_subset`）。
    - **Inheritance**: 预计算展开集（例如 `Fire(5)` 自动展开为 `[5, 2]`，其中 `2` 是 `Elemental`）。

### 3.4 标签计算管线 (Tag Pipeline)
1. **Context Injection (上下文注入)**:
    - **静态注入**: 根据技能 ID 注入固有标签（如 `Tag_Fire`）。
    - **动态注入**: 辅助技能（如“狂雷”）向主技能注入新标签（如 `Tag_Burst`）。
    - **环境注入**: 将 `context_flags`（如低血、移动中）合并入当前标签集。
2. **Tag Inheritance (继承展开)**:
    - 基于 `tags_registry` 自动为节点注入所有父级标签（如 `Fire` -> `Elemental`）。
3. **Modifier Matching (修正匹配)**:
    - 遍历属性池，执行 `if context_tags.contains_all(modifier.requirements)`。
    - **自动判定**：例如装备赋予“击中造成减速”，则自动激活“敌人受控制时增伤”的词缀。
4. **Conversion & Tag Retention**:
    - 执行转化 DAG（有向无环图）。
    - 保留历史标签（Tag Trace），确保转化后的伤害能同时吃到两端的加成。
5. **Final Calculation**:
    - Base -> Inc -> More -> Crit -> Mitigation.
6. **Output Generation**:
    - 返回 `DPS`, `EHP` 以及 `**debug_trace**` (标签匹配溯源，用于 UI 解释)。



---

## 4. 数据层规范 (Data Specifications - Agent A)
### 4.1 命名空间 (Namespace)
+ **格式**: `dot.notation` (全小写)
+ **数值**: 百分比一律存为小数 (`0.15` = 15%)
+ **标准键名示例**:
    - `dmg.phys.min`, `dmg.fire.max`
    - `crit.chance`, `crit.dmg`
    - `mod.inc.dmg.all` (全局Inc)
    - `mod.more.dmg.all` (全局More)

### 4.2 必须包含的表
1. `**items_meta**`: 包含 `base_type` (基底), `implicit_stats` (基底词缀),`is_two_handed` (boolean) 字段 (用于互斥判断)。.
2. `**affixes**`: 包含 `group`, `tier`,  `template_text`,  `min_val`, `max_val`,`stats` (JSONB), `tags`.
3. `**unique_affixes**`: 包含 `item_id`, `tier`, `line_index`, `variant_type` (区分是否侵蚀), `template_text`,  `min_val`, `max_val`, `stats` (JSONB), `tags`.
4. `**unique_items**`: 传奇装备表。包含`id`,`display_name`,`type`,`slot`,`icon`,
5. `**skills**`: 包含 `growth_table_id` (关联等级成长表), `tags`, `effectiveness`.
6. `**hero_traits**`: 英雄特性节点数据（仅占位处理）。
7. `**hero_memories**`: 英雄追忆节点数据（仅占位处理）。

### 4.3 标签注册表 (`tags_registry`)
采用 JSON 配置化管理，支持层级扩展。

JSON

```plain
{
  "Tag_Fire": {
    "id": 101, 
    "category": "Identity", 
    "parents": ["Tag_Elemental"], 
    "displayName": "火焰" 
  }
}
```

### 4.4 属性元数据 (`attributes_meta`)
增强 `ItemModifier` 结构，支持复杂交互：

+ `requirements`: string[] (e.g., `["Tag_Spell", "Tag_Fire"]`)
+ `condition`: string (e.g., `Tag_State_Low_Life`)
+ `action`: string (e.g., `On_Hit`)
+ `resource_interaction`: { type: "Mana", value: -10 }

### 4.5 打造与多态属性 (Crafting Logic)
+ **多态支持**: 数据库 `unique_affixes` 表支持 `Base` 和 `Corrupted` 变体。且装备打造属性支持范围调节（最大值、最小值）

---

## 5. 前端交互规范 (UI/UX - Agent C)
_此部分指导 Agent C (Frontend) 开发。目标：复刻 _[_Maxroll D4 Planner_](https://maxroll.gg/d4/planner)_ 的布局。_

### 5.1 状态管理 (Zustand Store)
+ `**useBuildStore**`:
    - `slots`: Record<SlotType, Item> (含 Hero Memories 槽位)
    - `skills`: { main: Skill, supports: Skill[] }
    - `config`: TargetConfig & PlayerState
    - `previewItem`: Item | null (悬停时触发)

### 5.2 宏观布局 (Holy Grail Layout)
采用标准的 **左-中-右** 响应式布局。

+ **Header**: 包含 `Save Profile`, `Share Link`, `Reset`。
+ **Left Sidebar (Inputs - 20%)**: **配置层**。放置“手动属性注入器”。
+ **Center Stage (Visuals - 50%)**: **视觉层**。放置“纸娃娃 (Paper Doll)”。
+ **Right Sidebar (Outputs - 30%)**: **数据层**。放置“动态面板 (Stats)”。

### 5.3 详细组件拆解
#### **A. 左侧栏：手动注入器 (The Injector)**
_替代 Maxroll 的 "Skill Tree/Paragon" 区域。_

+ **组件**: `ScrollArea` + `Accordion` (折叠面板)。
+ **内容**:
    - **Base Stats**: 力量/敏捷/智慧输入框。
    - **Global Modifiers (核心功能)**:
        * 一组带有 Label 的 Input 框。
        * `Global Inc Damage %`: [ 1600 ]
        * `Crit Chance %`: [ 350 ]
        * `Crit Multiplier %`: [ 40 ]
    - **Custom Affix**: “添加自定义词缀”按钮 (用于处理特殊机制)。
+ **交互**: `onChange` 事件需带 500ms 防抖，触发 WASM 重算。

#### **B. 中间层：纸娃娃 (The Paper Doll)**
_完全复刻 Maxroll 的装备展示。_

+ **布局**:
    - 围绕角色立绘/轮廓，左侧 5 槽 (头/胸/手/腿/项链)，右侧 5 槽 (戒1/戒2/武1/武2/其他)。
    - 中间下方放置 **Skill Bar** (5个圆形技能槽)。
+ **交互 - 装备选择 (The Drawer)**:
    - 点击槽位 -> 从屏幕右侧滑出 `Sheet` (Shadcn 组件)。
    - **Ghost Preview (幽灵预览)**:
        * 当鼠标悬停在列表中的装备上时，**不要关闭 Drawer**。
        * 触发全局状态 `previewItem` 更新。
        * **右侧数据面板实时跳动**，显示红/绿差值。

#### **C. 右侧栏：动态数据流 (Reactive Stats)**
_复刻 Maxroll 的 "Stats" 面板。_

+ **Sticky**: 确保滚动时常驻可见。
+ **Header**: 显示超大字体的 `DPS` 和 `EHP`。
+ **Diff System (差值系统)**:
    - 这是本系统的灵魂。
    - 逻辑：`DisplayValue = PreviewMode ? (NewStat - OldStat) : CurrentStat`
    - 样式：
        * 正收益: `+15.2% ▲` (绿色, Bold)
        * 负收益: `-5.4% ▼` (红色)
        * 无变化: 灰色或隐藏

### 5.4 打造与微调 (Micro-Adjustments)
+ **操作**：右键已穿戴装备 -> 弹出 `Item` 悬浮窗。
+ **功能**：
    - **Unequip/Change：**卸下/更换装备。
    - **Add/Delete Modifier**: 增加/删除装备词缀。
    - **Edit Modifier: **编辑装备词缀（下拉菜单搜索添加）。
    - **Edit Slider**: 拖动选择词缀数值范围 (Rolls)。
    - **Edit Toggle**: 切换词缀 `Corrupted` (侵蚀) 状态。

---

## 6. Agent 协作 Prompt 指南 (Vibe Coding)
### 给 Agent A (DB) 的指令：
"你需要设计 TLI 的数据库 Schema。请创建 `hero_traits`, `skills`, `items` 表。注意：`skills` 表必须支持 JSONB 格式的等级成长表（Level Scaling）。所有百分比字段必须注释说明为小数。请给出 PostgreSQL 的 CREATE 语句和一段针对 '狂人1' 特性的 Seed Data。"

### 给 Agent B (Rust) 的指令：
"请实现 `calculate_dps` 核心函数。

1. **Input**: 使用 `ts-rs` 导出的 `CalculatorInput` 结构。
2. **逻辑**: 实现 'Stat Pool' 聚合器。
3. **重点**: 请实现 'Tag Retention'（标签记忆）逻辑——如果伤害从物理转化为火焰，它必须同时享受 'Physical Increase' 和 'Fire Increase' 的加成。
4. **测试**: 编写单元测试，验证 100 物理基础伤在 50% 转化火焰 + 10% 物理Inc + 10% 火焰Inc 下的最终数值。"

### 给 Agent C (Frontend) 的指令：
"请构建 UI 骨架。

1. **布局**: 左侧 Config (Shadcn Form)，中间 PaperDoll (Grid)，右侧 Dashboard (Cards)。
2. **Worker**: 编写一个 `calculation.worker.ts`，在其中导入 WASM 模块。
3. **Store**: 使用 Zustand 实现 `setPreviewItem` Action。当该 Action 被触发时，防抖 200ms 后向 Worker 发送计算请求，并将返回的 Diff 渲染在 Dashboard 上。"

