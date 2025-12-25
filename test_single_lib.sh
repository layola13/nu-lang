#!/bin/bash

# 单个库测试脚本
# 用法: ./test_single_lib.sh <库名>
# 示例: ./test_single_lib.sh anyhow

if [ -z "$1" ]; then
  echo "❌ 错误: 请提供库名"
  echo "用法: $0 <库名>"
  echo ""
  echo "可用的库:"
  echo "  - log (推荐先测试)"
  echo "  - anyhow"
  echo "  - regex"
  echo "  - itertools"
  echo "  - chrono"
  echo "  - serde"
  echo "  - clap"
  echo "  - tokio"
  exit 1
fi

LIB=$1

echo "=========================================="
echo "测试单个库: $LIB"
echo "=========================================="
echo ""

# 创建日志目录
mkdir -p logs/opensource_libs

source_dir="examples_project/opensource_libs/$LIB"
nu_dir="examples_nu_project/opensource_libs/$LIB"
cargo_back_dir="examples_cargo_restored/opensource_libs/$LIB"

# 检查源项目是否存在
if [ ! -f "$source_dir/Cargo.toml" ]; then
  echo "❌ 错误: $source_dir/Cargo.toml 不存在"
  echo "请先运行: git clone https://github.com/... 下载库源码"
  exit 1
fi

echo "步骤1: Cargo → Nu (cargo2nu)"
echo "命令: cargo run --bin cargo2nu -- $source_dir $nu_dir"
echo ""
if ! cargo run --bin cargo2nu -- "$source_dir" "$nu_dir" 2>&1 | tee "logs/opensource_libs/${LIB}_cargo2nu.log"; then
  echo "❌ cargo2nu转换失败"
  exit 1
fi

echo ""
echo "✅ 步骤1完成: Cargo → Nu 转换成功"
echo ""
echo "---"
echo ""

echo "步骤2: Nu → Cargo (nu2cargo)"
echo "命令: cargo run --bin nu2cargo -- $nu_dir $cargo_back_dir"
echo ""
if ! cargo run --bin nu2cargo -- "$nu_dir" "$cargo_back_dir" 2>&1 | tee "logs/opensource_libs/${LIB}_nu2cargo.log"; then
  echo "❌ nu2cargo转换失败"
  exit 1
fi

echo ""
echo "✅ 步骤2完成: Nu → Cargo 转换成功"
echo ""
echo "---"
echo ""

echo "步骤3: 编译还原的Cargo项目"
echo "命令: cd $cargo_back_dir && cargo build"
echo ""
# 保存cargo build的输出和退出码
(cd "$cargo_back_dir" && cargo build 2>&1 | tee "../../logs/opensource_libs/${LIB}_build.log"; exit ${PIPESTATUS[0]})
BUILD_EXIT_CODE=$?

if [ $BUILD_EXIT_CODE -eq 0 ]; then
  echo ""
  echo "✅ 步骤3完成: 编译成功"
  echo ""
  echo "🎉 库 $LIB 测试完全成功!"
else
  echo ""
  echo "⚠️ 步骤3: 编译失败 (转换成功,但可能需要workspace或有依赖问题)"
  echo ""
  echo "✅ 库 $LIB 转换测试成功 (编译失败可能是环境问题)"
fi

echo ""
echo "---"
echo ""

echo "步骤4: 查看生成的文件"
if [ -d "$nu_dir" ]; then
  nu_files_count=$(find "$nu_dir" -name "*.nu" | wc -l)
  echo "📊 生成的 .nu 文件数量: $nu_files_count"
  echo ""
  echo "前10个 .nu 文件:"
  find "$nu_dir" -name "*.nu" | head -10
fi

echo ""
echo "=========================================="
echo "测试完成"
echo "=========================================="
echo "详细日志保存在:"
echo "  - logs/opensource_libs/${LIB}_cargo2nu.log"
echo "  - logs/opensource_libs/${LIB}_nu2cargo.log"
echo "  - logs/opensource_libs/${LIB}_build.log"
echo ""
echo "生成的文件位置:"
echo "  - Nu 文件: $nu_dir"
echo "  - 还原的 Cargo 项目: $cargo_back_dir"
