#!/bin/bash

# 批量测试所有子项目的Cargo→Nu→Cargo→编译流程
# 测试12个子项目

set -e

echo "=========================================="
echo "批量测试所有examples_project子项目"
echo "=========================================="

# 子项目列表
projects=(
  "calculator"
  "dijkstra"
  "file_processor"
  "hello_rust"
  "test_closure_enhancements"
  "test_closures_temp"
  "test_error_prop_temp"
  "test_if_project"
  "test_literals_project"
  "test_pattern_temp"
  "test_simple_use"
  "test_stdlib_integration"
  "todo_list"
)

success_count=0
fail_count=0
failed_projects=()

for project in "${projects[@]}"; do
  echo ""
  echo "=========================================="
  echo "测试项目: $project"
  echo "=========================================="
  
  cargo_dir="examples_project/$project"
  nu_dir="examples_nu_project/$project"
  
  # 检查Cargo项目是否存在
  if [ ! -f "$cargo_dir/Cargo.toml" ]; then
    echo "❌ 跳过: $cargo_dir/Cargo.toml 不存在"
    fail_count=$((fail_count + 1))
    failed_projects+=("$project (Cargo.toml missing)")
    continue
  fi
  
  echo "步骤1: Cargo → Nu (cargo2nu)"
  if ! cargo run --bin cargo2nu -- "$cargo_dir" "$nu_dir"; then
    echo "❌ cargo2nu转换失败: $project"
    fail_count=$((fail_count + 1))
    failed_projects+=("$project (cargo2nu failed)")
    continue
  fi
  
  echo "步骤2: Nu → Cargo (nu2cargo)"
  cargo_back_dir="examples_cargo_restored/$project"
  if ! cargo run --bin nu2cargo -- "$nu_dir" "$cargo_back_dir"; then
    echo "❌ nu2cargo转换失败: $project"
    fail_count=$((fail_count + 1))
    failed_projects+=("$project (nu2cargo failed)")
    continue
  fi
  
  echo "步骤3: 编译还原的Cargo项目"
  if ! (cd "$cargo_back_dir" && cargo build 2>&1); then
    echo "❌ 编译失败: $project"
    fail_count=$((fail_count + 1))
    failed_projects+=("$project (build failed)")
    continue
  fi
  
  echo "步骤4: 验证可执行文件生成"
  
  # 从Cargo.toml读取包名
  package_name=$(grep '^name = ' "$cargo_back_dir/Cargo.toml" | head -1 | sed 's/name = "\(.*\)"/\1/')
  
  # 检查可执行文件是否存在（使用包名）
  if [ ! -f "$cargo_back_dir/target/debug/$package_name" ]; then
    echo "❌ 可执行文件未生成: $project (期望: $package_name)"
    fail_count=$((fail_count + 1))
    failed_projects+=("$project (no executable: $package_name)")
    continue
  fi
  
  echo "✅ 项目测试成功: $project"
  success_count=$((success_count + 1))
done

echo ""
echo "=========================================="
echo "测试完成统计"
echo "=========================================="
echo "总项目数: ${#projects[@]}"
echo "成功: $success_count"
echo "失败: $fail_count"
echo "成功率: $((success_count * 100 / ${#projects[@]}))%"

if [ $fail_count -gt 0 ]; then
  echo ""
  echo "失败项目列表:"
  for failed in "${failed_projects[@]}"; do
    echo "  - $failed"
  done
fi

echo ""
if [ $success_count -eq ${#projects[@]} ]; then
  echo "🎉 所有项目测试通过！"
  exit 0
else
  echo "⚠️ 部分项目测试失败"
  exit 1
fi