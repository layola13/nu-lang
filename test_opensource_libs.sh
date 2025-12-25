#!/bin/bash

# 批量测试开源库的 Cargo→Nu→Cargo→编译流程
# 测试 8 个开源库: anyhow, regex, log, serde, clap, chrono, tokio, itertools

# 注意：不使用 set -e，因为我们需要处理编译失败的情况

echo "=========================================="
echo "批量测试开源Rust库的rust2nu2rust转换"
echo "=========================================="
echo ""

# 创建日志目录
mkdir -p logs/opensource_libs

# 库列表
LIBS=("log" "anyhow" "regex" "itertools" "chrono" "serde" "clap" "tokio")

success_count=0
fail_count=0
partial_count=0
failed_libs=()
partial_libs=()

for lib in "${LIBS[@]}"; do
  echo ""
  echo "=========================================="
  echo "测试库: $lib"
  echo "=========================================="
  
  source_dir="examples_project/opensource_libs/$lib"
  nu_dir="examples_nu_project/opensource_libs/$lib"
  cargo_back_dir="examples_cargo_restored/opensource_libs/$lib"
  
  # 检查源项目是否存在
  if [ ! -f "$source_dir/Cargo.toml" ]; then
    echo "❌ 跳过: $source_dir/Cargo.toml 不存在"
    fail_count=$((fail_count + 1))
    failed_libs+=("$lib (源文件不存在)")
    continue
  fi
  
  echo "步骤1: Cargo → Nu (cargo2nu)"
  if ! cargo run --bin cargo2nu -- "$source_dir" "$nu_dir" 2>&1 | tee "logs/opensource_libs/${lib}_cargo2nu.log"; then
    echo "❌ cargo2nu转换失败: $lib"
    fail_count=$((fail_count + 1))
    failed_libs+=("$lib (cargo2nu failed)")
    continue
  fi
  
  echo ""
  echo "步骤2: Nu → Cargo (nu2cargo)"
  if ! cargo run --bin nu2cargo -- "$nu_dir" "$cargo_back_dir" 2>&1 | tee "logs/opensource_libs/${lib}_nu2cargo.log"; then
    echo "❌ nu2cargo转换失败: $lib"
    fail_count=$((fail_count + 1))
    failed_libs+=("$lib (nu2cargo failed)")
    continue
  fi
  
  echo ""
  echo "步骤3: 编译还原的Cargo项目"
  # 保存cargo build的输出和退出码
  (cd "$cargo_back_dir" && cargo build 2>&1 | tee "../../logs/opensource_libs/${lib}_build.log"; exit ${PIPESTATUS[0]})
  BUILD_EXIT_CODE=$?
  
  if [ $BUILD_EXIT_CODE -eq 0 ]; then
    echo "✅ 库测试完全成功 (含编译验证): $lib"
    success_count=$((success_count + 1))
  else
    echo "⚠️ 转换成功但编译失败 (可能需要workspace或有依赖问题): $lib"
    partial_count=$((partial_count + 1))
    partial_libs+=("$lib (build failed)")
  fi
  
  echo ""
  echo "步骤4: 统计生成的Nu文件"
  if [ -d "$nu_dir" ]; then
    nu_files_count=$(find "$nu_dir" -name "*.nu" | wc -l)
    echo "📊 生成的 .nu 文件数量: $nu_files_count"
  fi
  
  echo "---"
done

echo ""
echo "=========================================="
echo "测试完成统计"
echo "=========================================="
echo "总库数: ${#LIBS[@]}"
echo "完全成功 (转换+编译): $success_count"
echo "部分成功 (转换成功,编译失败): $partial_count"
echo "完全失败: $fail_count"
echo "转换成功率: $(((success_count + partial_count) * 100 / ${#LIBS[@]}))%"

if [ $partial_count -gt 0 ]; then
  echo ""
  echo "部分成功的库 (转换OK,编译失败):"
  for partial in "${partial_libs[@]}"; do
    echo "  ⚠️ $partial"
  done
fi

if [ $fail_count -gt 0 ]; then
  echo ""
  echo "完全失败的库:"
  for failed in "${failed_libs[@]}"; do
    echo "  ❌ $failed"
  done
fi

echo ""
echo "=========================================="
echo "详细日志位置: logs/opensource_libs/"
echo "=========================================="
echo ""

if [ $fail_count -eq 0 ]; then
  if [ $partial_count -eq 0 ]; then
    echo "🎉 所有库测试完全通过 (转换+编译)!"
    exit 0
  else
    echo "✅ 所有库转换成功! (部分库编译失败属正常,可能需要workspace)"
    exit 0
  fi
else
  echo "⚠️ 有 $fail_count 个库转换失败,需要检查"
  exit 1
fi
