/**
 * TLI Core TypeScript Bindings
 * 
 * 这些类型由 ts-rs 自动生成，与 Rust 结构体保持同步
 */

// ============================================================
// 输入类型
// ============================================================

/** 计算器主输入 */
export interface CalculatorInput {
  /** 动态上下文标志 */
  context_flags: Record<string, boolean>;
  /** 动态上下文数值 */
  context_values: Record<string, number>;
  /** 目标配置 */
  target_config: TargetConfig;
  /** 装备列表 */
  items: ItemData[];
  /** 主技能 */
  active_skill: SkillData;
  /** 辅助技能列表 */
  support_skills: SkillData[];
  /** 全局属性覆盖 */
  global_overrides: Record<string, number>;
  /** 预览槽位 */
  preview_slot?: PreviewSlot;
}

/** 预览槽位 */
export interface PreviewSlot {
  slot_type: SlotType;
  item: ItemData;
}

/** 目标配置 */
export interface TargetConfig {
  /** 目标等级 */
  level: number;
  /** 防御常数 */
  defense_constant: number;
  /** 抗性映射 */
  resistances: Record<string, number>;
  /** 通用减伤 */
  generic_dr: number;
  /** 护甲值 */
  armor: number;
  /** 闪避值 */
  evasion: number;
}

/** 槽位类型 */
export type SlotType =
  | 'weapon_main'
  | 'weapon_off'
  | 'helmet'
  | 'chest'
  | 'gloves'
  | 'boots'
  | 'amulet'
  | 'ring1'
  | 'ring2'
  | 'belt'
  | 'memory1'
  | 'memory2'
  | 'memory3'
  | 'memory4'
  | 'memory5'
  | 'memory6';

// ============================================================
// 装备类型
// ============================================================

/** 装备数据 */
export interface ItemData {
  /** 唯一 ID */
  id: string;
  /** 基底类型 */
  base_type: string;
  /** 槽位 */
  slot: SlotType;
  /** 是否双手 */
  is_two_handed: boolean;
  /** 固有属性 */
  implicit_stats: Record<string, number>;
  /** 词缀列表 */
  affixes: AffixData[];
  /** 标签 */
  tags: string[];
  /** 是否侵蚀 */
  is_corrupted: boolean;
}

/** 词缀数据 */
export interface AffixData {
  /** ID */
  id: string;
  /** 词缀组 */
  group: string;
  /** 当前数值 */
  value: number;
  /** 属性效果 */
  stats: Record<string, number>;
  /** 标签 */
  tags: string[];
  /** 生效条件 */
  requirements: string[];
  /** 是否局部 */
  is_local: boolean;
}

// ============================================================
// 技能类型
// ============================================================

/** 技能数据 */
export interface SkillData {
  /** ID */
  id: string;
  /** 类型 */
  skill_type: SkillType;
  /** 伤害类型 */
  damage_type?: string;
  /** 是否攻击 */
  is_attack: boolean;
  /** 等级 */
  level: number;
  /** 基础伤害 */
  base_damage: Record<string, number>;
  /** 基础时间 */
  base_time: number;
  /** 冷却 */
  cooldown?: number;
  /** 魔力消耗 */
  mana_cost: number;
  /** 有效系数 */
  effectiveness: number;
  /** 标签 */
  tags: string[];
  /** 自带属性 */
  stats: Record<string, number>;
  /** 注入标签 */
  injected_tags: string[];
  /** 魔力倍率 */
  mana_multiplier: number;
}

/** 技能类型 */
export type SkillType = 'active' | 'support' | 'aura';

// ============================================================
// 输出类型
// ============================================================

/** 计算结果 */
export interface CalculatorOutput {
  /** 理论 DPS */
  dps_theoretical: number;
  /** 有效 DPS */
  dps_effective: number;
  /** 单次命中 */
  hit_damage: number;
  /** 攻击速率 */
  rate: number;
  /** 暴击率 */
  crit_chance: number;
  /** 暴击伤害 */
  crit_multiplier: number;
  /** 命中率 */
  hit_chance: number;
  /** EHP 系列 */
  ehp_series: EhpSeries;
  /** 伤害构成 */
  damage_breakdown: DamageBreakdown;
  /** 调试追踪 */
  debug_trace: TraceEntry[];
}

/** EHP 系列 */
export interface EhpSeries {
  physical: number;
  fire: number;
  cold: number;
  lightning: number;
  chaos: number;
}

/** 伤害构成 */
export interface DamageBreakdown {
  /** 按类型分布 */
  by_type: Record<string, number>;
  /** 基础伤害 */
  base_damage: number;
  /** Inc 总和 */
  total_increased: number;
  /** More 乘积 */
  total_more: number;
  /** 转化后分布 */
  after_conversion: Record<string, DamageWithHistory>;
}

/** 带历史的伤害 */
export interface DamageWithHistory {
  damage: number;
  history_tags: string[];
}

/** 调试追踪条目 */
export interface TraceEntry {
  phase: string;
  description: string;
  values: Record<string, number>;
  matched_tags: string[];
}

// ============================================================
// 工具类型
// ============================================================

/** 创建默认输入 */
export function createDefaultInput(): CalculatorInput {
  return {
    context_flags: {},
    context_values: {},
    target_config: {
      level: 100,
      defense_constant: 0,
      resistances: {},
      generic_dr: 0,
      armor: 0,
      evasion: 0,
    },
    items: [],
    active_skill: {
      id: '',
      skill_type: 'active',
      is_attack: false,
      level: 1,
      base_damage: {},
      base_time: 1.0,
      mana_cost: 0,
      effectiveness: 1.0,
      tags: [],
      stats: {},
      injected_tags: [],
      mana_multiplier: 1.0,
    },
    support_skills: [],
    global_overrides: {},
  };
}

/** 创建默认目标配置 */
export function createDefaultTarget(): TargetConfig {
  return {
    level: 100,
    defense_constant: 0,
    resistances: {
      physical: 0,
      fire: 0,
      cold: 0,
      lightning: 0,
      chaos: 0,
    },
    generic_dr: 0,
    armor: 0,
    evasion: 0,
  };
}

/** 伤害类型列表 */
export const DAMAGE_TYPES = ['physical', 'fire', 'cold', 'lightning', 'chaos'] as const;
export type DamageType = typeof DAMAGE_TYPES[number];

/** 槽位类型列表 */
export const SLOT_TYPES: SlotType[] = [
  'weapon_main',
  'weapon_off',
  'helmet',
  'chest',
  'gloves',
  'boots',
  'amulet',
  'ring1',
  'ring2',
  'belt',
  'memory1',
  'memory2',
  'memory3',
  'memory4',
  'memory5',
  'memory6',
];

