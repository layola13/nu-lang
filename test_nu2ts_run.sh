#!/bin/bash

# 测试脚本：运行所有转换后的TypeScript项目

echo "🚀 Testing nu2ts generated projects..."
echo "========================================"

# 先重新转换所有项目
./test_nu2ts_all.sh > /dev/null 2>&1

# 应用后处理修复
if [ -x "./fix_nu2ts_output.sh" ]; then
    ./fix_nu2ts_output.sh > /dev/null 2>&1
fi

OUTPUT_BASE="/tmp/nu2ts_test"

PROJECTS=(
    "hello_rust"
    "test_literals_project"
    "test_if_project"
    "test_simple_use"
    "calculator"
    "dijkstra"
    "file_processor"
    "test_closure_enhancements"
    "test_closures_temp"
    "test_error_prop_temp"
    "test_pattern_temp"
    "test_stdlib_integration"
    "todo_list"
)

SUCCESS_COUNT=0
FAIL_COUNT=0
FAILED_PROJECTS=()

for project in "${PROJECTS[@]}"; do
    echo ""
    echo "🧪 Testing: $project"
    echo "----------------------------------------"
    
    TS_FILE="$OUTPUT_BASE/$project/src/main.ts"
    
    if [ ! -f "$TS_FILE" ]; then
        echo "❌ File not found: $TS_FILE"
        ((FAIL_COUNT++))
        FAILED_PROJECTS+=("$project (file not found)")
        continue
    fi
    
    # 运行TypeScript文件
    echo "Running: bun run $TS_FILE"
    if timeout 5s bun run "$TS_FILE" 2>&1; then
        echo "✅ Success: $project"
        ((SUCCESS_COUNT++))
    else
        EXIT_CODE=$?
        echo "❌ Failed: $project (exit code: $EXIT_CODE)"
        ((FAIL_COUNT++))
        FAILED_PROJECTS+=("$project")
    fi
done

echo ""
echo "========================================"
echo "📊 Execution Summary"
echo "========================================"
echo "✅ Successful: $SUCCESS_COUNT"
echo "❌ Failed:     $FAIL_COUNT"
echo "📁 Total:      ${#PROJECTS[@]}"

if [ $FAIL_COUNT -gt 0 ]; then
    echo ""
    echo "Failed projects:"
    for failed in "${FAILED_PROJECTS[@]}"; do
        echo "  - $failed"
    done
    exit 1
fi

echo ""
echo "🎉 All projects ran successfully!"
exit 0