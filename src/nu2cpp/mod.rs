// Nu to C++ Converter
// 将Nu代码转换为标准C++代码
//
// Architecture (v2.0):
// - cpp_ast: C++ AST definitions (types, expressions, statements)
// - cpp_codegen: AST -> C++ source code generator
// - converter (this file): Nu AST -> C++ AST converter

use anyhow::Result;

// 导出 sourcemap 模块
pub mod sourcemap;
pub use sourcemap::SourceMap;

// v2.0: 新增 C++ AST 模块
pub mod ast_converter;
pub mod cpp_ast;
pub mod cpp_codegen;
pub use ast_converter::NuToCppAstConverter;
pub use cpp_ast::*;
pub use cpp_codegen::CppCodegen;

pub struct Nu2CppConverter {
    // 转换上下文
    context: ConversionContext,
}

/// v2.0: 增强的C++上下文状态机
/// 参考nu2rust的ConversionContext设计，增加C++特有的访问修饰符跟踪
#[derive(Default, Clone)]
struct ConversionContext {
    // 跟踪当前作用域
    in_trait: bool,
    in_class: bool,
    in_struct_block: bool,
    in_namespace: bool,

    // v2.0新增: C++访问修饰符状态（用于智能生成访问控制）
    in_public_section: bool,
    in_private_section: bool,
    in_protected_section: bool,

    // v2.0新增: 模板和函数作用域
    in_template: bool,
    in_function: bool,

    // C++特有上下文
    current_class_name: Option<String>,
    has_constructor: bool,

    // 🔴 新增：enum变体跟踪（用于生成std::variant using语句）
    in_enum_block: bool,
    current_enum_name: Option<String>,
    enum_variants: Vec<String>,
}

impl Nu2CppConverter {
    pub fn new() -> Self {
        Self {
            context: ConversionContext::default(),
        }
    }

    /// 转换 Nu 代码为 C++ 代码
    pub fn convert(&self, nu_code: &str) -> Result<String> {
        self.convert_with_sourcemap(nu_code, None)
    }

    /// 转换 Nu 代码为 C++ 代码，并记录行号映射
    pub fn convert_with_sourcemap(
        &self,
        nu_code: &str,
        mut sourcemap: Option<&mut SourceMap>,
    ) -> Result<String> {
        // 添加必要的C++头文件
        let mut output = String::from(
            "#include <cstdint>\n\
             #include <string>\n\
             #include <iostream>\n\
             #include <vector>\n\
             #include <memory>\n\
             #include <optional>\n\n",
        );

        let lines: Vec<&str> = nu_code.lines().collect();
        let mut context = ConversionContext::default();

        let mut i = 0;
        let mut cpp_line = 8; // 从第8行开始（跳过头文件）

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();
            let nu_line = i + 1;

            // 保留空行
            if trimmed.is_empty() {
                output.push('\n');
                cpp_line += 1;
                i += 1;
                continue;
            }

            // 保留注释行
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
                if let Some(ref mut sm) = sourcemap {
                    sm.add_mapping(cpp_line, nu_line);
                }
                output.push_str(line);
                output.push('\n');
                cpp_line += 1;
                i += 1;
                continue;
            }

            // 处理属性标记（转换为C++注解）- 修复错误1: #D预处理指令错误
            if trimmed.starts_with("#[") || trimmed.starts_with("#![") || trimmed.starts_with("#D")
            {
                // C++没有直接对应的属性，转为注释
                if let Some(ref mut sm) = sourcemap {
                    sm.add_mapping(cpp_line, nu_line);
                }
                output.push_str(&format!("// {}\n", trimmed));
                cpp_line += 1;
                i += 1;
                continue;
            }

            // 记录当前行的映射
            if let Some(ref mut sm) = sourcemap {
                sm.add_mapping(cpp_line, nu_line);
            }

            // 处理各种Nu语法
            if let Some(mut converted) = self.convert_line(line, &lines, &mut i, &mut context)? {
                // P0修复: 在impl块内，对所有转换后的行进行Self和self替换
                if context.in_class {
                    if let Some(ref class_name) = context.current_class_name {
                        // 替换 Self { x, y } 为 ClassName(x, y)
                        converted = self.convert_self_initializer(&converted, class_name);
                        // 替换剩余的Self
                        converted = converted.replace("Self", class_name);
                    }
                    // 替换 self.x 为 this->x
                    converted = self.replace_self_with_this(&converted);
                }

                // v2.0: 结构体字段public标签智能添加
                // 注意：convert_struct已经处理了字段格式转换，这里只需要添加public:标签
                let final_converted = if context.in_struct_block {
                    let trimmed = line.trim();
                    // 检测是否是字段行（已经被convert_struct转换过）
                    if !trimmed.is_empty()
                        && !trimmed.starts_with("//")
                        && !trimmed.starts_with("public:")
                        && !trimmed.starts_with("S ")
                        && !trimmed.starts_with("s ")
                        && trimmed.contains(':')
                        && !trimmed.contains("::")
                        && !trimmed.starts_with("fn ")
                        && !trimmed.starts_with('}')
                        && !trimmed.starts_with("F ")
                    {
                        // 只在第一个字段前添加public:标签
                        if !context.in_public_section {
                            context.in_public_section = true;
                            format!("public:\n    {}", converted.trim())
                        } else {
                            converted
                        }
                    } else {
                        converted
                    }
                } else {
                    converted
                };

                if !final_converted.is_empty() {
                    let leading_whitespace: String =
                        line.chars().take_while(|c| c.is_whitespace()).collect();
                    let trimmed_converted = final_converted.trim_start();
                    output.push_str(&leading_whitespace);
                    output.push_str(trimmed_converted);
                    output.push('\n');
                    cpp_line += 1;
                }
            }

            i += 1;
        }

        Ok(output)
    }

    /// v2.0: 优先级驱动的模式匹配（参考nu2rust v1.8.15）
    ///
    /// 关键原则（基于nu2rust的16次迭代经验）：
    /// 1. Loop (L) 已经在Function (F) 之前 ✓
    /// 2. 添加If not (?!) 检查在 If (?) 之前
    /// 3. 添加Unsafe/const函数优先级
    /// 4. 使用模式守卫避免函数调用被误判为定义
    fn convert_line(
        &self,
        line: &str,
        _lines: &[&str],
        _index: &mut usize,
        context: &mut ConversionContext,
    ) -> Result<Option<String>> {
        let trimmed = line.trim();

        // v2.0优先级1: Loop: L (已经正确在Function之前！)
        if trimmed.starts_with("L ") || trimmed == "L {" || trimmed.starts_with("L(") {
            return Ok(Some(self.convert_loop(trimmed)?));
        }

        // v2.0优先级2: If not: ?! (必须在 If (?) 之前检查)
        if trimmed.starts_with("?! ") {
            return Ok(Some(self.convert_if_not(trimmed)?));
        }

        // v2.0优先级3: If: if 或 ?
        if trimmed.starts_with("if ") || trimmed.starts_with("if(") {
            return Ok(Some(self.convert_if(trimmed)?));
        }
        if trimmed.starts_with("? ") || trimmed.starts_with("?(") {
            return Ok(Some(self.convert_if_legacy(trimmed)?));
        }

        // v2.0优先级4: Match: M -> switch
        if trimmed.starts_with("M ") || trimmed.starts_with("M(") {
            return Ok(Some(self.convert_match(trimmed)?));
        }

        // v2.0优先级5: Unsafe/const函数（在普通函数之前）
        if trimmed.starts_with("unsafe F ") || trimmed.starts_with("unsafe f ") {
            return Ok(Some(self.convert_unsafe_function(trimmed, context)?));
        }
        if trimmed.starts_with("const F ") || trimmed.starts_with("const f ") {
            return Ok(Some(self.convert_const_function(trimmed, context)?));
        }

        // v2.0优先级6: 函数定义: F/f
        // 添加模式守卫：避免函数调用被误判为定义
        if trimmed.starts_with("F ") || trimmed.starts_with("f ") {
            let after_marker = &trimmed[2..];

            // 模式守卫1: 如果紧跟 '(' 则是函数调用，不是定义
            if after_marker.starts_with('(') || after_marker.starts_with("();") {
                return Ok(Some(self.convert_expression(trimmed)?));
            }

            // 模式守卫2: 包含括号才是函数定义
            if after_marker.contains('(') {
                return Ok(Some(self.convert_function(trimmed, context)?));
            }

            // 否则作为表达式处理
            return Ok(Some(self.convert_expression(trimmed)?));
        }

        // 结构体: S/s
        if trimmed.starts_with("S ") || trimmed.starts_with("s ") {
            let after_keyword = &trimmed[2..];
            let first_char = after_keyword.chars().next();
            if let Some(c) = first_char {
                if c.is_alphabetic() || c == '_' {
                    if trimmed.ends_with('{') {
                        context.in_struct_block = true;
                    }
                    return Ok(Some(self.convert_struct(trimmed)?));
                }
            }
        }

        if (trimmed.starts_with("E ") || trimmed.starts_with("e ")) && !trimmed.contains("=>") {
            let result = self.convert_enum(trimmed, context)?;
            // 检查是否是enum定义开始
            if result.starts_with("enum ") && result.ends_with('{') {
                context.in_enum_block = true;
                // 提取enum名称
                let enum_name = result
                    .trim_start_matches("enum ")
                    .trim_end_matches(" {")
                    .trim();
                context.current_enum_name = Some(enum_name.to_string());
                context.enum_variants.clear();
            }
            return Ok(Some(result));
        }

        // Trait: TR/tr
        if trimmed.starts_with("TR ") || trimmed.starts_with("tr ") {
            context.in_trait = true;
            return Ok(Some(self.convert_trait(trimmed)?));
        }

        // Impl: I
        if trimmed.starts_with("I ") {
            context.in_class = true;
            // 提取结构体名称用于后续的Self替换
            let impl_content = &trimmed[2..];
            let struct_name = if impl_content.contains(" for ") {
                impl_content
                    .split(" for ")
                    .nth(1)
                    .unwrap_or("")
                    .trim()
                    .trim_end_matches(" {")
                    .to_string()
            } else {
                impl_content.trim().trim_end_matches(" {").to_string()
            };
            context.current_class_name = Some(struct_name);
            return Ok(Some(self.convert_impl(trimmed)?));
        }

        // 检测块结束
        if trimmed == "}" {
            if context.in_enum_block {
                context.in_enum_block = false;
                // 🔴 生成 using EnumName = std::variant<Variant1, Variant2, ...>;
                if let Some(enum_name) = context.current_enum_name.clone() {
                    if !context.enum_variants.is_empty() {
                        let variants = context.enum_variants.join(", ");
                        context.enum_variants.clear();
                        context.current_enum_name = None;
                        return Ok(Some(format!(
                            "}};\nusing {} = std::variant<{}>;",
                            enum_name, variants
                        )));
                    }
                }
                context.current_enum_name = None;
                return Ok(Some("};".to_string()));
            }
            if context.in_class {
                context.in_class = false;
                context.current_class_name = None;
                // impl块结束不需要分号
                return Ok(Some("}".to_string()));
            }
            if context.in_struct_block {
                context.in_struct_block = false;
                return Ok(Some("};".to_string()));
            }
            if context.in_trait {
                context.in_trait = false;
                return Ok(Some("};".to_string()));
            }
            return Ok(Some("}".to_string()));
        }

        // 模块: DM/D -> namespace
        if trimmed.starts_with("DM ") {
            return Ok(Some(self.convert_pub_module(trimmed)?));
        }
        if trimmed.starts_with("D ") {
            return Ok(Some(self.convert_module(trimmed)?));
        }

        // 变量声明: l/v
        if trimmed.starts_with("l ") {
            return Ok(Some(self.convert_let(trimmed)?));
        }
        if trimmed.starts_with("v ") {
            let after_v = &trimmed[2..];
            let is_declaration = after_v
                .chars()
                .next()
                .map(|c| c.is_alphabetic() || c == '_' || c == '(')
                .unwrap_or(false)
                && !after_v.starts_with('.');

            if is_declaration {
                return Ok(Some(self.convert_let_mut(trimmed)?));
            }
        }

        // Return语句: <
        if trimmed.starts_with("< ") || trimmed == "<" {
            return Ok(Some(self.convert_return(trimmed)?));
        }

        // Break/Continue: br/ct
        if trimmed.starts_with("br;") || trimmed.starts_with("br ") || trimmed == "br" {
            return Ok(Some("break;".to_string()));
        }
        if trimmed.starts_with("ct;") || trimmed.starts_with("ct ") || trimmed == "ct" {
            return Ok(Some("continue;".to_string()));
        }

        // Print语句: >
        if trimmed.starts_with("> ") {
            return Ok(Some(self.convert_print(trimmed)?));
        }

        // Use语句 -> Include: u/U
        if trimmed.starts_with("u ") || trimmed.starts_with("U ") {
            return Ok(Some(self.convert_use(trimmed)?));
        }

        // Type alias: t
        if trimmed.starts_with("t ") {
            let content = &trimmed[2..];
            if !content.starts_with("+=") && !content.starts_with("-=") {
                return Ok(Some(self.convert_type_alias(trimmed)?));
            }
        }

        // Const: C/CP
        if trimmed.starts_with("C ") {
            return Ok(Some(self.convert_const(trimmed, false)?));
        }
        if trimmed.starts_with("CP ") {
            return Ok(Some(self.convert_const(trimmed, true)?));
        }

        // 其他情况：转换类型和表达式
        Ok(Some(self.convert_expression(trimmed)?))
    }

    fn convert_function(&self, line: &str, context: &ConversionContext) -> Result<String> {
        let is_pub = line.starts_with("F ");
        let content = &line[2..];

        let converted = self.convert_types_in_string(content);

        // 检查是否是main函数 - 必须在所有其他处理之前
        if converted.trim_start().starts_with("main(")
            || converted.trim_start().starts_with("main ")
        {
            // main函数特殊处理: 直接返回int main()，不添加任何修饰符
            return self.convert_main_function(&converted);
        }

        // 修复错误3: &self参数处理
        // 检查是否包含&self或&mut self参数
        let has_self_param = converted.contains("&self") || converted.contains("&mut self");

        let visibility = if context.in_class {
            "" // 类内方法不需要public:前缀
        } else if is_pub {
            ""
        } else {
            // 修复错误3: 如果有&self参数，不添加static
            if has_self_param {
                ""
            } else {
                "static "
            }
        };

        // 替换Self为实际的类名，并移除&self参数
        let mut result = self.convert_function_signature(&converted, context)?;
        if let Some(ref class_name) = context.current_class_name {
            result = result.replace("Self", class_name);
        }

        // 修复错误3: 移除&self和&mut self参数
        result = self.remove_self_parameter(&result);

        Ok(format!("{}{}", visibility, result))
    }

    fn convert_main_function(&self, sig: &str) -> Result<String> {
        // 根据NU2CPP23.md规范: f main() 必须生成 int main()
        // 不添加static修饰符，不管是F还是f

        // 提取函数体（如果有）
        if let Some(brace_pos) = sig.find('{') {
            let rest = &sig[brace_pos..];
            return Ok(format!("int main() {}", rest));
        }

        // 如果有参数列表但没有函数体
        if let Some(paren_start) = sig.find('(') {
            if let Some(paren_end) = sig[paren_start..].find(')') {
                let after_paren = &sig[paren_start + paren_end + 1..].trim();
                if !after_paren.is_empty() {
                    return Ok(format!("int main() {}", after_paren));
                }
            }
        }

        Ok("int main()".to_string())
    }

    fn convert_function_signature(&self, sig: &str, context: &ConversionContext) -> Result<String> {
        if let Some(arrow_pos) = sig.find("->") {
            let before_arrow = sig[..arrow_pos].trim();
            let after_arrow = sig[arrow_pos + 2..].trim();

            let ret_type = if let Some(brace_pos) = after_arrow.find('{') {
                after_arrow[..brace_pos].trim()
            } else {
                after_arrow
            };

            if let Some(paren_pos) = before_arrow.find('(') {
                let func_name = before_arrow[..paren_pos].trim();
                let params_end = if let Some(close_paren) = before_arrow.rfind(')') {
                    close_paren + 1
                } else {
                    before_arrow.len()
                };
                let params = &before_arrow[paren_pos..params_end];

                let converted_params = self.convert_function_params(params)?;

                let rest = if after_arrow.contains('{') {
                    let brace_pos = after_arrow.find('{').unwrap();
                    &after_arrow[brace_pos..]
                } else {
                    ""
                };

                // 处理函数体 - P0修复: 需要传递context来进行Self替换
                let mut formatted_rest = if !rest.is_empty() {
                    self.format_function_body(rest, ret_type)?
                } else {
                    String::new()
                };

                // P0修复3: 在impl块内，替换函数体中的Self为实际类名
                if let Some(ref class_name) = context.current_class_name {
                    // 替换 Self { x, y } 为 return ClassName(x, y);
                    formatted_rest = self.convert_self_initializer(&formatted_rest, class_name);
                    // 然后替换剩余的Self
                    formatted_rest = formatted_rest.replace("Self", class_name);
                }

                // P0修复1: 检测new()关键字冲突
                if func_name == "new" {
                    // 检查参数是否为空（去除括号后）
                    let params_inner = converted_params
                        .trim_matches(|c| c == '(' || c == ')')
                        .trim();
                    if params_inner.is_empty() {
                        // 默认构造函数: Type new() -> Type()
                        return Ok(format!("{}() {}", ret_type, formatted_rest));
                    } else {
                        // 带参构造函数: Type new(params) -> static Type create(params)
                        return Ok(format!(
                            "static {} create{} {}",
                            ret_type, converted_params, formatted_rest
                        ));
                    }
                }

                return Ok(format!(
                    "{} {}{} {}",
                    ret_type, func_name, converted_params, formatted_rest
                ));
            }
        }

        if let Some(paren_pos) = sig.find('(') {
            let func_name = sig[..paren_pos].trim();
            // 找到匹配的右括号
            let params_end = if let Some(close_paren) = sig[paren_pos..].find(')') {
                paren_pos + close_paren + 1
            } else {
                sig.len()
            };
            let params = &sig[paren_pos..params_end];
            let rest = sig[params_end..].trim();
            let converted_params = self.convert_function_params(params)?;

            // 处理函数体 - P0修复: 在impl块内替换Self
            let mut formatted_rest = if !rest.is_empty() {
                self.format_function_body(rest, "void")?
            } else {
                String::new()
            };

            // P0修复3: 在impl块内，替换函数体中的Self为实际类名
            if let Some(ref class_name) = context.current_class_name {
                // 替换 Self { x, y } 为 return ClassName(x, y);
                formatted_rest = self.convert_self_initializer(&formatted_rest, class_name);
                // 然后替换剩余的Self
                formatted_rest = formatted_rest.replace("Self", class_name);
            }

            // P0修复1: 检测new()关键字冲突（无返回类型版本）
            if func_name == "new" {
                let params_inner = converted_params
                    .trim_matches(|c| c == '(' || c == ')')
                    .trim();
                if params_inner.is_empty() {
                    // 无法推断类型名，保持原样并添加注释
                    return Ok(format!(
                        "/* constructor */ new{} {}",
                        converted_params, formatted_rest
                    ));
                } else {
                    // 转换为静态工厂方法
                    return Ok(format!(
                        "static auto create{} {}",
                        converted_params, formatted_rest
                    ));
                }
            }

            return Ok(format!(
                "void {}{} {}",
                func_name, converted_params, formatted_rest
            ));
        }

        Ok(sig.to_string())
    }

    /// 格式化函数体，处理多余分号和缺失的return语句
    fn format_function_body(&self, body: &str, ret_type: &str) -> Result<String> {
        let body_trimmed = body.trim();

        if !body_trimmed.starts_with('{') {
            // 没有大括号的函数体（不应该发生，但做防御性处理）
            return Ok(body.to_string());
        }

        // 提取大括号内的内容
        if body_trimmed.len() < 2 {
            return Ok(body.to_string());
        }

        // 找到匹配的右大括号
        let mut depth = 0;
        let mut end_pos = 0;
        for (i, ch) in body_trimmed.char_indices() {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
                if depth == 0 {
                    end_pos = i;
                    break;
                }
            }
        }

        if end_pos == 0 {
            return Ok(body.to_string());
        }

        // 提取大括号之间的内容
        let inner = &body_trimmed[1..end_pos];
        let trimmed_inner = inner.trim();

        // P0修复: 转换self为this->
        let converted_inner = self.replace_self_with_this(trimmed_inner);

        // 如果函数体为空
        if converted_inner.is_empty() {
            return Ok("{}".to_string());
        }

        // 检查是否需要添加return语句
        // 如果返回类型不是void，且函数体是单个表达式
        if ret_type != "void" && ret_type != "()" {
            // 检查内容是否已经有return、或是控制流语句
            let starts_with_control = converted_inner.starts_with("return")
                || converted_inner.starts_with("if")
                || converted_inner.starts_with("while")
                || converted_inner.starts_with("for")
                || converted_inner.starts_with("switch");

            let ends_with_terminator =
                converted_inner.ends_with(';') || converted_inner.ends_with('}');

            // 如果不是控制流语句，也不以分号或}结尾，那么这是一个需要return的表达式
            if !starts_with_control && !ends_with_terminator {
                return Ok(format!("{{ return {}; }}", converted_inner));
            }
        }

        // 重构函数体，包含self->this的转换
        let rest_of_body = &body_trimmed[end_pos..];
        Ok(format!("{{ {} {}", converted_inner, rest_of_body))
    }

    fn convert_function_params(&self, params: &str) -> Result<String> {
        let mut result = String::from("(");
        let inner = params.trim_matches(|c| c == '(' || c == ')');

        if !inner.is_empty() {
            let parts: Vec<&str> = inner.split(',').collect();
            let mut first = true;
            for part in parts.iter() {
                let trimmed = part.trim();

                // 修复错误3: 跳过&self和&mut self参数
                if trimmed == "&self" || trimmed == "&mut self" || trimmed.starts_with("self:") {
                    continue;
                }

                if !first {
                    result.push_str(", ");
                }
                first = false;

                if let Some(colon_pos) = trimmed.find(':') {
                    let param_name = trimmed[..colon_pos].trim();
                    let param_type = trimmed[colon_pos + 1..].trim();

                    // P0修复4: 修复参数引用语法 &Type name -> const Type& name
                    let converted_type =
                        if param_type.starts_with('&') && !param_type.starts_with("&self") {
                            let inner_type = param_type[1..].trim();
                            format!("const {}&", self.convert_types_in_string(inner_type))
                        } else {
                            self.convert_types_in_string(param_type)
                        };

                    result.push_str(&format!("{} {}", converted_type, param_name));
                } else {
                    result.push_str(trimmed);
                }
            }
        }

        result.push(')');
        Ok(result)
    }

    /// 修复错误3: 从函数签名中移除&self参数
    fn remove_self_parameter(&self, signature: &str) -> String {
        let mut result = signature.to_string();

        // 移除参数列表中的&self或&mut self
        if let Some(start) = result.find('(') {
            if let Some(end) = result.rfind(')') {
                let params = &result[start + 1..end];
                let new_params: Vec<&str> = params
                    .split(',')
                    .map(|p| p.trim())
                    .filter(|p| {
                        !p.is_empty()
                            && *p != "&self"
                            && *p != "&mut self"
                            && !p.starts_with("self:")
                    })
                    .collect();

                let before = &result[..start + 1];
                let after = &result[end..];
                result = format!("{}{}{}", before, new_params.join(", "), after);
            }
        }

        result
    }

    fn convert_struct(&self, line: &str) -> Result<String> {
        let is_pub_marker = line.starts_with("S ");
        let content = &line[2..];

        // P0修复: 强化struct字段语法转换 - 必须在任何其他逻辑之前检查
        // 检查是否有成员声明（包含冒号但不是::），且不是struct定义行
        if content.contains(':')
            && !content.contains("::")
            && !content.ends_with('{')
            && !content.starts_with("//")
            && !content.starts_with("struct ")  // 不是struct定义行
            && !content.contains('(')  // 不是tuple struct
            && !content.contains('<')
        // 不是泛型定义
        {
            // 这是结构体成员声明，需要转换 "name: Type," 为 "Type name;"
            let parts: Vec<&str> = content.splitn(2, ':').collect();
            if parts.len() == 2 {
                let member_name = parts[0].trim();
                let member_type = parts[1].trim().trim_end_matches(',').trim();

                // 应用引用类型转换
                let converted_type =
                    if member_type.starts_with('&') && !member_type.starts_with("&self") {
                        let inner_type = member_type[1..].trim();
                        format!("const {}&", self.convert_types_in_string(inner_type))
                    } else {
                        self.convert_types_in_string(member_type)
                    };

                // 返回正确的C++格式：Type name; (不是 name: Type,)
                return Ok(format!("{} {};", converted_type, member_name));
            }
        }

        // 🔴 新增：处理泛型struct定义 struct Name<T> { -> template<typename T> struct Name {
        if content.contains('<') && (content.ends_with('{') || content.contains(" {")) {
            // 提取泛型参数
            if let Some(angle_start) = content.find('<') {
                if let Some(angle_end) = content.find('>') {
                    let struct_name = content[..angle_start].trim();
                    let generics = &content[angle_start + 1..angle_end];
                    let rest = &content[angle_end + 1..].trim();

                    // 转换泛型参数：T, U -> typename T, typename U
                    let generic_params: Vec<String> = generics
                        .split(',')
                        .map(|g| format!("typename {}", g.trim()))
                        .collect();

                    let template_params = generic_params.join(", ");

                    // 生成 template<typename T> struct Name {
                    return Ok(format!(
                        "template<{}> struct {} {}",
                        template_params, struct_name, rest
                    ));
                }
            }
        }

        // 处理 tuple struct 字段的可见性
        // 检查是否是tuple struct: struct Name(Type1, Type2);
        let converted =
            if content.contains('(') && content.contains(')') && content.trim().ends_with(';') {
                // Tuple struct
                let struct_parts: Vec<&str> = content.splitn(2, '(').collect();
                if struct_parts.len() == 2 {
                    let struct_name = struct_parts[0];
                    let rest = struct_parts[1]; // "Type1, Type2);"

                    // 提取字段类型部分
                    if let Some(close_paren_pos) = rest.find(')') {
                        let fields = &rest[..close_paren_pos]; // "Type1, Type2"
                        let suffix = &rest[close_paren_pos..]; // ");" 或 ")"

                        // 转换字段类型
                        let converted_fields: Vec<String> = fields
                            .split(',')
                            .map(|f| {
                                let trimmed = f.trim();
                                if trimmed.is_empty() {
                                    String::new()
                                } else {
                                    self.convert_types_in_string(trimmed)
                                }
                            })
                            .filter(|s| !s.is_empty())
                            .collect();

                        let converted_name = self.convert_types_in_string(struct_name);
                        let converted_fields_str = converted_fields.join(", ");
                        let real_suffix = if suffix.starts_with(')') {
                            &suffix[1..]
                        } else {
                            suffix
                        };
                        format!(
                            "{}({}){}",
                            converted_name, converted_fields_str, real_suffix
                        )
                    } else {
                        self.convert_types_in_string(content)
                    }
                } else {
                    self.convert_types_in_string(content)
                }
            } else {
                self.convert_types_in_string(content)
            };

        Ok(format!("struct {}", converted))
    }

    fn convert_enum(&self, line: &str, context: &mut ConversionContext) -> Result<String> {
        let content = &line[2..];

        // 根据NU2CPP23.md规范: E Shape { Circle(f32) }
        // 应该生成: struct Circle { float _0; }; using Shape = std::variant<Circle>;

        // 检查是否是带数据的enum变体（在enum块内）
        if content.contains('(') && !content.ends_with('{') && !content.contains("enum ") {
            // 这是enum变体，如 Move { x: i32, y: i32 } 或 Circle(f32)
            let parts: Vec<&str> = content.splitn(2, '(').collect();
            if parts.len() == 2 {
                let variant_name = parts[0].trim();
                // 🔴 记录变体名称
                context.enum_variants.push(variant_name.to_string());
                let variant_data = parts[1].trim().trim_end_matches(')').trim_end_matches(',');

                // 转换类型
                let converted_type = self.convert_types_in_string(variant_data);

                // 检查是否有多个字段（逗号分隔）
                let fields: Vec<&str> = variant_data.split(',').collect();

                if fields.len() == 1 {
                    // 单字段tuple variant: Circle(f32) -> struct Circle { float _0; };
                    return Ok(format!(
                        "struct {} {{ {} _0; }};",
                        variant_name, converted_type
                    ));
                } else {
                    // 多字段: 生成带编号的字段
                    let mut struct_def = format!("struct {} {{", variant_name);
                    for (i, field) in fields.iter().enumerate() {
                        let field_type = self.convert_types_in_string(field.trim());
                        struct_def.push_str(&format!(" {} _{};", field_type, i));
                    }
                    struct_def.push_str(" };");
                    return Ok(struct_def);
                }
            }
        }

        // 检查是否是带命名字段的变体: Move { x: i32, y: i32 }
        if content.contains('{') && !content.contains("enum ") && !content.ends_with('{') {
            let parts: Vec<&str> = content.splitn(2, '{').collect();
            if parts.len() == 2 {
                let variant_name = parts[0].trim();
                // 🔴 记录变体名称
                context.enum_variants.push(variant_name.to_string());
                let fields_str = parts[1].trim().trim_end_matches('}').trim_end_matches(',');

                // 解析字段: x: i32, y: i32
                let mut struct_def = format!("struct {} {{", variant_name);
                for field in fields_str.split(',') {
                    let field = field.trim();
                    if let Some(colon_pos) = field.find(':') {
                        let field_name = field[..colon_pos].trim();
                        let field_type = field[colon_pos + 1..].trim();
                        let converted_type = self.convert_types_in_string(field_type);
                        struct_def.push_str(&format!(" {} {};", converted_type, field_name));
                    }
                }
                struct_def.push_str(" };");
                return Ok(struct_def);
            }
        }

        // 简单enum变体（无数据）
        if context.in_enum_block && !content.contains("enum ") && !content.ends_with('{') {
            let variant_name = content.trim().trim_end_matches(',');
            if !variant_name.is_empty()
                && !variant_name.contains('(')
                && !variant_name.contains('{')
            {
                context.enum_variants.push(variant_name.to_string());
                // 简单变体转为空struct
                return Ok(format!("struct {};", variant_name));
            }
        }

        // 简单enum变体（无数据）
        if context.in_enum_block && !content.contains("enum ") && !content.ends_with('{') {
            let variant_name = content.trim().trim_end_matches(',');
            if !variant_name.is_empty()
                && !variant_name.contains('(')
                && !variant_name.contains('{')
            {
                context.enum_variants.push(variant_name.to_string());
                // 简单变体转为空struct
                return Ok(format!("struct {};", variant_name));
            }
        }

        let converted = self.convert_types_in_string(content);

        // 检查是否是enum定义行（包含enum关键字或以{结尾）
        if content.ends_with('{') || content.contains("enum ") {
            Ok(format!("enum {}", converted))
        } else {
            // 这是enum变体（简单形式），保持原样
            Ok(converted)
        }
    }

    fn convert_trait(&self, line: &str) -> Result<String> {
        let content = &line[3..];
        let converted = self.convert_types_in_string(content);
        Ok(format!("class {} {{", converted.trim_end_matches(" {")))
    }

    fn convert_impl(&self, line: &str) -> Result<String> {
        let content = &line[2..];

        // 根据NU2CPP23.md: impl块方法应该注入到struct定义中
        // 但在逐行转换中，我们无法回溯修改前面的struct定义
        // 因此这里生成注释，并在方法转换时不添加作用域前缀

        if content.contains(" for ") {
            // trait implementation: impl Trait for Type
            let parts: Vec<&str> = content.split(" for ").collect();
            if parts.len() == 2 {
                let trait_name = parts[0].trim();
                let type_name = parts[1].trim().trim_end_matches(" {").trim();

                // C++中trait实现需要在类定义内部或使用concept
                // 生成注释说明，方法会在后续转换中处理
                return Ok(format!(
                    "// Implementation of {} for {} - methods follow",
                    trait_name, type_name
                ));
            }
        }

        // 普通impl块: impl Type { ... }
        let type_name = content.trim().trim_end_matches(" {").trim();

        // 注意: C++中需要将这些方法注入到struct定义中
        // 在逐行转换模式下，我们只能生成带作用域的方法定义
        // 格式: ReturnType ClassName::methodName(params) { body }
        Ok(format!("// Methods for {}", type_name))
    }

    fn convert_module(&self, line: &str) -> Result<String> {
        let content = &line[2..];
        let converted = self.convert_types_in_string(content);
        Ok(format!("namespace {}", converted))
    }

    fn convert_pub_module(&self, line: &str) -> Result<String> {
        let content = &line[3..];
        let converted = self.convert_types_in_string(content);
        Ok(format!("namespace {}", converted))
    }

    fn convert_let(&self, line: &str) -> Result<String> {
        let content = &line[2..];

        // 检查是否包含闭包（|...|）- 如果是闭包，暂时不转换，保持原样
        // 闭包转换会在后续的行处理中完成
        if content.contains('|') && !content.contains("||") {
            // 可能是闭包，直接转换类型但不处理闭包语法
            let converted = self.convert_inline_keywords(content)?;
            let converted = self.convert_macros(&converted)?;
            // 跳过闭包转换，只做类型转换
            let mut result = content.to_string();

            // 基本类型转换
            result = result
                .replace("i32", "int32_t")
                .replace("i64", "int64_t")
                .replace("u32", "uint32_t")
                .replace("u64", "uint64_t")
                .replace("f32", "float")
                .replace("f64", "double");

            // P1-1: 修复变量声明语法
            let result = self.fix_variable_declaration(&result)?;

            return Ok(format!("const auto {}", result));
        }

        let converted = self.convert_inline_keywords(content)?;
        let converted = self.convert_macros(&converted)?;
        let converted = self.convert_types_in_string(&converted);

        // P1-1: 修复变量声明语法 auto name : Type → Type name
        let converted = self.fix_variable_declaration(&converted)?;

        Ok(format!("const auto {}", converted))
    }

    fn convert_let_mut(&self, line: &str) -> Result<String> {
        let content = &line[2..];

        // 检查是否包含闭包（|...|）- 如果是闭包，暂时不转换
        if content.contains('|') && !content.contains("||") {
            // 可能是闭包，直接转换类型但不处理闭包语法
            let mut result = content.to_string();

            // 基本类型转换
            result = result
                .replace("i32", "int32_t")
                .replace("i64", "int64_t")
                .replace("u32", "uint32_t")
                .replace("u64", "uint64_t")
                .replace("f32", "float")
                .replace("f64", "double");

            // P1-1: 修复变量声明语法
            let result = self.fix_variable_declaration(&result)?;

            return Ok(format!("auto {}", result));
        }

        let converted = self.convert_inline_keywords(content)?;
        let converted = self.convert_macros(&converted)?;
        let converted = self.convert_types_in_string(&converted);

        // P1-1: 修复变量声明语法 auto name : Type → Type name
        let converted = self.fix_variable_declaration(&converted)?;

        Ok(format!("auto {}", converted))
    }

    /// P1-1增强: 修复变量声明语法：name : Type = value → name = value (类型推导)
    fn fix_variable_declaration(&self, content: &str) -> Result<String> {
        // 如果包含 : 检查是否是类型标注（排除 :: 命名空间分隔符和其他情况）
        if !content.contains(':') {
            return Ok(content.to_string());
        }

        // 检查是否已经是合法的C++语法（如果有::则可能是命名空间）
        // 但对于`name : Type = value`这种情况，我们需要转换

        // 查找第一个冒号的位置
        if let Some(colon_pos) = content.find(':') {
            // 检查是否是 :: （C++命名空间）
            if colon_pos + 1 < content.len() && content.chars().nth(colon_pos + 1) == Some(':') {
                // 这是 ::，不是类型标注
                return Ok(content.to_string());
            }

            // 检查冒号前面是否是一个标识符（变量名）
            let before_colon = &content[..colon_pos];
            let var_name = before_colon.trim();

            // 检查是否是合法的变量名（字母、数字、下划线）
            if var_name.is_empty() || !var_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                // 不是合法的变量名，可能是其他语法，保持原样
                return Ok(content.to_string());
            }

            // 提取类型和值部分
            let after_colon = &content[colon_pos + 1..];

            // 查找等号
            if let Some(eq_pos) = after_colon.find('=') {
                // 格式：name : Type = value
                // 转换为：name = value (移除类型标注，使用auto推导)
                let after_eq = after_colon[eq_pos + 1..].trim();
                return Ok(format!("{} = {}", var_name, after_eq));
            } else if after_colon.trim().ends_with(';') {
                // 格式：name : Type;
                // 转换为：name; (移除类型标注) - 但这种情况很少见
                return Ok(format!("{};", var_name));
            } else {
                // 只有类型没有初始化，这种情况保持原样或标记错误
                // 例如: name : Type
                // C++中使用auto必须有初始化值，所以这是错误
                return Ok(content.to_string());
            }
        }

        Ok(content.to_string())
    }

    fn convert_return(&self, line: &str) -> Result<String> {
        if line == "<" {
            Ok("return;".to_string())
        } else {
            let content = &line[2..];
            let mut converted = self.convert_types_in_string(content);

            // P0修复: 转换self为this->
            converted = self.replace_self_with_this(&converted);

            Ok(format!("return {};", converted.trim_end_matches(';')))
        }
    }

    fn convert_loop(&self, line: &str) -> Result<String> {
        if line == "L {" {
            return Ok("while (true) {".to_string());
        }

        if let Some(content) = line.strip_prefix("L ") {
            if content.contains(" in ") {
                // P0修复: for循环，特别处理范围表达式
                let parts: Vec<&str> = content.splitn(2, " in ").collect();
                if parts.len() == 2 {
                    let var_name = parts[0].trim();
                    let range_expr = parts[1].trim().trim_end_matches(" {").trim();

                    // 检查是否是范围表达式 0..10
                    if range_expr.contains("..") {
                        let range_parts: Vec<&str> = range_expr.splitn(2, "..").collect();
                        if range_parts.len() == 2 {
                            let start = range_parts[0].trim();
                            let end = range_parts[1].trim();
                            // 转换为标准C++ for循环
                            return Ok(format!(
                                "for (int {} = {}; {} < {}; {}++) {{",
                                var_name, start, var_name, end, var_name
                            ));
                        }
                    }

                    // 其他情况使用range-based for
                    let converted = self.convert_inline_keywords(range_expr)?;
                    let converted = self.convert_types_in_string(&converted);
                    return Ok(format!("for (auto {} : {}) {{", var_name, converted));
                }

                // 回退到原来的逻辑
                let converted = self.convert_inline_keywords(content)?;
                let converted = self.convert_types_in_string(&converted);
                return Ok(format!("for (auto {}", converted));
            }
            return Ok("while (true) {".to_string());
        }

        Ok(line.to_string())
    }

    fn convert_if(&self, line: &str) -> Result<String> {
        let content = if line.starts_with("if(") {
            &line[2..]
        } else {
            &line[3..]
        };

        let converted = self.convert_inline_keywords(content)?;
        let converted = self.convert_macros(&converted)?;
        let converted = self.convert_types_in_string(&converted);

        // 确保条件表达式被括号包围
        let trimmed = converted.trim();
        if trimmed.starts_with('(') {
            Ok(format!("if {}", converted))
        } else {
            // 找到第一个{的位置，将之前的内容用括号包围
            if let Some(brace_pos) = trimmed.find('{') {
                let condition = trimmed[..brace_pos].trim();
                let rest = &trimmed[brace_pos..];
                Ok(format!("if ({}) {}", condition, rest))
            } else {
                // 整个都是条件
                Ok(format!("if ({})", converted))
            }
        }
    }

    fn convert_if_legacy(&self, line: &str) -> Result<String> {
        let content = if line.starts_with("?(") {
            &line[1..]
        } else {
            &line[2..]
        };

        let converted = self.convert_inline_keywords(content)?;
        let converted = self.convert_types_in_string(&converted);
        Ok(format!("if {}", converted))
    }

    /// v2.0新增: 转换 If not 语句 (?! -> if !)
    fn convert_if_not(&self, line: &str) -> Result<String> {
        let content = &line[3..]; // 跳过 "?! "

        let converted = self.convert_inline_keywords(content)?;
        let converted = self.convert_macros(&converted)?;
        let converted = self.convert_types_in_string(&converted);
        let converted = self.replace_self_with_this(&converted);

        // 确保条件表达式被括号包围
        let trimmed = converted.trim();
        if trimmed.starts_with('(') {
            Ok(format!("if (!{}", converted))
        } else {
            // 找到第一个{的位置，将之前的内容用括号包围
            if let Some(brace_pos) = trimmed.find('{') {
                let condition = trimmed[..brace_pos].trim();
                let rest = &trimmed[brace_pos..];
                Ok(format!("if (!({})) {}", condition, rest))
            } else {
                // 整个都是条件
                Ok(format!("if (!({})))", converted))
            }
        }
    }

    /// v2.0新增: 转换 unsafe 函数
    fn convert_unsafe_function(&self, line: &str, context: &ConversionContext) -> Result<String> {
        let is_pub = line.starts_with("unsafe F ");
        let content = if is_pub { &line[9..] } else { &line[8..] }; // 跳过 "unsafe F " 或 "unsafe f "

        let converted = self.convert_types_in_string(content);

        // 检查是否是main函数
        if converted.trim_start().starts_with("main(") {
            return self.convert_main_function(&converted);
        }

        // 检查是否包含&self或&mut self参数
        let has_self_param = converted.contains("&self") || converted.contains("&mut self");

        let visibility = if context.in_class {
            "" // 类内方法不需要public:前缀
        } else if is_pub {
            ""
        } else {
            if has_self_param {
                ""
            } else {
                "static "
            }
        };

        // 替换Self为实际的类名，并移除&self参数
        let mut result = self.convert_function_signature(&converted, context)?;
        if let Some(ref class_name) = context.current_class_name {
            result = result.replace("Self", class_name);
        }

        result = self.remove_self_parameter(&result);

        // C++没有unsafe概念，添加注释说明
        Ok(format!("{}/* unsafe */ {}", visibility, result))
    }

    /// v2.0新增: 转换 const 函数
    fn convert_const_function(&self, line: &str, context: &ConversionContext) -> Result<String> {
        let is_pub = line.starts_with("const F ");
        let content = if is_pub { &line[8..] } else { &line[7..] }; // 跳过 "const F " 或 "const f "

        let converted = self.convert_types_in_string(content);

        // 检查是否是main函数
        if converted.trim_start().starts_with("main(") {
            return self.convert_main_function(&converted);
        }

        // 检查是否包含&self或&mut self参数
        let has_self_param = converted.contains("&self") || converted.contains("&mut self");

        let visibility = if context.in_class {
            "" // 类内方法不需要public:前缀
        } else if is_pub {
            ""
        } else {
            if has_self_param {
                ""
            } else {
                "static "
            }
        };

        // 替换Self为实际的类名，并移除&self参数
        let mut result = self.convert_function_signature(&converted, context)?;
        if let Some(ref class_name) = context.current_class_name {
            result = result.replace("Self", class_name);
        }

        result = self.remove_self_parameter(&result);

        // C++使用constexpr (C++11+)
        Ok(format!("{}constexpr {}", visibility, result))
    }

    /// 辅助方法: 替换 self 为 this (C++风格)
    fn replace_self_with_this(&self, s: &str) -> String {
        s.replace("self.", "this->").replace("self", "(*this)")
    }

    /// P0修复3: 转换 Self { x, y } 为 return ClassName(x, y);
    fn convert_self_initializer(&self, s: &str, class_name: &str) -> String {
        let mut result = s.to_string();

        // 查找 Self { ... } 模式
        if result.contains("Self {") {
            let mut i = 0;
            let chars: Vec<char> = result.chars().collect();
            let mut new_result = String::new();

            while i < chars.len() {
                // 查找 "Self {"
                if i + 6 <= chars.len() {
                    let slice: String = chars[i..i.min(i + 6)].iter().collect();
                    if slice.starts_with("Self {") {
                        // 找到匹配的 }
                        let start = i + 5; // 跳过 "Self "
                        i += 6; // 跳过 "Self {"
                        let mut depth = 1;
                        let brace_start = i;

                        while i < chars.len() && depth > 0 {
                            if chars[i] == '{' {
                                depth += 1;
                            } else if chars[i] == '}' {
                                depth -= 1;
                            }
                            if depth > 0 {
                                i += 1;
                            }
                        }

                        // 提取初始化列表
                        let init_list: String = chars[brace_start..i].iter().collect();
                        let init_list = init_list.trim();

                        // 转换为C++构造函数调用
                        new_result.push_str(&format!("return {}({})", class_name, init_list));
                        i += 1; // 跳过 }
                        continue;
                    }
                }

                new_result.push(chars[i]);
                i += 1;
            }

            result = new_result;
        }

        result
    }

    fn convert_match(&self, line: &str) -> Result<String> {
        let content = if line.starts_with("M(") {
            &line[1..]
        } else {
            &line[2..]
        };

        // 检查是否是 match arm (包含 =>)
        if content.contains("=>") {
            return self.convert_match_arm(content);
        }

        let converted = self.convert_inline_keywords(content)?;
        let converted = self.convert_macros(&converted)?;
        let converted = self.convert_types_in_string(&converted);

        // P1-3: 基础match→switch转换
        // 格式：M value { ... } 或 M(value) { ... }

        // 提取被匹配的值
        let match_value = if converted.contains('{') {
            converted.split('{').next().unwrap_or("").trim()
        } else {
            &converted
        };

        // 提取match值（去除括号）
        let match_expr = if match_value.starts_with('(') && match_value.ends_with(')') {
            match_value.trim_matches(|c| c == '(' || c == ')').trim()
        } else {
            match_value
        };

        // 检查是否有大括号
        if converted.contains('{') {
            // 完整的match块: M value { ... }
            let rest = if let Some(pos) = converted.find('{') {
                &converted[pos..]
            } else {
                ""
            };

            // 转换为switch语句
            Ok(format!("switch ({}) {}", match_expr, rest))
        } else {
            // 只有match表达式头部
            Ok(format!("switch ({}) {{", match_expr))
        }
    }

    /// P1-3增强: 转换 match arm，完整处理 => 语法（switch case格式）
    fn convert_match_arm(&self, line: &str) -> Result<String> {
        // 检查是否包含 => 操作符
        if !line.contains("=>") {
            return Ok(line.to_string());
        }

        let parts: Vec<&str> = line.splitn(2, "=>").collect();
        if parts.len() != 2 {
            return Ok(line.to_string());
        }

        let pattern = parts[0].trim();
        let expr = parts[1].trim().trim_end_matches(',');

        // 转换表达式中的宏和类型
        let converted_expr = self.convert_macros(expr)?;
        let converted_expr = self.convert_types_in_string(&converted_expr);

        // 处理下划线通配符模式 _ => expr
        if pattern == "_" {
            return Ok(format!("default: {}; break;", converted_expr));
        }

        // 处理多分支模式 "a" | "b" | "c" => expr
        if pattern.contains('|') {
            let patterns: Vec<&str> = pattern.split('|').map(|s| s.trim()).collect();
            let mut result = String::new();
            for (i, p) in patterns.iter().enumerate() {
                result.push_str(&format!("case {}: ", p));
                if i == patterns.len() - 1 {
                    // 最后一个模式添加动作
                    result.push_str(&format!("{}; break;", converted_expr));
                }
            }
            return Ok(result);
        }

        // 处理enum变体模式
        if pattern.contains("::") {
            // 枚举变体: Operator::Add => "+"
            let enum_parts: Vec<&str> = pattern.split("::").collect();
            if enum_parts.len() == 2 {
                let enum_name = enum_parts[0].trim();
                let variant = enum_parts[1].trim();
                return Ok(format!(
                    "case {}::{}: {}; break;",
                    enum_name, variant, converted_expr
                ));
            }
        }

        // 处理Result/Option模式（使用if-else而不是switch）
        if pattern == "None" {
            return Ok(format!("default: {}; break;", converted_expr));
        } else if pattern.starts_with("Some(") || pattern.starts_with("Ok(") {
            let var = if pattern.starts_with("Some(") {
                pattern.trim_start_matches("Some(").trim_end_matches(')')
            } else {
                pattern.trim_start_matches("Ok(").trim_end_matches(')')
            };
            return Ok(format!(
                "if (value.has_value()) {{ auto {} = value.value(); {}; }}",
                var, converted_expr
            ));
        } else if pattern.starts_with("Err(") {
            let var = pattern.trim_start_matches("Err(").trim_end_matches(')');
            return Ok(format!(
                "default: {{ auto {} = value.error(); {}; }} break;",
                var, converted_expr
            ));
        }

        // 字符串或其他字面量模式 - 标准case语句
        Ok(format!("case {}: {}; break;", pattern, converted_expr))
    }

    fn convert_print(&self, line: &str) -> Result<String> {
        let content = &line[2..];
        let converted = self.convert_types_in_string(content);
        Ok(format!(
            "std::cout << {} << std::endl;",
            converted.trim_end_matches(';')
        ))
    }

    fn convert_use(&self, line: &str) -> Result<String> {
        let content = &line[2..];
        // C++使用不同的include系统，将use语句转换为注释
        Ok(format!("// use {}", content.trim()))
    }

    fn convert_type_alias(&self, line: &str) -> Result<String> {
        let content = &line[2..];
        let converted = self.convert_types_in_string(content);
        Ok(format!("using {}", converted))
    }

    fn convert_const(&self, line: &str, is_pub: bool) -> Result<String> {
        let content = if is_pub { &line[3..] } else { &line[2..] };
        let converted = self.convert_types_in_string(content);
        Ok(format!("const {}", converted))
    }

    fn convert_expression(&self, line: &str) -> Result<String> {
        // P0修复: 检查是否是结构体字段声明 (在struct块内的 name: Type, 格式)
        // 这是一个fallback，处理那些没有被convert_struct捕获的字段
        let trimmed = line.trim();
        if trimmed.contains(':')
            && !trimmed.contains("::")
            && !trimmed.starts_with("//")
            && !trimmed.starts_with("if")
            && !trimmed.starts_with("for")
            && !trimmed.starts_with("while")
            && !trimmed.starts_with("match")
            && !trimmed.contains('(')
            && !trimmed.contains('{')
        {
            // 可能是结构体字段，尝试转换
            let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
            if parts.len() == 2 {
                let member_name = parts[0].trim();
                let member_type = parts[1].trim().trim_end_matches(',').trim();

                // 检查member_name是否是合法标识符
                if member_name.chars().all(|c| c.is_alphanumeric() || c == '_')
                    && !member_name.is_empty()
                {
                    // 应用类型转换
                    let converted_type =
                        if member_type.starts_with('&') && !member_type.starts_with("&self") {
                            let inner_type = member_type[1..].trim();
                            format!("const {}&", self.convert_types_in_string(inner_type))
                        } else {
                            self.convert_types_in_string(member_type)
                        };

                    // 返回正确的C++格式：Type name;
                    return Ok(format!("{} {};", converted_type, member_name));
                }
            }
        }

        let mut result = self.convert_inline_keywords(line)?;

        // P1-6: 先转换vec!宏（在其他宏之前）
        if result.contains("vec!") || result.contains("vec !") {
            result = self.convert_vec_bang_macro(&result);
        }

        result = self.convert_macros(&result)?;
        result = self.convert_types_in_string(&result);

        // P1-1: 全局应用变量声明语法修复 (关键修复)
        result = self.fix_variable_declaration(&result)?;

        // P1-3: 转换 as 类型转换
        result = self.convert_as_cast(&result)?;

        // P1-5: 转换 format! 宏
        result = self.convert_format_macro(&result)?;

        // P1-7: 转换 std::string::from() - 再次确保转换
        if result.contains("::from(") {
            result = self.convert_string_from(&result)?;
        }

        // 修复函数调用格式：确保函数名和参数之间有正确的括号
        result = self.fix_function_calls(&result)?;

        Ok(result)
    }

    /// P1-3: 转换 as 类型转换：(expr as Type) → static_cast<Type>(expr)
    fn convert_as_cast(&self, content: &str) -> Result<String> {
        let mut result = String::new();
        let char_indices: Vec<(usize, char)> = content.char_indices().collect();
        let mut i = 0;

        while i < char_indices.len() {
            // 查找 " as " 模式
            if i + 4 <= char_indices.len() {
                let current_byte = char_indices[i].0;
                let end_byte = if i + 4 < char_indices.len() {
                    char_indices[i + 4].0
                } else {
                    content.len()
                };

                if current_byte < content.len()
                    && end_byte <= content.len()
                    && &content[current_byte..end_byte] == " as "
                {
                    // 找到 as 关键字，需要向前找表达式起点
                    let mut expr_start = 0;
                    let mut depth = 0;
                    let mut j = i - 1;

                    // 向前查找匹配的表达式（处理括号）
                    loop {
                        if char_indices[j].1 == ')' {
                            depth += 1;
                        } else if char_indices[j].1 == '(' {
                            if depth == 0 {
                                expr_start = j;
                                break;
                            }
                            depth -= 1;
                        } else if depth == 0
                            && (char_indices[j].1.is_whitespace()
                                || char_indices[j].1 == ','
                                || char_indices[j].1 == '='
                                || char_indices[j].1 == '{')
                        {
                            expr_start = j + 1;
                            break;
                        }

                        if j == 0 {
                            expr_start = 0;
                            break;
                        }
                        j -= 1;
                    }

                    // 提取表达式
                    let expr_start_byte = char_indices[expr_start].0;
                    let expr_end_byte = char_indices[i].0;
                    let expr = content[expr_start_byte..expr_end_byte].trim();

                    // 跳过 " as "
                    i += 4;

                    // 提取类型（查找到下一个空格、逗号或括号）
                    let type_start = i;
                    while i < char_indices.len() {
                        let ch = char_indices[i].1;
                        if ch.is_whitespace() || ch == ',' || ch == ')' || ch == ';' || ch == '}' {
                            break;
                        }
                        i += 1;
                    }

                    let type_start_byte = char_indices[type_start].0;
                    let type_end_byte = if i < char_indices.len() {
                        char_indices[i].0
                    } else {
                        content.len()
                    };
                    let target_type = content[type_start_byte..type_end_byte].trim();

                    // 移除已添加的表达式
                    let expr_len = expr.len();
                    if result.len() >= expr_len {
                        result.truncate(result.len() - expr_len);
                    }

                    // 转换为 static_cast
                    result.push_str(&format!("static_cast<{}>({})", target_type, expr));
                    continue;
                }
            }

            result.push(char_indices[i].1);
            i += 1;
        }

        Ok(result)
    }

    /// P1-5: 转换 format! 宏
    fn convert_format_macro(&self, content: &str) -> Result<String> {
        let mut result = String::new();
        let char_indices: Vec<(usize, char)> = content.char_indices().collect();
        let mut i = 0;

        while i < char_indices.len() {
            // 检查是否是 format!
            if i + 7 <= char_indices.len() {
                let current_byte = char_indices[i].0;
                let end_byte = if i + 7 < char_indices.len() {
                    char_indices[i + 7].0
                } else {
                    content.len()
                };

                if current_byte < content.len()
                    && end_byte <= content.len()
                    && &content[current_byte..end_byte] == "format!"
                {
                    i += 7;

                    // 跳过空白
                    while i < char_indices.len() && char_indices[i].1.is_whitespace() {
                        i += 1;
                    }

                    // 期望是 (
                    if i < char_indices.len() && char_indices[i].1 == '(' {
                        i += 1;
                        let start_i = i;
                        let mut depth = 1;
                        let mut in_string = false;
                        let mut escape_next = false;

                        // 找到匹配的右括号
                        while i < char_indices.len() && depth > 0 {
                            if escape_next {
                                escape_next = false;
                                i += 1;
                                continue;
                            }

                            if char_indices[i].1 == '\\' {
                                escape_next = true;
                            } else if char_indices[i].1 == '"' {
                                in_string = !in_string;
                            } else if !in_string {
                                if char_indices[i].1 == '(' {
                                    depth += 1;
                                } else if char_indices[i].1 == ')' {
                                    depth -= 1;
                                }
                            }

                            if depth > 0 {
                                i += 1;
                            }
                        }

                        let start_byte = char_indices[start_i].0;
                        let end_byte = if i < char_indices.len() {
                            char_indices[i].0
                        } else {
                            content.len()
                        };

                        if start_byte <= content.len() && end_byte <= content.len() {
                            let args = &content[start_byte..end_byte];
                            i += 1;

                            // 转换 format! 为字符串拼接或 std::format (C++20)
                            result.push_str(&self.convert_format_args(args)?);
                            continue;
                        }
                    }
                }
            }

            result.push(char_indices[i].1);
            i += 1;
        }

        Ok(result)
    }

    /// 转换 format! 参数为C++字符串拼接
    fn convert_format_args(&self, args: &str) -> Result<String> {
        let args = args.trim();

        if args.is_empty() {
            return Ok("\"\"".to_string());
        }

        // 检查是否有格式化参数
        if args.contains("{}") {
            // 查找第一个字符串后的逗号
            let mut in_string = false;
            let mut escape_next = false;
            let mut split_pos = None;

            for (byte_idx, ch) in args.char_indices() {
                if escape_next {
                    escape_next = false;
                    continue;
                }
                if ch == '\\' {
                    escape_next = true;
                    continue;
                }
                if ch == '"' {
                    in_string = !in_string;
                }
                if !in_string && ch == ',' {
                    split_pos = Some(byte_idx);
                    break;
                }
            }

            if let Some(pos) = split_pos {
                let format_str = args[..pos].trim();
                let values = args[pos + 1..].trim();

                // 将格式字符串和值组合成C++字符串拼接
                let format_inner = format_str.trim_matches('"');
                let format_parts: Vec<&str> = format_inner.split("{}").collect();
                let value_parts: Vec<&str> = values.split(',').map(|s| s.trim()).collect();

                let mut output = String::new();
                for (i, part) in format_parts.iter().enumerate() {
                    if !part.is_empty() {
                        if !output.is_empty() {
                            output.push_str(" + ");
                        }
                        output.push_str(&format!("\"{}\"", part));
                    }
                    if i < value_parts.len() {
                        if !output.is_empty() {
                            output.push_str(" + ");
                        }
                        output.push_str(&format!("std::to_string({})", value_parts[i]));
                    }
                }

                return Ok(output);
            }
        }

        // 简单字符串
        Ok(args.to_string())
    }

    /// P1-7: 转换 std::string::from() 和 String::from()
    fn convert_string_from(&self, content: &str) -> Result<String> {
        let mut result = String::new();
        let chars: Vec<char> = content.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // 检查 std::string::from(
            if i + 18 <= chars.len() {
                let slice: String = chars[i..i + 18].iter().collect();
                if slice == "std::string::from(" {
                    i += 18; // 跳过 std::string::from(
                    let start = i;
                    let mut depth = 1;
                    let mut in_string = false;
                    let mut escape_next = false;

                    // 找到匹配的右括号
                    while i < chars.len() && depth > 0 {
                        if escape_next {
                            escape_next = false;
                            i += 1;
                            continue;
                        }
                        if chars[i] == '\\' {
                            escape_next = true;
                        } else if chars[i] == '"' {
                            in_string = !in_string;
                        } else if !in_string {
                            if chars[i] == '(' {
                                depth += 1;
                            } else if chars[i] == ')' {
                                depth -= 1;
                            }
                        }
                        if depth > 0 {
                            i += 1;
                        }
                    }

                    let arg: String = chars[start..i].iter().collect();
                    i += 1; // 跳过 )
                    result.push_str(&arg); // 直接使用参数（应该是字符串字面量）
                    continue;
                }
            }

            // 检查 String::from(
            if i + 13 <= chars.len() {
                let slice: String = chars[i..i + 13].iter().collect();
                if slice == "String::from(" {
                    i += 13; // 跳过 String::from(
                    let start = i;
                    let mut depth = 1;
                    let mut in_string = false;
                    let mut escape_next = false;

                    // 找到匹配的右括号
                    while i < chars.len() && depth > 0 {
                        if escape_next {
                            escape_next = false;
                            i += 1;
                            continue;
                        }
                        if chars[i] == '\\' {
                            escape_next = true;
                        } else if chars[i] == '"' {
                            in_string = !in_string;
                        } else if !in_string {
                            if chars[i] == '(' {
                                depth += 1;
                            } else if chars[i] == ')' {
                                depth -= 1;
                            }
                        }
                        if depth > 0 {
                            i += 1;
                        }
                    }

                    let arg: String = chars[start..i].iter().collect();
                    i += 1; // 跳过 )
                    result.push_str(&arg); // 直接使用参数
                    continue;
                }
            }

            result.push(chars[i]);
            i += 1;
        }

        Ok(result)
    }

    /// 修复函数调用格式，确保函数名和参数列表正确分隔
    fn fix_function_calls(&self, content: &str) -> Result<String> {
        let result = content.to_string();

        // 查找可能的错误模式：标识符后直接跟着std::tuple<...>
        // 这通常意味着函数名和参数被粘连了
        if result.contains("std::tuple<") {
            let mut fixed = String::new();
            let char_indices: Vec<(usize, char)> = result.char_indices().collect();
            let mut i = 0;

            while i < char_indices.len() {
                let (byte_pos, ch) = char_indices[i];

                // 查找标识符
                if ch.is_alphabetic() || ch == '_' {
                    let start_idx = i;
                    let start_byte = byte_pos;

                    while i < char_indices.len() && {
                        let (_, c) = char_indices[i];
                        c.is_alphanumeric() || c == '_'
                    } {
                        i += 1;
                    }

                    let end_byte = if i < char_indices.len() {
                        char_indices[i].0
                    } else {
                        result.len()
                    };

                    let identifier = &result[start_byte..end_byte];
                    fixed.push_str(identifier);

                    // 检查是否紧跟着 std::tuple<（安全检查字节边界）
                    if i + 11 <= char_indices.len() {
                        let tuple_start = if i < char_indices.len() {
                            char_indices[i].0
                        } else {
                            result.len()
                        };
                        let tuple_end = if i + 11 < char_indices.len() {
                            char_indices[i + 11].0
                        } else {
                            result.len()
                        };

                        if tuple_start < result.len()
                            && tuple_end <= result.len()
                            && &result[tuple_start..tuple_end] == "std::tuple<"
                        {
                            // 这是一个函数调用被错误转换了
                            // 将 std::tuple<...> 转换回 (...)
                            fixed.push('(');
                            i += 11; // 跳过 "std::tuple<"

                            let mut depth = 1;
                            let mut args = String::new();

                            while i < char_indices.len() && depth > 0 {
                                let (_, c) = char_indices[i];
                                if c == '<' {
                                    depth += 1;
                                    args.push(c);
                                } else if c == '>' {
                                    depth -= 1;
                                    if depth > 0 {
                                        args.push(c);
                                    }
                                } else {
                                    args.push(c);
                                }
                                i += 1;
                            }

                            fixed.push_str(&args);
                            fixed.push(')');
                            continue;
                        }
                    }
                }

                if i < char_indices.len() {
                    fixed.push(char_indices[i].1);
                    i += 1;
                }
            }

            return Ok(fixed);
        }

        Ok(result)
    }

    /// 转换宏调用（println!, V!, Some, Ok, Err 等）
    fn convert_macros(&self, content: &str) -> Result<String> {
        let mut result = content.to_string();

        // 转换 println! 宏
        if result.contains("println!") {
            result = self.convert_println_macro(&result)?;
        }

        // 转换 V![...] 宏为 std::vector<T>{...}
        if result.contains("V![") {
            result = self.convert_vec_macro(&result)?;
        }

        // 转换 Ok(...) 为 return value（用于Result类型）
        if result.contains("Ok(") {
            result = self.convert_ok_constructor(&result)?;
        }

        // 转换 Err(...) 为 return std::unexpected(...)（用于Result类型）
        if result.contains("Err(") {
            result = self.convert_err_constructor(&result)?;
        }

        // 转换 Some(...) 为 std::optional<T>{...}
        if result.contains("Some(") {
            result = self.convert_some_constructor(&result)?;
        }

        // 转换 None 为 std::nullopt
        result = result.replace("None", "std::nullopt");

        Ok(result)
    }

    /// 转换 println! 宏
    fn convert_println_macro(&self, content: &str) -> Result<String> {
        let mut result = String::new();

        // 使用char_indices()来正确跟踪字节位置（借鉴nu2rust）
        let char_indices: Vec<(usize, char)> = content.char_indices().collect();
        let mut i = 0;

        while i < char_indices.len() {
            let (current_byte_pos, _) = char_indices[i];

            // 检查是否是println!（需要8个字符）
            if i + 8 <= char_indices.len() {
                // 安全地计算end_byte_pos
                let end_byte_pos = if i + 8 < char_indices.len() {
                    char_indices[i + 8].0
                } else {
                    content.len()
                };

                // 使用char边界安全的字符串切片
                if current_byte_pos < content.len()
                    && end_byte_pos <= content.len()
                    && &content[current_byte_pos..end_byte_pos] == "println!"
                {
                    // 找到 println!
                    i += 8;

                    // 跳过空白
                    while i < char_indices.len() && char_indices[i].1.is_whitespace() {
                        i += 1;
                    }

                    // 期望是 (
                    if i < char_indices.len() && char_indices[i].1 == '(' {
                        i += 1;
                        let start_i = i;
                        let mut depth = 1;
                        let mut in_string = false;
                        let mut escape_next = false;

                        // 找到匹配的右括号
                        while i < char_indices.len() && depth > 0 {
                            let ch = char_indices[i].1;

                            if escape_next {
                                escape_next = false;
                                i += 1;
                                continue;
                            }

                            if ch == '\\' {
                                escape_next = true;
                            } else if ch == '"' {
                                in_string = !in_string;
                            } else if !in_string {
                                if ch == '(' {
                                    depth += 1;
                                } else if ch == ')' {
                                    depth -= 1;
                                }
                            }

                            if depth > 0 {
                                i += 1;
                            }
                        }

                        // 使用字节位置来切片（确保在char边界）
                        let start_byte = if start_i < char_indices.len() {
                            char_indices[start_i].0
                        } else {
                            content.len()
                        };
                        let end_byte = if i < char_indices.len() {
                            char_indices[i].0
                        } else {
                            content.len()
                        };

                        // 安全检查字节边界
                        if start_byte <= content.len()
                            && end_byte <= content.len()
                            && start_byte <= end_byte
                        {
                            let args = &content[start_byte..end_byte];
                            i += 1; // 跳过 )

                            // 转换 println! 参数
                            result.push_str(&self.convert_println_args(args)?);
                            continue;
                        }
                    }
                }
            }

            result.push(char_indices[i].1);
            i += 1;
        }

        Ok(result)
    }

    /// 转换 println! 的参数
    fn convert_println_args(&self, args: &str) -> Result<String> {
        let args = args.trim();

        if args.is_empty() {
            return Ok("std::cout << std::endl".to_string());
        }

        // 检查是否有格式化参数（包含 {} 或 逗号）
        // 需要小心处理字符串内的逗号
        if args.contains("{}") {
            // 格式化输出：println!("text: {}", value)
            // 找到第一个字符串结束后的逗号位置
            let mut in_string = false;
            let mut escape_next = false;
            let mut split_pos = None;

            // 使用char_indices()来正确处理UTF-8字符边界
            for (byte_idx, ch) in args.char_indices() {
                if escape_next {
                    escape_next = false;
                    continue;
                }
                if ch == '\\' {
                    escape_next = true;
                    continue;
                }
                if ch == '"' {
                    in_string = !in_string;
                }
                if !in_string && ch == ',' {
                    split_pos = Some(byte_idx);
                    break;
                }
            }

            if let Some(pos) = split_pos {
                let format_str = args[..pos].trim();
                let values = args[pos + 1..].trim();

                // 解析格式字符串并替换 {}
                let mut output = String::from("std::cout << ");
                let format_inner = format_str.trim_matches('"');

                // 将格式字符串按 {} 分割
                let format_parts: Vec<&str> = format_inner.split("{}").collect();
                let value_parts: Vec<&str> = values.split(',').map(|s| s.trim()).collect();

                for (i, part) in format_parts.iter().enumerate() {
                    if !part.is_empty() {
                        output.push_str(&format!("\"{}\" << ", part));
                    }
                    if i < value_parts.len() {
                        output.push_str(&format!("{} << ", value_parts[i]));
                    }
                }

                output.push_str("std::endl");
                return Ok(output);
            }
        }

        // 简单输出：println!("text")
        Ok(format!("std::cout << {} << std::endl", args))
    }

    /// 转换 V![...] 宏为 std::vector<T>{...}
    fn convert_vec_macro(&self, content: &str) -> Result<String> {
        let mut result = String::new();
        let mut i = 0;
        let chars: Vec<char> = content.chars().collect();

        while i < chars.len() {
            if i + 2 <= chars.len() && chars[i] == 'V' && chars[i + 1] == '!' {
                i += 2;

                // 跳过空白
                while i < chars.len() && chars[i].is_whitespace() {
                    i += 1;
                }

                if i < chars.len() && chars[i] == '[' {
                    i += 1;
                    let start = i;
                    let mut depth = 1;

                    // 找到匹配的右括号
                    while i < chars.len() && depth > 0 {
                        if chars[i] == '[' {
                            depth += 1;
                        } else if chars[i] == ']' {
                            depth -= 1;
                        }
                        if depth > 0 {
                            i += 1;
                        }
                    }

                    let elements = &content[start..i];
                    i += 1; // 跳过 ]

                    // 推断类型（简单实现：查看第一个元素）
                    let type_hint = if !elements.trim().is_empty() {
                        let first_elem = elements.split(',').next().unwrap_or("").trim();
                        if first_elem.parse::<i32>().is_ok() {
                            "int32_t"
                        } else if first_elem.parse::<f64>().is_ok() {
                            "double"
                        } else {
                            "auto"
                        }
                    } else {
                        "auto"
                    };

                    result.push_str(&format!("std::vector<{}>{{{}}}", type_hint, elements));
                    continue;
                }
            }

            result.push(chars[i]);
            i += 1;
        }

        Ok(result)
    }

    /// 转换 Some(...) 为 std::optional<T>{...}
    fn convert_some_constructor(&self, content: &str) -> Result<String> {
        let mut result = String::new();
        let char_indices: Vec<(usize, char)> = content.char_indices().collect();
        let mut i = 0;

        while i < char_indices.len() {
            // 使用char_indices来正确处理UTF-8边界
            let (byte_pos, _) = char_indices[i];

            // 检查是否是 "Some"（需要4个字符）
            if i + 4 <= char_indices.len() {
                let end_byte = if i + 4 < char_indices.len() {
                    char_indices[i + 4].0
                } else {
                    content.len()
                };

                if byte_pos < content.len()
                    && end_byte <= content.len()
                    && &content[byte_pos..end_byte] == "Some"
                {
                    // 检查是否是独立的 Some（不是某个标识符的一部分）
                    let is_word_start = i == 0
                        || !char_indices[i - 1].1.is_alphanumeric() && char_indices[i - 1].1 != '_';
                    let is_word_end = i + 4 >= char_indices.len()
                        || !char_indices[i + 4].1.is_alphanumeric() && char_indices[i + 4].1 != '_';

                    if is_word_start && is_word_end {
                        i += 4;

                        // 跳过空白
                        while i < char_indices.len() && char_indices[i].1.is_whitespace() {
                            i += 1;
                        }

                        if i < char_indices.len() && char_indices[i].1 == '(' {
                            i += 1;
                            let start_byte = if i < char_indices.len() {
                                char_indices[i].0
                            } else {
                                content.len()
                            };
                            let mut depth = 1;
                            let mut in_string = false;
                            let mut escape_next = false;

                            // 找到匹配的右括号
                            while i < char_indices.len() && depth > 0 {
                                if escape_next {
                                    escape_next = false;
                                    i += 1;
                                    continue;
                                }

                                if char_indices[i].1 == '\\' {
                                    escape_next = true;
                                } else if char_indices[i].1 == '"' {
                                    in_string = !in_string;
                                } else if !in_string {
                                    if char_indices[i].1 == '(' {
                                        depth += 1;
                                    } else if char_indices[i].1 == ')' {
                                        depth -= 1;
                                    }
                                }

                                if depth > 0 {
                                    i += 1;
                                }
                            }

                            let end_byte = if i < char_indices.len() {
                                char_indices[i].0
                            } else {
                                content.len()
                            };

                            if start_byte <= content.len() && end_byte <= content.len() {
                                let value = &content[start_byte..end_byte];
                                i += 1; // 跳过 )

                                // 推断类型
                                let type_hint = if value.trim().parse::<i32>().is_ok() {
                                    "int32_t"
                                } else if value.trim().parse::<f64>().is_ok() {
                                    "double"
                                } else if value.trim().starts_with('"') {
                                    "std::string"
                                } else {
                                    "auto"
                                };

                                if type_hint == "auto" {
                                    result.push_str(&format!("std::make_optional({})", value));
                                } else {
                                    result.push_str(&format!(
                                        "std::optional<{}>{{{}}}",
                                        type_hint, value
                                    ));
                                }
                                continue;
                            }
                        }
                    }
                }
            }

            result.push(char_indices[i].1);
            i += 1;
        }

        Ok(result)
    }

    /// 转换 Ok(...) 为返回值
    fn convert_ok_constructor(&self, content: &str) -> Result<String> {
        let mut result = String::new();
        let char_indices: Vec<(usize, char)> = content.char_indices().collect();
        let mut i = 0;

        while i < char_indices.len() {
            if i + 2 <= char_indices.len() {
                let current_byte_pos = char_indices[i].0;
                let end_byte_pos = if i + 2 < char_indices.len() {
                    char_indices[i + 2].0
                } else {
                    content.len()
                };

                if current_byte_pos < content.len()
                    && end_byte_pos <= content.len()
                    && &content[current_byte_pos..end_byte_pos] == "Ok"
                {
                    let is_word_start = i == 0
                        || (!char_indices[i - 1].1.is_alphanumeric()
                            && char_indices[i - 1].1 != '_');
                    let is_word_end = i + 2 >= char_indices.len()
                        || (!char_indices[i + 2].1.is_alphanumeric()
                            && char_indices[i + 2].1 != '_');

                    if is_word_start && is_word_end {
                        i += 2;
                        while i < char_indices.len() && char_indices[i].1.is_whitespace() {
                            i += 1;
                        }

                        if i < char_indices.len() && char_indices[i].1 == '(' {
                            i += 1;
                            let start_byte = if i < char_indices.len() {
                                char_indices[i].0
                            } else {
                                content.len()
                            };
                            let mut depth = 1;
                            let mut in_string = false;
                            let mut escape_next = false;

                            while i < char_indices.len() && depth > 0 {
                                if escape_next {
                                    escape_next = false;
                                    i += 1;
                                    continue;
                                }

                                if char_indices[i].1 == '\\' {
                                    escape_next = true;
                                } else if char_indices[i].1 == '"' {
                                    in_string = !in_string;
                                } else if !in_string {
                                    if char_indices[i].1 == '(' {
                                        depth += 1;
                                    } else if char_indices[i].1 == ')' {
                                        depth -= 1;
                                    }
                                }

                                if depth > 0 {
                                    i += 1;
                                }
                            }

                            let end_byte = if i < char_indices.len() {
                                char_indices[i].0
                            } else {
                                content.len()
                            };

                            if start_byte <= content.len()
                                && end_byte <= content.len()
                                && start_byte <= end_byte
                            {
                                let value = &content[start_byte..end_byte];
                                i += 1;
                                result.push_str(&format!("return {}", value));
                                continue;
                            }
                        }
                    }
                }
            }

            result.push(char_indices[i].1);
            i += 1;
        }

        Ok(result)
    }

    /// 转换 Err(...) 为 std::unexpected(...)
    fn convert_err_constructor(&self, content: &str) -> Result<String> {
        let mut result = String::new();
        let char_indices: Vec<(usize, char)> = content.char_indices().collect();
        let mut i = 0;

        while i < char_indices.len() {
            if i + 3 <= char_indices.len() {
                let current_byte_pos = char_indices[i].0;
                let end_byte_pos = if i + 3 < char_indices.len() {
                    char_indices[i + 3].0
                } else {
                    content.len()
                };

                if current_byte_pos < content.len()
                    && end_byte_pos <= content.len()
                    && &content[current_byte_pos..end_byte_pos] == "Err"
                {
                    let is_word_start = i == 0
                        || (!char_indices[i - 1].1.is_alphanumeric()
                            && char_indices[i - 1].1 != '_');
                    let is_word_end = i + 3 >= char_indices.len()
                        || (!char_indices[i + 3].1.is_alphanumeric()
                            && char_indices[i + 3].1 != '_');

                    if is_word_start && is_word_end {
                        i += 3;
                        while i < char_indices.len() && char_indices[i].1.is_whitespace() {
                            i += 1;
                        }

                        if i < char_indices.len() && char_indices[i].1 == '(' {
                            i += 1;
                            let start_byte = if i < char_indices.len() {
                                char_indices[i].0
                            } else {
                                content.len()
                            };
                            let mut depth = 1;
                            let mut in_string = false;
                            let mut escape_next = false;

                            while i < char_indices.len() && depth > 0 {
                                if escape_next {
                                    escape_next = false;
                                    i += 1;
                                    continue;
                                }

                                if char_indices[i].1 == '\\' {
                                    escape_next = true;
                                } else if char_indices[i].1 == '"' {
                                    in_string = !in_string;
                                } else if !in_string {
                                    if char_indices[i].1 == '(' {
                                        depth += 1;
                                    } else if char_indices[i].1 == ')' {
                                        depth -= 1;
                                    }
                                }

                                if depth > 0 {
                                    i += 1;
                                }
                            }

                            let end_byte = if i < char_indices.len() {
                                char_indices[i].0
                            } else {
                                content.len()
                            };

                            if start_byte <= content.len()
                                && end_byte <= content.len()
                                && start_byte <= end_byte
                            {
                                let error_value = &content[start_byte..end_byte];
                                i += 1;
                                result
                                    .push_str(&format!("return std::unexpected({})", error_value));
                                continue;
                            }
                        }
                    }
                }
            }

            result.push(char_indices[i].1);
            i += 1;
        }

        Ok(result)
    }

    /// 递归转换行内的Nu关键字 (用于单行中的多个语句)
    fn convert_inline_keywords(&self, content: &str) -> Result<String> {
        let mut result = String::new();
        let mut i = 0;
        let chars: Vec<char> = content.chars().collect();

        while i < chars.len() {
            // 跳过字符串字面量，不对其中的内容进行转换
            if chars[i] == '"' {
                result.push(chars[i]);
                i += 1;
                let mut prev_char = '"';
                while i < chars.len() {
                    let current = chars[i];
                    result.push(current);
                    i += 1;
                    if current == '"' && prev_char != '\\' {
                        break;
                    }
                    prev_char = current;
                }
                continue;
            }

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

            // break: br 或 br;
            if remaining.starts_with("br;") || remaining.starts_with("br ") || remaining == "br" {
                let is_start_boundary =
                    i == 0 || (!chars[i - 1].is_alphanumeric() && chars[i - 1] != '_');
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

            // continue: ct 或 ct;
            if remaining.starts_with("ct;") || remaining.starts_with("ct ") || remaining == "ct" {
                let is_start_boundary =
                    i == 0 || (!chars[i - 1].is_alphanumeric() && chars[i - 1] != '_');
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

            // 默认情况：复制字符
            result.push(chars[i]);
            i += 1;
        }

        Ok(result)
    }

    /// v1.8.1: 带边界检查的类型替换辅助函数（借鉴nu2rust）
    /// 只在单字母类型缩写前后是非字母数字时才替换
    /// 例如: "R <" -> "std::expected<" 但 "YEAR <" 保持不变
    fn replace_type_with_boundary(s: &str, from: &str, to: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = s.chars().collect();
        let from_chars: Vec<char> = from.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // 检查是否匹配 from 模式
            let mut matches = true;
            if i + from_chars.len() <= chars.len() {
                for (j, fc) in from_chars.iter().enumerate() {
                    if chars[i + j] != *fc {
                        matches = false;
                        break;
                    }
                }
            } else {
                matches = false;
            }

            if matches {
                // 检查前边界: 前一个字符不能是字母或数字或下划线
                let has_start_boundary =
                    i == 0 || (!chars[i - 1].is_alphanumeric() && chars[i - 1] != '_');

                if has_start_boundary {
                    result.push_str(to);
                    i += from_chars.len();
                    continue;
                }
            }

            result.push(chars[i]);
            i += 1;
        }

        result
    }

    /// P1-0: 转换闭包语法 |x| expr -> [](auto x) { return expr; }
    fn convert_closure_syntax(&self, s: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // 查找闭包起始符 |
            if chars[i] == '|' {
                let closure_start = i;
                i += 1;

                // 收集参数列表
                let params_start = i;
                while i < chars.len() && chars[i] != '|' {
                    i += 1;
                }

                if i >= chars.len() {
                    // 没有找到结束的|，不是闭包
                    result.push('|');
                    i = closure_start + 1;
                    continue;
                }

                let params: String = chars[params_start..i].iter().collect();
                i += 1; // 跳过结束的 |

                // 跳过空白
                while i < chars.len() && chars[i].is_whitespace() {
                    i += 1;
                }

                // 检查是否有返回类型标注 -> Type
                let mut return_type = String::new();
                if i + 1 < chars.len() && chars[i] == '-' && chars[i + 1] == '>' {
                    i += 2; // 跳过 ->
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }

                    // 收集返回类型（直到遇到 { 或空白）
                    let ret_start = i;
                    while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '{' {
                        i += 1;
                    }
                    return_type = chars[ret_start..i].iter().collect();

                    // 跳过空白
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                }

                // 检查闭包体
                let has_brace = i < chars.len() && chars[i] == '{';

                if has_brace {
                    // 多行闭包: |x| { body }
                    i += 1; // 跳过 {
                    let body_start = i;
                    let mut depth = 1;

                    while i < chars.len() && depth > 0 {
                        if chars[i] == '{' {
                            depth += 1;
                        } else if chars[i] == '}' {
                            depth -= 1;
                        }
                        if depth > 0 {
                            i += 1;
                        }
                    }

                    let body: String = chars[body_start..i].iter().collect();
                    i += 1; // 跳过 }

                    // 转换参数
                    let cpp_params = self.convert_closure_params(&params);

                    // 处理闭包体：检查是否需要添加return
                    let body_trimmed = body.trim();
                    let needs_return = !body_trimmed.is_empty()
                        && !body_trimmed.starts_with("return")
                        && !body_trimmed.contains(';')
                        && !body_trimmed.contains('{');

                    let formatted_body = if needs_return {
                        format!("return {};", body_trimmed)
                    } else {
                        body_trimmed.to_string()
                    };

                    // 生成C++ lambda - 修复：确保格式正确，大括号在同一行
                    if !return_type.is_empty() {
                        let cpp_ret_type = self.convert_types_in_string(&return_type);
                        result.push_str(&format!(
                            "[]({}) -> {} {{ {} }}",
                            cpp_params, cpp_ret_type, formatted_body
                        ));
                    } else {
                        result.push_str(&format!("[]({}) {{ {} }}", cpp_params, formatted_body));
                    }
                } else {
                    // 单行闭包: |x| expr
                    // 查找表达式结束（逗号、分号、右括号等）
                    let expr_start = i;
                    let mut depth = 0;

                    while i < chars.len() {
                        if chars[i] == '(' || chars[i] == '[' || chars[i] == '{' {
                            depth += 1;
                        } else if chars[i] == ')' || chars[i] == ']' || chars[i] == '}' {
                            if depth == 0 {
                                break;
                            }
                            depth -= 1;
                        } else if depth == 0 && (chars[i] == ',' || chars[i] == ';') {
                            break;
                        }
                        i += 1;
                    }

                    let expr: String = chars[expr_start..i].iter().collect();

                    // 转换参数
                    let cpp_params = self.convert_closure_params(&params);

                    // 生成C++ lambda
                    if !return_type.is_empty() {
                        let cpp_ret_type = self.convert_types_in_string(&return_type);
                        result.push_str(&format!(
                            "[]({})->{} {{ return {}; }}",
                            cpp_params,
                            cpp_ret_type,
                            expr.trim()
                        ));
                    } else {
                        result.push_str(&format!(
                            "[]({}) {{ return {}; }}",
                            cpp_params,
                            expr.trim()
                        ));
                    }
                }

                continue;
            }

            result.push(chars[i]);
            i += 1;
        }

        result
    }

    /// 转换闭包参数列表
    fn convert_closure_params(&self, params: &str) -> String {
        if params.trim().is_empty() {
            return String::new();
        }

        let parts: Vec<&str> = params.split(',').collect();
        let mut result_parts = Vec::new();

        for part in parts {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                continue;
            }

            // 检查是否有类型标注 name: Type
            if let Some(colon_pos) = trimmed.find(':') {
                let param_name = trimmed[..colon_pos].trim();
                let param_type = trimmed[colon_pos + 1..].trim();
                let cpp_type = self.convert_types_in_string(param_type);
                result_parts.push(format!("{} {}", cpp_type, param_name));
            } else {
                // 无类型标注，使用auto
                result_parts.push(format!("auto {}", trimmed));
            }
        }

        result_parts.join(", ")
    }

    /// 🔴 CRITICAL FIX: 转换Nu语言类型后缀为C++类型转换
    ///
    /// 这是修复test_examples_roundtrip_cpp.sh的关键函数
    ///
    /// 转换规则：
    /// - 1i32 → (int32_t)1
    /// - 2u64 → (uint64_t)2
    /// - 1.5f32 → (float)1.5
    /// - 100usize → (size_t)100
    ///
    /// 支持的后缀：
    /// - 整数：i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, isize, usize
    /// - 浮点：f32, f64
    /// - 别名：int8_t, int16_t, int32_t, int64_t, uint8_t, uint16_t, uint32_t, uint64_t
    fn convert_type_suffix(&self, s: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // 检查是否是数字的开始（可能有负号）
            let is_number_start = chars[i].is_ascii_digit()
                || (chars[i] == '-' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit());

            if is_number_start {
                let num_start = i;

                // 收集数字部分（包括可能的负号、小数点、科学计数法）
                if chars[i] == '-' {
                    i += 1;
                }

                // 整数部分
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }

                // 小数部分
                if i < chars.len() && chars[i] == '.' {
                    i += 1;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }

                // 科学计数法 (e或E)
                if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                    i += 1;
                    if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
                        i += 1;
                    }
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }

                let number: String = chars[num_start..i].iter().collect();

                // 检查是否有类型后缀
                let suffix_start = i;
                let mut has_suffix = false;
                let mut suffix = String::new();

                // 尝试匹配类型后缀
                if i < chars.len() && (chars[i].is_alphabetic() || chars[i] == '_') {
                    // 收集可能的后缀
                    let mut temp_suffix = String::new();
                    let mut j = i;

                    while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                        temp_suffix.push(chars[j]);
                        j += 1;
                    }

                    // 检查是否是有效的类型后缀
                    let cpp_type = match temp_suffix.as_str() {
                        "i8" | "int8_t" => Some("int8_t"),
                        "i16" | "int16_t" => Some("int16_t"),
                        "i32" | "int32_t" => Some("int32_t"),
                        "i64" | "int64_t" => Some("int64_t"),
                        "i128" => Some("__int128"),
                        "u8" | "uint8_t" => Some("uint8_t"),
                        "u16" | "uint16_t" => Some("uint16_t"),
                        "u32" | "uint32_t" => Some("uint32_t"),
                        "u64" | "uint64_t" => Some("uint64_t"),
                        "u128" => Some("__uint128_t"),
                        "isize" => Some("intptr_t"),
                        "usize" => Some("size_t"),
                        "f32" => Some("float"),
                        "f64" => Some("double"),
                        _ => None,
                    };

                    if let Some(cpp_t) = cpp_type {
                        // 找到有效的类型后缀
                        has_suffix = true;
                        suffix = cpp_t.to_string();
                        i = j; // 跳过后缀

                        // 生成C++类型转换
                        result.push_str(&format!("({}){}", suffix, number));
                        continue;
                    }
                }

                // 没有类型后缀，保持原数字
                result.push_str(&number);
                continue;
            }

            // 不是数字，直接复制字符
            result.push(chars[i]);
            i += 1;
        }

        result
    }

    /// 转换Nu类型到C++类型
    fn convert_types_in_string(&self, s: &str) -> String {
        let mut result = s.to_string();

        // 🔴 CRITICAL: 类型后缀转换必须最先执行（在所有其他转换之前）
        // 转换 1i32 -> (int32_t)1, 2u64 -> (uint64_t)2, 1.5f32 -> (float)1.5
        result = self.convert_type_suffix(&result);

        // P1-6: 转换vec!宏 vec![1,2,3] 或 vec ! [1,2,3] -> std::vector<int>{1,2,3}
        // 必须在其他转换之前处理，因为宏可能包含类型
        if result.contains("vec!") || result.contains("vec !") {
            result = self.convert_vec_bang_macro(&result);
        }

        // 禁用闭包转换 - 闭包是多行的，在逐行转换中无法正确处理
        // TODO: 需要在文件级别处理闭包，而不是在逐行转换中
        // result = self.convert_closure_syntax(&result);

        // P1-7: 转换 std::string::from() 和 String::from()
        if result.contains("::from(") {
            result = self.convert_string_from(&result).unwrap_or(result.clone());
        }

        // P1-4: 转换元组访问语法 a.0, a.1 -> std::get<0>(a), std::get<1>(a)
        result = self.convert_tuple_access(&result);

        // 1. 字符串切片类型转换 - 必须在String之前处理
        // &str → std::string_view
        result = result.replace("&str", "std::string_view");

        // 2. 切片类型转换 - 使用正则表达式处理 &[T]
        // &[T] → const std::vector<T>& 或 std::span<T>
        // 简单实现：查找 &[...] 模式
        if result.contains("&[") {
            result = self.convert_slice_types(&result);
        }

        // 3. usize/isize类型转换 - 必须在其他类型之前
        result = result
            .replace("usize", "size_t")
            .replace("isize", "ptrdiff_t");

        // 4. 基本类型转换 - 注意替换顺序，避免重复替换
        result = result
            .replace("i8", "int8_t")
            .replace("i16", "int16_t")
            .replace("i32", "int32_t")
            .replace("i64", "int64_t")
            .replace("u8", "uint8_t")
            .replace("u16", "uint16_t")
            .replace("u32", "uint32_t")
            .replace("u64", "uint64_t")
            .replace("f32", "float")
            .replace("f64", "double")
            .replace("bool", "bool");

        // 5. String类型替换 - 必须一次性完成，避免重复
        result = result.replace("String", "std::string");
        result = result.replace("Str", "std::string");

        // 6. 智能指针类型转换 - 使用边界检查避免误转换
        result = Self::replace_type_with_boundary(&result, "V<", "std::vector<");
        result = Self::replace_type_with_boundary(&result, "O<", "std::optional<");
        result = Self::replace_type_with_boundary(&result, "R<", "std::expected<");
        result = Self::replace_type_with_boundary(&result, "A<", "std::shared_ptr<");
        result = Self::replace_type_with_boundary(&result, "B<", "std::unique_ptr<");
        result = Self::replace_type_with_boundary(&result, "X<", "std::mutex<");

        // 7. 引用类型 - C++ 没有 mut 概念
        result = result
            .replace("&!", "&") // &! -> 普通引用
            .replace("&mut ", "&"); // &mut -> 普通引用

        // 8. 容器类型
        result = result
            .replace("Vec<", "std::vector<")
            .replace("HashMap<", "std::unordered_map<")
            .replace("HashSet<", "std::unordered_set<")
            .replace("Option<", "std::optional<")
            .replace("Result<", "std::expected<");

        // 9. 函数类型
        result = result
            .replace("Fn<", "std::function<")
            .replace("FnMut<", "std::function<")
            .replace("FnOnce<", "std::function<");

        // 10. 元组类型转换 - (T1, T2, T3) → std::tuple<T1, T2, T3>
        if result.contains('(') && result.contains(',') {
            result = self.convert_tuple_types(&result);
        }

        // 11. 特殊构造函数转换
        // ::new() → 构造函数或工厂方法
        result = result.replace("::new()", "()");

        // 12. I/O 转换
        result = result.replace("io::stdin()", "std::cin");
        result = result.replace("io::stdout()", "std::cout");
        result = result.replace("io::stderr()", "std::cerr");

        // 13. 处理泛型尖括号，避免 >> 被解析为右移
        result = result.replace(">>", "> >");

        // 15. 移除多余空格
        result = result.replace(" ::", "::");
        result = result.replace(":: ", "::");
        result = result.replace(" <", "<");
        result = result.replace(" >", ">");

        result
    }

    /// 转换切片类型 &[T] → const std::vector<T>& 或 std::span<T>
    fn convert_slice_types(&self, s: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // 查找 &[
            if i + 1 < chars.len() && chars[i] == '&' && chars[i + 1] == '[' {
                i += 2; // 跳过 &[
                let start = i;
                let mut depth = 1;

                // 找到匹配的 ]
                while i < chars.len() && depth > 0 {
                    if chars[i] == '[' {
                        depth += 1;
                    } else if chars[i] == ']' {
                        depth -= 1;
                    }
                    if depth > 0 {
                        i += 1;
                    }
                }

                if depth == 0 {
                    let inner_type: String = chars[start..i].iter().collect();
                    // 使用 std::span<T> 作为切片类型 (C++20)
                    result.push_str(&format!("std::span<{}>", inner_type.trim()));
                    i += 1; // 跳过 ]
                    continue;
                }
            }

            result.push(chars[i]);
            i += 1;
        }

        result
    }

    /// 转换元组类型 (T1, T2, T3) → std::tuple<T1, T2, T3>
    /// 只在返回类型位置转换元组，不转换函数参数列表、函数调用和闭包参数
    fn convert_tuple_types(&self, s: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // 查找可能的元组定义 (T1, T2, ...)
            if chars[i] == '(' {
                let start = i;

                // 检查左括号前面是否有标识符（函数名）或闭包符号
                let mut is_function_call = false;
                let mut is_closure = false;
                if start > 0 {
                    let mut j = start - 1;
                    // 跳过空白
                    while j > 0 && chars[j].is_whitespace() {
                        j -= 1;
                    }
                    // 如果前面是标识符字符，这是函数调用
                    if chars[j].is_alphanumeric() || chars[j] == '_' {
                        is_function_call = true;
                    }
                    // 如果前面是]，这可能是闭包的参数列表
                    if chars[j] == ']' {
                        is_closure = true;
                    }
                }

                i += 1;
                let mut depth = 1;
                let mut has_comma = false;
                let mut has_colon = false;

                // 检查是否是元组（包含逗号但不包含冒号）
                let mut check_i = i;
                while check_i < chars.len() && depth > 0 {
                    if chars[check_i] == '(' {
                        depth += 1;
                    } else if chars[check_i] == ')' {
                        depth -= 1;
                    } else if chars[check_i] == ',' && depth == 1 {
                        has_comma = true;
                    } else if chars[check_i] == ':' && depth == 1 {
                        has_colon = true;
                    }
                    check_i += 1;
                }

                // 如果是元组（有逗号但没有冒号，且不是函数调用或闭包），转换为 std::tuple
                // 有冒号说明是函数参数列表 (name: Type)，不转换
                // is_function_call 说明是函数调用，不转换
                // is_closure 说明是闭包参数，不转换
                if has_comma && !has_colon && !is_function_call && !is_closure && depth == 0 {
                    let inner: String = chars[start + 1..check_i - 1].iter().collect();
                    result.push_str(&format!("std::tuple<{}>", inner.trim()));
                    i = check_i;
                    continue;
                }

                // 否则保持原样
                i = start + 1;
                result.push('(');
                continue;
            }

            result.push(chars[i]);
            i += 1;
        }

        result
    }

    /// 修复错误8: 转换范围语法 a[x..y] 和 &parts[1..parts.len()-1]
    fn convert_range_syntax(&self, s: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // 查找数组索引 [
            if chars[i] == '[' {
                let start_bracket = i;
                i += 1;
                let range_start = i;

                // 查找 .. 范围操作符
                let mut found_range = false;
                let mut range_op_pos = 0;
                let mut depth = 1;
                let mut j = i;

                while j < chars.len() && depth > 0 {
                    if chars[j] == '[' {
                        depth += 1;
                    } else if chars[j] == ']' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    } else if depth == 1
                        && j + 1 < chars.len()
                        && chars[j] == '.'
                        && chars[j + 1] == '.'
                    {
                        found_range = true;
                        range_op_pos = j;
                    }
                    j += 1;
                }

                if found_range {
                    // 提取范围的起始和结束
                    let start_expr: String = chars[range_start..range_op_pos].iter().collect();
                    let start_expr = start_expr.trim();

                    // 跳过 ..
                    let end_start = range_op_pos + 2;
                    let end_expr: String = chars[end_start..j].iter().collect();
                    let end_expr = end_expr.trim();

                    // 查找被索引的对象
                    let mut obj_start = start_bracket;
                    while obj_start > 0
                        && (chars[obj_start - 1].is_alphanumeric()
                            || chars[obj_start - 1] == '_'
                            || chars[obj_start - 1] == ']')
                    {
                        obj_start -= 1;
                    }

                    let obj_name: String = chars[obj_start..start_bracket].iter().collect();

                    // 移除已经添加的对象名
                    if result.ends_with(&obj_name) {
                        result.truncate(result.len() - obj_name.len());
                    }

                    // 转换为 C++ 的 std::span 或子串操作
                    if end_expr.is_empty() {
                        // a[x..] -> std::vector(a.begin() + x, a.end())
                        result.push_str(&format!(
                            "std::vector<decltype({}[0])>({}.begin() + {}, {}.end())",
                            obj_name, obj_name, start_expr, obj_name
                        ));
                    } else {
                        // a[x..y] -> std::vector(a.begin() + x, a.begin() + y)
                        result.push_str(&format!(
                            "std::vector<decltype({}[0])>({}.begin() + {}, {}.begin() + {})",
                            obj_name, obj_name, start_expr, obj_name, end_expr
                        ));
                    }

                    i = j + 1;
                    continue;
                }

                // 不是范围语法，保持原样
                i = start_bracket;
            }

            result.push(chars[i]);
            i += 1;
        }

        result
    }

    /// 修复错误10: 转换迭代器方法链
    fn convert_iterator_chains(&self, s: &str) -> String {
        let mut result = s.to_string();

        // .lines() -> 需要特殊处理
        // 对于字符串，使用自定义的lines函数或循环
        if result.contains(".lines()") {
            result = result.replace(".lines()", ".lines() /* TODO: implement lines iterator */");
        }

        // .split_whitespace() -> 需要分割处理
        if result.contains(".split_whitespace()") {
            result = result.replace(
                ".split_whitespace()",
                ".split_whitespace() /* TODO: implement split_whitespace */",
            );
        }

        // .chars() -> C++字符串迭代
        result = result.replace(".chars()", " /* iterate chars */");

        // .bytes() -> 字节迭代
        result = result.replace(".bytes()", " /* iterate bytes */");

        // .split(pat) -> 保持，C++可能需要自定义

        // .trim() -> 需要自定义trim函数
        if result.contains(".trim()") {
            result = result.replace(".trim()", " /* TODO: implement trim */");
        }

        // .to_uppercase() / .to_lowercase()
        result = result.replace(".to_uppercase()", " /* TODO: implement to_uppercase */");
        result = result.replace(".to_lowercase()", " /* TODO: implement to_lowercase */");

        result
    }

    /// P1-4: 转换元组访问语法 a.0 -> std::get<0>(a)
    fn convert_tuple_access(&self, s: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // 查找 .数字 模式
            if i > 0 && chars[i] == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                // 🔧 修复：检查前一个字符是否是数字，如果是则这是浮点数字面量，不是元组访问
                if chars[i - 1].is_ascii_digit() {
                    // 这是浮点数字面量，如 10.0, 5.0, 不转换
                    result.push(chars[i]);
                    i += 1;
                    continue;
                }

                // 向前查找标识符
                let mut ident_end = i;
                let mut ident_start = i - 1;

                // 跳过前面的空白
                while ident_start > 0 && chars[ident_start].is_whitespace() {
                    ident_start -= 1;
                }

                // 找到标识符的开始
                while ident_start > 0
                    && (chars[ident_start].is_alphanumeric() || chars[ident_start] == '_')
                {
                    ident_start -= 1;
                }

                // 调整位置
                if ident_start > 0
                    || !chars[ident_start].is_alphanumeric() && chars[ident_start] != '_'
                {
                    ident_start += 1;
                }

                let ident: String = chars[ident_start..ident_end].iter().collect();
                let ident = ident.trim();

                // 🔧 修复：如果标识符是数字，这是浮点数，不转换
                if ident.chars().all(|c| c.is_ascii_digit()) {
                    result.push(chars[i]);
                    i += 1;
                    continue;
                }

                // 收集数字索引
                i += 1; // 跳过 .
                let digit_start = i;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                let index: String = chars[digit_start..i].iter().collect();

                // 移除已添加的标识符
                if result.ends_with(ident) {
                    result.truncate(result.len() - ident.len());
                }

                // 生成 std::get<N>(ident)
                result.push_str(&format!("std::get<{}>({})", index, ident));
                continue;
            }

            result.push(chars[i]);
            i += 1;
        }

        result
    }

    /// P1-6: 转换vec!宏 vec![1,2,3] 或 vec ! [1,2,3] -> std::vector<int>{1,2,3}
    fn convert_vec_bang_macro(&self, s: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // 查找 vec! 或 vec !
            if i + 3 <= chars.len() {
                let slice: String = chars[i..i + 3].iter().collect();
                // 检查是否是 "vec" 且前后有边界（不是某个标识符的一部分）
                let is_word_start =
                    i == 0 || (!chars[i - 1].is_alphanumeric() && chars[i - 1] != '_');
                let is_word_boundary = i + 3 >= chars.len()
                    || (!chars[i + 3].is_alphanumeric() && chars[i + 3] != '_');

                if slice == "vec" && is_word_start && is_word_boundary {
                    let vec_start = i;
                    i += 3; // 跳过 vec

                    // 跳过空白（关键修复：处理 vec ! 情况）
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }

                    // 检查是否是 !
                    if i < chars.len() && chars[i] == '!' {
                        i += 1; // 跳过 !

                        // 跳过空白（关键修复：处理 ! [ 之间的空格）
                        while i < chars.len() && chars[i].is_whitespace() {
                            i += 1;
                        }

                        // 期望 [
                        if i < chars.len() && chars[i] == '[' {
                            i += 1;
                            let start = i;
                            let mut depth = 1;

                            // 找到匹配的 ]
                            while i < chars.len() && depth > 0 {
                                if chars[i] == '[' {
                                    depth += 1;
                                } else if chars[i] == ']' {
                                    depth -= 1;
                                }
                                if depth > 0 {
                                    i += 1;
                                }
                            }

                            let elements: String = chars[start..i].iter().collect();
                            i += 1; // 跳过 ]

                            // 推断类型
                            let type_hint = if !elements.trim().is_empty() {
                                let first_elem = elements.split(',').next().unwrap_or("").trim();
                                if first_elem.parse::<i32>().is_ok() {
                                    "int"
                                } else if first_elem.parse::<f64>().is_ok() {
                                    "double"
                                } else if first_elem.starts_with('"') {
                                    "std::string"
                                } else {
                                    "auto"
                                }
                            } else {
                                "auto"
                            };

                            result.push_str(&format!("std::vector<{}>{{{}}}", type_hint, elements));
                            continue;
                        }
                    }

                    // 不是vec!，恢复并继续
                    i = vec_start;
                }
            }

            result.push(chars[i]);
            i += 1;
        }

        result
    }

    /// 修复问题3: 转换where子句为C++20 requires子句
    /// wh T: Trait -> requires Trait<T>
    /// where T: Trait -> requires Trait<T>
    fn convert_where_clause(&self, s: &str) -> String {
        let mut result = s.to_string();

        // 检查是否包含where子句
        if !result.contains("wh ") && !result.contains("where ") {
            return result;
        }

        // 处理 "wh " 格式（Nu简写）
        if result.contains("wh ") {
            // 查找 wh 的位置
            let chars: Vec<char> = result.chars().collect();
            let mut i = 0;
            let mut new_result = String::new();

            while i < chars.len() {
                // 查找 "wh " 模式
                if i + 3 <= chars.len() {
                    let slice: String = chars[i..i.min(i + 3)].iter().collect();
                    if slice == "wh " {
                        // 检查是否是单词边界
                        let is_word_start =
                            i == 0 || (!chars[i - 1].is_alphanumeric() && chars[i - 1] != '_');

                        if is_word_start {
                            // 找到where子句，跳过并提取约束
                            i += 3; // 跳过 "wh "

                            // 提取类型变量和trait约束
                            // 格式: T: Trait 或 T : Trait
                            let constraint_start = i;

                            // 找到约束结束位置（遇到{或行尾）
                            while i < chars.len() && chars[i] != '{' && chars[i] != '\n' {
                                i += 1;
                            }

                            let constraint: String = chars[constraint_start..i].iter().collect();
                            let constraint = constraint.trim();

                            // 解析约束: T: Trait -> requires Trait<T>
                            if let Some(colon_pos) = constraint.find(':') {
                                let type_var = constraint[..colon_pos].trim();
                                let trait_name = constraint[colon_pos + 1..].trim();

                                // 根据NU2CPP23.md规范: wh T: Graph -> requires Graph<T>
                                new_result
                                    .push_str(&format!("requires {}<{}>", trait_name, type_var));
                            }
                            // 如果无法解析，就移除where子句（不添加任何内容）

                            continue;
                        }
                    }
                }

                new_result.push(chars[i]);
                i += 1;
            }

            result = new_result;
        }

        // 处理 "where " 格式（完整关键字）
        // 在C++中，requires子句应该在函数签名的末尾
        // 在逐行转换中很难精确定位，所以这里选择移除
        result = result.replace("where ", "/* where */ ");

        result
    }
}

impl Default for Nu2CppConverter {
    fn default() -> Self {
        Self::new()
    }
}
