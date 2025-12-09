#!/bin/bash
# TLI Core WASM 构建脚本

set -e

echo "🔨 Building TLI Core WASM package..."

# 检查 wasm-pack 是否安装
if ! command -v wasm-pack &> /dev/null; then
    echo "❌ wasm-pack not found. Installing..."
    cargo install wasm-pack
fi

# 构建 WASM 包
wasm-pack build --target web --out-dir ../pkg --release

# 导出 TypeScript 类型
echo "📝 Generating TypeScript bindings..."
cargo test --features ts-rs -- --ignored export_bindings 2>/dev/null || true

# 复制类型定义到 pkg 目录
if [ -d "../bindings" ]; then
    cp -r ../bindings/*.ts ../pkg/ 2>/dev/null || true
fi

echo "✅ Build complete! Output in ../pkg/"
echo ""
echo "Usage in JavaScript/TypeScript:"
echo "  import init, { calculate, version } from './pkg/tli_core.js';"
echo "  await init();"
echo "  const result = calculate(JSON.stringify(input));"

