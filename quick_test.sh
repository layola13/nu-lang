#!/bin/bash
cd /home/sonygod/projects/nu

echo "1. 编译转换器..."
cargo build --release --bin nu2rust 2>&1 | tail -5

echo "2. 转换测试..."
rm -rf examples_nu_project/opensource_libs/log examples_cargo_restored/opensource_libs/log
cargo run --release --bin cargo2nu -- examples_project/opensource_libs/log examples_nu_project/opensource_libs/log 2>&1 | tail -2
cargo run --release --bin nu2cargo -- examples_nu_project/opensource_libs/log examples_cargo_restored/opensource_libs/log 2>&1 | tail -2

echo "3. 编译并统计错误..."
cd examples_cargo_restored/opensource_libs/log
ERROR_COUNT=$(cargo build 2>&1 | grep "error\[" | wc -l)
echo "✅ 当前错误数: $ERROR_COUNT"

echo "错误分布："
cd /home/sonygod/projects/nu/examples_cargo_restored/opensource_libs/log
cargo build 2>&1 | grep -E "^error:" | sort | uniq -c | sort -rn | head -5
