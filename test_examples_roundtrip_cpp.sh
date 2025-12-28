#!/bin/bash

# 自动化往返测试脚本 - C++ 版本
# 功能：将examples/*.nu转换为C++，生成CMakeLists.txt，编译验证
# 步骤：nu2cpp -> nu2cmake -> cmake -> 编译验证

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
echo -e "${GREEN}Examples 目录 C++ 转换测试脚本${NC}"
echo -e "${GREEN}测试流程: Nu -> C++ -> CMake -> 编译${NC}"
echo -e "${GREEN}==========================================${NC}"
echo ""

# 检查examples目录是否存在
if [ ! -d "examples" ]; then
    echo -e "${RED}错误: examples 目录不存在${NC}"
    exit 1
fi

# 创建临时输出目录
TEMP_CPP_DIR="temp_examples_cpp"
TEMP_BUILD_DIR="temp_examples_cpp_build"

echo -e "${BLUE}清理并创建临时目录...${NC}"
rm -rf "$TEMP_CPP_DIR" "$TEMP_BUILD_DIR"
mkdir -p "$TEMP_CPP_DIR" "$TEMP_BUILD_DIR"

echo -e "${YELLOW}开始处理 examples/*.nu 文件${NC}"
echo "=========================================="

# 遍历examples目录下的所有.nu文件
for nu_file in examples/*.nu; do
    if [ -f "$nu_file" ]; then
        TOTAL_FILES=$((TOTAL_FILES + 1))
        filename=$(basename "$nu_file" .nu)
        
        echo ""
        echo -e "${BLUE}[${TOTAL_FILES}] 处理文件: ${GREEN}$filename.nu${NC}"
        echo "----------------------------------------"
        
        # 标记当前文件是否成功
        file_success=true
        
        # 步骤1: nu2cpp 转换 (使用新的 AST 转换器)
        echo -e "  ${YELLOW}[1/4]${NC} Nu -> C++ 转换 (AST mode)..."
        nu2cpp_output=$(mktemp)
        if cargo run --bin nu2cpp -- "$nu_file" "$TEMP_CPP_DIR/$filename.cpp" -f --use-ast 2>"$nu2cpp_output"; then
            echo -e "  ${GREEN}✓${NC} Nu -> C++ 成功"
            rm -f "$nu2cpp_output"
        else
            echo -e "  ${RED}✗${NC} Nu -> C++ 失败"
            echo -e "  ${RED}错误信息:${NC}"
            head -20 "$nu2cpp_output" | sed 's/^/    /'
            rm -f "$nu2cpp_output"
            FAIL_COUNT=$((FAIL_COUNT + 1))
            FAILED_FILES+=("$filename (步骤1: nu2cpp)")
            file_success=false
            continue
        fi
        
        # 步骤2: 为每个文件创建独立的构建目录
        echo -e "  ${YELLOW}[2/4]${NC} 创建独立构建目录..."
        file_dir="$TEMP_CPP_DIR/$filename"
        mkdir -p "$file_dir"
        
        # 复制C++文件到独立目录
        cp "$TEMP_CPP_DIR/$filename.cpp" "$file_dir/"
        
        # 检查是否有main函数，决定编译类型
        if grep -q "int main\|void main" "$file_dir/$filename.cpp"; then
            target_type="executable"
            cmake_target="add_executable($filename $filename.cpp)"
        else
            target_type="library"
            cmake_target="add_library($filename STATIC $filename.cpp)"
        fi
        
        # 生成 CMakeLists.txt
        cmake_file="$file_dir/CMakeLists.txt"
        cat > "$cmake_file" << EOF
cmake_minimum_required(VERSION 3.15)
project($filename CXX)

# 使用 C++23 标准 (GCC 14+/Clang 17+)
set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# 源文件
$cmake_target

# 编译选项
if(MSVC)
    target_compile_options($filename PRIVATE /W4)
else()
    target_compile_options($filename PRIVATE -Wall -Wextra -pedantic)
endif()
EOF
        echo -e "  ${GREEN}✓${NC} 构建目录和CMakeLists.txt创建成功"
        
        # 步骤3: CMake 配置
        echo -e "  ${YELLOW}[3/4]${NC} CMake 配置..."
        build_dir="$TEMP_BUILD_DIR/$filename"
        mkdir -p "$build_dir"
        cmake_output=$(mktemp)
        if cmake -B "$build_dir" -S "$file_dir" -DCMAKE_BUILD_TYPE=Debug 2>"$cmake_output" 1>/dev/null; then
            echo -e "  ${GREEN}✓${NC} CMake 配置成功"
            rm -f "$cmake_output"
        else
            echo -e "  ${RED}✗${NC} CMake 配置失败"
            echo -e "  ${RED}错误信息:${NC}"
            head -20 "$cmake_output" | sed 's/^/    /'
            rm -f "$cmake_output"
            FAIL_COUNT=$((FAIL_COUNT + 1))
            FAILED_FILES+=("$filename (步骤3: cmake配置)")
            file_success=false
            continue
        fi
        
        # 步骤4: 编译验证
        echo -e "  ${YELLOW}[4/4]${NC} 编译验证..."
        compile_output=$(mktemp)
        if cmake --build "$build_dir" 2>"$compile_output"; then
            echo -e "  ${GREEN}✓${NC} 编译成功"
            rm -f "$compile_output"
            
            # 检查输出文件是否生成
            if [ "$target_type" = "executable" ]; then
                if [ -f "$build_dir/$filename" ] || [ -f "$build_dir/$filename.exe" ]; then
                    echo -e "  ${GREEN}✓${NC} 可执行文件已生成"
                else
                    echo -e "  ${YELLOW}⚠${NC} 可执行文件未找到"
                fi
            else
                if [ -f "$build_dir/lib$filename.a" ] || [ -f "$build_dir/$filename.lib" ]; then
                    echo -e "  ${GREEN}✓${NC} 库文件已生成"
                else
                    echo -e "  ${YELLOW}⚠${NC} 库文件未找到"
                fi
            fi
        else
            echo -e "  ${RED}✗${NC} 编译失败"
            echo -e "  ${RED}编译错误信息:${NC}"
            head -30 "$compile_output" | sed 's/^/    /'
            rm -f "$compile_output"
            FAIL_COUNT=$((FAIL_COUNT + 1))
            FAILED_FILES+=("$filename (步骤4: 编译)")
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
echo -e "  C++ 文件目录: ${BLUE}$TEMP_CPP_DIR${NC}"
echo -e "  构建目录: ${BLUE}$TEMP_BUILD_DIR${NC}"

# 默认保留临时目录供调试
echo ""
echo -e "${YELLOW}✓ 临时目录已保留，可手动检查转换结果${NC}"
echo -e "${YELLOW}如需删除，请运行: rm -rf $TEMP_CPP_DIR $TEMP_BUILD_DIR${NC}"

# 根据结果返回退出码
echo ""
if [ $FAIL_COUNT -eq 0 ]; then
    echo -e "${GREEN}🎉 所有测试通过！${NC}"
    exit 0
else
    echo -e "${RED}⚠️  部分测试失败，请检查失败文件${NC}"
    exit 1
fi