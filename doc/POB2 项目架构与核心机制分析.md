## 一、项目概述
Path of Building 2 是一个针对 Path of Exile 2 游戏的离线构建规划器,用于计算角色的伤害、防御等各项数值。 README.md:1-20

## 二、核心数据架构设计
### 1. Modifier 存储系统 (三层架构)
项目采用了三层 Modifier 存储架构:

+ ModStore (基类): 提供统一的 Modifier 查询接口,支持条件判断、乘数计算等复杂逻辑 ModStore.lua:1-35
+ ModDB (数据库模式): 使用哈希表按名称分类存储 Modifier,查询效率高 ModDB.lua:1-30
+ ModList (列表模式): 使用扁平数组存储,适合临时计算和小规模数据 ModList.lua:1-25

### 2. Modifier 类型系统
支持多种修改器类型:

+ BASE: 基础加成
+ INC/RED: 百分比增加/减少
+ MORE/LESS: 乘法增加/减少
+ FLAG: 布尔标志
+ OVERRIDE: 覆盖值
+ LIST: 列表类型 ModParser.lua:56-144

### 3. 条件和乘数系统
Modifier 支持复杂的条件标签系统,包括:

+ Multiplier: 基于乘数的缩放
+ Condition: 条件判断
+ PerStat: 基于属性值缩放
+ SkillType/SkillName: 技能类型/名称过滤
+ ItemCondition: 物品条件 ModStore.lua:286-363

## 三、核心计算系统
### 1. 计算流程管理 (CalcPerform)
主计算引擎按以下顺序执行:

1. 合并 Keystone 修改器
2. 初始化召唤物技能
3. 合并药剂效果
4. 计算属性和设置条件
5. 处理 Buff/Debuff
6. 处理充能和杂项 Buff
7. 计算防御和进攻数值 CalcPerform.lua:844-853

### 2. 属性和条件处理
系统会根据装备和技能设置各种条件标志:

+ 武器类型条件 (UsingAxe, UsingSword 等)
+ 双持条件 (DualWielding)
+ 护甲条件 (UsingShield, UsingHelmet 等)
+ 战斗条件 (AttackedRecently, HitRecently 等) CalcPerform.lua:129-234

### 3. 充能系统
管理各种充能类型:

+ 基础充能: Power/Frenzy/Endurance
+ 特殊充能: Brutal/Absorption/Affliction
+ 其他充能: Blood/Spirit/Inspiration CalcPerform.lua:665-817

## 四、伤害计算系统
### 1. 伤害类型和转换
支持五种伤害类型的转换链: Physical → Lightning → Cold → Fire → Chaos CalcOffence.lua:32-43

### 2. 伤害计算流程
+ 转换伤害计算: 处理伤害类型转换 CalcOffence.lua:64-86
+ 获得伤害计算: 处理额外获得的伤害 CalcOffence.lua:88-105
+ 最终伤害计算: 应用 INC 和 MORE 修改器 CalcOffence.lua:108-153

### 3. 范围效果计算
包含半径计算和断点分析功能: CalcOffence.lua:155-161

## 五、防御计算系统
### 1. 生命/魔力/灵力池
支持三种资源池的计算,包括转换、增减、MORE 修改器: CalcDefence.lua:54-107

### 2. 预留系统
管理技能的生命/魔力/灵力预留: CalcDefence.lua:153-200

### 3. 命中率和护甲计算
+ 命中率公式: `(accuracy * 1.5) / (accuracy + evasion) * 100`
+ 护甲减伤公式: `armour / (armour + raw * ratio)` CalcDefence.lua:30-52

## 六、物品系统
### 1. 物品解析
支持从游戏中直接复制粘贴物品,包括:

+ 稀有度识别
+ 词缀解析
+ 品质应用
+ 腐化状态 Item.lua:53-57

### 2. Catalyst 系统
支持催化剂增强特定标签的词缀效果: Item.lua:30-51

## 七、关键技术特点
### 1. 缓存优化
使用全局缓存避免重复计算技能数据: CalcPerform.lua:20-38

### 2. Modifier 聚合方法
提供统一接口聚合不同类型的修改器:

+ `Sum()`: 累加
+ `More()`: 乘法累乘
+ `Flag()`: 布尔判断
+ `Override()`: 覆盖
+ `Max()`: 最大值 ModStore.lua:114-128

### 3. 动作速度计算
考虑减速免疫和时间锁链上限: CalcPerform.lua:820-842

## Notes
这是一个高度模块化的游戏数值计算系统,核心设计思想包括:

1. 分层架构: ModStore → ModDB/ModList 的三层设计,兼顾性能和灵活性
2. 标签系统: 通过丰富的标签实现复杂的条件和缩放逻辑
3. 计算顺序: 严格的计算顺序确保数值依赖关系正确
4. 缓存机制: 避免重复计算,提升性能
5. 可扩展性: 易于添加新的修改器类型和游戏机制

该系统能够准确模拟 Path of Exile 2 的复杂游戏机制,为玩家提供精确的构建规划工具。  
  


