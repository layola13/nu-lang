#!/bin/bash

# 测试所有TypeScript文件并收集错误信息
output_file="test_failures_analysis.txt"
> "$output_file"

total=0
failed=0
passed=0

for f in temp_examples_ts/*.ts; do
    total=$((total + 1))
    filename=$(basename "$f")
    
    # 运行测试并捕获输出
    output=$(bun run "$f" 2>&1)
    exit_code=$?
    
    if [ $exit_code -ne 0 ]; then
        failed=$((failed + 1))
        echo "=== FAILED: $filename ===" >> "$output_file"
        echo "$output" | head -5 >> "$output_file"
        echo "" >> "$output_file"
    else
        passed=$((passed + 1))
    fi
done

echo "==================== 测试总结 ====================" >> "$output_file"
echo "总计: $total 个测试文件" >> "$output_file"
echo "通过: $passed 个" >> "$output_file"
echo "失败: $failed 个" >> "$output_file"
echo "" >> "$output_file"

cat "$output_file"