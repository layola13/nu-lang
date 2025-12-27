#!/bin/bash
# Nu2TS输出修复脚本
# 用于修复已知的转换问题

echo "修复nu2ts生成的TypeScript文件..."

# 修复1: 闭包/lambda中缺少的闭合括号 - => expr; -> => expr);
find /tmp/nu2ts_test -name "*.ts" -type f -exec sed -i 's/=> \([a-zA-Z_][a-zA-Z0-9_. ]*\);$/=> \1);/g' {} \;

# 修复2: Some(desc) 转换后的多余括号 - = desc); -> = desc;
find /tmp/nu2ts_test -name "*.ts" -type f -exec sed -i 's/= \([a-zA-Z_][a-zA-Z0-9_]*\));/= \1;/g' {} \;

# 修复3: 元组解构语法 - const (a, b, c) -> const [a, b, c]  
find /tmp/nu2ts_test -name "*.ts" -type f -exec sed -i 's/const (\([^)]*\),/const [\1,/g' {} \;
find /tmp/nu2ts_test -name "*.ts" -type f -exec sed -i 's/const \[(\([^)]*\),/const [\1,/g' {} \;

# 修复4: for循环中的范围表达式 - 0..n -> Array.from({length: n}, (_, i) => i)
find /tmp/nu2ts_test -name "*.ts" -type f -exec sed -i 's/for (const \([a-z]\) of \([0-9]\+\)\.\.\([0-9]\+\))/for (const \1 of Array.from({length: \3}, (_, i) => i))/g' {} \;

# 修复5: 错误的类型注解语法 - { String: :new()} -> ""
find /tmp/nu2ts_test -name "*.ts" -type f -exec sed -i 's/{ String: :new()}/""/g' {} \;

# 修复6: 错误的元组类型 - [(String, usize)] -> Array<[string, number]>
find /tmp/nu2ts_test -name "*.ts" -type f -exec sed -i 's/\[(String, usize)\]/Array<[string, number]>/g' {} \;
find /tmp/nu2ts_test -name "*.ts" -type f -exec sed -i 's/\[(string, usize)\]/Array<[string, number]>/g' {} \;

echo "修复完成！"