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
    // 跟踪当前作用域
    in_trait: bool,
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
        let mut context = ConversionContext::default();

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
            if let Some(converted) = self.convert_line(line, &lines, &mut i, &mut context)? {
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
        context: &mut ConversionContext,
    ) -> Result<Option<String>> {
        let trimmed = line.trim();

        // Loop: L (必须在函数定义之前检查，避免 "L {" 被误判为函数)
        if trimmed.starts_with("L ") || trimmed == "L {" {
            return Ok(Some(self.convert_loop(trimmed)?));
        }

        // If: ?
        if trimmed.starts_with("? ") {
            return Ok(Some(self.convert_if(trimmed)?));
        }

        // Match: M
        if trimmed.starts_with("M ") {
            return Ok(Some(self.convert_match(trimmed)?));
        }

        // 函数定义: F/f (F=pub fn, f=fn)
        if trimmed.starts_with("F ") || trimmed.starts_with("f ") {
            return Ok(Some(self.convert_function(trimmed, context)?));
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
            context.in_impl = true;
            return Ok(Some(self.convert_impl(trimmed, lines, index)?));
        }

        // 检测impl块结束
        if trimmed == "}" && context.in_impl {
            context.in_impl = false;
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

        // Break语句: br 或 br;
        if trimmed.starts_with("br;") || trimmed.starts_with("br ") || trimmed == "br" {
            return Ok(Some(self.convert_break(trimmed)?));
        }

        // Continue语句: ct 或 ct;
        if trimmed.starts_with("ct;") || trimmed.starts_with("ct ") || trimmed == "ct" {
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

    fn convert_function(&self, line: &str, context: &ConversionContext) -> Result<String> {
        let is_pub = line.starts_with("F ");
        let content = &line[2..];

        // 在trait或impl块内，不添加pub修饰符（trait方法不能有pub）
        let visibility = if is_pub && !context.in_impl && !context.in_trait {
            "pub "
        } else {
            ""
        };
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
        // 先转换关键字，再转换类型
        let converted = self.convert_inline_keywords(content)?;
        let converted = self.convert_types_in_string(&converted);
        Ok(format!("let {}", converted))
    }

    fn convert_let_mut(&self, line: &str) -> Result<String> {
        let content = &line[2..];
        // 先转换关键字，再转换类型
        let converted = self.convert_inline_keywords(content)?;
        let converted = self.convert_types_in_string(&converted);
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
        if line == "br" || line == "br;" || line.starts_with("br;") {
            Ok("break;".to_string())
        } else {
            let content = &line[3..];
            let converted = self.convert_types_in_string(content);
            Ok(format!("break {}", converted))
        }
    }

    fn convert_continue(&self, line: &str) -> Result<String> {
        if line == "ct" || line == "ct;" || line.starts_with("ct;") {
            Ok("continue;".to_string())
        } else {
            let content = &line[3..];
            let converted = self.convert_types_in_string(content);
            Ok(format!("continue {}", converted))
        }
    }

    fn convert_loop(&self, line: &str) -> Result<String> {
        if line == "L {" {
            Ok("loop {".to_string())
        } else if let Some(content) = line.strip_prefix("L ") {
            // 先检查是否以 { 开头 - 这是 loop
            if content.starts_with("{") {
                // L { ... } - 无限循环
                let body = &content[1..]; // 跳过 {
                let converted_body = self.convert_inline_keywords(body)?;
                let converted_body = self.convert_types_in_string(&converted_body);
                Ok(format!("loop {{ {}", converted_body))
            } else if content.contains(" in ") || content.contains(": ") {
                // 检查是否是for循环（L var in iter 或 L var: iter）
                let brace_pos = content.find('{');
                let in_pos = content.find(" in ");
                let colon_pos = content.find(": ");
                
                // 找到最早出现的分隔符位置
                let separator_pos = match (in_pos, colon_pos) {
                    (Some(in_p), Some(c_p)) => Some(in_p.min(c_p)),
                    (Some(in_p), None) => Some(in_p),
                    (None, Some(c_p)) => Some(c_p),
                    _ => None,
                };
                
                let is_for_loop = match (separator_pos, brace_pos) {
                    (Some(sep_p), Some(brace_p)) => sep_p < brace_p, // 分隔符在 { 之前
                    (Some(_), None) => true, // 有分隔符但没有 {
                    _ => false,
                };
                
                if is_for_loop {
                    // for 循环: L var in iter { ... } 或 L var: iter { ... }
                    // 将 : 替换为 in
                    let normalized = content.replace(": ", " in ");
                    let converted = self.convert_inline_keywords(&normalized)?;
                    let converted = self.convert_types_in_string(&converted);
                    Ok(format!("for {}", converted))
                } else {
                    // loop { ... 中包含 for ... }
                    let body = content;
                    let converted_body = self.convert_inline_keywords(body)?;
                    let converted_body = self.convert_types_in_string(&converted_body);
                    Ok(format!("loop {{ {}", converted_body))
                }
            } else {
                // 无限循环
                Ok("loop {".to_string())
            }
        } else {
            Ok(line.to_string())
        }
    }

    fn convert_if(&self, line: &str) -> Result<String> {
        let content = &line[2..]; // 跳过 "? "
        
        // 检查是否是三元表达式: ? condition { value } else { value }
        // 这种情况下不应该添加 "if"，而是直接转换为 "if condition { value } else { value }"
        let trimmed = content.trim();
        if !trimmed.starts_with("let ") && !trimmed.starts_with("if ") {
            // 可能是三元表达式，检查是否有 { ... } else { ... } 模式
            if trimmed.contains('{') && trimmed.contains("} else {") {
                // 三元表达式，直接转换
                let converted = self.convert_inline_keywords(content)?;
                let converted = self.convert_types_in_string(&converted);
                return Ok(format!("if {}", converted));
            }
        }
        
        // 递归处理if语句内容
        let converted = self.convert_inline_keywords(content)?;
        let converted = self.convert_types_in_string(&converted);
        Ok(format!("if {}", converted))
    }

    fn convert_match(&self, line: &str) -> Result<String> {
        let content = &line[2..]; // 跳过 "M "
        // 递归处理match语句内容
        let converted = self.convert_inline_keywords(content)?;
        let converted = self.convert_types_in_string(&converted);
        Ok(format!("match {}", converted))
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
        // 递归处理表达式中的关键字
        let result = self.convert_inline_keywords(line)?;
        // 必须调用convert_types_in_string来转换&!等类型修饰符
        let result = self.convert_types_in_string(&result);
        Ok(result)
    }

    /// 递归转换行内的Nu关键字 (用于单行中的多个语句)
    fn convert_inline_keywords(&self, content: &str) -> Result<String> {
        let mut result = String::new();
        let mut i = 0;
        let chars: Vec<char> = content.chars().collect();

        while i < chars.len() {
            // 跳过空白
            while i < chars.len() && chars[i].is_whitespace() {
                result.push(chars[i]);
                i += 1;
            }

            if i >= chars.len() {
                break;
            }

            // 检查关键字
            let remaining: String = chars[i..].iter().collect();

            // break: br 或 br; (使用双字母避免与变量b冲突)
            if remaining.starts_with("br;") || remaining.starts_with("br ") || remaining == "br" {
                let is_start_boundary = i == 0 || (!chars[i-1].is_alphanumeric() && chars[i-1] != '_');
                if is_start_boundary {
                    if remaining.starts_with("br;") {
                        result.push_str("break;");
                        i += 3;
                    } else if remaining.starts_with("br ") || remaining == "br" {
                        result.push_str("break");
                        i += 2;
                    }
                    continue;
                }
            }

            // continue: ct 或 ct; (使用双字母避免与变量c冲突)
            if remaining.starts_with("ct;") || remaining.starts_with("ct ") || remaining == "ct" {
                let is_start_boundary = i == 0 || (!chars[i-1].is_alphanumeric() && chars[i-1] != '_');
                if is_start_boundary {
                    if remaining.starts_with("ct;") {
                        result.push_str("continue;");
                        i += 3;
                    } else if remaining.starts_with("ct ") || remaining == "ct" {
                        result.push_str("continue");
                        i += 2;
                    }
                    continue;
                }
            }

            // if: ? (检查是否是三元表达式的开始)
            if remaining.starts_with("? ") {
                result.push_str("if ");
                i += 2;
                continue;
            }

            // match: M (需要确保不是其他标识符的一部分)
            // M必须前后都有明确的边界
            if remaining.starts_with("M ") {
                let has_start_boundary = i == 0 || (!chars[i-1].is_alphanumeric() && chars[i-1] != '_');
                if has_start_boundary {
                    result.push_str("match ");
                    i += 2;
                    continue;
                }
            }

            // loop: L { (必须在 for 之前检查)
            if remaining.starts_with("L {") {
                result.push_str("loop {");
                i += 3;
                continue;
            }

            // for: L ... in (检查直到下一个花括号前是否有 in)
            if remaining.starts_with("L ") {
                // 查找下一个 { 的位置
                let mut j = i + 2;
                let mut found_in = false;
                while j < chars.len() && chars[j] != '{' {
                    if j + 3 < chars.len()
                        && chars[j] == ' '
                        && chars[j+1] == 'i'
                        && chars[j+2] == 'n'
                        && chars[j+3] == ' ' {
                        found_in = true;
                        break;
                    }
                    j += 1;
                }
                
                if found_in {
                    result.push_str("for ");
                    i += 2;
                    continue;
                }
            }

            // 默认情况：复制字符
            result.push(chars[i]);
            i += 1;
        }

        Ok(result)
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

        // v1.7: String 不再缩写为 Str，因此无需 Str -> String 转换
        // 直接进行其他类型和关键字替换
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
            .replace(" wh ", " where ")
            .replace(" I<", " impl<")
            .replace("I<", "impl<")
            .replace(")!", ")?");

        // V! -> vec! (宏调用)
        result = result
            .replace("V![", "vec![")
            .replace("V! [", "vec![");

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

        // v1.7: String 不再缩写为 Str
        let nu_code = "l name: String = \"test\".to_string();";
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
