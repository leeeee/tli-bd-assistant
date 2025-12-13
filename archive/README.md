# Archive 归档目录

本目录存放已废弃或仅供参考的历史文件。

## 目录结构

```
archive/
├── meta-json/                    # 原始数据源 JSON
│   ├── active_skills_with_details.json   # 主动技能数据（已导入 Supabase）
│   ├── gears_unique.json                 # 传奇装备数据（已导入 Supabase）
│   └── support_skills_with_details.json  # 辅助技能数据（已导入 Supabase）
└── doc-reference/                # 参考文档
    └── 闪电链.md                  # 闪电链技能参考数据
```

## 说明

### meta-json/
这些 JSON 文件是从游戏数据解析而来的原始数据源，已通过 MCP 工具导入到 Supabase 数据库。
如需更新数据，请直接修改 `supabase/seed.sql` 或使用 MCP 工具操作数据库。

### doc-reference/
临时参考文档，数据已整合到数据库或开发规划文档中。

---

*归档日期: 2025-12-14*

