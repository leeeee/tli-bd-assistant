//! TypeScript 类型导出测试
//! 
//! 运行: cargo test -- --ignored export_bindings

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

    // 导出所有类型 (ts-rs 7.x 使用 export 而非 export_all)
    CalculatorInput::export().unwrap();
    CalculatorOutput::export().unwrap();
    TargetConfig::export().unwrap();
    ItemData::export().unwrap();
    SkillData::export().unwrap();
    AffixData::export().unwrap();
    SlotType::export().unwrap();
    SkillType::export().unwrap();
    EhpSeries::export().unwrap();
    DamageBreakdown::export().unwrap();
    DamageWithHistory::export().unwrap();
    TraceEntry::export().unwrap();
    PreviewSlot::export().unwrap();

    println!("TypeScript bindings exported to ../bindings/");
}

