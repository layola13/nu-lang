// Nu to Rust Converter
// 将Nu代码转换回标准Rust代码

use anyhow::Result;

pub struct Nu2RustConverter {
    // 转换上下文 - 预留用于未来扩展
    #[allow(dead_code)]
    context: ConversionContext,
}

#[derive(Default)]
struct ConversionContext {
    // 跟踪当前作用域 - 预留用于未来扩展
    #[allow(dead_code)]
    in_trait: bool,
    #[allow(dead_code)]
    in_impl: bool,
}

impl Nu2RustConverter {
    pub fn new() -> Self {
        Self {
            context: ConversionContext::default(),
        }
    }

    pub fn convert(&self, nu_code: &str) -> Result<String> {
        let mut output = String::new();
        let lines: Vec<&str> = nu_code.lines().collect();

        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();

            // 保留空行和注释（不跳过）
            if line.is_empty() {
                output.push('\n');
                i += 1;
                continue;
            }

            // 保留注释行
            if line.starts_with("//") || line.starts_with("/*") || line.starts_with("*") {
                output.push_str(lines[i]);
                output.push('\n');
                i += 1;
                continue;
            }

            // 处理属性标记
            if line.starts_with("#D") {
                // 转换 #D(Debug) -> #[derive(Debug)]
                let attr_content = line.trim_start_matches("#D");
                output.push_str(&format!("#[derive{}]\n", attr_content));
                i += 1;
                continue;
            }

            // 其他属性（如 #[test], #[cfg(test)]）保持原样
            if line.starts_with("#[") && line.ends_with("]") {
                output.push_str(line);
                output.push('\n');
                i += 1;
                continue;
            }

            // 处理各种Nu语法
            if let Some(converted) = self.convert_line(line, &lines, &mut i)? {
                output.push_str(&converted);
                output.push('\n');
            }

            i += 1;
        }

        Ok(output)
    }

    fn convert_line(
        &self,
        line: &str,
        lines: &[&str],
        index: &mut usize,
    ) -> Result<Option<String>> {
        let trimmed = line.trim();

        // 函数定义: F/f (F=pub fn, f=fn)
        if trimmed.starts_with("F ") || trimmed.starts_with("f ") {
            return Ok(Some(self.convert_function(trimmed)?));
        }

        // 异步函数: ~F/~f
        if trimmed.starts_with("~F ") || trimmed.starts_with("~f ") {
            return Ok(Some(self.convert_async_function(trimmed)?));
        }

        // 结构体: S (v1.5.1: 移除了 s，只有 S)
        if trimmed.starts_with("S ") {
            return Ok(Some(self.convert_struct(trimmed, lines, index)?));
        }

        // 枚举: E (v1.5.1: 移除了 e，只有 E)
        if trimmed.starts_with("E ") {
            return Ok(Some(self.convert_enum(trimmed, lines, index)?));
        }

        // Trait: TR/tr
        if trimmed.starts_with("TR ") || trimmed.starts_with("tr ") {
            return Ok(Some(self.convert_trait(trimmed, lines, index)?));
        }

        // Impl块: I
        if trimmed.starts_with("I ") {
            return Ok(Some(self.convert_impl(trimmed, lines, index)?));
        }

        // 模块: D (v1.5.1: D=mod，不是M)
        if trimmed.starts_with("D ") {
            return Ok(Some(self.convert_module(trimmed)?));
        }

        // 变量声明: l/v
        if trimmed.starts_with("l ") {
            return Ok(Some(self.convert_let(trimmed)?));
        }
        if trimmed.starts_with("v ") {
            return Ok(Some(self.convert_let_mut(trimmed)?));
        }

        // Return语句: <
        if trimmed.starts_with("< ") || trimmed == "<" {
            return Ok(Some(self.convert_return(trimmed)?));
        }

        // Break语句: b
        if trimmed.starts_with("b ") || trimmed == "b" {
            return Ok(Some(self.convert_break(trimmed)?));
        }

        // Continue语句: c
        if trimmed.starts_with("c ") || trimmed == "c" {
            return Ok(Some(self.convert_continue(trimmed)?));
        }

        // Print语句: >
        if trimmed.starts_with("> ") {
            return Ok(Some(self.convert_print(trimmed)?));
        }

        // Use语句: u/U
        if trimmed.starts_with("u ") || trimmed.starts_with("U ") {
            return Ok(Some(self.convert_use(trimmed)?));
        }

        // Const: C
        if trimmed.starts_with("C ") {
            return Ok(Some(self.convert_const(trimmed)?));
        }

        // Static: ST
        if trimmed.starts_with("ST ") {
            return Ok(Some(self.convert_static(trimmed)?));
        }

        // 其他情况：转换类型和表达式
        Ok(Some(self.convert_expression(trimmed)?))
    }

    fn convert_function(&self, line: &str) -> Result<String> {
        let is_pub = line.starts_with("F ");
        let content = &line[2..];

        let visibility = if is_pub { "pub " } else { "" };
        let mut converted = self.convert_types_in_string(content);

        // 处理 !self -> mut self (按值接收的可变self)
        converted = converted.replace("(!self", "(mut self");

        Ok(format!("{}fn {}", visibility, converted))
    }

    fn convert_async_function(&self, line: &str) -> Result<String> {
        let is_pub = line.starts_with("~F ");
        let content = &line[3..];

        let visibility = if is_pub { "pub " } else { "" };
        let converted = self.convert_types_in_string(content);

        Ok(format!("{}async fn {}", visibility, converted))
    }

    fn convert_struct(&self, line: &str, _lines: &[&str], _index: &mut usize) -> Result<String> {
        // Nu v1.5.1: 只有 S（移除了 s）
        // 可见性由标识符首字母决定（Go风格）
        let content = &line[2..]; // 跳过 "S "
        let converted = self.convert_types_in_string(content);

        // 检查结构体名称的首字母是否大写来决定可见性
        let is_pub = content
            .trim()
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false);

        let visibility = if is_pub { "pub " } else { "" };
        Ok(format!("{}struct {}", visibility, converted))
    }

    fn convert_enum(&self, line: &str, _lines: &[&str], _index: &mut usize) -> Result<String> {
        // Nu v1.5.1: 只有 E（移除了 e）
        // 可见性由标识符首字母决定（Go风格）
        let content = &line[2..]; // 跳过 "E "
        let converted = self.convert_types_in_string(content);

        // 检查枚举名称的首字母是否大写来决定可见性
        let is_pub = content
            .trim()
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false);

        let visibility = if is_pub { "pub " } else { "" };
        Ok(format!("{}enum {}", visibility, converted))
    }

    fn convert_trait(&self, line: &str, _lines: &[&str], _index: &mut usize) -> Result<String> {
        let is_pub = line.starts_with("TR ");
        let content = &line[3..];

        let visibility = if is_pub { "pub " } else { "" };
        let converted = self.convert_types_in_string(content);

        Ok(format!("{}trait {}", visibility, converted))
    }

    fn convert_impl(&self, line: &str, _lines: &[&str], _index: &mut usize) -> Result<String> {
        let content = &line[2..];
        let converted = self.convert_types_in_string(content);
        Ok(format!("impl {}", converted))
    }

    fn convert_module(&self, line: &str) -> Result<String> {
        // Nu v1.5.1: D=mod (由标识符首字母决定可见性)
        let content = &line[2..]; // 跳过 "D "
        let converted = self.convert_types_in_string(content);

        // 检查模块名称的首字母是否大写来决定可见性
        let is_pub = content
            .trim()
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false);

        let visibility = if is_pub { "pub " } else { "" };
        Ok(format!("{}mod {}", visibility, converted))
    }

    fn convert_let(&self, line: &str) -> Result<String> {
        let content = &line[2..];
        let converted = self.convert_types_in_string(content);
        Ok(format!("let {}", converted))
    }

    fn convert_let_mut(&self, line: &str) -> Result<String> {
        let content = &line[2..];
        let converted = self.convert_types_in_string(content);
        Ok(format!("let mut {}", converted))
    }

    fn convert_return(&self, line: &str) -> Result<String> {
        if line == "<" {
            Ok("return".to_string())
        } else {
            let content = &line[2..];
            let converted = self.convert_types_in_string(content);
            Ok(format!("return {}", converted))
        }
    }

    fn convert_break(&self, line: &str) -> Result<String> {
        if line == "b" {
            Ok("break".to_string())
        } else {
            let content = &line[2..];
            let converted = self.convert_types_in_string(content);
            Ok(format!("break {}", converted))
        }
    }

    fn convert_continue(&self, line: &str) -> Result<String> {
        if line == "c" {
            Ok("continue".to_string())
        } else {
            let content = &line[2..];
            let converted = self.convert_types_in_string(content);
            Ok(format!("continue {}", converted))
        }
    }

    fn convert_print(&self, line: &str) -> Result<String> {
        let content = &line[2..];
        let converted = self.convert_types_in_string(content);
        Ok(format!("println!{}", converted))
    }

    fn convert_use(&self, line: &str) -> Result<String> {
        let is_pub = line.starts_with("U ");
        let content = &line[2..];

        let visibility = if is_pub { "pub " } else { "" };
        Ok(format!("{}use {}", visibility, content))
    }

    fn convert_const(&self, line: &str) -> Result<String> {
        let content = &line[2..];
        let converted = self.convert_types_in_string(content);
        Ok(format!("const {}", converted))
    }

    fn convert_static(&self, line: &str) -> Result<String> {
        let content = &line[3..];
        let converted = self.convert_types_in_string(content);
        Ok(format!("static {}", converted))
    }

    fn convert_expression(&self, line: &str) -> Result<String> {
        Ok(self.convert_types_in_string(line))
    }

    /// 转换Nu类型回Rust类型
    fn convert_types_in_string(&self, s: &str) -> String {
        // 第一步：先转换 $| -> move | (在保护闭包之前)
        let mut result = s.replace("$|", "move |");

        // 第二步：保护闭包参数和返回类型，避免单字母变量被误转换
        // 识别闭包模式: |param1, param2| 或 |params| -> RetType 或 move |...|
        let mut protected_closures = Vec::new();

        // 查找所有闭包参数列表（包括返回类型）
        let chars: Vec<char> = result.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '|' {
                let start = i;
                i += 1;
                // 找到匹配的闭包结束符 |
                while i < chars.len() && chars[i] != '|' {
                    i += 1;
                }
                if i < chars.len() {
                    i += 1; // 包含结束的 |

                    // v1.6: 检查是否有返回类型 -> Type
                    // 跳过空白
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    // 检查 ->
                    if i + 1 < chars.len() && chars[i] == '-' && chars[i + 1] == '>' {
                        i += 2;
                        // 跳过空白
                        while i < chars.len() && chars[i].is_whitespace() {
                            i += 1;
                        }
                        // 找到返回类型的结束（遇到 { 或 空格+非字母数字）
                        while i < chars.len() {
                            if chars[i] == '{' || chars[i] == ';' {
                                break;
                            }
                            i += 1;
                        }
                    }

                    let closure_signature: String = chars[start..i].iter().collect();
                    protected_closures.push(closure_signature);
                }
            } else {
                i += 1;
            }
        }

        // 用占位符替换闭包参数
        for (idx, closure) in protected_closures.iter().enumerate() {
            result = result.replacen(closure, &format!("__CLOSURE_PARAMS_{}__", idx), 1);
        }

        // 执行类型和关键字替换
        result = result
            .replace(" V ", " Vec ")
            .replace("V::", "Vec::")
            .replace(" V<", " Vec<")
            .replace("V<", "Vec<")
            .replace("O<", "Option<")
            .replace("R<", "Result<")
            .replace("A<", "Arc<")
            .replace("X<", "Mutex<")
            .replace("B<", "Box<")
            .replace("&!", "&mut ")
            .replace(".~", ".await")
            .replace("$|", "move |")
            .replace(" wh ", " where ") // wh -> where（只在有空格边界时）
            .replace(" I<", " impl<")
            .replace("I<", "impl<")
            .replace(")!", ")?");

        // Str -> String 精确替换：覆盖所有Str作为类型出现的模式
        result = result
            // 静态方法调用
            .replace("Str ::", "String::")
            .replace("Str::", "String::")
            // 泛型参数（元组和单个参数）
            .replace("(Str ", "(String ")
            .replace("(Str,", "(String,")
            .replace("(Str)", "(String)")
            .replace(", Str ", ", String ")
            .replace(", Str,", ", String,")
            .replace(", Str)", ", String)")
            .replace("< Str ", "<String ")
            .replace("< Str,", "<String,")
            .replace("< Str>", "<String>")
            .replace("<Str ", "<String ")
            .replace("<Str,", "<String,")
            .replace("<Str>", "<String>")
            // 类型标注
            .replace(": Str ", ": String ")
            .replace(": Str)", ": String)")
            .replace(": Str,", ": String,")
            .replace(": Str>", ": String>")
            .replace(": Str;", ": String;")
            // 返回类型
            .replace("-> Str ", "-> String ")
            .replace("-> Str)", "-> String)")
            .replace("-> Str{", "-> String{")
            .replace("-> Str;", "-> String;")
            // 引用类型
            .replace("& Str ", "&String ")
            .replace("& Str,", "&String,")
            .replace("& Str)", "&String)")
            .replace("& Str>", "&String>")
            .replace("&Str ", "&String ")
            .replace("&Str,", "&String,")
            .replace("&Str)", "&String)")
            .replace("&Str>", "&String>")
            // Vec/数组元素
            .replace("Str ]", "String]")
            .replace("Str]", "String]")
            // 结尾边界
            .replace("Str>", "String>")
            .replace("Str,", "String,")
            .replace("Str;", "String;")
            .replace("Str)", "String)")
            .replace("Str{", "String{")
            .replace("Str\n", "String\n")
            // 通用空格边界
            .replace(" Str ", " String ");

        // 恢复闭包参数（保持原样）
        for (idx, closure) in protected_closures.iter().enumerate() {
            result = result.replace(&format!("__CLOSURE_PARAMS_{}__", idx), closure);
        }

        // 移除函数调用中的多余空格: ") (" -> ")("
        result.replace(") (", ")(")
    }
}

impl Default for Nu2RustConverter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_function() {
        let converter = Nu2RustConverter::new();

        let nu_code = "F add(a: i32, b: i32) -> i32 {";
        let rust_code = converter.convert(nu_code).unwrap();
        assert!(rust_code.contains("pub fn add"));

        let nu_code2 = "f helper(x: i32) -> i32 {";
        let rust_code2 = converter.convert(nu_code2).unwrap();
        assert!(rust_code2.contains("fn helper"));
    }

    #[test]
    fn test_convert_struct() {
        let converter = Nu2RustConverter::new();

        let nu_code = "S Person {";
        let rust_code = converter.convert(nu_code).unwrap();
        assert!(rust_code.contains("pub struct Person"));
    }

    #[test]
    fn test_convert_types() {
        let converter = Nu2RustConverter::new();

        let nu_code = "l name: Str = \"test\".to_string();";
        let rust_code = converter.convert(nu_code).unwrap();
        assert!(rust_code.contains("let name: String"));
        assert!(rust_code.contains("to_string()"));
    }

    #[test]
    fn test_convert_variables() {
        let converter = Nu2RustConverter::new();

        let nu_code = "l x = 5;";
        let rust_code = converter.convert(nu_code).unwrap();
        assert!(rust_code.contains("let x = 5"));

        let nu_code2 = "v count = 0;";
        let rust_code2 = converter.convert(nu_code2).unwrap();
        assert!(rust_code2.contains("let mut count = 0"));
    }

    #[test]
    fn test_convert_return() {
        let converter = Nu2RustConverter::new();

        let nu_code = "< x + 1";
        let rust_code = converter.convert(nu_code).unwrap();
        assert!(rust_code.contains("return x + 1"));
    }
}
