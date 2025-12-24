#!/bin/bash

# 测试脚本：转换所有examples_nu_project项目到TypeScript

echo "🚀 Testing nu2ts on all Nu projects..."
echo "========================================"

OUTPUT_BASE="/tmp/nu2ts_test"
rm -rf "$OUTPUT_BASE"
mkdir -p "$OUTPUT_BASE"

PROJECTS=(
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

SUCCESS_COUNT=0
FAIL_COUNT=0
FAILED_PROJECTS=()

for project in "${PROJECTS[@]}"; do
    echo ""
    echo "📦 Converting: $project"
    echo "----------------------------------------"
    
    INPUT_DIR="examples_nu_project/$project"
    OUTPUT_DIR="$OUTPUT_BASE/$project"
    
    if [ ! -d "$INPUT_DIR" ]; then
        echo "⚠️  Project not found: $INPUT_DIR"
        ((FAIL_COUNT++))
        FAILED_PROJECTS+=("$project (not found)")
        continue
    fi
    
    if ./target/release/nu2ts "$INPUT_DIR" -P -o "$OUTPUT_DIR" -f; then
        echo "✅ Success: $project"
        ((SUCCESS_COUNT++))
        
        # 检查生成的文件
        if [ -f "$OUTPUT_DIR/package.json" ] && [ -f "$OUTPUT_DIR/tsconfig.json" ]; then
            echo "   ✓ Config files generated"
        fi
        
        if [ -d "$OUTPUT_DIR/src" ]; then
            TS_FILES=$(find "$OUTPUT_DIR/src" -name "*.ts" | wc -l)
            echo "   ✓ TypeScript files: $TS_FILES"
        fi
    else
        echo "❌ Failed: $project"
        ((FAIL_COUNT++))
        FAILED_PROJECTS+=("$project")
    fi
done

echo ""
echo "========================================"
echo "📊 Conversion Summary"
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
fi

echo ""
echo "📂 Output directory: $OUTPUT_BASE"
echo ""

if [ $SUCCESS_COUNT -eq ${#PROJECTS[@]} ]; then
    echo "🎉 All projects converted successfully!"
    exit 0
else
    echo "⚠️  Some projects failed to convert."
    exit 1
fi