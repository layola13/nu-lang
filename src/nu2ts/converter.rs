// Nu to TypeScript Converter
// 将Nu代码转换为TypeScript代码（v1.6.2 AST架构）

use super::codegen::TsCodegen;
use super::parser::Parser;
use super::runtime::{generate_micro_runtime, generate_runtime_import};
use super::types::{ConversionContext, RuntimeMode, TsConfig};
use anyhow::{Context, Result};

pub struct Nu2TsConverter {
    config: TsConfig,
}

impl Nu2TsConverter {
    pub fn new(config: TsConfig) -> Self {
        Self { config }
    }

    pub fn with_config(config: TsConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self {
            config: TsConfig::default(),
        }
    }

    pub fn config(&self) -> &TsConfig {
        &self.config
    }

    /// 主转换方法：将Nu代码转换为TypeScript（使用AST架构）
    pub fn convert(&self, nu_code: &str) -> Result<String> {
        // 1. 解析 Nu 代码为 AST
        let mut parser = Parser::new(nu_code);
        let file = parser.parse_file().context("Failed to parse Nu code")?;

        // 2. 生成 TypeScript 代码
        let mut codegen = TsCodegen::new(self.config.clone());
        let ts_code = codegen
            .generate_file(&file)
            .context("Failed to generate TypeScript code")?;

        Ok(ts_code)
    }

    /// 旧版转换方法（兼容性保留）
    pub fn convert_legacy(&self, nu_code: &str) -> Result<String> {
        let mut output = String::new();
        let mut context = ConversionContext::default();

        // 添加运行时（如果配置为内联模式）
        if self.config.runtime_mode == RuntimeMode::Inline {
            output.push_str(generate_micro_runtime());
            output.push('\n');
        } else {
            output.push_str(generate_runtime_import());
            output.push('\n');
        }

        let lines: Vec<&str> = nu_code.lines().collect();

        // 第一遍：收集所有结构体、枚举和impl块定义
        self.collect_definitions(&lines, &mut context)?;

        // 第二遍：生成代码
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            // 保留空行
            if trimmed.is_empty() {
                output.push('\n');
                i += 1;
                continue;
            }

            // 保留注释行
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
                output.push_str(line);
                output.push('\n');
                i += 1;
                continue;
            }

            // 处理属性标记
            if trimmed.starts_with("#D") || trimmed.starts_with("#[") {
                output.push_str(&format!("// {}\n", trimmed));
                i += 1;
                continue;
            }

            // 处理E、S、I块
            if trimmed.starts_with("E ") {
                // 枚举：生成完整定义并跳过内容
                if let Some(converted) = self.convert_line(trimmed, &lines, &mut i, &mut context)? {
                    output.push_str(&converted);
                    output.push('\n');
                }
                // 跳过枚举块内容
                let mut brace_count = if trimmed.contains('{') { 1 } else { 0 };
                if brace_count == 0 {
                    i += 1;
                    if i < lines.len() && lines[i].trim() == "{" {
                        brace_count = 1;
                    }
                }
                if brace_count > 0 {
                    i += 1;
                    while i < lines.len() && brace_count > 0 {
                        let l = lines[i].trim();
                        brace_count += l.matches('{').count();
                        brace_count = brace_count.saturating_sub(l.matches('}').count());
                        i += 1;
                    }
                    i -= 1;
                }
                i += 1;
                continue;
            }

            if trimmed.starts_with("S ") {
                // 结构体：只输出注释，跳过内容
                if let Some(converted) = self.convert_line(trimmed, &lines, &mut i, &mut context)? {
                    output.push_str(&converted);
                    output.push('\n');
                }
                // 跳过struct块内容
                let mut brace_count = if trimmed.contains('{') { 1 } else { 0 };
                if brace_count == 0 {
                    i += 1;
                    if i < lines.len() && lines[i].trim() == "{" {
                        brace_count = 1;
                    }
                }
                if brace_count > 0 {
                    i += 1;
                    while i < lines.len() && brace_count > 0 {
                        let l = lines[i].trim();
                        brace_count += l.matches('{').count();
                        brace_count = brace_count.saturating_sub(l.matches('}').count());
                        i += 1;
                    }
                    i -= 1;
                }
                i += 1;
                continue;
            }

            if trimmed.starts_with("I ") {
                // impl块：生成class头部（如果有对应struct）或namespace（如果是enum）
                if let Some(converted) = self.convert_line(trimmed, &lines, &mut i, &mut context)? {
                    output.push_str(&converted);
                    output.push('\n');
                }
                // 对于struct的impl（in_impl=true）或enum的impl（in_enum_impl=true），不跳过内容
                if context.in_impl || context.in_enum_impl {
                    // 让后续的函数定义自然地作为方法被处理
                    i += 1;
                    continue;
                } else {
                    // 对于既不是struct也不是enum的impl，跳过内容
                    let mut brace_count = if trimmed.contains('{') { 1 } else { 0 };
                    if brace_count == 0 {
                        i += 1;
                        if i < lines.len() && lines[i].trim() == "{" {
                            brace_count = 1;
                        }
                    }
                    if brace_count > 0 {
                        i += 1;
                        while i < lines.len() && brace_count > 0 {
                            let l = lines[i].trim();
                            brace_count += l.matches('{').count();
                            brace_count = brace_count.saturating_sub(l.matches('}').count());
                            i += 1;
                        }
                        i -= 1;
                    }
                    i += 1;
                    continue;
                }
            }

            // 检测并转换 match 块
            if trimmed.starts_with("M ") {
                // 解析 Match 语句
                match self.parse_match_statement(&lines, i) {
                    Ok(match_ast) => {
                        // 生成 TypeScript if-chain
                        match self.generate_match_ifchain(&match_ast, &mut context) {
                            Ok(converted) => {
                                output.push_str(&converted);
                                output.push('\n');
                            }
                            Err(e) => {
                                // 生成失败，输出注释
                                output.push_str(&format!("// Match conversion failed: {}\n", e));
                                output.push_str(&format!("// {}\n", trimmed));
                            }
                        }
                        // 跳到 match 块结束
                        i = match_ast.end_line;
                        i += 1;
                        continue;
                    }
                    Err(e) => {
                        // 解析失败，跳过并输出注释
                        output.push_str(&format!("// Match parsing failed: {}\n", e));
                        output.push_str(&format!("// {}\n", trimmed));

                        // 跳过整个match块
                        let mut brace_count = 0;
                        let mut found_open = false;
                        while i < lines.len() {
                            let l = lines[i].trim();
                            if l.contains("{") {
                                found_open = true;
                                brace_count += l.matches('{').count();
                            }
                            if l.contains("}") {
                                brace_count = brace_count.saturating_sub(l.matches('}').count());
                            }
                            output.push_str(&format!("// {}\n", lines[i]));
                            i += 1;
                            if found_open && brace_count == 0 {
                                break;
                            }
                        }
                        continue;
                    }
                }
            }

            // 处理各种Nu语法
            if let Some(converted) = self.convert_line(line, &lines, &mut i, &mut context)? {
                output.push_str(&converted);
                output.push('\n');
            }

            i += 1;
        }

        // 检查是否有main函数，如果有则自动调用
        if output.contains("function main(") {
            output.push_str("\n// Auto-call main function\nmain();\n");
        }

        Ok(output)
    }

    /// 第一遍：收集所有定义（枚举、结构体、impl块）
    fn collect_definitions(&self, lines: &[&str], context: &mut ConversionContext) -> Result<()> {
        let mut i = 0;
        while i < lines.len() {
            let trimmed = lines[i].trim();

            // 收集枚举定义
            if trimmed.starts_with("E ") {
                self.collect_enum_definition(lines, &mut i, context)?;
            }
            // 收集结构体定义
            else if trimmed.starts_with("S ") {
                self.collect_struct_definition(lines, &mut i, context)?;
            }
            // 收集impl块
            else if trimmed.starts_with("I ") {
                self.collect_impl_definition(lines, &mut i, context)?;
            }

            i += 1;
        }
        Ok(())
    }

    fn convert_line(
        &self,
        line: &str,
        _lines: &[&str],
        _index: &mut usize,
        context: &mut ConversionContext,
    ) -> Result<Option<String>> {
        let trimmed = line.trim();

        // Loop: L
        if trimmed.starts_with("L ") || trimmed == "L {" {
            return Ok(Some(self.convert_loop(trimmed, context)?));
        }

        // If: ?
        if trimmed.starts_with("? ") {
            return Ok(Some(self.convert_if(trimmed, context)?));
        }

        // Match: M
        if trimmed.starts_with("M ") {
            return Ok(Some(self.convert_match_stmt(trimmed, context)?));
        }

        // 函数定义: F/f
        if trimmed.starts_with("F ") || trimmed.starts_with("f ") {
            context.in_function = true;
            context.reset_temp_counter();
            return Ok(Some(self.convert_function(trimmed, context)?));
        }

        // 异步函数: ~F/~f
        if trimmed.starts_with("~F ") || trimmed.starts_with("~f ") {
            context.in_function = true;
            context.reset_temp_counter();
            return Ok(Some(self.convert_async_function(trimmed, context)?));
        }

        // 结构体: S
        if trimmed.starts_with("S ") {
            return Ok(Some(self.convert_struct(trimmed, context)?));
        }

        // 枚举: E
        if trimmed.starts_with("E ") {
            return Ok(Some(self.convert_enum(trimmed, context)?));
        }

        // Trait: TR/tr
        if trimmed.starts_with("TR ") || trimmed.starts_with("tr ") {
            context.in_trait = true;
            return Ok(Some(self.convert_trait(trimmed)?));
        }

        // Impl块: I
        if trimmed.starts_with("I ") {
            context.in_impl = true;
            return Ok(Some(self.convert_impl(trimmed, context)?));
        }

        // 检测块结束
        if trimmed == "}" {
            // 只有顶级的}（没有缩进或少量缩进）才关闭impl
            let indent_level = line.len() - line.trim_start().len();

            if context.in_impl && indent_level == 0 {
                // 这是impl块的结束（struct的class）
                context.in_impl = false;
                context.current_class = None;
                context.current_impl = None;
                // 返回结束的大括号以关闭class
                return Ok(Some("}".to_string()));
            }
            if context.in_enum_impl && indent_level == 0 {
                // 这是enum impl namespace的结束
                context.in_enum_impl = false;
                context.current_impl_enum = None;
                context.current_impl = None;
                // 返回结束的大括号以关闭namespace
                return Ok(Some("}".to_string()));
            }
            if context.in_function {
                context.in_function = false;
                context.reset_temp_counter();
            }
        }

        // 模块: D
        if trimmed.starts_with("D ") {
            return Ok(Some(self.convert_module(trimmed)?));
        }

        // 变量声明: l/v
        if trimmed.starts_with("l ") {
            return Ok(Some(self.convert_let(trimmed, context)?));
        }
        if trimmed.starts_with("v ") {
            return Ok(Some(self.convert_let_mut(trimmed, context)?));
        }

        // Return语句: <
        if trimmed.starts_with("< ") || trimmed == "<" {
            return Ok(Some(self.convert_return(trimmed, context)?));
        }

        // Break/Continue语句
        if trimmed.starts_with("br") {
            return Ok(Some("break;".to_string()));
        }
        if trimmed.starts_with("ct") {
            return Ok(Some("continue;".to_string()));
        }

        // Print语句: >
        if trimmed.starts_with("> ") {
            return Ok(Some(self.convert_print(trimmed, context)?));
        }

        // Use语句: u/U
        if trimmed.starts_with("u ") || trimmed.starts_with("U ") {
            return Ok(Some(self.convert_use(trimmed)?));
        }

        // Const: C
        if trimmed.starts_with("C ") {
            return Ok(Some(self.convert_const(trimmed, context)?));
        }

        // Static: ST
        if trimmed.starts_with("ST ") {
            return Ok(Some(self.convert_static(trimmed, context)?));
        }

        // 其他情况：转换表达式
        // 如果在函数中且是简单表达式（可能是返回值），添加return
        let mut converted = self.convert_expression(trimmed, context)?;

        if context.in_function
            && !trimmed.is_empty()
            && !converted.starts_with("const ")
            && !converted.starts_with("let ")
            && !converted.starts_with("if ")
            && !converted.starts_with("for ")
            && !converted.starts_with("while ")
            && !converted.starts_with("return ")
            && !converted.starts_with("console.log")
            && !converted.starts_with("throw ")
            && !converted.starts_with("switch ")
            && !converted.starts_with("// ")
            && !converted.contains(" = ")
            && !trimmed.ends_with("{")
            && !trimmed.ends_with("}")
            && !trimmed.ends_with(";")
        {
            // 简单的二元运算或变量引用，可能是返回值
            if converted.contains(" + ")
                || converted.contains(" - ")
                || converted.contains(" * ")
                || converted.contains(" / ")
                || (converted.chars().all(|c| c.is_alphanumeric() || c == '_')
                    && !converted.is_empty())
            {
                converted = format!("return {}", converted);
            }
        }

        Ok(Some(converted))
    }

    // ============ 类型转换 ============

    fn convert_type(&self, nu_type: &str) -> String {
        let trimmed = nu_type.trim();

        // 首先处理引用类型（包括 &str, &mut str 等）
        if trimmed.starts_with("&mut ") {
            return self.convert_type(&trimmed[5..]);
        }
        if trimmed.starts_with("&") {
            return self.convert_type(&trimmed[1..]);
        }

        match trimmed {
            // 基础类型
            "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "isize" | "usize" => {
                "number"
            }
            "f32" | "f64" => "number",
            "bool" => "boolean",
            "char" => "string",
            "String" | "str" => "string",
            "()" => "void",

            // 智能指针直接擦除
            _ if trimmed.starts_with("Box<") => {
                let inner = &trimmed[4..trimmed.len() - 1];
                return self.convert_type(inner);
            }
            _ if trimmed.starts_with("Arc<") => {
                let inner = &trimmed[4..trimmed.len() - 1];
                return self.convert_type(inner);
            }
            _ if trimmed.starts_with("Mutex<") => {
                let inner = &trimmed[6..trimmed.len() - 1];
                return self.convert_type(inner);
            }

            // 容器类型
            _ if trimmed.starts_with("Vec<") || trimmed.starts_with("V<") => {
                let start = if trimmed.starts_with("Vec<") { 4 } else { 2 };
                let inner = &trimmed[start..trimmed.len() - 1];
                return format!("Array<{}>", self.convert_type(inner));
            }

            _ if trimmed.starts_with("Option<") || trimmed.starts_with("O<") => {
                let start = if trimmed.starts_with("Option<") { 7 } else { 2 };
                let inner = &trimmed[start..trimmed.len() - 1];
                return format!("{} | null", self.convert_type(inner));
            }

            _ if trimmed.starts_with("Result<") || trimmed.starts_with("R<") => {
                let start = if trimmed.starts_with("Result<") { 7 } else { 2 };
                // 提取尖括号内的内容
                let inner = &trimmed[start..trimmed.len() - 1];
                return format!("Result<{}>", self.convert_type_params(inner));
            }

            _ if trimmed.starts_with("HashMap<") => {
                let inner = &trimmed[8..trimmed.len() - 1];
                return format!("Map<{}>", self.convert_type_params(inner));
            }

            _ if trimmed.starts_with("HashSet<") => {
                let inner = &trimmed[8..trimmed.len() - 1];
                return format!("Set<{}>", self.convert_type(inner));
            }

            // 元组类型
            _ if trimmed.starts_with("(") && trimmed.ends_with(")") && trimmed.contains(",") => {
                let inner = &trimmed[1..trimmed.len() - 1];
                let parts: Vec<&str> = inner.split(',').collect();
                let converted: Vec<String> =
                    parts.iter().map(|p| self.convert_type(p.trim())).collect();
                return format!("[{}]", converted.join(", "));
            }

            // 默认：保持原样
            _ => trimmed,
        }
        .to_string()
    }

    fn convert_type_params(&self, params: &str) -> String {
        // 移除多余空格，处理带空格的类型参数
        let cleaned = params
            .trim()
            .replace(" , ", ", ")
            .replace(" ,", ", ")
            .replace(",  ", ", ");

        // 分割参数并转换每个类型
        cleaned
            .split(',')
            .map(|p| self.convert_type(p.trim()))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn convert_types_in_string(&self, s: &str) -> String {
        let mut result = s.to_string();

        // 转换简写类型
        result = result
            .replace(" V<", " Array<")
            .replace("V<", "Array<")
            .replace(": i32", ": number")
            .replace(": i64", ": number")
            .replace(": u32", ": number")
            .replace(": u64", ": number")
            .replace(": f32", ": number")
            .replace(": f64", ": number")
            .replace(": f32", ": number")
            .replace(": f64", ": number")
            .replace(": bool", ": boolean")
            .replace(": String", ": string")
            .replace(": &str", ": string")
            .replace(":  str", ": string") // 处理空格情况
            .replace(": & str", ": string")
            .replace(" str ", " string ")
            .replace(" str>", " string>")
            .replace(" str)", " string)");

        result
    }

    // ============ 函数定义 ============

    fn convert_function(&self, line: &str, context: &ConversionContext) -> Result<String> {
        let is_pub = line.starts_with("F ");
        let content = &line[2..];

        if context.in_impl {
            // 在class的impl块内，生成方法（带缩进，无function关键字）
            let converted = self.convert_function_signature(content)?;
            Ok(format!("    {}", converted))
        } else if context.in_enum_impl {
            // 在enum的namespace内，生成导出的静态函数
            let converted = self.convert_function_signature(content)?;
            Ok(format!("    export function {}", converted))
        } else {
            // 全局函数
            let export = if is_pub { "export " } else { "" };
            let converted = self.convert_function_signature(content)?;
            Ok(format!("{}function {}", export, converted))
        }
    }

    fn convert_async_function(&self, line: &str, context: &ConversionContext) -> Result<String> {
        let is_pub = line.starts_with("~F ");
        let content = &line[3..];

        let export = if is_pub && !context.in_impl {
            "export "
        } else {
            ""
        };
        let mut converted = self.convert_function_signature(content)?;

        // 包装返回类型为Promise
        if let Some(arrow_pos) = converted.find("->") {
            let (params, ret_type) = converted.split_at(arrow_pos);
            let ret_type = ret_type[2..].trim();
            let ret_ts = self.convert_type(ret_type);
            converted = format!("{}: Promise<{}>", params.trim_end(), ret_ts);
        }

        Ok(format!("{}async function {}", export, converted))
    }

    fn convert_function_signature(&self, sig: &str) -> Result<String> {
        let mut result = sig.to_string();

        // 移除尾部的大括号（如果存在）
        let has_brace = result.trim_end().ends_with('{');
        if has_brace {
            result = result
                .trim_end()
                .trim_end_matches('{')
                .trim_end()
                .to_string();
        }

        // 转换 self 参数 - 先处理带引用的self
        result = result
            .replace("(&mut self", "(this")
            .replace("(&self", "(this")
            .replace("(&!self", "(this")
            .replace("(self", "(this")
            .replace("(!self", "(this");

        // 清理剩余的引用符号（在参数中）
        // 使用更精确的替换，避免影响字符串内容
        let mut cleaned = String::new();
        let mut in_string = false;
        let mut chars = result.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '"' || ch == '\'' {
                in_string = !in_string;
                cleaned.push(ch);
            } else if !in_string && ch == '&' {
                // 跳过&符号（引用）
                // 如果后面是 "mut "，也跳过
                if chars.peek() == Some(&'m') {
                    let lookahead: String = chars.clone().take(4).collect();
                    if lookahead.starts_with("mut ") {
                        chars.next(); // m
                        chars.next(); // u
                        chars.next(); // t
                        chars.next(); // space
                    }
                } else if chars.peek() == Some(&' ') {
                    // 跳过 "& " 中的空格
                    chars.next();
                }
            } else {
                cleaned.push(ch);
            }
        }
        result = cleaned;

        // 转换返回类型箭头
        if let Some(arrow_pos) = result.find("->") {
            let (params, ret_type) = result.split_at(arrow_pos);
            let ret_type = ret_type[2..].trim();
            let ret_ts = self.convert_type(ret_type);
            result = format!("{}: {}", params.trim(), ret_ts);
        }

        result = self.convert_types_in_string(&result);

        // 添加开始的大括号
        if has_brace {
            result.push_str(" {");
        }

        Ok(result)
    }

    // ============ 结构体和类型定义 ============

    fn convert_struct(&self, line: &str, context: &mut ConversionContext) -> Result<String> {
        let content = &line[2..];

        let struct_name = content
            .split_whitespace()
            .next()
            .unwrap_or("")
            .split('{')
            .next()
            .unwrap_or("")
            .trim();

        context.current_class = Some(struct_name.to_string());

        // 不输出，等待impl块合并
        // 如果没有对应的impl块，会在convert_impl中处理
        Ok(format!("// struct {}", struct_name))
    }

    fn convert_enum(&self, line: &str, context: &ConversionContext) -> Result<String> {
        let content = &line[2..];

        // 提取枚举名
        let enum_name = content
            .split_whitespace()
            .next()
            .unwrap_or("")
            .split('{')
            .next()
            .unwrap_or("")
            .trim();

        // 从context中获取枚举信息
        if let Some(enum_info) = context.enums.get(enum_name) {
            return self.generate_enum_output(enum_info);
        }

        // 兜底：如果没有收集到信息，使用原来的简单处理
        let is_pub = content
            .trim()
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false);

        let export = if is_pub { "export " } else { "" };
        let converted = self.convert_types_in_string(content);

        Ok(format!("{}type {}", export, converted))
    }

    fn convert_trait(&self, line: &str) -> Result<String> {
        let is_pub = line.starts_with("TR ");
        let content = &line[3..];

        let export = if is_pub { "export " } else { "" };
        let converted = self.convert_types_in_string(content);

        Ok(format!("{}interface {}", export, converted))
    }

    fn convert_impl(&self, line: &str, context: &mut ConversionContext) -> Result<String> {
        let content = &line[2..];

        let type_name = content
            .split_whitespace()
            .next()
            .unwrap_or("")
            .split('{')
            .next()
            .unwrap_or("")
            .trim();

        context.current_impl = Some(type_name.to_string());

        // 如果对应的struct存在，生成class头部+字段（不关闭大括号）
        if let Some(struct_info) = context.structs.get(type_name) {
            // 只有在生成class时才设置in_impl
            context.in_impl = true;
            context.current_class = Some(type_name.to_string());

            let export = if struct_info.is_public { "export " } else { "" };
            let mut output = format!("{}class {} {{\n", export, struct_info.name);

            // 添加字段
            for field in &struct_info.fields {
                let cleaned_field = field.trim().trim_end_matches(',').trim();
                if !cleaned_field.is_empty() {
                    let converted_field = self.convert_types_in_string(cleaned_field);
                    output.push_str(&format!("    {};\n", converted_field));
                }
            }

            // 不关闭大括号，让后续的方法定义自然添加进来
            return Ok(output.trim_end().to_string());
        }

        // 如果是enum的impl，生成namespace来包装静态方法
        if context.enums.contains_key(type_name) {
            context.in_impl = false; // 不是class，是namespace
            context.in_enum_impl = true; // 标记进入enum impl
            context.current_impl_enum = Some(type_name.to_string());
            return Ok(format!("\nexport namespace {} {{", type_name));
        }

        // 其他情况，返回注释
        context.in_impl = false;
        Ok(format!("// impl {}", type_name))
    }

    fn convert_module(&self, line: &str) -> Result<String> {
        let content = &line[2..];

        let is_pub = content
            .trim()
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false);

        let export = if is_pub { "export " } else { "" };

        Ok(format!("{}namespace {}", export, content))
    }

    // ============ 变量声明 ============

    fn convert_let(&self, line: &str, context: &mut ConversionContext) -> Result<String> {
        let content = &line[2..];
        let converted = self.convert_expression(content, context)?;
        let type_converted = self.convert_types_in_string(&converted);
        Ok(format!("const {}", type_converted))
    }

    fn convert_let_mut(&self, line: &str, context: &mut ConversionContext) -> Result<String> {
        let content = &line[2..];
        let converted = self.convert_expression(content, context)?;
        let type_converted = self.convert_types_in_string(&converted);
        Ok(format!("let {}", type_converted))
    }

    fn convert_const(&self, line: &str, context: &mut ConversionContext) -> Result<String> {
        let content = &line[2..];
        let converted = self.convert_expression(content, context)?;
        let type_converted = self.convert_types_in_string(&converted);
        Ok(format!("const {}", type_converted))
    }

    fn convert_static(&self, line: &str, context: &mut ConversionContext) -> Result<String> {
        let content = &line[3..];
        let converted = self.convert_expression(content, context)?;
        let type_converted = self.convert_types_in_string(&converted);
        Ok(format!("let {}", type_converted))
    }

    // ============ 控制流 ============

    fn convert_loop(&self, line: &str, context: &mut ConversionContext) -> Result<String> {
        if line == "L {" {
            return Ok("while (true) {".to_string());
        }

        let content = &line[2..];

        // 检查是否是for循环
        if content.contains(" in ") || content.contains(": ") {
            let normalized = content.replace(": ", " in ");
            let converted = self.convert_expression(&normalized, context)?;
            return Ok(format!("for {}", converted));
        }

        // 否则是无限循环
        Ok("while (true) {".to_string())
    }

    fn convert_if(&self, line: &str, context: &mut ConversionContext) -> Result<String> {
        let content = &line[2..].trim();

        // 检查if后面是否有条件表达式和大括号
        if let Some(brace_pos) = content.find('{') {
            let condition = content[..brace_pos].trim();
            let rest = &content[brace_pos..];
            let converted_cond = self.convert_expression(condition, context)?;

            // 处理rest中的宏（可能包含println!等）
            let converted_rest = self.convert_macros(rest)?;

            Ok(format!("if ({}) {}", converted_cond, converted_rest))
        } else {
            // 没有大括号，只有条件
            let converted = self.convert_expression(content, context)?;
            Ok(format!("if ({})", converted))
        }
    }

    fn convert_match_stmt(&self, line: &str, context: &mut ConversionContext) -> Result<String> {
        let content = &line[2..].trim();

        // 提取match的表达式（去掉可能的大括号）
        let expr = if content.ends_with('{') {
            content[..content.len() - 1].trim()
        } else {
            content
        };

        let converted = self.convert_expression(expr, context)?;

        // 生成switch语句
        Ok(format!("switch ({}) {{", converted))
    }

    fn convert_return(&self, line: &str, context: &mut ConversionContext) -> Result<String> {
        if line == "<" {
            Ok("return;".to_string())
        } else {
            let content = &line[2..];
            let converted = self.convert_expression(content, context)?;
            Ok(format!("return {};", converted))
        }
    }

    fn convert_print(&self, line: &str, context: &mut ConversionContext) -> Result<String> {
        let content = &line[2..].trim();

        // 直接处理println!宏
        // 格式: println!("format", args) 或 println!("text")
        if content.starts_with("println!") || content.starts_with("println !") {
            let mut expr = content
                .replace("println!", "")
                .replace("println !", "")
                .trim()
                .to_string();

            // 移除外层括号
            if expr.starts_with('(') && expr.ends_with(')') {
                expr = expr[1..expr.len() - 1].to_string();
            }

            // 检查是否有格式化参数（包含{}和逗号分隔的参数）
            if expr.contains("{}") && expr.contains(',') {
                // 有格式化参数，使用$fmt
                if !self.config.no_format {
                    return Ok(format!("console.log($fmt({}))", expr));
                } else {
                    // 简化：转换为模板字符串（暂不实现完整转换）
                    return Ok(format!("console.log($fmt({}))", expr));
                }
            } else {
                // 无格式化或简单文本
                return Ok(format!("console.log({})", expr));
            }
        }

        // 否则作为普通表达式处理
        let converted = self.convert_expression(content, context)?;
        Ok(format!("console.log({})", converted))
    }

    fn convert_use(&self, line: &str) -> Result<String> {
        let is_pub = line.starts_with("U ");
        let content = line[2..].trim().trim_end_matches(';').trim();

        let export_keyword = if is_pub { "export " } else { "" };

        // 转换use路径
        // u std::collections::HashMap -> import { HashMap } from 'std/collections';
        // u std::io::{self, Write} -> import * as io from 'std/io'; (简化处理)
        // u ./module -> import * as module from './module';

        if content.contains("::") {
            // 检查是否有花括号（多个导入项）
            if content.contains("{") && content.contains("}") {
                // 简化处理：只取模块路径，忽略具体导入项
                // u std::io::{self, Write} -> 注释掉，因为TS不支持这种复杂导入
                return Ok(format!("// TODO: Manual import needed: {}", content));
            }

            // 标准库导入 - 移除空格
            let cleaned = content.replace(" ", "");
            let parts: Vec<&str> = cleaned.split("::").collect();
            if parts.len() >= 2 {
                let module_path = parts[..parts.len() - 1].join("/");
                let item = parts[parts.len() - 1];
                return Ok(format!(
                    "{}import {{ {} }} from '{}/';",
                    export_keyword, item, module_path
                ));
            }
        }

        // 相对路径导入
        Ok(format!(
            "{}import * as {} from '{}';",
            export_keyword,
            content.trim_start_matches("./").replace("/", "_"),
            content
        ))
    }

    // ============ 表达式转换 ============

    fn convert_expression(&self, expr: &str, context: &mut ConversionContext) -> Result<String> {
        let mut result = expr.to_string();

        // 处理?操作符（错误传播）
        if result.contains(")?") {
            result = self.desugar_try_operator(&result, context)?;
        }

        // 处理宏调用
        result = self.convert_macros(&result)?;

        // 处理链式调用
        result = self.strip_chain_methods(&result);

        // 处理.await
        result = result.replace(".~", "await ");

        // 处理闭包
        result = result.replace("$|", "");
        result = result.replace("|", "");

        Ok(result)
    }

    // ============ 错误处理：?操作符展开 ============

    fn desugar_try_operator(&self, expr: &str, context: &mut ConversionContext) -> Result<String> {
        // 简化处理：生成注释提示
        // 完整实现需要AST级别的分析

        if !context.in_function {
            return Ok(expr.replace(")?", ")"));
        }

        // 生成临时变量
        let temp_var = context.next_temp_var();

        // 查找函数调用
        if let Some(pos) = expr.find(")?") {
            let func_call = &expr[..pos + 1];
            let remaining = &expr[pos + 2..];

            let expanded = format!(
                "const {} = {};\nif ({}.tag === 'err') return {};\nconst val = {}.val{}",
                temp_var, func_call, temp_var, temp_var, temp_var, remaining
            );

            return Ok(expanded);
        }

        Ok(expr.to_string())
    }

    // ============ 宏转换 ============

    fn convert_macros(&self, expr: &str) -> Result<String> {
        let mut result = expr.to_string();

        // println! -> console.log (包括空格处理)
        // 需要处理格式化参数
        while result.contains("println!") || result.contains("println !") {
            // 找到println!的位置
            let start = result.find("println!").or_else(|| result.find("println !"));
            if let Some(start_pos) = start {
                // 跳过"println!"或"println !"
                let macro_end = if result[start_pos..].starts_with("println!") {
                    start_pos + 8
                } else {
                    start_pos + 9
                };

                // 查找宏参数（括号内的内容）
                if let Some(paren_start) = result[macro_end..].find('(') {
                    let paren_start = macro_end + paren_start;
                    // 简单查找匹配的右括号（不考虑嵌套）
                    if let Some(paren_end) = result[paren_start + 1..].find(')') {
                        let paren_end = paren_start + 1 + paren_end;
                        let args = &result[paren_start + 1..paren_end];

                        // 检查是否有格式化参数
                        let replacement = if args.contains("{}") && args.contains(',') {
                            format!("console.log($fmt({}))", args)
                        } else {
                            format!("console.log({})", args)
                        };

                        // 替换整个println!调用
                        result = format!(
                            "{}{}{}",
                            &result[..start_pos],
                            replacement,
                            &result[paren_end + 1..]
                        );
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // format! -> $fmt
        if result.contains("format!") {
            if self.config.no_format {
                // 使用模板字符串（简化处理）
                result = result.replace("format!(", "(");
            } else {
                result = result.replace("format!(", "$fmt(");
            }
        }

        // panic! -> throw Error
        if result.contains("panic!") {
            result = result.replace("panic!(", "throw new Error(");
        }

        // vec! -> [] (包括空格处理)
        // 移除"vec!"和"vec !"，但保留"["
        result = result.replace("vec![", "[");
        result = result.replace("vec ![", "[");
        result = result.replace("vec !", "");
        result = result.replace("V![", "[");
        result = result.replace("V ![", "[");
        result = result.replace("V !", "");

        // Some/None 转换
        // Some(value) -> value (TypeScript中null足以表示None)
        // None -> null
        result = result.replace("Some(", "(");
        result = result.replace("Some (", "(");
        result = result.replace("None", "null");

        // assert! -> if (!...) throw
        // 简化处理
        if result.contains("assert!") {
            result =
                result.replace("assert!(", "if (!(") + ")) throw new Error('Assertion failed');";
        }

        // todo!/unimplemented!
        result = result.replace("todo!()", "throw new Error('TODO: Not implemented')");
        result = result.replace("unimplemented!()", "throw new Error('Unimplemented')");

        Ok(result)
    }

    // ============ 链式调用剥离 ============

    fn strip_chain_methods(&self, expr: &str) -> String {
        let mut result = expr.to_string();

        // 删除.iter()和.into_iter()
        result = result.replace(".iter()", "");
        result = result.replace(".into_iter()", "");

        // 删除.collect()
        result = result.replace(".collect()", "");
        result = result.replace(".collect::<Vec<", ""); // 清理turbofish
        result = result.replace(".collect::<", "");

        // .clone() -> structuredClone()
        if result.contains(".clone()") {
            result = result.replace(".clone()", "");
            // 简化：直接删除，因为JS对象是引用
            // 如果需要深拷贝，应该用structuredClone包装整个表达式
        }

        // .len() -> .length (包括空格处理)
        result = result.replace(".len()", ".length");
        result = result.replace(". len()", ".length");
        result = result.replace(". len ()", ".length");

        // .unwrap() -> !  (对Option)
        // 对Result需要用$unwrap
        if result.contains(".unwrap()") {
            // 简化处理：假设是Option
            result = result.replace(".unwrap()", "!");
        }

        // .unwrap_or(v) -> ?? v
        if result.contains(".unwrap_or(") {
            // 找到unwrap_or的参数
            if let Some(start) = result.find(".unwrap_or(") {
                if let Some(end) = result[start..].find(")") {
                    let default_val = &result[start + 12..start + end];
                    let before = &result[..start];
                    let after = &result[start + end + 1..];
                    result = format!("{} ?? {}{}", before, default_val, after);
                }
            }
        }

        // .enumerate() -> .map((val, idx) => ...)
        // 这个需要更复杂的处理，暂时保留

        result
    }

    // ============ 收集定义的辅助方法 ============

    fn collect_enum_definition(
        &self,
        lines: &[&str],
        index: &mut usize,
        context: &mut ConversionContext,
    ) -> Result<()> {
        use super::types::{EnumInfo, EnumVariant};

        let line = lines[*index].trim();
        let content = &line[2..]; // 跳过 "E "

        let is_pub = content
            .trim()
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false);

        let enum_name = content
            .split_whitespace()
            .next()
            .unwrap_or("")
            .split('{')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();

        let mut variants = Vec::new();

        // 查找开始的大括号
        let mut brace_found = content.contains('{');
        if !brace_found {
            *index += 1;
            if *index < lines.len() && lines[*index].trim() == "{" {
                brace_found = true;
            }
        }

        if brace_found {
            *index += 1;
            // 收集变体
            while *index < lines.len() {
                let variant_line = lines[*index].trim();

                if variant_line == "}" {
                    break;
                }

                if !variant_line.is_empty() && !variant_line.starts_with("//") {
                    // 解析变体：可能是 "Add," 或 "InvalidOperator(String),"
                    let variant_str = variant_line.trim_end_matches(',').trim();

                    if let Some(paren_pos) = variant_str.find('(') {
                        // 带数据的变体
                        let variant_name = variant_str[..paren_pos].trim().to_string();
                        let data_type = variant_str
                            [paren_pos + 1..variant_str.rfind(')').unwrap_or(variant_str.len())]
                            .trim()
                            .to_string();
                        variants.push(EnumVariant {
                            name: variant_name,
                            data: Some(data_type),
                        });
                    } else {
                        // 简单变体
                        variants.push(EnumVariant {
                            name: variant_str.to_string(),
                            data: None,
                        });
                    }
                }

                *index += 1;
            }
        }

        context.enums.insert(
            enum_name.clone(),
            EnumInfo {
                name: enum_name,
                variants,
                is_public: is_pub,
            },
        );

        Ok(())
    }

    fn collect_struct_definition(
        &self,
        lines: &[&str],
        index: &mut usize,
        context: &mut ConversionContext,
    ) -> Result<()> {
        use super::types::StructInfo;

        let line = lines[*index].trim();
        let content = &line[2..]; // 跳过 "S "

        let is_pub = content
            .trim()
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false);

        let struct_name = content
            .split_whitespace()
            .next()
            .unwrap_or("")
            .split('{')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();

        let mut fields = Vec::new();

        // 查找开始的大括号
        let mut brace_found = content.contains('{');
        if !brace_found {
            *index += 1;
            if *index < lines.len() && lines[*index].trim() == "{" {
                brace_found = true;
            }
        }

        if brace_found {
            *index += 1;
            // 收集字段
            while *index < lines.len() {
                let field_line = lines[*index].trim();

                if field_line == "}" {
                    break;
                }

                if !field_line.is_empty() && !field_line.starts_with("//") {
                    fields.push(field_line.to_string());
                }

                *index += 1;
            }
        }

        context.structs.insert(
            struct_name.clone(),
            StructInfo {
                name: struct_name,
                fields,
                is_public: is_pub,
            },
        );

        Ok(())
    }

    fn collect_impl_definition(
        &self,
        lines: &[&str],
        index: &mut usize,
        context: &mut ConversionContext,
    ) -> Result<()> {
        use super::types::ImplInfo;

        let line = lines[*index].trim();
        let content = &line[2..]; // 跳过 "I "

        let type_name = content
            .split_whitespace()
            .next()
            .unwrap_or("")
            .split('{')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();

        let mut methods = Vec::new();

        // 查找开始的大括号
        let mut brace_found = content.contains('{');
        if !brace_found {
            *index += 1;
            if *index < lines.len() && lines[*index].trim() == "{" {
                brace_found = true;
            }
        }

        if brace_found {
            *index += 1;
            let mut method_lines = Vec::new();
            let mut brace_count = 1;

            // 收集方法（需要处理嵌套的大括号）
            while *index < lines.len() && brace_count > 0 {
                let method_line = lines[*index];
                let trimmed = method_line.trim();

                // 统计大括号
                brace_count += trimmed.matches('{').count();
                brace_count = brace_count.saturating_sub(trimmed.matches('}').count());

                if brace_count > 0 {
                    method_lines.push(method_line.to_string());
                }

                *index += 1;
            }

            // 将收集的行组合成方法字符串
            if !method_lines.is_empty() {
                methods.push(method_lines.join("\n"));
            }
        }

        context
            .impls
            .entry(type_name.clone())
            .or_insert_with(Vec::new)
            .push(ImplInfo {
                target: type_name,
                methods,
            });

        Ok(())
    }

    fn generate_enum_output(&self, enum_info: &super::types::EnumInfo) -> Result<String> {
        let export = if enum_info.is_public { "export " } else { "" };

        // 检查是否所有变体都是简单的（无数据）
        let all_simple = enum_info.variants.iter().all(|v| v.data.is_none());

        if all_simple {
            // 生成TypeScript enum
            let mut output = format!("{}enum {} {{\n", export, enum_info.name);
            for variant in &enum_info.variants {
                output.push_str(&format!("    {},\n", variant.name));
            }
            output.push_str("}");
            Ok(output)
        } else {
            // 生成tagged union类型
            let mut output = format!("{}type {} =\n", export, enum_info.name);
            let variants: Vec<String> = enum_info
                .variants
                .iter()
                .map(|v| {
                    if let Some(data) = &v.data {
                        let ts_type = self.convert_type(data);
                        format!("    | {{ tag: '{}', value: {} }}", v.name, ts_type)
                    } else {
                        format!("    | {{ tag: '{}' }}", v.name)
                    }
                })
                .collect();
            output.push_str(&variants.join("\n"));
            output.push(';');
            Ok(output)
        }
    }

    fn generate_class_with_impl(
        &self,
        struct_info: &super::types::StructInfo,
        type_name: &str,
        context: &ConversionContext,
    ) -> Result<String> {
        let export = if struct_info.is_public { "export " } else { "" };

        let mut output = format!("{}class {} {{\n", export, struct_info.name);

        // 添加字段
        for field in &struct_info.fields {
            let converted_field = self.convert_types_in_string(field);
            output.push_str(&format!("    {};\n", converted_field));
        }

        // 添加方法（如果有impl块）
        if let Some(impls) = context.impls.get(type_name) {
            for impl_info in impls {
                for method in &impl_info.methods {
                    // 将方法转换为class方法格式
                    let method_converted = self.convert_impl_method(method)?;
                    output.push_str(&method_converted);
                    output.push('\n');
                }
            }
        }

        output.push('}');
        Ok(output)
    }

    fn convert_impl_method(&self, method: &str) -> Result<String> {
        // 简化处理：直接返回方法体，由后续的convert_line处理
        // 这里只是占位，实际的方法转换会在第二遍扫描时完成
        Ok(format!("    // {}", method.lines().next().unwrap_or("")))
    }

    // ============ Match 语句处理 ============

    /// 解析 Match 语句
    fn parse_match_statement(
        &self,
        lines: &[&str],
        start: usize,
    ) -> Result<super::types::MatchAst> {
        use super::types::{MatchArm, MatchAst, MatchPattern};

        let first_line = lines[start].trim();

        // 提取匹配目标: "M expr {" -> "expr"
        let target = if let Some(pos) = first_line.find('{') {
            first_line[2..pos].trim().to_string()
        } else {
            first_line[2..].trim().to_string()
        };

        // 收集所有行直到匹配的 }
        let mut brace_count = if first_line.contains('{') { 1 } else { 0 };
        let mut i = start + 1;

        if brace_count == 0 && i < lines.len() && lines[i].trim() == "{" {
            brace_count = 1;
            i += 1;
        }

        let mut arms = Vec::new();
        let mut current_pattern: Option<String> = None;
        let mut current_body = String::new();
        let mut arm_brace_count = 0;

        while i < lines.len() && brace_count > 0 {
            let line = lines[i];
            let trimmed = line.trim();

            // 更新大括号计数
            brace_count += trimmed.matches('{').count();
            brace_count = brace_count.saturating_sub(trimmed.matches('}').count());

            // 检测分支开始: "Ok(val):" 或 "Err(e):"
            if trimmed.ends_with(':') && !trimmed.contains('{') {
                // 保存上一个分支
                if let Some(pat) = current_pattern.take() {
                    arms.push(MatchArm {
                        pattern: self.parse_match_pattern(&pat)?,
                        guard: None,
                        body: current_body.trim().to_string(),
                    });
                    current_body.clear();
                }

                // 开始新分支
                current_pattern = Some(trimmed.trim_end_matches(':').to_string());
                arm_brace_count = 0;
            } else if trimmed.contains(":{") {
                // 单行形式: "Ok(val): { body }"
                let parts: Vec<&str> = trimmed.splitn(2, ":{").collect();
                if parts.len() == 2 {
                    // 保存上一个分支
                    if let Some(pat) = current_pattern.take() {
                        arms.push(MatchArm {
                            pattern: self.parse_match_pattern(&pat)?,
                            guard: None,
                            body: current_body.trim().to_string(),
                        });
                        current_body.clear();
                    }

                    current_pattern = Some(parts[0].trim().to_string());
                    let body_part = parts[1];
                    current_body.push_str(body_part.trim_end_matches('}').trim());
                    arm_brace_count = 1;
                    arm_brace_count += body_part.matches('{').count();
                    arm_brace_count =
                        arm_brace_count.saturating_sub(body_part.matches('}').count());

                    if arm_brace_count == 0 {
                        // 分支完成
                        if let Some(pat) = current_pattern.take() {
                            arms.push(MatchArm {
                                pattern: self.parse_match_pattern(&pat)?,
                                guard: None,
                                body: current_body.trim().to_string(),
                            });
                            current_body.clear();
                        }
                    }
                }
            } else if current_pattern.is_some() {
                // 收集分支体
                if trimmed == "{" {
                    arm_brace_count = 1;
                } else if arm_brace_count > 0 {
                    arm_brace_count += trimmed.matches('{').count();
                    arm_brace_count = arm_brace_count.saturating_sub(trimmed.matches('}').count());

                    if arm_brace_count == 0 && trimmed.ends_with('}') {
                        // 分支结束
                        if let Some(pat) = current_pattern.take() {
                            arms.push(MatchArm {
                                pattern: self.parse_match_pattern(&pat)?,
                                guard: None,
                                body: current_body.trim().to_string(),
                            });
                            current_body.clear();
                        }
                    } else {
                        current_body.push_str(line);
                        current_body.push('\n');
                    }
                } else {
                    current_body.push_str(line);
                    current_body.push('\n');
                }
            }

            i += 1;
        }

        // 推断目标类型
        let target_type = self.infer_match_target_type(&arms);

        Ok(MatchAst {
            target,
            target_type,
            arms,
            start_line: start,
            end_line: i.saturating_sub(1),
        })
    }

    /// 解析匹配模式
    fn parse_match_pattern(&self, pattern: &str) -> Result<super::types::MatchPattern> {
        use super::types::MatchPattern;

        let trimmed = pattern.trim();

        if trimmed.starts_with("Ok(") && trimmed.ends_with(')') {
            let binding = trimmed[3..trimmed.len() - 1].trim().to_string();
            return Ok(MatchPattern::ResultOk { binding });
        }

        if trimmed.starts_with("Err(") && trimmed.ends_with(')') {
            let binding = trimmed[4..trimmed.len() - 1].trim().to_string();
            return Ok(MatchPattern::ResultErr { binding });
        }

        if trimmed.starts_with("Some(") && trimmed.ends_with(')') {
            let binding = trimmed[5..trimmed.len() - 1].trim().to_string();
            return Ok(MatchPattern::OptionSome { binding });
        }

        if trimmed == "None" {
            return Ok(MatchPattern::OptionNone);
        }

        if trimmed == "_" {
            return Ok(MatchPattern::Wildcard);
        }

        // 字面量
        Ok(MatchPattern::Literal {
            value: trimmed.to_string(),
        })
    }

    /// 根据分支模式推断目标类型
    fn infer_match_target_type(&self, arms: &[super::types::MatchArm]) -> Option<String> {
        use super::types::MatchPattern;

        for arm in arms {
            match &arm.pattern {
                MatchPattern::ResultOk { .. } | MatchPattern::ResultErr { .. } => {
                    return Some("Result".to_string());
                }
                MatchPattern::OptionSome { .. } | MatchPattern::OptionNone => {
                    return Some("Option".to_string());
                }
                _ => {}
            }
        }
        None
    }

    /// 生成 TypeScript if-chain
    fn generate_match_ifchain(
        &self,
        match_ast: &super::types::MatchAst,
        context: &mut ConversionContext,
    ) -> Result<String> {
        use super::types::MatchPattern;

        let temp_var = format!("_match{}", context.temp_var_counter);
        context.temp_var_counter += 1;

        let mut output = String::new();

        // 计算匹配目标
        output.push_str(&format!(
            "const {} = {};\n",
            temp_var,
            self.convert_expression(&match_ast.target, context)?
        ));

        // 生成分支
        let match_type = match_ast.target_type.as_deref();
        let mut is_first = true;

        for arm in &match_ast.arms {
            let condition = self.generate_match_condition(&temp_var, &arm.pattern, match_type)?;

            let prefix = if is_first { "if" } else { "else if" };
            is_first = false;

            // 生成绑定变量
            let binding = self.generate_pattern_binding(&arm.pattern, &temp_var)?;

            // 转换分支体
            let body_lines: Vec<&str> = arm.body.lines().collect();
            let mut body_converted = String::new();
            for line in body_lines {
                let converted_line = self.convert_expression(line.trim(), context)?;
                if !converted_line.is_empty() {
                    body_converted.push_str("    ");
                    body_converted.push_str(&converted_line);
                    if !converted_line.ends_with(';') && !converted_line.ends_with('}') {
                        body_converted.push(';');
                    }
                    body_converted.push('\n');
                }
            }

            output.push_str(&format!(
                "{} ({}) {{\n{}{}}}\n",
                prefix,
                condition,
                if binding.is_empty() {
                    String::new()
                } else {
                    format!("{};\n", binding)
                },
                body_converted
            ));
        }

        Ok(output.trim_end().to_string())
    }

    /// 生成匹配条件
    fn generate_match_condition(
        &self,
        temp_var: &str,
        pattern: &super::types::MatchPattern,
        _match_type: Option<&str>,
    ) -> Result<String> {
        use super::types::MatchPattern;

        Ok(match pattern {
            MatchPattern::ResultOk { .. } => {
                format!("{}.tag === 'ok'", temp_var)
            }
            MatchPattern::ResultErr { .. } => {
                format!("{}.tag === 'err'", temp_var)
            }
            MatchPattern::OptionSome { .. } => {
                format!("{} !== null", temp_var)
            }
            MatchPattern::OptionNone => {
                format!("{} === null", temp_var)
            }
            MatchPattern::Literal { value } => {
                format!("{} === {}", temp_var, value)
            }
            MatchPattern::Wildcard => "true".to_string(),
        })
    }

    /// 生成模式绑定代码
    fn generate_pattern_binding(
        &self,
        pattern: &super::types::MatchPattern,
        temp_var: &str,
    ) -> Result<String> {
        use super::types::MatchPattern;

        Ok(match pattern {
            MatchPattern::ResultOk { binding } => {
                format!("    const {} = {}.val", binding, temp_var)
            }
            MatchPattern::ResultErr { binding } => {
                format!("    const {} = {}.err", binding, temp_var)
            }
            MatchPattern::OptionSome { binding } => {
                format!("    const {} = {}", binding, temp_var)
            }
            _ => String::new(),
        })
    }
}

impl Default for Nu2TsConverter {
    fn default() -> Self {
        Self::with_default_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_types() {
        let converter = Nu2TsConverter::with_default_config();

        assert_eq!(converter.convert_type("i32"), "number");
        assert_eq!(converter.convert_type("String"), "string");
        assert_eq!(converter.convert_type("bool"), "boolean");
        assert_eq!(converter.convert_type("Vec<i32>"), "Array<number>");
        assert_eq!(converter.convert_type("Option<String>"), "string | null");
    }

    #[test]
    fn test_convert_function() {
        let converter = Nu2TsConverter::with_default_config();
        let nu_code = "F add(a: i32, b: i32) -> i32 {";

        let result = converter.convert(nu_code).unwrap();
        assert!(result.contains("export function add"));
        assert!(result.contains("number"));
    }

    #[test]
    fn test_convert_let() {
        let converter = Nu2TsConverter::with_default_config();
        let nu_code = "l x = 5;";

        // Use convert_legacy which has line-level Nu syntax processing
        let result = converter.convert_legacy(nu_code).unwrap();
        assert!(
            result.contains("const x = 5"),
            "Expected 'const x = 5' in result: {}",
            result
        );
    }

    #[test]
    fn test_convert_macros() {
        let converter = Nu2TsConverter::with_default_config();

        let result = converter.convert_macros("println!(\"Hello\")").unwrap();
        assert_eq!(result, "console.log(\"Hello\")");

        let result2 = converter.convert_macros("panic!(\"Error\")").unwrap();
        assert_eq!(result2, "throw new Error(\"Error\")");
    }

    #[test]
    fn test_strip_chain_methods() {
        let converter = Nu2TsConverter::with_default_config();

        let result = converter.strip_chain_methods("arr.iter().map(|x| x * 2).collect()");
        assert_eq!(result, "arr.map(|x| x * 2)");

        let result2 = converter.strip_chain_methods("s.len()");
        assert_eq!(result2, "s.length");
    }
}
