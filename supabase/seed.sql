-- ============================================================
-- TLI BD 决策辅助系统 - Seed Data
-- ============================================================
-- 注意：所有百分比数值存储为小数（0.15 = 15%）
-- ============================================================

-- ============================================================
-- 1. 标签注册表 Seed
-- ============================================================
INSERT INTO tags_registry (tag_key, category, display_name, parents, description) VALUES
-- 伤害类型标签
('Tag_Damage', 'Identity', '伤害', '{}', '所有伤害的根标签'),
('Tag_Physical', 'Identity', '物理', '{"Tag_Damage"}', '物理伤害'),
('Tag_Elemental', 'Identity', '元素', '{"Tag_Damage"}', '元素伤害的父标签'),
('Tag_Fire', 'Identity', '火焰', '{"Tag_Elemental"}', '火焰伤害'),
('Tag_Cold', 'Identity', '冰霜', '{"Tag_Elemental"}', '冰霜伤害'),
('Tag_Lightning', 'Identity', '闪电', '{"Tag_Elemental"}', '闪电伤害'),
('Tag_Chaos', 'Identity', '混沌', '{"Tag_Damage"}', '混沌伤害'),

-- 攻击类型标签
('Tag_Attack', 'Identity', '攻击', '{}', '攻击技能'),
('Tag_Melee', 'Identity', '近战', '{"Tag_Attack"}', '近战攻击'),
('Tag_Ranged', 'Identity', '远程', '{"Tag_Attack"}', '远程攻击'),
('Tag_Projectile', 'Identity', '投射物', '{}', '投射物技能'),
('Tag_Spell', 'Identity', '法术', '{}', '法术技能'),
('Tag_AOE', 'Identity', '范围', '{}', '范围效果'),

-- DOT标签
('Tag_DOT', 'Identity', '持续伤害', '{}', '持续伤害效果'),
('Tag_Ignite', 'Identity', '点燃', '{"Tag_DOT", "Tag_Fire"}', '火焰DOT'),
('Tag_Bleed', 'Identity', '流血', '{"Tag_DOT", "Tag_Physical"}', '物理DOT'),
('Tag_Poison', 'Identity', '中毒', '{"Tag_DOT", "Tag_Chaos"}', '混沌DOT'),

-- 武器标签
('Tag_Weapon', 'Identity', '武器', '{}', '武器类型'),
('Tag_Sword', 'Identity', '剑', '{"Tag_Weapon"}', '剑类武器'),
('Tag_Axe', 'Identity', '斧', '{"Tag_Weapon"}', '斧类武器'),
('Tag_Staff', 'Identity', '长杖', '{"Tag_Weapon"}', '法杖类武器'),
('Tag_Bow', 'Identity', '弓', '{"Tag_Weapon"}', '弓类武器'),
('Tag_Wand', 'Identity', '法杖', '{"Tag_Weapon"}', '短法杖'),

-- 装备类型标签
('Tag_OneHanded', 'Identity', '单手', '{}', '单手武器'),
('Tag_TwoHanded', 'Identity', '双手', '{}', '双手武器'),
('Tag_Shield', 'Identity', '盾牌', '{}', '盾牌'),

-- 护甲标签
('Tag_Armor', 'Identity', '护甲', '{}', '护甲装备'),
('Tag_Helmet', 'Identity', '头盔', '{"Tag_Armor"}', '头部护甲'),
('Tag_Chest', 'Identity', '胸甲', '{"Tag_Armor"}', '胸部护甲'),
('Tag_Gloves', 'Identity', '手套', '{"Tag_Armor"}', '手部护甲'),
('Tag_Boots', 'Identity', '鞋子', '{"Tag_Armor"}', '脚部护甲'),

-- 饰品标签
('Tag_Accessory', 'Identity', '饰品', '{}', '饰品装备'),
('Tag_Amulet', 'Identity', '项链', '{"Tag_Accessory"}', '颈部饰品'),
('Tag_Ring', 'Identity', '戒指', '{"Tag_Accessory"}', '手指饰品'),
('Tag_Belt', 'Identity', '腰带', '{"Tag_Accessory"}', '腰部饰品'),

-- 技能机制标签
('Tag_Chain', 'Identity', '连锁', '{}', '连锁弹射类技能'),
('Tag_Aura', 'Identity', '光环', '{}', '光环技能'),
('Tag_Minion', 'Identity', '召唤物', '{}', '召唤物'),
('Tag_Channeling', 'Identity', '吟唱', '{}', '吟唱技能'),
('Tag_Duration', 'Identity', '持续时间', '{}', '有持续时间的效果'),
('Tag_Crit', 'Identity', '暴击', '{}', '暴击相关'),
('Tag_Burst', 'Identity', '爆发', '{}', '爆发技能'),
('Tag_Support', 'Identity', '辅助', '{}', '辅助技能'),

-- 机制标签
('Mech_Blessing', 'Mechanic', '祝福', '{}', '祝福层数机制'),
('Mech_Charge_Frenzy', 'Mechanic', '狂乱球', '{}', '狂乱球充能'),
('Mech_Charge_Power', 'Mechanic', '能量球', '{}', '能量球充能'),
('Mech_Charge_Endurance', 'Mechanic', '耐力球', '{}', '耐力球充能'),
('Mech_Rage', 'Mechanic', '怒火', '{}', '怒火值'),
('Mech_Fortify', 'Mechanic', '护体', '{}', '护体层数'),

-- 规则标签
('Rule_Conversion', 'Rule', '伤害转化', '{}', '启用伤害转化计算'),
('Rule_Extra_As', 'Rule', '额外获得', '{}', '额外获得伤害'),
('Rule_Local', 'Rule', '局部属性', '{}', '标记为局部属性'),
('Rule_Global', 'Rule', '全局属性', '{}', '标记为全局属性'),
('Rule_Lucky', 'Rule', '幸运', '{}', '幸运判定（取两次最高）'),
('Rule_CannotCrit', 'Rule', '无法暴击', '{}', '禁止暴击'),
('Rule_Penetrate', 'Rule', '穿透', '{}', '抗性穿透'),

-- 状态标签
('Tag_State_Low_Life', 'State', '低血量', '{}', '生命值低于35%'),
('Tag_State_Full_Life', 'State', '满血量', '{}', '生命值100%'),
('Tag_State_Low_Mana', 'State', '低魔力', '{}', '魔力低于35%'),
('Tag_State_Full_Mana', 'State', '满魔力', '{}', '魔力100%'),
('Tag_State_Moving', 'State', '移动中', '{}', '角色正在移动'),
('Tag_State_Stationary', 'State', '静止', '{}', '角色静止不动'),
('Tag_State_Recently_Crit', 'State', '近期暴击', '{}', '近期造成暴击'),
('Tag_State_Recently_Killed', 'State', '近期击杀', '{}', '近期击杀敌人'),
('Tag_State_Enemy_Chilled', 'State', '敌人被冰缓', '{}', '目标处于冰缓状态'),
('Tag_State_Enemy_Frozen', 'State', '敌人被冻结', '{}', '目标处于冻结状态'),
('Tag_State_Enemy_Shocked', 'State', '敌人被感电', '{}', '目标处于感电状态'),
('Tag_State_Enemy_Ignited', 'State', '敌人被点燃', '{}', '目标处于点燃状态'),
('Tag_State_Enemy_Controlled', 'State', '敌人受控制', '{}', '目标处于控制状态');

-- ============================================================
-- 2. 属性元数据 Seed
-- ============================================================
INSERT INTO attributes_meta (attr_key, display_name, value_type, is_local, requirements, description) VALUES
-- 基础属性
('base.str', '力量', 'int', FALSE, '{}', '基础力量'),
('base.dex', '敏捷', 'int', FALSE, '{}', '基础敏捷'),
('base.int', '智慧', 'int', FALSE, '{}', '基础智慧'),
('base.life', '生命', 'int', FALSE, '{}', '基础生命值'),
('base.mana', '魔力', 'int', FALSE, '{}', '基础魔力值'),
('base.es', '能量护盾', 'int', FALSE, '{}', '基础能量护盾'),

-- 伤害基础
('dmg.phys.min', '物理伤害下限', 'float', TRUE, '{}', '武器物理伤害下限（局部）'),
('dmg.phys.max', '物理伤害上限', 'float', TRUE, '{}', '武器物理伤害上限（局部）'),
('dmg.fire.min', '火焰伤害下限', 'float', FALSE, '{}', '火焰伤害下限'),
('dmg.fire.max', '火焰伤害上限', 'float', FALSE, '{}', '火焰伤害上限'),
('dmg.cold.min', '冰霜伤害下限', 'float', FALSE, '{}', '冰霜伤害下限'),
('dmg.cold.max', '冰霜伤害上限', 'float', FALSE, '{}', '冰霜伤害上限'),
('dmg.lightning.min', '闪电伤害下限', 'float', FALSE, '{}', '闪电伤害下限'),
('dmg.lightning.max', '闪电伤害上限', 'float', FALSE, '{}', '闪电伤害上限'),
('dmg.chaos.min', '混沌伤害下限', 'float', FALSE, '{}', '混沌伤害下限'),
('dmg.chaos.max', '混沌伤害上限', 'float', FALSE, '{}', '混沌伤害上限'),

-- 暴击
('crit.chance', '暴击率', 'percent', FALSE, '{}', '暴击率（百分比）'),
('crit.dmg', '暴击伤害', 'percent', FALSE, '{}', '暴击伤害倍率'),
('crit.chance.local', '武器暴击率', 'percent', TRUE, '{}', '武器局部暴击率'),

-- 攻速
('speed.attack', '攻击速度', 'percent', FALSE, '{}', '攻击速度加成'),
('speed.cast', '施法速度', 'percent', FALSE, '{}', '施法速度加成'),
('speed.attack.local', '武器攻速', 'percent', TRUE, '{}', '武器局部攻击速度'),

-- Inc 修正（累加）
('mod.inc.dmg.all', '伤害增加', 'percent', FALSE, '{}', '所有伤害增加'),
('mod.inc.dmg.phys', '物理伤害增加', 'percent', FALSE, '{"Tag_Physical"}', '物理伤害增加'),
('mod.inc.dmg.fire', '火焰伤害增加', 'percent', FALSE, '{"Tag_Fire"}', '火焰伤害增加'),
('mod.inc.dmg.cold', '冰霜伤害增加', 'percent', FALSE, '{"Tag_Cold"}', '冰霜伤害增加'),
('mod.inc.dmg.lightning', '闪电伤害增加', 'percent', FALSE, '{"Tag_Lightning"}', '闪电伤害增加'),
('mod.inc.dmg.chaos', '混沌伤害增加', 'percent', FALSE, '{"Tag_Chaos"}', '混沌伤害增加'),
('mod.inc.dmg.elemental', '元素伤害增加', 'percent', FALSE, '{"Tag_Elemental"}', '元素伤害增加'),
('mod.inc.dmg.spell', '法术伤害增加', 'percent', FALSE, '{"Tag_Spell"}', '法术伤害增加'),
('mod.inc.dmg.attack', '攻击伤害增加', 'percent', FALSE, '{"Tag_Attack"}', '攻击伤害增加'),
('mod.inc.dmg.melee', '近战伤害增加', 'percent', FALSE, '{"Tag_Melee"}', '近战伤害增加'),
('mod.inc.dmg.projectile', '投射物伤害增加', 'percent', FALSE, '{"Tag_Projectile"}', '投射物伤害增加'),
('mod.inc.dmg.aoe', '范围伤害增加', 'percent', FALSE, '{"Tag_AOE"}', '范围伤害增加'),
('mod.inc.dmg.dot', '持续伤害增加', 'percent', FALSE, '{"Tag_DOT"}', '持续伤害增加'),

-- More 修正（相乘）
('mod.more.dmg.all', '总伤害提高', 'percent', FALSE, '{}', '总伤害提高（More乘区）'),
('mod.more.dmg.phys', '物理总伤害提高', 'percent', FALSE, '{"Tag_Physical"}', '物理总伤害提高'),
('mod.more.dmg.fire', '火焰总伤害提高', 'percent', FALSE, '{"Tag_Fire"}', '火焰总伤害提高'),
('mod.more.dmg.cold', '冰霜总伤害提高', 'percent', FALSE, '{"Tag_Cold"}', '冰霜总伤害提高'),
('mod.more.dmg.elemental', '元素总伤害提高', 'percent', FALSE, '{"Tag_Elemental"}', '元素总伤害提高'),

-- 穿透
('pen.phys', '物理穿透', 'percent', FALSE, '{"Tag_Physical"}', '物理抗性穿透'),
('pen.fire', '火焰穿透', 'percent', FALSE, '{"Tag_Fire"}', '火焰抗性穿透'),
('pen.cold', '冰霜穿透', 'percent', FALSE, '{"Tag_Cold"}', '冰霜抗性穿透'),
('pen.lightning', '闪电穿透', 'percent', FALSE, '{"Tag_Lightning"}', '闪电抗性穿透'),
('pen.chaos', '混沌穿透', 'percent', FALSE, '{"Tag_Chaos"}', '混沌抗性穿透'),
('pen.elemental', '元素穿透', 'percent', FALSE, '{"Tag_Elemental"}', '元素抗性穿透'),

-- 转化
('conv.phys_to_fire', '物理转火焰', 'percent', FALSE, '{}', '物理伤害转化为火焰'),
('conv.phys_to_cold', '物理转冰霜', 'percent', FALSE, '{}', '物理伤害转化为冰霜'),
('conv.phys_to_lightning', '物理转闪电', 'percent', FALSE, '{}', '物理伤害转化为闪电'),
('conv.cold_to_fire', '冰霜转火焰', 'percent', FALSE, '{}', '冰霜伤害转化为火焰'),
('conv.lightning_to_cold', '闪电转冰霜', 'percent', FALSE, '{}', '闪电伤害转化为冰霜'),

-- 额外获得
('extra.phys_as_fire', '物理额外火焰', 'percent', FALSE, '{}', '物理伤害的百分比额外获得为火焰'),
('extra.phys_as_cold', '物理额外冰霜', 'percent', FALSE, '{}', '物理伤害的百分比额外获得为冰霜'),
('extra.phys_as_lightning', '物理额外闪电', 'percent', FALSE, '{}', '物理伤害的百分比额外获得为闪电'),
('extra.fire_as_chaos', '火焰额外混沌', 'percent', FALSE, '{}', '火焰伤害的百分比额外获得为混沌'),

-- 防御
('def.armor', '护甲', 'int', FALSE, '{}', '护甲值'),
('def.evasion', '闪避', 'int', FALSE, '{}', '闪避值'),
('def.block', '格挡率', 'percent', FALSE, '{}', '攻击格挡率'),
('def.spell_block', '法术格挡率', 'percent', FALSE, '{}', '法术格挡率'),

-- 抗性
('res.phys', '物理抗性', 'percent', FALSE, '{}', '物理伤害减免'),
('res.fire', '火焰抗性', 'percent', FALSE, '{}', '火焰抗性'),
('res.cold', '冰霜抗性', 'percent', FALSE, '{}', '冰霜抗性'),
('res.lightning', '闪电抗性', 'percent', FALSE, '{}', '闪电抗性'),
('res.chaos', '混沌抗性', 'percent', FALSE, '{}', '混沌抗性'),

-- 命中
('acc.rating', '命中值', 'int', FALSE, '{}', '命中值'),
('acc.chance', '命中率', 'percent', FALSE, '{}', '命中率');

-- ============================================================
-- 3. 装备基底 Seed
-- ============================================================
INSERT INTO items_meta (base_type, display_name, slot, is_two_handed, implicit_stats, tags) VALUES
-- 单手剑
('sword_1h_phys', '破敌单手剑', 'weapon_main', FALSE, 
 '{"dmg.phys.min": 15, "dmg.phys.max": 30, "crit.chance.local": 0.05, "speed.attack.local": 1.3}',
 '{"Tag_Weapon", "Tag_Sword", "Tag_OneHanded"}'),

-- 双手剑
('sword_2h_phys', '斩魂巨剑', 'weapon_main', TRUE,
 '{"dmg.phys.min": 40, "dmg.phys.max": 80, "crit.chance.local": 0.05, "speed.attack.local": 1.0}',
 '{"Tag_Weapon", "Tag_Sword", "Tag_TwoHanded"}'),

-- 法杖
('wand_fire', '烈焰法杖', 'weapon_main', FALSE,
 '{"dmg.fire.min": 10, "dmg.fire.max": 25, "crit.chance.local": 0.07, "speed.attack.local": 1.5}',
 '{"Tag_Weapon", "Tag_Wand", "Tag_OneHanded"}'),

-- 长杖
('staff_2h_spell', '奥术长杖', 'weapon_main', TRUE,
 '{"mod.inc.dmg.spell": 0.20, "crit.chance.local": 0.06, "speed.attack.local": 1.1}',
 '{"Tag_Weapon", "Tag_Staff", "Tag_TwoHanded"}'),

-- 弓
('bow_2h', '迅捷长弓', 'weapon_main', TRUE,
 '{"dmg.phys.min": 20, "dmg.phys.max": 50, "crit.chance.local": 0.05, "speed.attack.local": 1.2}',
 '{"Tag_Weapon", "Tag_Bow", "Tag_TwoHanded"}'),

-- 盾牌
('shield_str', '铁壁圆盾', 'weapon_off', FALSE,
 '{"def.armor": 100, "def.block": 0.25}',
 '{"Tag_Shield"}'),

-- 头盔
('helmet_armor', '精钢头盔', 'helmet', FALSE,
 '{"def.armor": 50}',
 '{"Tag_Armor", "Tag_Helmet"}'),

-- 胸甲
('chest_armor', '重型铠甲', 'chest', FALSE,
 '{"def.armor": 150}',
 '{"Tag_Armor", "Tag_Chest"}'),

-- 手套
('gloves_armor', '铁手套', 'gloves', FALSE,
 '{"def.armor": 30}',
 '{"Tag_Armor", "Tag_Gloves"}'),

-- 鞋子
('boots_armor', '铁靴', 'boots', FALSE,
 '{"def.armor": 40}',
 '{"Tag_Armor", "Tag_Boots"}'),

-- 项链
('amulet_base', '银质项链', 'amulet', FALSE,
 '{}',
 '{"Tag_Accessory", "Tag_Amulet"}'),

-- 戒指
('ring_base', '银质戒指', 'ring', FALSE,
 '{}',
 '{"Tag_Accessory", "Tag_Ring"}'),

-- 腰带
('belt_base', '皮革腰带', 'belt', FALSE,
 '{}',
 '{"Tag_Accessory", "Tag_Belt"}');

-- ============================================================
-- 4. 打造词缀 Seed
-- ============================================================
INSERT INTO affixes (affix_group, tier, template_text, min_val, max_val, stats, tags, slot_restrictions) VALUES
-- 物理伤害增加
('inc_phys_dmg', 1, '增加 {0}% 物理伤害', 0.15, 0.20, '{"mod.inc.dmg.phys": "{0}"}', '{"Tag_Physical"}', '{}'),
('inc_phys_dmg', 2, '增加 {0}% 物理伤害', 0.21, 0.30, '{"mod.inc.dmg.phys": "{0}"}', '{"Tag_Physical"}', '{}'),
('inc_phys_dmg', 3, '增加 {0}% 物理伤害', 0.31, 0.40, '{"mod.inc.dmg.phys": "{0}"}', '{"Tag_Physical"}', '{}'),

-- 火焰伤害增加
('inc_fire_dmg', 1, '增加 {0}% 火焰伤害', 0.15, 0.20, '{"mod.inc.dmg.fire": "{0}"}', '{"Tag_Fire"}', '{}'),
('inc_fire_dmg', 2, '增加 {0}% 火焰伤害', 0.21, 0.30, '{"mod.inc.dmg.fire": "{0}"}', '{"Tag_Fire"}', '{}'),
('inc_fire_dmg', 3, '增加 {0}% 火焰伤害', 0.31, 0.45, '{"mod.inc.dmg.fire": "{0}"}', '{"Tag_Fire"}', '{}'),

-- 冰霜伤害增加
('inc_cold_dmg', 1, '增加 {0}% 冰霜伤害', 0.15, 0.20, '{"mod.inc.dmg.cold": "{0}"}', '{"Tag_Cold"}', '{}'),
('inc_cold_dmg', 2, '增加 {0}% 冰霜伤害', 0.21, 0.30, '{"mod.inc.dmg.cold": "{0}"}', '{"Tag_Cold"}', '{}'),
('inc_cold_dmg', 3, '增加 {0}% 冰霜伤害', 0.31, 0.45, '{"mod.inc.dmg.cold": "{0}"}', '{"Tag_Cold"}', '{}'),

-- 元素伤害增加
('inc_ele_dmg', 1, '增加 {0}% 元素伤害', 0.10, 0.15, '{"mod.inc.dmg.elemental": "{0}"}', '{"Tag_Elemental"}', '{}'),
('inc_ele_dmg', 2, '增加 {0}% 元素伤害', 0.16, 0.25, '{"mod.inc.dmg.elemental": "{0}"}', '{"Tag_Elemental"}', '{}'),
('inc_ele_dmg', 3, '增加 {0}% 元素伤害', 0.26, 0.35, '{"mod.inc.dmg.elemental": "{0}"}', '{"Tag_Elemental"}', '{}'),

-- 暴击率
('crit_chance', 1, '增加 {0}% 暴击率', 0.15, 0.25, '{"crit.chance": "{0}"}', '{"Tag_Crit"}', '{}'),
('crit_chance', 2, '增加 {0}% 暴击率', 0.26, 0.40, '{"crit.chance": "{0}"}', '{"Tag_Crit"}', '{}'),
('crit_chance', 3, '增加 {0}% 暴击率', 0.41, 0.60, '{"crit.chance": "{0}"}', '{"Tag_Crit"}', '{}'),

-- 暴击伤害
('crit_dmg', 1, '增加 {0}% 暴击伤害', 0.15, 0.25, '{"crit.dmg": "{0}"}', '{"Tag_Crit"}', '{}'),
('crit_dmg', 2, '增加 {0}% 暴击伤害', 0.26, 0.40, '{"crit.dmg": "{0}"}', '{"Tag_Crit"}', '{}'),
('crit_dmg', 3, '增加 {0}% 暴击伤害', 0.41, 0.55, '{"crit.dmg": "{0}"}', '{"Tag_Crit"}', '{}'),

-- 攻击速度
('attack_speed', 1, '增加 {0}% 攻击速度', 0.05, 0.08, '{"speed.attack": "{0}"}', '{}', '{}'),
('attack_speed', 2, '增加 {0}% 攻击速度', 0.09, 0.12, '{"speed.attack": "{0}"}', '{}', '{}'),
('attack_speed', 3, '增加 {0}% 攻击速度', 0.13, 0.18, '{"speed.attack": "{0}"}', '{}', '{}'),

-- 施法速度
('cast_speed', 1, '增加 {0}% 施法速度', 0.05, 0.08, '{"speed.cast": "{0}"}', '{"Tag_Spell"}', '{}'),
('cast_speed', 2, '增加 {0}% 施法速度', 0.09, 0.12, '{"speed.cast": "{0}"}', '{"Tag_Spell"}', '{}'),
('cast_speed', 3, '增加 {0}% 施法速度', 0.13, 0.18, '{"speed.cast": "{0}"}', '{"Tag_Spell"}', '{}'),

-- 生命
('flat_life', 1, '增加 {0} 最大生命', 30, 50, '{"base.life": "{0}"}', '{}', '{}'),
('flat_life', 2, '增加 {0} 最大生命', 51, 80, '{"base.life": "{0}"}', '{}', '{}'),
('flat_life', 3, '增加 {0} 最大生命', 81, 120, '{"base.life": "{0}"}', '{}', '{}'),

-- 抗性
('res_fire', 1, '增加 {0}% 火焰抗性', 0.10, 0.15, '{"res.fire": "{0}"}', '{}', '{}'),
('res_fire', 2, '增加 {0}% 火焰抗性', 0.16, 0.25, '{"res.fire": "{0}"}', '{}', '{}'),
('res_fire', 3, '增加 {0}% 火焰抗性', 0.26, 0.35, '{"res.fire": "{0}"}', '{}', '{}'),

('res_cold', 1, '增加 {0}% 冰霜抗性', 0.10, 0.15, '{"res.cold": "{0}"}', '{}', '{}'),
('res_cold', 2, '增加 {0}% 冰霜抗性', 0.16, 0.25, '{"res.cold": "{0}"}', '{}', '{}'),
('res_cold', 3, '增加 {0}% 冰霜抗性', 0.26, 0.35, '{"res.cold": "{0}"}', '{}', '{}'),

('res_lightning', 1, '增加 {0}% 闪电抗性', 0.10, 0.15, '{"res.lightning": "{0}"}', '{}', '{}'),
('res_lightning', 2, '增加 {0}% 闪电抗性', 0.16, 0.25, '{"res.lightning": "{0}"}', '{}', '{}'),
('res_lightning', 3, '增加 {0}% 闪电抗性', 0.26, 0.35, '{"res.lightning": "{0}"}', '{}', '{}'),

-- 穿透
('pen_fire', 1, '火焰伤害穿透 {0}% 火焰抗性', 0.05, 0.08, '{"pen.fire": "{0}"}', '{"Tag_Fire"}', '{}'),
('pen_fire', 2, '火焰伤害穿透 {0}% 火焰抗性', 0.09, 0.12, '{"pen.fire": "{0}"}', '{"Tag_Fire"}', '{}'),
('pen_cold', 1, '冰霜伤害穿透 {0}% 冰霜抗性', 0.05, 0.08, '{"pen.cold": "{0}"}', '{"Tag_Cold"}', '{}'),
('pen_cold', 2, '冰霜伤害穿透 {0}% 冰霜抗性', 0.09, 0.12, '{"pen.cold": "{0}"}', '{"Tag_Cold"}', '{}'),

-- 物理转火焰
('conv_phys_fire', 1, '{0}% 物理伤害转化为火焰伤害', 0.25, 0.25, '{"conv.phys_to_fire": "{0}"}', '{"Rule_Conversion"}', '{}'),
('conv_phys_fire', 2, '{0}% 物理伤害转化为火焰伤害', 0.50, 0.50, '{"conv.phys_to_fire": "{0}"}', '{"Rule_Conversion"}', '{}'),

-- 额外获得
('extra_phys_fire', 1, '获得 {0}% 物理伤害的额外火焰伤害', 0.10, 0.15, '{"extra.phys_as_fire": "{0}"}', '{"Rule_Extra_As"}', '{}'),
('extra_phys_fire', 2, '获得 {0}% 物理伤害的额外火焰伤害', 0.16, 0.25, '{"extra.phys_as_fire": "{0}"}', '{"Rule_Extra_As"}', '{}');

-- ============================================================
-- 5. 技能 Seed
-- ============================================================
INSERT INTO skills (id, display_name, skill_type, damage_type, is_attack, base_damage, base_time, cooldown, mana_cost, effectiveness, growth_table, tags, stats) VALUES
-- 火球术
('skill_fireball', '火球术', 'active', 'fire', FALSE,
 '{"dmg.fire.min": 15, "dmg.fire.max": 25}',
 0.8, NULL, 10, 1.0,
 '{"1": {"dmg.fire.min": 15, "dmg.fire.max": 25}, "10": {"dmg.fire.min": 50, "dmg.fire.max": 80}, "20": {"dmg.fire.min": 120, "dmg.fire.max": 180}}',
 '{"Tag_Spell", "Tag_Fire", "Tag_Projectile", "Tag_AOE"}',
 '{}'),

-- 冰矛
('skill_ice_spear', '冰矛', 'active', 'cold', FALSE,
 '{"dmg.cold.min": 12, "dmg.cold.max": 20}',
 0.7, NULL, 8, 0.9,
 '{"1": {"dmg.cold.min": 12, "dmg.cold.max": 20}, "10": {"dmg.cold.min": 45, "dmg.cold.max": 70}, "20": {"dmg.cold.min": 100, "dmg.cold.max": 160}}',
 '{"Tag_Spell", "Tag_Cold", "Tag_Projectile"}',
 '{"crit.chance": 0.02}'),

-- 闪电打击
('skill_lightning_strike', '闪电打击', 'active', 'lightning', TRUE,
 '{}',
 0.9, NULL, 12, 1.1,
 '{}',
 '{"Tag_Attack", "Tag_Melee", "Tag_Lightning", "Tag_AOE"}',
 '{"extra.phys_as_lightning": 0.5}'),

-- 重击
('skill_heavy_strike', '重击', 'active', 'physical', TRUE,
 '{}',
 1.0, NULL, 6, 1.5,
 '{}',
 '{"Tag_Attack", "Tag_Melee", "Tag_Physical"}',
 '{"mod.more.dmg.phys": 0.50}'),

-- 旋风斩
('skill_cyclone', '旋风斩', 'active', 'physical', TRUE,
 '{}',
 0.3, NULL, 2, 0.5,
 '{}',
 '{"Tag_Attack", "Tag_Melee", "Tag_Physical", "Tag_AOE", "Tag_Channeling"}',
 '{}'),

-- 闪电链 (完整技能等级示例)
('skill_chain_lightning', '闪电链', 'active', 'lightning', FALSE,
 '{"dmg.lightning.min": 1, "dmg.lightning.max": 23}',
 0.65, NULL, 8, 1.63,
 '{}',
 '{"Tag_Spell", "Tag_Lightning", "Tag_Chain"}',
 '{}'),

-- 辅助：狂雷
('support_frenzy', '狂雷', 'support', NULL, FALSE,
 '{}',
 0, NULL, 0, 1.3,
 '{}',
 '{"Tag_Support", "Tag_Burst"}',
 '{"mod.more.dmg.all": 0.30}'),

-- 辅助：增幅
('support_empower', '增幅', 'support', NULL, FALSE,
 '{}',
 0, NULL, 0, 1.4,
 '{}',
 '{"Tag_Support"}',
 '{"mod.more.dmg.all": 0.20}'),

-- 辅助：快速施法
('support_faster_casting', '快速施法', 'support', NULL, FALSE,
 '{}',
 0, NULL, 0, 1.2,
 '{}',
 '{"Tag_Support", "Tag_Spell"}',
 '{"speed.cast": 0.40}');

-- 辅助技能加成
INSERT INTO support_skill_modifiers (skill_id, level, mana_multiplier, stats, injected_tags, requirements) VALUES
('support_frenzy', 1, 1.30, '{"mod.more.dmg.all": 0.20}', '{"Tag_Burst"}', '{}'),
('support_frenzy', 10, 1.30, '{"mod.more.dmg.all": 0.30}', '{"Tag_Burst"}', '{}'),
('support_frenzy', 20, 1.30, '{"mod.more.dmg.all": 0.40}', '{"Tag_Burst"}', '{}'),
('support_empower', 1, 1.40, '{"mod.more.dmg.all": 0.15}', '{}', '{}'),
('support_empower', 10, 1.40, '{"mod.more.dmg.all": 0.20}', '{}', '{}'),
('support_empower', 20, 1.40, '{"mod.more.dmg.all": 0.25}', '{}', '{}'),
('support_faster_casting', 1, 1.20, '{"speed.cast": 0.30}', '{}', '{"Tag_Spell"}'),
('support_faster_casting', 10, 1.20, '{"speed.cast": 0.40}', '{}', '{"Tag_Spell"}'),
('support_faster_casting', 20, 1.20, '{"speed.cast": 0.50}', '{}', '{"Tag_Spell"}');

-- ============================================================
-- 5.1 技能等级成长表 Seed (闪电链完整数据)
-- ============================================================
-- 闪电链 1-20级数据 (21级及以上使用缩放规则计算)
INSERT INTO skill_level_data (skill_id, level, effectiveness, base_damage, mana_cost, base_time, extra_effects) VALUES
-- 1-20级：伤害倍率和基础点伤逐级提升
('skill_chain_lightning', 1, 1.63, '{"dmg.lightning.min": 1, "dmg.lightning.max": 23}', 8, 0.65, '{"chain_count": 2}'),
('skill_chain_lightning', 2, 1.64, '{"dmg.lightning.min": 1, "dmg.lightning.max": 26}', 8, 0.65, '{"chain_count": 2}'),
('skill_chain_lightning', 3, 1.64, '{"dmg.lightning.min": 2, "dmg.lightning.max": 34}', 9, 0.65, '{"chain_count": 2}'),
('skill_chain_lightning', 4, 1.65, '{"dmg.lightning.min": 2, "dmg.lightning.max": 47}', 9, 0.65, '{"chain_count": 2}'),
('skill_chain_lightning', 5, 1.66, '{"dmg.lightning.min": 3, "dmg.lightning.max": 60}', 10, 0.65, '{"chain_count": 2}'),
('skill_chain_lightning', 6, 1.66, '{"dmg.lightning.min": 4, "dmg.lightning.max": 74}', 10, 0.65, '{"chain_count": 2}'),
('skill_chain_lightning', 7, 1.67, '{"dmg.lightning.min": 4, "dmg.lightning.max": 84}', 11, 0.65, '{"chain_count": 2}'),
('skill_chain_lightning', 8, 1.68, '{"dmg.lightning.min": 6, "dmg.lightning.max": 106}', 11, 0.65, '{"chain_count": 2}'),
('skill_chain_lightning', 9, 1.69, '{"dmg.lightning.min": 7, "dmg.lightning.max": 138}', 12, 0.65, '{"chain_count": 2}'),
('skill_chain_lightning', 10, 1.69, '{"dmg.lightning.min": 9, "dmg.lightning.max": 168}', 12, 0.65, '{"chain_count": 2}'),
('skill_chain_lightning', 11, 1.70, '{"dmg.lightning.min": 11, "dmg.lightning.max": 217}', 13, 0.65, '{"chain_count": 2}'),
('skill_chain_lightning', 12, 1.71, '{"dmg.lightning.min": 13, "dmg.lightning.max": 256}', 13, 0.65, '{"chain_count": 2}'),
('skill_chain_lightning', 13, 1.72, '{"dmg.lightning.min": 16, "dmg.lightning.max": 311}', 14, 0.65, '{"chain_count": 2}'),
('skill_chain_lightning', 14, 1.73, '{"dmg.lightning.min": 19, "dmg.lightning.max": 367}', 14, 0.65, '{"chain_count": 2}'),
('skill_chain_lightning', 15, 1.73, '{"dmg.lightning.min": 23, "dmg.lightning.max": 437}', 15, 0.65, '{"chain_count": 2}'),
('skill_chain_lightning', 16, 1.74, '{"dmg.lightning.min": 32, "dmg.lightning.max": 617}', 15, 0.65, '{"chain_count": 2}'),
('skill_chain_lightning', 17, 1.74, '{"dmg.lightning.min": 38, "dmg.lightning.max": 729}', 16, 0.65, '{"chain_count": 2}'),
('skill_chain_lightning', 18, 1.75, '{"dmg.lightning.min": 53, "dmg.lightning.max": 1009}', 16, 0.65, '{"chain_count": 2}'),
('skill_chain_lightning', 19, 1.76, '{"dmg.lightning.min": 80, "dmg.lightning.max": 1512}', 17, 0.65, '{"chain_count": 2}'),
('skill_chain_lightning', 20, 1.77, '{"dmg.lightning.min": 95, "dmg.lightning.max": 1811}', 17, 0.65, '{"chain_count": 2}');

-- 火球术等级数据示例 (1, 10, 20级)
INSERT INTO skill_level_data (skill_id, level, effectiveness, base_damage, mana_cost, extra_effects) VALUES
('skill_fireball', 1, 1.00, '{"dmg.fire.min": 15, "dmg.fire.max": 25}', 10, '{"aoe_radius": 1.0}'),
('skill_fireball', 10, 1.10, '{"dmg.fire.min": 50, "dmg.fire.max": 80}', 14, '{"aoe_radius": 1.2}'),
('skill_fireball', 20, 1.20, '{"dmg.fire.min": 120, "dmg.fire.max": 180}', 18, '{"aoe_radius": 1.5}');

-- ============================================================
-- 6. 传奇装备 Seed
-- ============================================================
INSERT INTO unique_items (id, display_name, base_type, item_type, slot, level_req, flavor_text, tags) VALUES
('unique_sword_fire_001', '炎狱之刃', 'sword_1h_phys', 'weapon', 'weapon_main', 60,
 '在深渊的烈焰中锻造，只有最勇敢的战士才能驾驭它的力量。',
 '{"Tag_Weapon", "Tag_Sword", "Tag_OneHanded", "Tag_Fire"}'),

('unique_staff_spell_001', '星陨法杖', 'staff_2h_spell', 'weapon', 'weapon_main', 65,
 '据说这根法杖是从天上坠落的流星核心制成。',
 '{"Tag_Weapon", "Tag_Staff", "Tag_TwoHanded"}'),

('unique_chest_life_001', '不屈胸甲', 'chest_armor', 'armor', 'chest', 50,
 '穿戴者的意志永不屈服。',
 '{"Tag_Armor", "Tag_Chest"}');

-- 传奇装备词缀
INSERT INTO unique_affixes (item_id, tier, line_index, variant_type, template_text, min_val, max_val, stats, tags, is_implicit) VALUES
-- 炎狱之刃
('unique_sword_fire_001', 1, 0, 'base', '增加 {0}-{1} 物理伤害', 20, 35, '{"dmg.phys.min": "{0}", "dmg.phys.max": "{1}"}', '{}', TRUE),
('unique_sword_fire_001', 1, 1, 'base', '增加 {0}% 火焰伤害', 0.30, 0.50, '{"mod.inc.dmg.fire": "{0}"}', '{"Tag_Fire"}', FALSE),
('unique_sword_fire_001', 1, 2, 'base', '获得 {0}% 物理伤害的额外火焰伤害', 0.15, 0.25, '{"extra.phys_as_fire": "{0}"}', '{"Rule_Extra_As"}', FALSE),
('unique_sword_fire_001', 1, 3, 'base', '击中点燃敌人', NULL, NULL, '{}', '{"Tag_Ignite"}', FALSE),
-- 侵蚀变体
('unique_sword_fire_001', 1, 1, 'corrupted', '增加 {0}% 火焰伤害', 0.40, 0.60, '{"mod.inc.dmg.fire": "{0}"}', '{"Tag_Fire"}', FALSE),
('unique_sword_fire_001', 1, 2, 'corrupted', '获得 {0}% 物理伤害的额外火焰伤害', 0.20, 0.35, '{"extra.phys_as_fire": "{0}"}', '{"Rule_Extra_As"}', FALSE),

-- 星陨法杖
('unique_staff_spell_001', 1, 0, 'base', '增加 {0}% 法术伤害', 0.25, 0.35, '{"mod.inc.dmg.spell": "{0}"}', '{"Tag_Spell"}', TRUE),
('unique_staff_spell_001', 1, 1, 'base', '增加 {0}% 暴击率', 0.50, 0.80, '{"crit.chance": "{0}"}', '{"Tag_Crit"}', FALSE),
('unique_staff_spell_001', 1, 2, 'base', '增加 {0}% 施法速度', 0.10, 0.15, '{"speed.cast": "{0}"}', '{"Tag_Spell"}', FALSE),
('unique_staff_spell_001', 1, 3, 'base', '法术有 {0}% 几率造成双倍伤害', 0.08, 0.12, '{}', '{"Tag_Spell"}', FALSE),

-- 不屈胸甲
('unique_chest_life_001', 1, 0, 'base', '增加 {0} 最大生命', 80, 120, '{"base.life": "{0}"}', '{}', TRUE),
('unique_chest_life_001', 1, 1, 'base', '增加 {0}% 最大生命', 0.08, 0.12, '{}', '{}', FALSE),
('unique_chest_life_001', 1, 2, 'base', '增加 {0}% 全部元素抗性', 0.10, 0.15, '{"res.fire": "{0}", "res.cold": "{0}", "res.lightning": "{0}"}', '{}', FALSE),
('unique_chest_life_001', 1, 3, 'base', '受到的物理伤害降低 {0}%', 0.05, 0.08, '{}', '{}', FALSE);

-- ============================================================
-- 7. 英雄特性 Seed (占位)
-- ============================================================
INSERT INTO hero_traits (id, display_name, hero_class, trait_type, position, stats, tags) VALUES
('trait_berserker_1', '狂战士之怒', 'berserker', 'keystone',
 '{"x": 0, "y": 0}',
 '{"mod.more.dmg.all": 0.10, "mod.inc.dmg.melee": 0.20}',
 '{"Tag_Melee", "Tag_Attack"}'),

('trait_mage_1', '奥术精通', 'mage', 'keystone',
 '{"x": 0, "y": 0}',
 '{"mod.inc.dmg.spell": 0.25, "speed.cast": 0.10}',
 '{"Tag_Spell"}'),

('trait_ranger_1', '精准射击', 'ranger', 'keystone',
 '{"x": 0, "y": 0}',
 '{"crit.chance": 0.30, "mod.inc.dmg.projectile": 0.15}',
 '{"Tag_Projectile", "Tag_Ranged"}');

-- ============================================================
-- 8. 英雄追忆 Seed (占位)
-- ============================================================
INSERT INTO hero_memories (id, display_name, hero_class, memory_type, slot, tier, stats, special_effect, tags) VALUES
('memory_fire_1', '烈焰之忆', NULL, 'damage', 1, 1,
 '{"mod.inc.dmg.fire": 0.15, "pen.fire": 0.05}',
 '火焰技能有10%几率造成双倍点燃伤害',
 '{"Tag_Fire"}'),

('memory_crit_1', '致命之忆', NULL, 'offense', 2, 1,
 '{"crit.chance": 0.20, "crit.dmg": 0.15}',
 '暴击时有5%几率获得一层狂乱球',
 '{"Tag_Crit"}'),

('memory_defense_1', '坚韧之忆', NULL, 'defense', 3, 1,
 '{"base.life": 50, "def.armor": 100}',
 '受到致命伤害时有10%几率保留1点生命',
 '{}');

-- ============================================================
-- 9. 石板 Seed (占位)
-- ============================================================
INSERT INTO pacts (id, display_name, pact_type, tier, stats, tags) VALUES
('pact_fire_1', '炎狱契约', 'offense', 1,
 '{"mod.inc.dmg.fire": 0.20, "pen.fire": 0.05}',
 '{"Tag_Fire"}'),

('pact_cold_1', '寒霜契约', 'offense', 1,
 '{"mod.inc.dmg.cold": 0.20, "pen.cold": 0.05}',
 '{"Tag_Cold"}'),

('pact_life_1', '生命契约', 'defense', 1,
 '{"base.life": 100}',
 '{}');

