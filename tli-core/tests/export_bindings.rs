//! TypeScript 类型导出测试
//! 
//! 运行: cargo test --features ts-rs -- --ignored export_bindings

#[test]
#[ignore]
fn export_bindings() {
    use tli_core::types::*;
    use ts_rs::TS;
    use std::fs;
    use std::path::Path;

    let bindings_dir = Path::new("../bindings");
    if !bindings_dir.exists() {
        fs::create_dir_all(bindings_dir).unwrap();
    }

    // 导出所有类型
    CalculatorInput::export_all().unwrap();
    CalculatorOutput::export_all().unwrap();
    TargetConfig::export_all().unwrap();
    ItemData::export_all().unwrap();
    SkillData::export_all().unwrap();
    AffixData::export_all().unwrap();
    SlotType::export_all().unwrap();
    SkillType::export_all().unwrap();
    EhpSeries::export_all().unwrap();
    DamageBreakdown::export_all().unwrap();
    DamageWithHistory::export_all().unwrap();
    TraceEntry::export_all().unwrap();
    PreviewSlot::export_all().unwrap();

    println!("TypeScript bindings exported to ../bindings/");
}

