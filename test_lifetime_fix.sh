#!/bin/bash
# 测试生命周期标注修复

echo "=== 测试生命周期标注修复 ==="
echo ""

# 编译器已经在之前步骤编译好了
CONVERTER="./target/release/nu2ts"

# 测试文件列表
FILES=(
    "temp_examples_nu/lifetimes.nu"
    "temp_examples_nu/lifetimes_simple.nu"
    "temp_examples_nu/lifetimes_test.nu"
)

for file in "${FILES[@]}"; do
    if [ -f "$file" ]; then
        echo "=== 转换 $file ==="
        output_file="${file/temp_examples_nu/temp_examples_ts_new}"
        output_file="${output_file/.nu/.ts}"
        
        # 创建输出目录
        mkdir -p temp_examples_ts_new
        
        # 转换文件（忽略调试输出）
        $CONVERTER "$file" 2>/dev/null > "$output_file"
        
        # 检查生命周期残留
        echo "检查生命周期标注残留："
        grep -n "<'[a-z]" "$output_file" || echo "  ✓ 无泛型参数中的生命周期"
        grep -n "'[a-z] " "$output_file" || echo "  ✓ 无类型前的生命周期"
        grep -n "function<'" "$output_file" || echo "  ✓ 无函数签名中的生命周期"
        
        echo ""
    fi
done

echo "=== 测试完成 ==="
