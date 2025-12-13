## <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">概述</font>
<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">这个文档展示了 Path of Building 2 的核心计算系统架构，包括 Modifier 三层存储架构、类型解析系统、计算流程管理、伤害计算逻辑、条件标签系统和缓存优化机制。 </font><font style="color:rgb(96, 150, 255);background-color:rgb(248, 248, 248);">Common.lua:301-420</font>

## <font style="color:rgb(51, 51, 51);background-color:rgb(246, 246, 246);">Motivation</font>
<font style="color:rgb(51, 51, 51);background-color:rgb(246, 246, 246);">Path of Exile 2 中有成千上万的物品词缀和天赋效果，每个都以自然语言描述（如"+10% increased damage"）。系统需要将这些</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(246, 246, 246);">文本描述</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(246, 246, 246);">转换为可计算的数值，并支持复杂的条件判断（如"when using a sword"）。手动为每个效果编写代码是不现实的，因此需要自动化的解析系统。</font>

---

## <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">1. Modifier 三层存储架构</font>
### <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">架构概览</font>
```plain
Modifier 存储架构体系  
├── ModStore 基类  
│   ├── 定义统一接口和通用方法  
│   ├── 提供 Combine() 聚合方法  
│   └── 派生出具体实现类  
├── ModDB 哈希表存储  
│   ├── 继承自 ModStore  
│   ├── 按名称分类存储 Modifier  
│   └── 提供高效查询性能  
└── ModList 扁平数组存储  
    ├── 继承自 ModStore  
    ├── 使用扁平数组存储  
    └── 适合临时计算场景
```

### <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">核心组件</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">ModStore 基类定义</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 定义 Modifier 存储的基础接口和通用方法</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">ModDB 继承实现</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 基于哈希表按名称分类存储 Modifier，提供高效查询</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">ModList 扁平存储</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 使用数组扁平存储，适合临时计算和小规模数据</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">统一聚合接口</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 提供 Sum/More/Flag/Override 等统一的 Modifier 聚合方法</font>

### <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">设计优势</font>
**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">三层架构设计</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：系统采用继承结构，基类 ModStore 定义统一接口，派生出两个具体实现。ModDB 使用哈希表按名称分类存储，适合大规模数据查询；ModList 使用扁平数组，适合临时计算和小规模数据。</font>

**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">统一聚合接口</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：Combine() 方法是系统的核心，它根据修改器类型自动选择合适的聚合方式：Sum() 处理基础值，More() 处理乘法，Flag() 处理布尔值等。这种设计让上层代码无需关心具体实现细节。</font>

**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">性能优化</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：ModDB 的哈希表结构让相同名称的修改器查询达到 O(1) 时间复杂度，而 ModList 的线性结构在少量数据时更高效。两者都继承自 ModStore，可以无缝切换使用。</font>

---

## <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">2. Modifier 类型解析系统</font>
### <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">解析流程</font>
```plain
Modifier 解析系统  
├── 文本描述输入  
│   ├── 正则表达式匹配  
│   │   ├── 匹配数值模式 (%d+)%%  
│   │   └── 映射到类型 INC/RED/MORE  
│   └── 属性名称映射  
│       ├── 基础属性 strength/dex/int  
│       └── 映射到内部标识符 Str/Dex/Int  
└── 扩展属性处理  
    ├── 生命相关属性  
    │   ├── maximum life → Life  
    │   └── life regeneration → LifeRegen  
    └── 其他属性扩展  
        ├── 魔力/灵力属性  
        └── 防御属性映射
```

### <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">关键实现</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">正则表达式映射</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 使用正则表达式将文本描述映射到 Modifier 类型</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">属性名称映射</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 将游戏内属性名映射到内部标识符</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">属性扩展映射</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 扩展属性名称到内部 Modifier 的映射关系</font>

### <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">系统特点</font>
<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">系统通过两层映射实现文本到代码的转换：</font>

**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">正则表达式映射</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">使用预定义的模式匹配数值和修饰词：</font>

+ `**<font style="color:rgb(51, 51, 51);background-color:rgb(229, 229, 229);">"^(%d+)%% increased"</font>**`<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">→</font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font>`**<font style="color:rgb(51, 51, 51);background-color:rgb(229, 229, 229);">"INC"</font>**`
+ `**<font style="color:rgb(51, 51, 51);background-color:rgb(229, 229, 229);">"^(%d+)%% more"</font>**`<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">→</font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font>`**<font style="color:rgb(51, 51, 51);background-color:rgb(229, 229, 229);">"MORE"</font>**`

**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">属性名称映射</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">将游戏内属性名转换为内部标识符：</font>

+ `**<font style="color:rgb(51, 51, 51);background-color:rgb(229, 229, 229);">"strength"</font>**`<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">→</font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font>`**<font style="color:rgb(51, 51, 51);background-color:rgb(229, 229, 229);">"Str"</font>**`
+ `**<font style="color:rgb(51, 51, 51);background-color:rgb(229, 229, 229);">"maximum life"</font>**`<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">→</font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font>`**<font style="color:rgb(51, 51, 51);background-color:rgb(229, 229, 229);">"Life"</font>**`

<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">这种设计让系统能够</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">动态解析</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">任何符合格式的文本描述，无需为每个新效果编写特定代码，大大提高了系统的可扩展性。</font>

---

## <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">3. 核心计算流程管理</font>
### <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">执行流程</font>
```plain
计算流程管理  
├── calcs.perform() 主计算入口  
│   ├── 合并基石节点效果  
│   │   └── mergeKeystones(env)  
│   ├── 初始化技能系统  
│   │   ├── 为每个技能创建skillModList  
│   │   └── 为召唤物创建独立modDB  
│   ├── 处理药剂和光环效果  
│   │   └── 合并各类buff/debuff  
│   ├── 设置条件和属性  
│   │   └── doActorAttribsConditions()  
│   └── 最终数值计算  
│       ├── 防御属性计算  
│       │   └── calcs.defence()  
│       └── 进攻属性计算  
│           └── calcs.offence()
```

### <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">核心函数</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">计算入口函数</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 主计算引擎的入口点，按预定顺序执行所有计算步骤</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">合并基石效果</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 第一步：处理天赋树中的基石节点效果</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">初始化召唤物</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 为召唤物创建独立的 Modifier 数据库</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">防御计算</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 计算生命、护甲、抗性等防御属性</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">进攻计算</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 计算伤害、速度、命中等进攻属性</font>

### <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">执行顺序</font>
<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">计算流程从</font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font>`**<font style="color:rgb(51, 51, 51);background-color:rgb(229, 229, 229);">calcs.perform()</font>**`<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">开始，这是整个数值计算的入口点。系统按照以下顺序执行：</font>

1. **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">基石效果合并</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：首先处理天赋树中的基石节点，这些会提供全局性的规则改变</font>
2. **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">技能系统初始化</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：为每个技能创建独立的 Modifier 列表，召唤物获得自己的计算环境</font>
3. **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">药剂和光环处理</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：合并所有临时增益效果</font>
4. **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">条件和属性设置</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：根据装备和技能设置各种状态标志（如"使用斧头"、"双持"等）</font>
5. **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">最终数值计算</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：分别计算防御属性和进攻属性</font>

<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">这个严格的执行顺序确保了数值计算的准确性，因为后一步的计算可能依赖于前一步的结果。例如，装备提供的属性会影响技能的基础伤害，然后技能的加成再基于这个基础值进行计算。</font>

---

## <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">4. 伤害计算核心逻辑</font>
### <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">计算体系</font>
```plain
伤害计算系统  
├── 伤害类型定义  
│   └── 五种基础伤害类型转换链  
├── 伤害转换计算  
│   ├── 处理物理→闪电→冰→火→混沌  
│   └── calcConvertedDamage() 函数  
├── 伤害聚合计算  
│   ├── 应用 INC 和 MORE 修改器  
│   └── calcDamage() 主函数  
│   ├── INC 修改器应用  
│   │   └── 计算增加/减少效果  
│   └── MORE 修改器应用  
│       └── 计算更多/更少效果  
└── 最终伤害输出  
    └── 返回计算后的最小/最大伤害值
```

### <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">关键实现</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">伤害类型定义</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 定义五种基础伤害类型及其转换顺序</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">伤害转换计算</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 处理物理→闪电→冰→火→混沌的伤害转换链</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">伤害聚合计算</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 应用 INC 和 MORE 修改器计算最终伤害</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">INC 修改器应用</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 计算并应用所有增加/减少修改器</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">MORE 修改器应用</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 计算并应用所有更多/更少修改器</font>

### <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">计算机制</font>
**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">伤害类型体系</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：系统定义了五种基础伤害类型：物理、闪电、冰、火、混沌。这些类型按照固定顺序进行转换：物理→闪电→冰→火→混沌，形成了复杂的伤害转换链。</font>

**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">转换机制</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：</font>`**<font style="color:rgb(51, 51, 51);background-color:rgb(229, 229, 229);">calcConvertedDamage()</font>**`<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">函数处理伤害类型转换。系统会检查转换表，将一种伤害类型按比例转换为另一种类型，同时保留原始伤害的基础值。</font>

**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">修改器聚合</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：</font>`**<font style="color:rgb(51, 51, 51);background-color:rgb(229, 229, 229);">calcDamage()</font>**`<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">函数是伤害计算的核心。它首先收集所有相关的修改器，然后分两步应用：</font>

1. **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">INC 修改器</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：所有增加/减少效果累加后统一应用</font>
2. **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">MORE 修改器</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：所有更多/更少效果以乘法方式叠加</font>

**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">计算顺序</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：系统严格遵循游戏内的计算顺序：基础伤害 → 转换伤害 → 获得伤害 → INC 修改器 → MORE 修改器 → 最终伤害。这确保了计算结果与游戏内完全一致。</font>

**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">输出格式</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：最终返回最小和最大伤害值，为玩家提供完整的伤害范围预测，用于评估构建的实际战斗表现。</font>

---

## <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">5. 条件标签系统</font>
### <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">评估架构</font>
```plain
Modifier 条件标签系统  
├── EvalMod() 主评估函数  
│   ├── 遍历所有标签  
│   ├── 乘数标签处理  
│   │   ├── 获取目标数据库  
│   │   ├── 计算基础值  
│   │   ├── 乘数计算  
│   │   └── 应用缩放效果  
│   ├── 阈值条件处理  
│   │   ├── 获取阈值  
│   │   ├── 阈值判断  
│   │   └── 决定是否生效  
│   └── 其他标签类型处理  
└── 返回最终值
```

### <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">核心函数</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">Modifier 评估</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 评估单个 Modifier 的条件和乘数效果</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">乘数标签处理</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 处理基于其他属性值的乘数缩放</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">乘数计算</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 计算最终的乘数缩放值</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">阈值条件处理</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 处理基于阈值的条件判断</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">阈值判断</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 根据阈值决定 Modifier 是否生效</font>

### <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">系统特性</font>
**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">核心评估流程</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：所有修改器的评估都通过</font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font>`**<font style="color:rgb(51, 51, 51);background-color:rgb(229, 229, 229);">EvalMod()</font>**`<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">函数进行。这个函数遍历修改器携带的所有标签，根据标签类型执行不同的计算逻辑：</font>

**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">乘数标签</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">用于基于其他属性值的缩放计算。比如"每 5 点敏捷增加 1% 攻击速度"，系统会先获取当前敏捷值，除以 5 得到乘数，然后应用到基础值上。</font>

**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">阈值标签</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">用于条件判断。比如"生命值低于 50% 时效果翻倍"，系统会比较当前值与阈值，决定修改器是否生效。</font>

**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">标签处理机制</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：每个标签都可以指定不同的目标数据库（玩家、召唤物、敌人等），实现跨角色的属性引用。系统支持复杂的数学运算，包括除法、取整、上下限控制等，确保计算结果符合游戏规则。</font>

**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">调用路径</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：修改器评估主要通过两个入口触发：</font>`**<font style="color:rgb(51, 51, 51);background-color:rgb(229, 229, 229);">Combine()</font>**`<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">方法用于聚合计算，</font>`**<font style="color:rgb(51, 51, 51);background-color:rgb(229, 229, 229);">Tabulate()</font>**`<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">方法用于枚举计算。这两个方法都会调用</font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font>`**<font style="color:rgb(51, 51, 51);background-color:rgb(229, 229, 229);">EvalMod()</font>**`<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">来处理每个修改器的具体逻辑。</font>

<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">这个设计的优势是</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">高度可扩展</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">——添加新的条件类型只需要增加新的标签处理逻辑，而不需要修改现有的计算代码。</font>

---

## <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">6. 缓存优化机制</font>
### <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">缓存架构</font>
```plain
全局缓存系统  
├── 缓存数据结构定义  
│   └── GlobalCache 初始化  
├── 缓存键值生成  
│   └── cacheSkillUUID() 生成唯一ID  
├── 缓存访问流程  
│   ├── getCachedOutputValue() 入口  
│   ├── 检查缓存是否存在  
│   │   ├── 缓存命中：返回缓存值  
│   │   └── 缓存未命中：重新计算  
│   │       └── cacheData() 存储新值  
└── 缓存管理  
    └── wipeGlobalCache() 清理所有缓存
```

### <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">关键实现</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">全局缓存结构</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 定义多模式的全局缓存数据结构</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">技能 UUID 生成</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 为每个技能生成唯一标识符用于缓存</font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font><font style="color:rgb(96, 150, 255);background-color:rgb(248, 248, 248);">Common.lua:812</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">缓存值获取</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 从缓存中获取已计算的技能输出值</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">缓存检查</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 检查缓存是否存在，决定是否需要重新计算</font>
+ **<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">缓存清理</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">: 清理所有缓存数据以确保计算准确性</font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font><font style="color:rgb(96, 150, 255);background-color:rgb(248, 248, 248);">Common.lua:859</font>

### <font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">优化机制</font>
**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">缓存架构设计</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：系统采用</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">分层缓存结构</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">，为不同计算模式（MAIN、CALCS、CALCULATOR、CACHE）提供独立的缓存空间。每个缓存条目包含技能的完整输出数据，如伤害、速度、消耗等。</font>

**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">缓存键生成机制</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：每个技能通过</font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font>`**<font style="color:rgb(51, 51, 51);background-color:rgb(229, 229, 229);">cacheSkillUUID()</font>**`<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">生成</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">唯一标识符</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">。该 ID 基于技能名称、装备槽位、宝石位置和技能组索引构建，确保不同配置的技能有独立的缓存。</font>

**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">缓存访问流程</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：</font>`**<font style="color:rgb(51, 51, 51);background-color:rgb(229, 229, 229);">getCachedOutputValue()</font>**`<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">作为缓存访问的</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">统一入口</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">。首先检查缓存是否存在，如果命中则直接返回存储的数值；如果未命中，则触发完整计算并通过</font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font>`**<font style="color:rgb(51, 51, 51);background-color:rgb(229, 229, 229);">cacheData()</font>**`<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> </font><font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">将结果存入缓存。</font>

**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">缓存管理</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">：</font>`**<font style="color:rgb(51, 51, 51);background-color:rgb(229, 229, 229);">wipeGlobalCache()</font>**`<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);"> 提供</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">全局清理功能</font>**<font style="color:rgb(51, 51, 51);background-color:rgb(248, 248, 248);">，在用户修改构建配置时调用，确保所有缓存数据与当前配置保持同步，避免使用过时的计算结果。</font>

