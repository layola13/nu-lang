#!/bin/bash

# 自动化往返测试脚本
# 功能：将examples/*.rs转换为nu，再转回rs，最后编译验证
# 步骤：rust2nu -> nu2rust -> 编译验证

set -e  # 遇到错误立即退出（但我们会捕获单个文件的错误）

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 统计变量
TOTAL_FILES=0
SUCCESS_COUNT=0
FAIL_COUNT=0
FAILED_FILES=()

echo -e "${GREEN}==========================================${NC}"
echo -e "${GREEN}Examples 目录往返测试脚本${NC}"
echo -e "${GREEN}测试流程: Rust -> Nu -> Rust -> 编译${NC}"
echo -e "${GREEN}==========================================${NC}"
echo ""

# 检查examples目录是否存在
if [ ! -d "examples" ]; then
    echo -e "${RED}错误: examples 目录不存在${NC}"
    exit 1
fi

# 创建临时输出目录
TEMP_NU_DIR="temp_examples_nu"
TEMP_RS_DIR="temp_examples_rs_back"

echo -e "${BLUE}清理并创建临时目录...${NC}"
rm -rf "$TEMP_NU_DIR" "$TEMP_RS_DIR"
mkdir -p "$TEMP_NU_DIR" "$TEMP_RS_DIR"

echo -e "${YELLOW}开始处理 examples/*.rs 文件${NC}"
echo "=========================================="

# 遍历examples目录下的所有.rs文件
for rs_file in examples/*.rs; do
    if [ -f "$rs_file" ]; then
        TOTAL_FILES=$((TOTAL_FILES + 1))
        filename=$(basename "$rs_file" .rs)
        
        echo ""
        echo -e "${BLUE}[${TOTAL_FILES}] 处理文件: ${GREEN}$filename.rs${NC}"
        echo "----------------------------------------"
        
        # 标记当前文件是否成功
        file_success=true
        
        # 步骤1: rust2nu 转换
        echo -e "  ${YELLOW}[1/3]${NC} Rust -> Nu 转换..."
        rust2nu_output=$(mktemp)
        if cargo run --bin rust2nu -- "$rs_file" -o "$TEMP_NU_DIR/$filename.nu" -f 2>"$rust2nu_output"; then
            echo -e "  ${GREEN}✓${NC} Rust -> Nu 成功"
            rm -f "$rust2nu_output"
        else
            echo -e "  ${RED}✗${NC} Rust -> Nu 失败"
            echo -e "  ${RED}错误信息:${NC}"
            head -20 "$rust2nu_output" | sed 's/^/    /'
            rm -f "$rust2nu_output"
            FAIL_COUNT=$((FAIL_COUNT + 1))
            FAILED_FILES+=("$filename (步骤1: rust2nu)")
            file_success=false
            continue
        fi
        
        # 步骤2: nu2rust 转换
        echo -e "  ${YELLOW}[2/3]${NC} Nu -> Rust 转换..."
        nu2rust_output=$(mktemp)
        if cargo run --bin nu2rust -- "$TEMP_NU_DIR/$filename.nu" -o "$TEMP_RS_DIR/$filename.rs" -f 2>"$nu2rust_output"; then
            echo -e "  ${GREEN}✓${NC} Nu -> Rust 成功"
            rm -f "$nu2rust_output"
            
            # 复制依赖的模块目录（如果存在）
            if [ -d "examples/utils" ]; then
                cp -r examples/utils "$TEMP_RS_DIR/" 2>/dev/null || true
            fi
        else
            echo -e "  ${RED}✗${NC} Nu -> Rust 失败"
            echo -e "  ${RED}错误信息:${NC}"
            head -20 "$nu2rust_output" | sed 's/^/    /'
            rm -f "$nu2rust_output"
            FAIL_COUNT=$((FAIL_COUNT + 1))
            FAILED_FILES+=("$filename (步骤2: nu2rust)")
            file_success=false
            continue
        fi
        
        # 步骤3: 编译验证
        echo -e "  ${YELLOW}[3/3]${NC} 编译验证..."
        compile_output=$(mktemp)
        if rustc --crate-type lib "$TEMP_RS_DIR/$filename.rs" -o "/tmp/$filename.rlib" 2>"$compile_output"; then
            echo -e "  ${GREEN}✓${NC} 编译成功"
            rm -f "$compile_output"
        else
            echo -e "  ${RED}✗${NC} 编译失败"
            echo -e "  ${RED}编译错误信息:${NC}"
            head -20 "$compile_output" | sed 's/^/    /'
            rm -f "$compile_output"
            FAIL_COUNT=$((FAIL_COUNT + 1))
            FAILED_FILES+=("$filename (步骤3: 编译)")
            file_success=false
            continue
        fi
        
        # 如果所有步骤都成功
        if [ "$file_success" = true ]; then
            echo -e "  ${GREEN}━━━ 全部通过 ━━━${NC}"
            SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
        fi
    fi
done

# 输出测试报告
echo ""
echo ""
echo -e "${GREEN}==========================================${NC}"
echo -e "${GREEN}测试完成统计${NC}"
echo -e "${GREEN}==========================================${NC}"
echo -e "总文件数: ${BLUE}$TOTAL_FILES${NC}"
echo -e "${GREEN}成功数: $SUCCESS_COUNT${NC}"
echo -e "${RED}失败数: $FAIL_COUNT${NC}"

if [ $TOTAL_FILES -gt 0 ]; then
    success_rate=$((SUCCESS_COUNT * 100 / TOTAL_FILES))
    echo -e "成功率: ${BLUE}${success_rate}%${NC}"
fi

if [ $FAIL_COUNT -gt 0 ]; then
    echo ""
    echo -e "${RED}失败的文件列表:${NC}"
    for failed in "${FAILED_FILES[@]}"; do
        echo -e "  ${RED}✗${NC} $failed"
    done
fi

# 显示临时文件位置
echo ""
echo -e "${YELLOW}临时文件位置:${NC}"
echo -e "  Nu 文件目录: ${BLUE}$TEMP_NU_DIR${NC}"
echo -e "  Rust 文件目录: ${BLUE}$TEMP_RS_DIR${NC}"

# 默认保留临时目录供调试
echo ""
echo -e "${YELLOW}✓ 临时目录已保留，可手动检查转换结果${NC}"
echo -e "${YELLOW}如需删除，请运行: rm -rf $TEMP_NU_DIR $TEMP_RS_DIR${NC}"

# 根据结果返回退出码
echo ""
if [ $FAIL_COUNT -eq 0 ]; then
    echo -e "${GREEN}🎉 所有测试通过！${NC}"
    exit 0
else
    echo -e "${RED}⚠️  部分测试失败，请检查失败文件${NC}"
    exit 1
fi