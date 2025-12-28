// C++ Code Generator
// 从 Nu AST 生成现代 C++ 代码

use anyhow::{Result, anyhow};
use super::sourcemap::SourceMap;

pub struct CppCodeGenerator {
    indent_level: usize,
}

impl CppCodeGenerator {
    pub fn new() -> Self {
        Self { indent_level: 0 }
    }

    /// 生成 C++ 代码
    pub fn generate(&self, nu_code: &str, mut sourcemap: Option<&mut SourceMap>) -> Result<String> {
        let mut output = String::new();
        let lines: Vec<&str> = nu_code.lines().collect();
        
        let mut cpp_line = 1;
        
        for (nu_line_idx, line) in lines.iter().enumerate() {
            let nu_line = nu_line_idx + 1;
            let trimmed = line.trim();
            
            // 跳过空行和注释
            if trimmed.is_empty() {
                output.push('\n');
                cpp_line += 1;
                continue;
            }
            
            if trimmed.starts_with("//") {
                if let Some(ref mut sm) = sourcemap {
                    sm.add_mapping(cpp_line, nu_line);
                }
                output.push_str(line);
                output.push('\n');
                cpp_line += 1;
                continue;
            }
            
            // 记录映射
            if let Some(ref mut sm) = sourcemap {
                sm.add_mapping(cpp_line, nu_line);
            }
            
            // 转换各种 Nu 语法
            if let Some(converted) = self.convert_line(trimmed)? {
                let leading_whitespace: String = line.chars()
                    .take_while(|c| c.is_whitespace())
                    .collect();
                output.push_str(&leading_whitespace);
                output.push_str(&converted);
                output.push('\n');
                cpp_line += 1;
            }
        }
        
        Ok(output)
    }

    fn convert_line(&self, line: &str) -> Result<Option<String>> {
        // 函数定义: F/f
        if line.starts_with("F ") || line.starts_with("f ") {
            return Ok(Some(self.convert_function(line)?));
        }
        
        // 变量声明: l/v
        if line.starts_with("l ") {
            return Ok(Some(self.convert_let(line)?));
        }
        if line.starts_with("v ") {
            let after_v = &line[2..];
            if after_v.chars().next().map(|c| c.is_alphabetic() || c == '_').unwrap_or(false) {
                return Ok(Some(self.convert_let_mut(line)?));
            }
        }
        
        // Return: <
        if line.starts_with("< ") {
            return Ok(Some(self.convert_return(line)?));
        }
        
        // println! 宏
        if line.contains("println!") {
            return Ok(Some(self.convert_println(line)?));
        }
        
        // 结构体: S
        if line.starts_with("S ") {
            return Ok(Some(self.convert_struct(line)?));
        }
        
        // 枚举: E
        if line.starts_with("E ") {
            return Ok(Some(self.convert_enum(line)?));
        }
        
        // Impl: I
        if line.starts_with("I ") {
            return Ok(Some(self.convert_impl(line)?));
        }
        
        // 默认：转换类型和表达式
        Ok(Some(self.convert_expression(line)?))
    }

    fn convert_function(&self, line: &str) -> Result<String> {
        let is_pub = line.starts_with("F ");
        let content = &line[2..];
        
        // 解析函数签名
        // F add(a: i32, b: i32) -> i32 {
        // -> int32_t add(int32_t a, int32_t b) {
        
        let converted = self.convert_types(content);
        
        // 处理返回类型
        let result = if converted.contains("->") {
            // 有返回类型: name(params) -> RetType {
            let parts: Vec<&str> = converted.splitn(2, "->").collect();
            if parts.len() == 2 {
                let func_part = parts[0].trim();
                let ret_and_body = parts[1].trim();
                
                // 分离返回类型和函数体
                let ret_type = if let Some(brace_pos) = ret_and_body.find('{') {
                    ret_and_body[..brace_pos].trim()
                } else {
                    ret_and_body
                };
                
                let body_part = if let Some(brace_pos) = ret_and_body.find('{') {
                    &ret_and_body[brace_pos..]
                } else {
                    ""
                };
                
                format!("{} {} {}", ret_type, func_part, body_part)
            } else {
                converted
            }
        } else if converted.contains("main()") {
            // main 函数特殊处理
            converted.replace("main()", "main()")
                .replace("main() {", "main() {\n    return 0;\n}")
        } else {
            // 无返回类型: void
            format!("void {}", converted)
        };
        
        // 添加可见性（C++ 没有 pub 关键字，通过头文件控制）
        Ok(result)
    }

    fn convert_let(&self, line: &str) -> Result<String> {
        let content = &line[2..];
        let converted = self.convert_types(content);
        
        // l x: i32 = 5; -> int32_t x = 5;
        // 如果有类型注解，直接使用
        if converted.contains(':') {
            let result = converted.replace(":", "");
            Ok(result)
        } else {
            // 使用 auto
            Ok(format!("auto {}", converted))
        }
    }

    fn convert_let_mut(&self, line: &str) -> Result<String> {
        let content = &line[2..];
        let converted = self.convert_types(content);
        
        // C++ 中所有变量默认可变，不需要特殊标记
        if converted.contains(':') {
            let result = converted.replace(":", "");
            Ok(result)
        } else {
            Ok(format!("auto {}", converted))
        }
    }

    fn convert_return(&self, line: &str) -> Result<String> {
        let content = &line[2..];
        let converted = self.convert_types(content);
        Ok(format!("return {};", converted.trim_end_matches(';')))
    }

    fn convert_println(&self, line: &str) -> Result<String> {
        // println!("Hello") -> std::cout << "Hello" << std::endl;
        // println!("{}", x) -> std::cout << x << std::endl;
        
        if let Some(start) = line.find("println!(") {
            let after_println = &line[start + 9..];
            if let Some(end) = after_println.rfind(')') {
                let content = &after_println[..end];
                
                // 简单的字符串字面量
                if content.starts_with('"') && content.ends_with('"') {
                    return Ok(format!("std::cout << {} << std::endl;", content));
                }
                
                // 格式化输出
                if content.contains("\"{") || content.contains("{}")  {
                    // println!("{}", x) -> std::cout << x << std::endl;
                    let parts: Vec<&str> = content.split(',').collect();
                    if parts.len() >= 2 {
                        let vars = parts[1..].iter()
                            .map(|s| s.trim())
                            .collect::<Vec<_>>()
                            .join(" << \", \" << ");
                        return Ok(format!("std::cout << {} << std::endl;", vars));
                    }
                }
                
                // 默认：直接输出
                return Ok(format!("std::cout << {} << std::endl;", content));
            }
        }
        
        Ok(line.to_string())
    }

    fn convert_struct(&self, line: &str) -> Result<String> {
        let content = &line[2..];
        let converted = self.convert_types(content);
        
        // S Person { -> struct Person {
        Ok(format!("struct {}", converted))
    }

    fn convert_enum(&self, line: &str) -> Result<String> {
        let content = &line[2..];
        let converted = self.convert_types(content);
        
        // E Color { -> enum class Color {
        Ok(format!("enum class {}", converted))
    }

    fn convert_impl(&self, line: &str) -> Result<String> {
        // I Person { -> // impl Person (C++ 在结构体内定义方法)
        Ok(format!("// {}", line))
    }

    fn convert_expression(&self, line: &str) -> Result<String> {
        Ok(self.convert_types(line))
    }

    /// 转换 Nu 类型为 C++ 类型
    fn convert_types(&self, s: &str) -> String {
        s.replace("i32", "int32_t")
            .replace("i64", "int64_t")
            .replace("u32", "uint32_t")
            .replace("u64", "uint64_t")
            .replace("f32", "float")
            .replace("f64", "double")
            .replace("String", "std::string")
            .replace("str", "std::string_view")
            .replace("V<", "std::vector<")
            .replace("O<", "std::optional<")
            .replace("R<", "std::expected<")
            .replace("B<", "std::unique_ptr<")
            .replace("A<", "std::shared_ptr<")
    }
}

impl Default for CppCodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_types() {
        let gen = CppCodeGenerator::new();
        assert_eq!(gen.convert_types("i32"), "int32_t");
        assert_eq!(gen.convert_types("String"), "std::string");
        assert_eq!(gen.convert_types("V<i32>"), "std::vector<int32_t>");
    }

    #[test]
    fn test_convert_function() {
        let gen = CppCodeGenerator::new();
        let result = gen.convert_function("F add(a: i32, b: i32) -> i32 {").unwrap();
        assert!(result.contains("int32_t"));
        assert!(result.contains("add"));
    }
}