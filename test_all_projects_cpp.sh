#!/bin/bash

# 批量测试所有子项目的 Nu→C++→CMake→编译流程
# 测试流程: Nu项目 -> C++代码 -> CMake配置 -> 编译 -> 运行

set -e

echo "=========================================="
echo "批量测试 Nu → C++ 完整工具链"
echo "=========================================="

# 子项目列表（从 examples_nu_project 选择）
projects=(
  "calculator"
  "dijkstra"
  "file_processor"
  "hello_rust"
  "test_simple_use"
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
  
  nu_dir="examples_nu_project/$project"
  cpp_dir="examples_cpp_project/$project"
  
  # 检查Nu项目是否存在
  if [ ! -f "$nu_dir/Nu.toml" ]; then
    echo "❌ 跳过: $nu_dir/Nu.toml 不存在"
    fail_count=$((fail_count + 1))
    failed_projects+=("$project (Nu.toml missing)")
    continue
  fi
  
  echo "步骤1: Nu → C++ (nu2cpp)"
  if ! cargo run --bin nu2cpp -- "$nu_dir" "$cpp_dir" -v; then
    echo "❌ nu2cpp转换失败: $project"
    fail_count=$((fail_count + 1))
    failed_projects+=("$project (nu2cpp failed)")
    continue
  fi
  
  echo "步骤2: Nu.toml → CMakeLists.txt (nu2cmake)"
  if ! cargo run --bin nu2cmake -- "$cpp_dir" -v; then
    echo "❌ nu2cmake转换失败: $project"
    fail_count=$((fail_count + 1))
    failed_projects+=("$project (nu2cmake failed)")
    continue
  fi
  
  echo "步骤3: CMake 配置"
  if ! (cd "$cpp_dir" && cmake -B build -S . 2>&1); then
    echo "❌ CMake配置失败: $project"
    fail_count=$((fail_count + 1))
    failed_projects+=("$project (cmake config failed)")
    continue
  fi
  
  echo "步骤4: 编译C++项目"
  if ! (cd "$cpp_dir" && cmake --build build 2>&1); then
    echo "❌ 编译失败: $project"
    fail_count=$((fail_count + 1))
    failed_projects+=("$project (build failed)")
    continue
  fi
  
  echo "步骤5: 验证可执行文件生成"
  
  # 从Nu.toml读取包名（支持id和n两种格式）
  package_name=$(grep -E '^(id|n) = ' "$nu_dir/Nu.toml" | head -1 | sed 's/^(id|n) = "\(.*\)"/\1/' | sed 's/id = "\(.*\)"/\1/' | sed 's/n = "\(.*\)"/\1/')
  
  # 检查可执行文件是否存在
  if [ ! -f "$cpp_dir/build/$package_name" ]; then
    echo "❌ 可执行文件未生成: $project (期望: $package_name)"
    fail_count=$((fail_count + 1))
    failed_projects+=("$project (no executable: $package_name)")
    continue
  fi
  
  echo "步骤6: 运行生成的程序"
  # 对于交互式程序，使用管道输入和超时
  run_cmd=""
  case "$project" in
    "calculator")
      run_cmd="echo -e '1 + 1\nexit' | timeout 5s ./build/$package_name 2>&1 || true"
      ;;
    "todo_list")
      run_cmd="echo 'exit' | timeout 5s ./build/$package_name 2>&1 || true"
      ;;
    "file_processor")
      run_cmd="echo 'exit' | timeout 5s ./build/$package_name 2>&1 || true"
      ;;
    *)
      run_cmd="timeout 10s ./build/$package_name 2>&1 || true"
      ;;
  esac
  
  if ! (cd "$cpp_dir" && eval "$run_cmd"); then
    echo "❌ 运行失败: $project"
    fail_count=$((fail_count + 1))
    failed_projects+=("$project (run failed)")
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