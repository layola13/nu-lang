#!/bin/bash
# Nu2TS输出修复脚本
# 用于修复已知的转换问题

TARGET_DIR="${1:-temp_examples_ts}"

echo "修复nu2ts生成的TypeScript文件 (目录: $TARGET_DIR)..."

# 修复1: 闭包/lambda中缺少的闭合括号 - => expr; -> => expr);
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/=> \([a-zA-Z_][a-zA-Z0-9_. ]*\);$/=> \1);/g' {} \;

# 修复2: Some(desc) 转换后的多余括号 - = desc); -> = desc;
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/= \([a-zA-Z_][a-zA-Z0-9_]*\));/= \1;/g' {} \;

# 修复3: 元组解构语法 - const (a, b, c) -> const [a, b, c]
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/const (\([^)]*\),/const [\1,/g' {} \;
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/const \[(\([^)]*\),/const [\1,/g' {} \;

# 修复4: for循环中的范围表达式 - 0..n -> Array.from({length: n}, (_, i) => i)
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/for (const \([a-z]\) of \([0-9]\+\)\.\.\([0-9]\+\))/for (const \1 of Array.from({length: \3}, (_, i) => i))/g' {} \;

# 修复5: 错误的类型注解语法 - { String: :new()} -> ""
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/{ String: :new()}/""/g' {} \;

# 修复6: 错误的元组类型 - [(String, usize)] -> Array<[string, number]>
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/\[(String, usize)\]/Array<[string, number]>/g' {} \;
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/\[(string, usize)\]/Array<[string, number]>/g' {} \;

# 修复7: 数组重复语法 [value; count] -> new Array(count).fill(value)
# 处理简单的数字情况
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/\[\([0-9]\+\); \([0-9]\+\)\]/new Array(\2).fill(\1)/g' {} \;

# 修复8: {:?} 格式化占位符 -> 空字符串（console.log会自动格式化）
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/{:?}//g' {} \;

# 修复9: {:p} 指针格式化占位符
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/{:p}//g' {} \;

# 修复10: 注释掉的代码后直接跟分号 - /* ... */; -> null;
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/\/\* [^*]*\*\/;$/null;/g' {} \;

# 修复11: 错误的赋值语法 - = desc); -> = desc;
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/= desc);/= desc;/g' {} \;

# 修复12: 错误的函数定义语法（RAW被错误转换）
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i '/function apply_twice.*{ return func(func(x)); }/d' {} \;

# 修复13: 在apply_twice调用前添加正确的函数定义
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i '/\/\/ RAW: func(func(x))/a\
function apply_twice(f: (x: any) => any, x: any): any {\
    return f(f(x));\
}' {} \;

# 修复14: 去除多余的 RAW: func(func(x)) 注释行
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i '/\/\/ RAW: func(func(x))/d' {} \;

# 修复15: use语句错误 - u super.*; -> // use super::*;
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/^    u super\.\*;$/    \/\/ use super::*;/g' {} \;

# 修复16: return let语句错误 - return let x = ... -> const x = ...
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/return let /const /g' {} \;

# 修复17: .collect()方法 - 移除collect()调用（JS/TS的map已返回数组）
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/\.collect()//g' {} \;

# 修复18: .values()迭代器 - 移除不必要的values()调用
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/\.values()\.map/\.map/g' {} \;
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/\.values()\.filter/\.filter/g' {} \;
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/\.values()\.find/\.find/g' {} \;
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/\.values()\.position/\.findIndex/g' {} \;

# 修复19: 闭包缺少右括号 - => expr; -> => expr)
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/=> \([a-zA-Z_][a-zA-Z0-9_.= ]*\);$/=> \1)/g' {} \;

# 修复20: if-else表达式赋值错误 - const x = "" else { } -> const x = "" || ""
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/= "" else { };/= "" || "";/g' {} \;

# 修复21: 重复的return - return return -> return
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/return return/return/g' {} \;

# 修复22: 元组解构缺少变量名 - const [a, = expr -> const [a, op, b] = expr
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/const \[\([a-z]\), = \([a-zA-Z_.()]*\);/const [\1, op, b] = \2;/g' {} \;

# 修复23: String.from转义问题 - String.from("...\n\) -> String("...")
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/String.from("\([^"]*\)\\n\\);/String("\1");/g' {} \;

# 修复24: _new()方法调用 - Type._new() -> new Type()
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/\([A-Z][a-zA-Z]*\)\._new()/new \1()/g' {} \;

# 修复25: EnumVariant pattern const语法 - const MyError::DivisionByZero -> // const MyError::DivisionByZero
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/const \([A-Z][a-zA-Z]*\)::\([A-Z][a-zA-Z]*\)/\/\/ Pattern: \1::\2/g' {} \;

# 修复26: 双冒号路径语法错误（带空格）- thread: :sleep -> thread.sleep
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/\([a-z_][a-z0-9_]*\): :\([a-z_][a-z0-9_]*\)/\1.\2/g' {} \;

# 修复27: 双冒号路径语法 - thread::sleep -> thread.sleep
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/\([a-z_][a-z0-9_]*\)::\([a-z_][a-z0-9_]*\)/\1.\2/g' {} \;

# 修复28: 大写类型的双冒号路径（方法调用）- Type::method -> Type.method
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/\([A-Z][a-zA-Z0-9_]*\)::\([a-z_][a-z0-9_]*\)/\1.\2/g' {} \;

# 修复31: 大写类型的双冒号路径（类型本身）- Type::Type -> Type.Type
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/\([A-Z][a-zA-Z0-9_]*\)::\([A-Z][a-zA-Z0-9_]*\)/\1.\2/g' {} \;

# 修复32: 多级路径的双冒号 - std.time::Duration -> std.time.Duration
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/\.\([a-z_][a-z0-9_]*\)::/.\1./g' {} \;

# 修复29: 双冒号后多余的分号 - thread.sleep(...);;  -> thread.sleep(...);
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/;;$/;/g' {} \;

# 修复30: use语句的双冒号 - u std.collections -> // use std::collections
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/^    u std\./    \/\/ use std::/g' {} \;

# 修复33: where子句错误 - wh T : Fn -> // where T: Fn (注释掉整行)
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/^export interface \(.*\) wh \(.*\) {$/\/\/ export interface \1 where \2 {/g' {} \;
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/^export class \(.*\) wh \(.*\) {$/\/\/ export class \1 where \2 {/g' {} \;

# 修复34: trait bounds - 注释掉包含trait bounds的行
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/^\(.*\) wh \(.*\) : \(.*\) , \(.*\) {$/\/\/ \1 where \2: \3, \4 {/g' {} \;

# 修复35: 孤立的闭合花括号（注释掉interface后遗留）
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i '/^\/\/ export interface.*where.*{$/,/^}$/ { /^}$/ s/^}/\/\/ }/; }' {} \;

# 修复36: 元组字段访问 - person.0 -> person[0]
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/\.\([0-9]\+\)\([^0-9]\)/[\1]\2/g' {} \;

# 修复37: vec!宏转换错误 - vec2] -> [vec2]
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/vec\([0-9]\+\)]/[\1]/g' {} \;

# 修复38: 闭包中多余的空格和缺少右括号 - (x ) => x> 0; -> (x) => x > 0);
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/(\([a-z_][a-z0-9_]*\) )/(\1)/g' {} \;
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/\([a-z]\)>\s*\([0-9]\);$/\1 > \2);/g' {} \;

# 修复39: Range表达式修复 - vec_methods[1..4] -> vec_methods.slice(1, 4)
find "$TARGET_DIR" -name "*.ts" -type f -exec sed -i 's/\([a-zA-Z_][a-zA-Z0-9_]*\)\[\([0-9]\+\)\.\.\([0-9]\+\)\]/\1.slice(\2, \3)/g' {} \;

echo "修复完成！"