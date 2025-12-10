-- ============================================================
-- TLI (火炬之光：无限) BD 决策辅助系统 - 数据库 Schema
-- ============================================================
-- 命名空间规范：
--   - 格式：dot.notation（全小写）
--   - 数值：百分比存为小数（0.15 = 15%）
--   - 标准键名：dmg.phys.min, mod.inc.dmg.all, crit.chance 等
-- ============================================================

-- 启用必要扩展
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ============================================================
-- 1. 标签注册表 (tags_registry)
-- 用于标签整数化映射，支持层级继承
-- ============================================================
CREATE TABLE tags_registry (
    id SERIAL PRIMARY KEY,
    tag_key VARCHAR(64) NOT NULL UNIQUE,          -- 如 'Tag_Fire', 'Tag_Spell'
    category VARCHAR(32) NOT NULL,                 -- Identity / Mechanic / Rule / State
    display_name VARCHAR(64) NOT NULL,             -- 显示名称，如 '火焰'
    parents TEXT[] DEFAULT '{}',                   -- 父级标签数组，如 ['Tag_Elemental']
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_tags_category ON tags_registry(category);
CREATE INDEX idx_tags_key ON tags_registry(tag_key);

COMMENT ON TABLE tags_registry IS '标签注册表，用于WASM整数化映射与继承展开';
COMMENT ON COLUMN tags_registry.category IS '标签分类: Identity(身份)/Mechanic(机制)/Rule(规则)/State(状态)';
COMMENT ON COLUMN tags_registry.parents IS '父级标签数组，用于继承展开计算';

-- ============================================================
-- 2. 属性元数据表 (attributes_meta)
-- 定义所有可能的属性键及其元信息
-- ============================================================
CREATE TABLE attributes_meta (
    id SERIAL PRIMARY KEY,
    attr_key VARCHAR(64) NOT NULL UNIQUE,          -- 如 'dmg.phys.min', 'mod.inc.dmg.fire'
    display_name VARCHAR(128) NOT NULL,            -- 显示名称
    value_type VARCHAR(16) NOT NULL DEFAULT 'float', -- float / int / bool / percent
    is_local BOOLEAN DEFAULT FALSE,                -- 是否为局部属性（如武器物理点伤）
    requirements TEXT[] DEFAULT '{}',              -- 生效所需标签，如 ['Tag_Spell', 'Tag_Fire']
    condition VARCHAR(64),                         -- 触发条件，如 'Tag_State_Low_Life'
    action VARCHAR(64),                            -- 行为触发器，如 'On_Hit'
    resource_type VARCHAR(32),                     -- 资源类型，如 'Mana', 'Life'
    resource_value DECIMAL(10,4),                  -- 资源交互值（负数为消耗）
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_attr_key ON attributes_meta(attr_key);
CREATE INDEX idx_attr_local ON attributes_meta(is_local);

COMMENT ON TABLE attributes_meta IS '属性元数据表，定义所有属性键及其交互规则';
COMMENT ON COLUMN attributes_meta.value_type IS '值类型: float/int/bool/percent';
COMMENT ON COLUMN attributes_meta.is_local IS '局部属性（如武器物理点伤）在汇总前先修正装备基础';
COMMENT ON COLUMN attributes_meta.requirements IS '属性生效所需的标签数组';

-- ============================================================
-- 3. 装备基底表 (items_meta)
-- 存储装备基底类型及其固有属性
-- ============================================================
CREATE TABLE items_meta (
    id SERIAL PRIMARY KEY,
    base_type VARCHAR(64) NOT NULL UNIQUE,         -- 基底类型，如 'sword_1h', 'staff_2h'
    display_name VARCHAR(128) NOT NULL,            -- 显示名称
    slot VARCHAR(32) NOT NULL,                     -- 槽位：weapon_main, weapon_off, helmet, chest, gloves, boots, amulet, ring, belt
    is_two_handed BOOLEAN DEFAULT FALSE,           -- 是否为双手武器（用于互斥判断）
    implicit_stats JSONB DEFAULT '{}',             -- 基底固有词缀 JSONB，如 {"dmg.phys.min": 10, "dmg.phys.max": 20}
    base_requirements JSONB DEFAULT '{}',          -- 装备需求，如 {"level": 60, "str": 100}
    tags TEXT[] DEFAULT '{}',                      -- 装备标签，如 ['Tag_Weapon', 'Tag_Sword']
    icon VARCHAR(256),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_items_meta_slot ON items_meta(slot);
CREATE INDEX idx_items_meta_two_handed ON items_meta(is_two_handed);

COMMENT ON TABLE items_meta IS '装备基底数据表';
COMMENT ON COLUMN items_meta.implicit_stats IS '基底固有词缀，JSONB格式，百分比存为小数';
COMMENT ON COLUMN items_meta.is_two_handed IS '双手武器标记，用于双持/双手互斥判断';

-- ============================================================
-- 4. 打造词缀模板表 (affixes)
-- 存储所有可打造的词缀模板
-- ============================================================
CREATE TABLE affixes (
    id SERIAL PRIMARY KEY,
    affix_group VARCHAR(64) NOT NULL,              -- 词缀组，同组互斥
    tier INTEGER NOT NULL DEFAULT 1,               -- 词缀等级 T1-T7
    template_text VARCHAR(512) NOT NULL,           -- 模板文本，如 '增加 {0}% 物理伤害'
    min_val DECIMAL(10,4) NOT NULL,                -- 最小值
    max_val DECIMAL(10,4) NOT NULL,                -- 最大值
    stats JSONB NOT NULL DEFAULT '{}',             -- 属性效果，如 {"mod.inc.dmg.phys": "{0}"}
    tags TEXT[] DEFAULT '{}',                      -- 词缀标签，用于匹配生效条件
    slot_restrictions TEXT[] DEFAULT '{}',         -- 可出现的槽位限制，空表示无限制
    item_level_req INTEGER DEFAULT 1,              -- 物品等级需求
    weight INTEGER DEFAULT 1000,                   -- 出现权重
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_affixes_group ON affixes(affix_group);
CREATE INDEX idx_affixes_tier ON affixes(tier);
CREATE INDEX idx_affixes_tags ON affixes USING GIN(tags);

COMMENT ON TABLE affixes IS '打造词缀模板表';
COMMENT ON COLUMN affixes.affix_group IS '词缀组ID，同组词缀互斥';
COMMENT ON COLUMN affixes.stats IS '属性效果JSONB，{0}为占位符，运行时替换为实际数值';
COMMENT ON COLUMN affixes.min_val IS '数值下限，百分比存为小数';
COMMENT ON COLUMN affixes.max_val IS '数值上限，百分比存为小数';

-- ============================================================
-- 5. 传奇装备主表 (unique_items)
-- 存储所有传奇/暗金装备
-- ============================================================
CREATE TABLE unique_items (
    id VARCHAR(64) PRIMARY KEY,                    -- 唯一ID，如 'unique_sword_fire_001'
    display_name VARCHAR(128) NOT NULL,            -- 显示名称
    base_type VARCHAR(64) NOT NULL REFERENCES items_meta(base_type),
    item_type VARCHAR(32) NOT NULL,                -- 装备类型：weapon, armor, accessory
    slot VARCHAR(32) NOT NULL,                     -- 槽位
    rarity VARCHAR(16) DEFAULT 'unique',           -- unique / legendary
    level_req INTEGER DEFAULT 1,                   -- 等级需求
    icon VARCHAR(256),
    flavor_text TEXT,                              -- 背景故事文本
    tags TEXT[] DEFAULT '{}',
    is_limited BOOLEAN DEFAULT FALSE,              -- 是否为限定装备（一个角色只能装备一件）
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_unique_items_slot ON unique_items(slot);
CREATE INDEX idx_unique_items_type ON unique_items(item_type);

COMMENT ON TABLE unique_items IS '传奇/暗金装备主表';
COMMENT ON COLUMN unique_items.is_limited IS '限定装备标记，同类限定装备只能装备一件';

-- ============================================================
-- 6. 传奇装备词缀表 (unique_affixes)
-- 存储传奇装备的特殊词缀（支持侵蚀变体）
-- ============================================================
CREATE TABLE unique_affixes (
    id SERIAL PRIMARY KEY,
    item_id VARCHAR(64) NOT NULL REFERENCES unique_items(id) ON DELETE CASCADE,
    tier INTEGER DEFAULT 1,                        -- 词缀等级
    line_index INTEGER NOT NULL,                   -- 词缀行索引（用于排序显示）
    variant_type VARCHAR(16) DEFAULT 'base',       -- base / corrupted（侵蚀变体）
    template_text VARCHAR(512) NOT NULL,           -- 模板文本
    min_val DECIMAL(10,4),                         -- 最小值（可空表示固定值）
    max_val DECIMAL(10,4),                         -- 最大值
    fixed_val DECIMAL(10,4),                       -- 固定值（当 min=max 时使用）
    stats JSONB NOT NULL DEFAULT '{}',             -- 属性效果 JSONB
    tags TEXT[] DEFAULT '{}',
    is_implicit BOOLEAN DEFAULT FALSE,             -- 是否为固有词缀
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_unique_affixes_item ON unique_affixes(item_id);
CREATE INDEX idx_unique_affixes_variant ON unique_affixes(variant_type);
CREATE UNIQUE INDEX idx_unique_affixes_order ON unique_affixes(item_id, line_index, variant_type);

COMMENT ON TABLE unique_affixes IS '传奇装备词缀表，支持基础版本与侵蚀变体';
COMMENT ON COLUMN unique_affixes.variant_type IS '变体类型: base(基础) / corrupted(侵蚀)';
COMMENT ON COLUMN unique_affixes.line_index IS '词缀显示顺序索引';

-- ============================================================
-- 7. 技能表 (skills)
-- 存储所有主动技能数据
-- ============================================================
CREATE TABLE skills (
    id VARCHAR(64) PRIMARY KEY,                    -- 技能ID，如 'skill_fireball'
    display_name VARCHAR(128) NOT NULL,            -- 显示名称
    skill_type VARCHAR(32) NOT NULL,               -- 技能类型：active / support / aura
    damage_type VARCHAR(32),                       -- 主伤害类型：physical / fire / cold / lightning / chaos
    is_attack BOOLEAN DEFAULT FALSE,               -- 是否为攻击（影响攻速/施法速度）
    base_damage JSONB DEFAULT '{}',                -- 基础伤害，如 {"dmg.fire.min": 10, "dmg.fire.max": 20}
    base_time DECIMAL(6,3) DEFAULT 1.0,            -- 基础时间（秒），用于计算 Rate
    cooldown DECIMAL(6,3),                         -- 冷却时间（秒），空表示无冷却
    mana_cost INTEGER DEFAULT 0,                   -- 魔力消耗
    effectiveness DECIMAL(6,4) DEFAULT 1.0,        -- Damage Effectiveness，如 1.0 = 100%
    growth_table JSONB DEFAULT '{}',               -- 等级成长表，如 {"1": {"base_damage": 10}, "20": {"base_damage": 100}}
    tags TEXT[] DEFAULT '{}',                      -- 技能标签，如 ['Tag_Spell', 'Tag_Fire', 'Tag_AOE']
    stats JSONB DEFAULT '{}',                      -- 技能自带属性加成
    icon VARCHAR(256),
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_skills_type ON skills(skill_type);
CREATE INDEX idx_skills_tags ON skills USING GIN(tags);

COMMENT ON TABLE skills IS '技能数据表';
COMMENT ON COLUMN skills.effectiveness IS 'Damage Effectiveness，附加伤害的有效系数，1.0=100%';
COMMENT ON COLUMN skills.growth_table IS '等级成长表JSONB，键为等级，值为该等级的属性覆盖';
COMMENT ON COLUMN skills.base_time IS '基础施放/攻击时间（秒），Rate = 1/base_time * speed_modifiers';

-- ============================================================
-- 8. 技能等级成长表 (skill_level_data)
-- 存储每个技能每个等级的具体数值
-- ============================================================
CREATE TABLE skill_level_data (
    id SERIAL PRIMARY KEY,
    skill_id VARCHAR(64) NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    level INTEGER NOT NULL,                        -- 技能等级 (1-20 详细数据, 21+ 使用公式)
    effectiveness DECIMAL(6,4) NOT NULL,           -- 伤害倍率，如 1.63 = 163%
    base_damage JSONB NOT NULL DEFAULT '{}',       -- 基础伤害，如 {"dmg.lightning.min": 1, "dmg.lightning.max": 23}
    mana_cost INTEGER,                             -- 该等级魔力消耗（可覆盖技能默认值）
    base_time DECIMAL(6,3),                        -- 该等级施法/攻击时间（可覆盖技能默认值）
    extra_effects JSONB DEFAULT '{}',              -- 额外效果，如 {"chain_count": 2, "aoe_radius": 10}
    stats JSONB DEFAULT '{}',                      -- 该等级额外属性加成
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(skill_id, level)
);

CREATE INDEX idx_skill_level_skill ON skill_level_data(skill_id);
CREATE INDEX idx_skill_level_level ON skill_level_data(level);

COMMENT ON TABLE skill_level_data IS '技能等级成长表，存储1-20级详细数据';
COMMENT ON COLUMN skill_level_data.effectiveness IS '伤害倍率（Damage Effectiveness），1.63 = 163%';
COMMENT ON COLUMN skill_level_data.base_damage IS '该等级基础伤害，JSONB格式';
COMMENT ON COLUMN skill_level_data.extra_effects IS '额外效果，如弹射次数、AOE半径等';

-- ============================================================
-- 9. 技能等级缩放规则表 (skill_scaling_rules)
-- 定义21级及以上的伤害缩放规则
-- ============================================================
CREATE TABLE skill_scaling_rules (
    id SERIAL PRIMARY KEY,
    skill_id VARCHAR(64) NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    level_range_start INTEGER NOT NULL,            -- 等级范围起始（含）
    level_range_end INTEGER,                       -- 等级范围结束（含），NULL表示无上限
    damage_multiplier_per_level DECIMAL(6,4) NOT NULL, -- 每级伤害乘数，如 1.10 = +10%
    scaling_type VARCHAR(32) DEFAULT 'multiplicative', -- 缩放类型: multiplicative / additive
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_skill_scaling_skill ON skill_scaling_rules(skill_id);

-- 插入默认的伤害性技能缩放规则（通用规则，skill_id 为空表示全局默认）
INSERT INTO skill_scaling_rules (skill_id, level_range_start, level_range_end, damage_multiplier_per_level, description)
VALUES 
    ('_default_damage', 21, 30, 1.10, '21-30级：每级额外+10%伤害（叠乘）'),
    ('_default_damage', 31, NULL, 1.08, '31级及以上：每级额外+8%伤害（叠乘）');

COMMENT ON TABLE skill_scaling_rules IS '技能等级缩放规则表，定义21级以上的伤害公式';
COMMENT ON COLUMN skill_scaling_rules.damage_multiplier_per_level IS '每级伤害乘数，叠乘计算';
COMMENT ON COLUMN skill_scaling_rules.level_range_end IS '等级范围结束，NULL表示无上限';

-- ============================================================
-- 10. 辅助技能加成表 (support_skill_modifiers)
-- 存储辅助技能对主技能的加成效果
-- ============================================================
CREATE TABLE support_skill_modifiers (
    id SERIAL PRIMARY KEY,
    skill_id VARCHAR(64) NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    level INTEGER NOT NULL DEFAULT 1,              -- 辅助技能等级
    mana_multiplier DECIMAL(6,4) DEFAULT 1.0,      -- 魔力倍率
    stats JSONB NOT NULL DEFAULT '{}',             -- 提供的属性加成
    injected_tags TEXT[] DEFAULT '{}',             -- 向主技能注入的标签
    requirements TEXT[] DEFAULT '{}',              -- 需求的主技能标签
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_support_mods_skill ON support_skill_modifiers(skill_id);
CREATE UNIQUE INDEX idx_support_mods_level ON support_skill_modifiers(skill_id, level);

COMMENT ON TABLE support_skill_modifiers IS '辅助技能等级加成表';
COMMENT ON COLUMN support_skill_modifiers.mana_multiplier IS '魔力消耗倍率，如1.3表示增加30%魔力消耗';
COMMENT ON COLUMN support_skill_modifiers.injected_tags IS '注入主技能的标签，如狂雷注入Tag_Burst';

-- ============================================================
-- 11. 英雄特性表 (hero_traits) - 占位结构
-- ============================================================
CREATE TABLE hero_traits (
    id VARCHAR(64) PRIMARY KEY,
    display_name VARCHAR(128) NOT NULL,
    hero_class VARCHAR(32) NOT NULL,               -- 英雄职业
    trait_type VARCHAR(32) NOT NULL,               -- 特性类型：passive / keystone
    position JSONB DEFAULT '{}',                   -- 天赋树位置 {"x": 0, "y": 0}
    connections TEXT[] DEFAULT '{}',               -- 连接的其他节点ID
    stats JSONB DEFAULT '{}',                      -- 提供的属性
    requirements JSONB DEFAULT '{}',               -- 解锁需求
    tags TEXT[] DEFAULT '{}',
    icon VARCHAR(256),
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_hero_traits_class ON hero_traits(hero_class);
CREATE INDEX idx_hero_traits_type ON hero_traits(trait_type);

COMMENT ON TABLE hero_traits IS '英雄特性节点表（占位结构）';

-- ============================================================
-- 12. 英雄追忆表 (hero_memories) - 占位结构
-- ============================================================
CREATE TABLE hero_memories (
    id VARCHAR(64) PRIMARY KEY,
    display_name VARCHAR(128) NOT NULL,
    hero_class VARCHAR(32),                        -- 职业限制，空表示通用
    memory_type VARCHAR(32) NOT NULL,              -- 追忆类型
    slot INTEGER NOT NULL,                         -- 追忆槽位 1-6
    tier INTEGER DEFAULT 1,                        -- 追忆等级
    stats JSONB DEFAULT '{}',                      -- 提供的属性
    special_effect TEXT,                           -- 特殊效果描述
    tags TEXT[] DEFAULT '{}',
    icon VARCHAR(256),
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_hero_memories_class ON hero_memories(hero_class);
CREATE INDEX idx_hero_memories_slot ON hero_memories(slot);

COMMENT ON TABLE hero_memories IS '英雄追忆节点表（占位结构）';

-- ============================================================
-- 13. 石板表 (pacts) - 用于存储石板数据
-- ============================================================
CREATE TABLE pacts (
    id VARCHAR(64) PRIMARY KEY,
    display_name VARCHAR(128) NOT NULL,
    pact_type VARCHAR(32) NOT NULL,                -- 石板类型
    tier INTEGER DEFAULT 1,
    stats JSONB DEFAULT '{}',
    tags TEXT[] DEFAULT '{}',
    icon VARCHAR(256),
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_pacts_type ON pacts(pact_type);

COMMENT ON TABLE pacts IS '石板数据表';

-- ============================================================
-- 14. 伤害转化规则表 (conversion_rules)
-- 定义伤害类型转化的DAG顺序
-- ============================================================
CREATE TABLE conversion_rules (
    id SERIAL PRIMARY KEY,
    from_type VARCHAR(32) NOT NULL,                -- 源伤害类型
    to_type VARCHAR(32) NOT NULL,                  -- 目标伤害类型
    priority INTEGER NOT NULL,                     -- 转化优先级（数字越小越先执行）
    is_allowed BOOLEAN DEFAULT TRUE,               -- 是否允许此转化路径
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(from_type, to_type)
);

-- 预定义转化优先级顺序：Physical -> Lightning -> Cold -> Fire -> Chaos
INSERT INTO conversion_rules (from_type, to_type, priority) VALUES
    ('physical', 'lightning', 1),
    ('physical', 'cold', 2),
    ('physical', 'fire', 3),
    ('physical', 'chaos', 4),
    ('lightning', 'cold', 5),
    ('lightning', 'fire', 6),
    ('lightning', 'chaos', 7),
    ('cold', 'fire', 8),
    ('cold', 'chaos', 9),
    ('fire', 'chaos', 10);

COMMENT ON TABLE conversion_rules IS '伤害转化DAG规则表，确保转化顺序正确';
COMMENT ON COLUMN conversion_rules.priority IS '转化优先级，数字越小越先执行';

-- ============================================================
-- 15. 目标配置模板表 (target_configs)
-- 预设的目标配置（用于快速选择）
-- ============================================================
CREATE TABLE target_configs (
    id VARCHAR(64) PRIMARY KEY,
    display_name VARCHAR(128) NOT NULL,
    level INTEGER NOT NULL DEFAULT 100,
    defense_constant DECIMAL(10,2) DEFAULT 0,      -- 防御常数
    resistances JSONB DEFAULT '{}',                -- 抗性 {"fire": 0.3, "cold": 0.3, ...}
    generic_dr DECIMAL(6,4) DEFAULT 0,             -- 通用减伤
    armor INTEGER DEFAULT 0,
    evasion INTEGER DEFAULT 0,
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE target_configs IS '目标配置预设表';
COMMENT ON COLUMN target_configs.resistances IS '目标抗性JSONB，百分比存为小数';
COMMENT ON COLUMN target_configs.generic_dr IS '通用减伤，百分比存为小数';

-- ============================================================
-- 默认目标配置
-- ============================================================
INSERT INTO target_configs (id, display_name, level, resistances, generic_dr) VALUES
    ('default', '默认目标', 100, '{"physical": 0, "fire": 0, "cold": 0, "lightning": 0, "chaos": 0}', 0),
    ('boss_t16', 'T16 Boss', 100, '{"physical": 0.3, "fire": 0.3, "cold": 0.3, "lightning": 0.3, "chaos": 0.3}', 0.2);

-- ============================================================
-- Row Level Security (RLS) - 基础配置
-- ============================================================
ALTER TABLE tags_registry ENABLE ROW LEVEL SECURITY;
ALTER TABLE attributes_meta ENABLE ROW LEVEL SECURITY;
ALTER TABLE items_meta ENABLE ROW LEVEL SECURITY;
ALTER TABLE affixes ENABLE ROW LEVEL SECURITY;
ALTER TABLE unique_items ENABLE ROW LEVEL SECURITY;
ALTER TABLE unique_affixes ENABLE ROW LEVEL SECURITY;
ALTER TABLE skills ENABLE ROW LEVEL SECURITY;
ALTER TABLE skill_level_data ENABLE ROW LEVEL SECURITY;
ALTER TABLE skill_scaling_rules ENABLE ROW LEVEL SECURITY;
ALTER TABLE support_skill_modifiers ENABLE ROW LEVEL SECURITY;
ALTER TABLE hero_traits ENABLE ROW LEVEL SECURITY;
ALTER TABLE hero_memories ENABLE ROW LEVEL SECURITY;
ALTER TABLE pacts ENABLE ROW LEVEL SECURITY;
ALTER TABLE conversion_rules ENABLE ROW LEVEL SECURITY;
ALTER TABLE target_configs ENABLE ROW LEVEL SECURITY;

-- 公开读取策略（所有表允许匿名读取）
CREATE POLICY "Allow public read" ON tags_registry FOR SELECT USING (true);
CREATE POLICY "Allow public read" ON attributes_meta FOR SELECT USING (true);
CREATE POLICY "Allow public read" ON items_meta FOR SELECT USING (true);
CREATE POLICY "Allow public read" ON affixes FOR SELECT USING (true);
CREATE POLICY "Allow public read" ON unique_items FOR SELECT USING (true);
CREATE POLICY "Allow public read" ON unique_affixes FOR SELECT USING (true);
CREATE POLICY "Allow public read" ON skills FOR SELECT USING (true);
CREATE POLICY "Allow public read" ON skill_level_data FOR SELECT USING (true);
CREATE POLICY "Allow public read" ON skill_scaling_rules FOR SELECT USING (true);
CREATE POLICY "Allow public read" ON support_skill_modifiers FOR SELECT USING (true);
CREATE POLICY "Allow public read" ON hero_traits FOR SELECT USING (true);
CREATE POLICY "Allow public read" ON hero_memories FOR SELECT USING (true);
CREATE POLICY "Allow public read" ON pacts FOR SELECT USING (true);
CREATE POLICY "Allow public read" ON conversion_rules FOR SELECT USING (true);
CREATE POLICY "Allow public read" ON target_configs FOR SELECT USING (true);

