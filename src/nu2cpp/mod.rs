// Nu to C++ Converter
// 将Nu代码转换为标准C++代码

use anyhow::Result;

// 导出 sourcemap 模块
pub mod sourcemap;
pub use sourcemap::SourceMap;

pub struct Nu2CppConverter {
    // 转换上下文
    context: ConversionContext,
}

#[derive(Default)]
struct ConversionContext {
    // 跟踪当前作用域
    in_trait: bool,
    in_class: bool,
    in_struct_block: bool,
    in_namespace: bool,
    // C++特有上下文
    current_class_name: Option<String>,
    has_constructor: bool,
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
             #include <optional>\n\n"
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
            if trimmed.starts_with("#[") || trimmed.starts_with("#![") || trimmed.starts_with("#D") {
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
                // v1.6.4: 结构体字段默认添加public（状态机精确控制，借鉴nu2rust）
                if context.in_struct_block {
                    let trimmed = line.trim();
                    // 检测是否是字段行（包含冒号，不是注释，不是已有public，不是struct定义本身）
                    if !trimmed.is_empty()
                        && !trimmed.starts_with("//")
                        && !trimmed.starts_with("public:")
                        && !trimmed.starts_with("S ")
                        && !trimmed.starts_with("s ")
                        && trimmed.contains(':')
                        && !trimmed.starts_with("fn ")
                        && !trimmed.starts_with('}')
                    {
                        // 在converted前添加public:（C++风格）
                        // 注意：C++不需要每个字段都加public，而是用public:标签
                        // 但为了简化，我们在第一个字段前添加public:
                        if !converted.trim().is_empty() && !converted.contains("public:") {
                            converted = format!("public:\n    {}", converted.trim());
                        }
                    }
                }
                
                if !converted.is_empty() {
                    let leading_whitespace: String = line.chars().take_while(|c| c.is_whitespace()).collect();
                    let trimmed_converted = converted.trim_start();
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

    fn convert_line(
        &self,
        line: &str,
        _lines: &[&str],
        _index: &mut usize,
        context: &mut ConversionContext,
    ) -> Result<Option<String>> {
        let trimmed = line.trim();

        // Loop: L
        if trimmed.starts_with("L ") || trimmed == "L {" || trimmed.starts_with("L(") {
            return Ok(Some(self.convert_loop(trimmed)?));
        }

        // If: if 或 ?
        if trimmed.starts_with("if ") || trimmed.starts_with("if(") {
            return Ok(Some(self.convert_if(trimmed)?));
        }
        if trimmed.starts_with("? ") || trimmed.starts_with("?(") {
            return Ok(Some(self.convert_if_legacy(trimmed)?));
        }

        // Match: M -> switch
        if trimmed.starts_with("M ") || trimmed.starts_with("M(") {
            return Ok(Some(self.convert_match(trimmed)?));
        }

        // 函数定义: F/f
        if trimmed.starts_with("F ") || trimmed.starts_with("f ") {
            let after_marker = &trimmed[2..];
            if after_marker.contains('(') {
                return Ok(Some(self.convert_function(trimmed, context)?));
            }
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

        // 枚举: E/e
        if trimmed.starts_with("E ") || trimmed.starts_with("e ") {
            if !trimmed.contains("=>") {
                return Ok(Some(self.convert_enum(trimmed)?));
            }
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
                impl_content.split(" for ").nth(1)
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
        
        // 检查是否是main函数
        if converted.trim_start().starts_with("main(") {
            // main函数特殊处理
            return self.convert_main_function(&converted);
        }
        
        // 修复错误3: &self参数处理
        // 检查是否包含&self或&mut self参数
        let has_self_param = converted.contains("&self") || converted.contains("&mut self");
        
        let visibility = if context.in_class {
            ""  // 类内方法不需要public:前缀
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
        let mut result = self.convert_function_signature(&converted)?;
        if let Some(ref class_name) = context.current_class_name {
            result = result.replace("Self", class_name);
        }
        
        // 修复错误3: 移除&self和&mut self参数
        result = self.remove_self_parameter(&result);
        
        Ok(format!("{}{}", visibility, result))
    }

    fn convert_main_function(&self, sig: &str) -> Result<String> {
        // main函数必须返回int - 但保留函数体内容
        if let Some(brace_pos) = sig.find('{') {
            let rest = &sig[brace_pos..];
            // 不添加任何内容，保留原始函数体
            return Ok(format!("int main() {}", rest));
        }
        Ok("int main()".to_string())
    }

    fn convert_function_signature(&self, sig: &str) -> Result<String> {
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
                
                // 处理函数体
                let formatted_rest = if !rest.is_empty() {
                    self.format_function_body(rest, ret_type)?
                } else {
                    String::new()
                };
                
                // P0修复1: 检测new()关键字冲突
                if func_name == "new" {
                    // 检查参数是否为空（去除括号后）
                    let params_inner = converted_params.trim_matches(|c| c == '(' || c == ')').trim();
                    if params_inner.is_empty() {
                        // 默认构造函数: Type new() -> Type()
                        return Ok(format!("{}() {}", ret_type, formatted_rest));
                    } else {
                        // 带参构造函数: Type new(params) -> static Type create(params)
                        return Ok(format!("static {} create{} {}", ret_type, converted_params, formatted_rest));
                    }
                }
                
                return Ok(format!("{} {}{} {}", ret_type, func_name, converted_params, formatted_rest));
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
            
            // 处理函数体
            let formatted_rest = if !rest.is_empty() {
                self.format_function_body(rest, "void")?
            } else {
                String::new()
            };
            
            // P0修复1: 检测new()关键字冲突（无返回类型版本）
            if func_name == "new" {
                let params_inner = converted_params.trim_matches(|c| c == '(' || c == ')').trim();
                if params_inner.is_empty() {
                    // 无法推断类型名，保持原样并添加注释
                    return Ok(format!("/* constructor */ new{} {}", converted_params, formatted_rest));
                } else {
                    // 转换为静态工厂方法
                    return Ok(format!("static auto create{} {}", converted_params, formatted_rest));
                }
            }
            
            return Ok(format!("void {}{} {}", func_name, converted_params, formatted_rest));
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
        
        // 如果函数体为空
        if trimmed_inner.is_empty() {
            return Ok("{}".to_string());
        }
        
        // 检查是否需要添加return语句
        // 如果返回类型不是void，且函数体是单个表达式
        if ret_type != "void" && ret_type != "()" {
            // 检查内容是否已经有return、或是控制流语句
            let starts_with_control = trimmed_inner.starts_with("return") ||
                                     trimmed_inner.starts_with("if") ||
                                     trimmed_inner.starts_with("while") ||
                                     trimmed_inner.starts_with("for") ||
                                     trimmed_inner.starts_with("switch");
            
            let ends_with_terminator = trimmed_inner.ends_with(';') || trimmed_inner.ends_with('}');
            
            // 如果不是控制流语句，也不以分号或}结尾，那么这是一个需要return的表达式
            if !starts_with_control && !ends_with_terminator {
                return Ok(format!("{{ return {}; }}", trimmed_inner));
            }
        }
        
        Ok(body.to_string())
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
                    let converted_type = if param_type.starts_with('&') && !param_type.starts_with("&self") {
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
                let params = &result[start+1..end];
                let new_params: Vec<&str> = params
                    .split(',')
                    .map(|p| p.trim())
                    .filter(|p| !p.is_empty() && *p != "&self" && *p != "&mut self" && !p.starts_with("self:"))
                    .collect();
                
                let before = &result[..start+1];
                let after = &result[end..];
                result = format!("{}{}{}", before, new_params.join(", "), after);
            }
        }
        
        result
    }

    fn convert_struct(&self, line: &str) -> Result<String> {
        let is_pub_marker = line.starts_with("S ");
        let content = &line[2..];
        
        // P0修复3: 强化struct字段语法转换
        // 检查是否有成员声明（包含冒号但不是::）
        if content.contains(':') && !content.contains("::") && !content.ends_with('{') && !content.starts_with("//") {
            // 这是结构体成员声明，需要转换 "name: Type," 为 "Type name;"
            let parts: Vec<&str> = content.splitn(2, ':').collect();
            if parts.len() == 2 {
                let member_name = parts[0].trim();
                let member_type = parts[1].trim().trim_end_matches(',').trim();
                
                // 应用引用类型转换
                let converted_type = if member_type.starts_with('&') && !member_type.starts_with("&self") {
                    let inner_type = member_type[1..].trim();
                    format!("const {}&", self.convert_types_in_string(inner_type))
                } else {
                    self.convert_types_in_string(member_type)
                };
                
                // 确保没有多余的逗号和空格
                // C++不需要每个字段都加public，使用public:标签（在convert_line中处理）
                return Ok(format!("{} {};", converted_type, member_name));
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
                        let real_suffix = if suffix.starts_with(')') { &suffix[1..] } else { suffix };
                        format!("{}({}){}", converted_name, converted_fields_str, real_suffix)
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

    fn convert_enum(&self, line: &str) -> Result<String> {
        let content = &line[2..];
        
        // P0修复5: enum带数据变体转换
        // 检查是否是带数据的enum变体（在enum块内）
        if content.contains('(') && !content.ends_with('{') && !content.contains("enum ") {
            // 这是enum变体，如 InvalidOperator(String)
            // C++需要使用std::variant或tagged union
            // 简化实现：转换为注释说明
            let parts: Vec<&str> = content.splitn(2, '(').collect();
            if parts.len() == 2 {
                let variant_name = parts[0].trim();
                let variant_data = parts[1].trim().trim_end_matches(')').trim_end_matches(',');
                let converted_type = self.convert_types_in_string(variant_data);
                
                // 转换为C++结构体变体（需要用户实现完整的variant）
                return Ok(format!("// {} variant with data: {}", variant_name, converted_type));
            }
            return Ok(format!("// enum variant: {}", content));
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
        
        // P0修复6: impl块转换
        if content.contains(" for ") {
            // trait implementation: impl Trait for Type
            let parts: Vec<&str> = content.split(" for ").collect();
            if parts.len() == 2 {
                let trait_name = parts[0].trim();
                let type_name = parts[1].trim().trim_end_matches(" {").trim();
                
                // C++需要在类定义内部实现接口
                // 这里生成注释，实际实现需要将方法添加到类内
                return Ok(format!("// Implementation of {} for {}", trait_name, type_name));
            }
        }
        
        // 普通impl块: impl Type { ... }
        // 提取类型名称
        let type_name = content.trim().trim_end_matches(" {").trim();
        
        // C++中impl块的方法需要以 ClassName::methodName 形式定义
        // 或者直接在类内部定义
        // 这里输出注释标记impl块开始
        Ok(format!("// Implementation for {}", type_name))
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
        let converted = self.convert_inline_keywords(content)?;
        let converted = self.convert_macros(&converted)?;
        let converted = self.convert_types_in_string(&converted);
        Ok(format!("const auto {}", converted))
    }

    fn convert_let_mut(&self, line: &str) -> Result<String> {
        let content = &line[2..];
        let converted = self.convert_inline_keywords(content)?;
        let converted = self.convert_macros(&converted)?;
        let converted = self.convert_types_in_string(&converted);
        Ok(format!("auto {}", converted))
    }

    fn convert_return(&self, line: &str) -> Result<String> {
        if line == "<" {
            Ok("return;".to_string())
        } else {
            let content = &line[2..];
            let converted = self.convert_types_in_string(content);
            Ok(format!("return {};", converted.trim_end_matches(';')))
        }
    }

    fn convert_loop(&self, line: &str) -> Result<String> {
        if line == "L {" {
            return Ok("while (true) {".to_string());
        }
        
        if let Some(content) = line.strip_prefix("L ") {
            if content.contains(" in ") {
                // for循环
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
        
        // 对于Result/Option类型，使用if-else而不是switch
        // 格式：M value { ... }转换为：// match on value
        Ok(format!("// match {}", converted))
    }
    
    /// P0修复2: 转换 match arm，完整处理 => 语法
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
        
        // 处理多分支模式 "a" | "b" | "c" => expr
        if pattern.contains('|') {
            let patterns: Vec<&str> = pattern.split('|').map(|s| s.trim()).collect();
            let conditions: Vec<String> = patterns.iter()
                .map(|p| format!("value == {}", p))
                .collect();
            let condition = conditions.join(" || ");
            return Ok(format!("if ({}) {{ {}; }}", condition, converted_expr));
        }
        
        // 处理enum变体模式
        if pattern.contains("::") {
            // 枚举变体: Operator::Add => "+"
            let enum_parts: Vec<&str> = pattern.split("::").collect();
            if enum_parts.len() == 2 {
                let enum_name = enum_parts[0].trim();
                let variant = enum_parts[1].trim();
                return Ok(format!("case {}::{}: return {};", enum_name, variant, converted_expr));
            }
        }
        
        // 处理Result/Option模式
        if pattern == "None" || pattern == "_" {
            Ok(format!("else {{ {}; }}", converted_expr))
        } else if pattern.starts_with("Some(") || pattern.starts_with("Ok(") {
            let var = if pattern.starts_with("Some(") {
                pattern.trim_start_matches("Some(").trim_end_matches(')')
            } else {
                pattern.trim_start_matches("Ok(").trim_end_matches(')')
            };
            Ok(format!("if (value.has_value()) {{ auto {} = value.value(); {}; }}", var, converted_expr))
        } else if pattern.starts_with("Err(") {
            let var = pattern.trim_start_matches("Err(").trim_end_matches(')');
            Ok(format!("else {{ auto {} = value.error(); {}; }}", var, converted_expr))
        } else {
            // 字符串或其他字面量模式
            Ok(format!("if (value == {}) {{ {}; }}", pattern, converted_expr))
        }
    }

    fn convert_print(&self, line: &str) -> Result<String> {
        let content = &line[2..];
        let converted = self.convert_types_in_string(content);
        Ok(format!("std::cout << {} << std::endl;", converted.trim_end_matches(';')))
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
        let mut result = self.convert_inline_keywords(line)?;
        result = self.convert_macros(&result)?;
        result = self.convert_types_in_string(&result);
        
        // 修复函数调用格式：确保函数名和参数之间有正确的括号
        result = self.fix_function_calls(&result)?;
        
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
                        
                        if tuple_start < result.len() && tuple_end <= result.len()
                            && &result[tuple_start..tuple_end] == "std::tuple<" {
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
                if current_byte_pos < content.len() && end_byte_pos <= content.len()
                    && &content[current_byte_pos..end_byte_pos] == "println!" {
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
                        if start_byte <= content.len() && end_byte <= content.len() && start_byte <= end_byte {
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
            if i + 2 <= chars.len() && chars[i] == 'V' && chars[i+1] == '!' {
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
        let mut i = 0;
        let chars: Vec<char> = content.chars().collect();
        
        while i < chars.len() {
            if i + 4 <= chars.len() && &content[i..i+4] == "Some" {
                // 检查是否是独立的 Some（不是某个标识符的一部分）
                let is_word_start = i == 0 || !chars[i-1].is_alphanumeric() && chars[i-1] != '_';
                let is_word_end = i + 4 >= chars.len() || !chars[i+4].is_alphanumeric() && chars[i+4] != '_';
                
                if is_word_start && is_word_end {
                    i += 4;
                    
                    // 跳过空白
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    
                    if i < chars.len() && chars[i] == '(' {
                        i += 1;
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
                        
                        let value = &content[start..i];
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
                            result.push_str(&format!("std::optional<{}>{{{}}}", type_hint, value));
                        }
                        continue;
                    }
                }
            }
            
            result.push(chars[i]);
            i += 1;
        }
        
        Ok(result)
    }
    
    /// 转换 Ok(...) 为返回值
    fn convert_ok_constructor(&self, content: &str) -> Result<String> {
        let mut result = String::new();
        let mut i = 0;
        let chars: Vec<char> = content.chars().collect();
        
        while i < chars.len() {
            if i + 2 <= chars.len() && i + 2 <= content.len() {
                let slice = &content[i..];
                if slice.starts_with("Ok") {
                    let is_word_start = i == 0 || (!chars[i-1].is_alphanumeric() && chars[i-1] != '_');
                    let is_word_end = i + 2 >= chars.len() || (!chars[i+2].is_alphanumeric() && chars[i+2] != '_');
                    
                    if is_word_start && is_word_end {
                        i += 2;
                        while i < chars.len() && chars[i].is_whitespace() {
                            i += 1;
                        }
                        
                        if i < chars.len() && chars[i] == '(' {
                            i += 1;
                            let start = i;
                            let mut depth = 1;
                            let mut in_string = false;
                            let mut escape_next = false;
                            
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
                            
                            let value = &content[start..i];
                            i += 1;
                            result.push_str(&format!("return {}", value));
                            continue;
                        }
                    }
                }
            }
            
            result.push(chars[i]);
            i += 1;
        }
        
        Ok(result)
    }
    
    /// 转换 Err(...) 为 std::unexpected(...)
    fn convert_err_constructor(&self, content: &str) -> Result<String> {
        let mut result = String::new();
        let mut i = 0;
        let chars: Vec<char> = content.chars().collect();
        
        while i < chars.len() {
            if i + 3 <= chars.len() && i + 3 <= content.len() {
                let slice = &content[i..];
                if slice.starts_with("Err") {
                    let is_word_start = i == 0 || (!chars[i-1].is_alphanumeric() && chars[i-1] != '_');
                    let is_word_end = i + 3 >= chars.len() || (!chars[i+3].is_alphanumeric() && chars[i+3] != '_');
                    
                    if is_word_start && is_word_end {
                        i += 3;
                        while i < chars.len() && chars[i].is_whitespace() {
                            i += 1;
                        }
                        
                        if i < chars.len() && chars[i] == '(' {
                            i += 1;
                            let start = i;
                            let mut depth = 1;
                            let mut in_string = false;
                            let mut escape_next = false;
                            
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
                            
                            let error_value = &content[start..i];
                            i += 1;
                            result.push_str(&format!("return std::unexpected({})", error_value));
                            continue;
                        }
                    }
                }
            }
            
            result.push(chars[i]);
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
                let has_start_boundary = i == 0 ||
                    (!chars[i - 1].is_alphanumeric() && chars[i - 1] != '_');
                
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

    /// 转换Nu类型到C++类型
    fn convert_types_in_string(&self, s: &str) -> String {
        // 第一步：保护闭包参数，避免单字母变量被误转换（借鉴nu2rust）
        let mut result = s.to_string();
        let mut protected_closures = Vec::new();

        // 查找所有闭包参数列表（|param1, param2| 或 |params| -> RetType）
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

                    // 检查是否有返回类型 -> Type
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    // 检查 ->
                    if i + 1 < chars.len() && chars[i] == '-' && chars[i + 1] == '>' {
                        i += 2;
                        while i < chars.len() && chars[i].is_whitespace() {
                            i += 1;
                        }
                        // 找到返回类型的结束（遇到 { 或 ; ）
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
            .replace("&!", "&")     // &! -> 普通引用
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

        // 13. 恢复闭包参数（借鉴nu2rust）
        for (idx, closure) in protected_closures.iter().enumerate() {
            // 转换闭包中的类型缩写
            let converted_closure = closure
                .replace("R<", "std::expected<")
                .replace("O<", "std::optional<")
                .replace("V<", "std::vector<")
                .replace("A<", "std::shared_ptr<")
                .replace("X<", "std::mutex<")
                .replace("B<", "std::unique_ptr<");
            result = result.replace(&format!("__CLOSURE_PARAMS_{}__", idx), &converted_closure);
        }

        // 14. 处理泛型尖括号，避免 >> 被解析为右移
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
    /// 只在返回类型位置转换元组，不转换函数参数列表和函数调用
    fn convert_tuple_types(&self, s: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;
        
        while i < chars.len() {
            // 查找可能的元组定义 (T1, T2, ...)
            if chars[i] == '(' {
                let start = i;
                
                // 检查左括号前面是否有标识符（函数名）
                let mut is_function_call = false;
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
                
                // 如果是元组（有逗号但没有冒号，且不是函数调用），转换为 std::tuple
                // 有冒号说明是函数参数列表 (name: Type)，不转换
                // is_function_call 说明是函数调用，不转换
                if has_comma && !has_colon && !is_function_call && depth == 0 {
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
                    } else if depth == 1 && j + 1 < chars.len() && chars[j] == '.' && chars[j+1] == '.' {
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
                    while obj_start > 0 && (chars[obj_start-1].is_alphanumeric() || chars[obj_start-1] == '_' || chars[obj_start-1] == ']') {
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
                        result.push_str(&format!("std::vector<decltype({}[0])>({}.begin() + {}, {}.end())", 
                            obj_name, obj_name, start_expr, obj_name));
                    } else {
                        // a[x..y] -> std::vector(a.begin() + x, a.begin() + y)
                        result.push_str(&format!("std::vector<decltype({}[0])>({}.begin() + {}, {}.begin() + {})", 
                            obj_name, obj_name, start_expr, obj_name, end_expr));
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
            result = result.replace(".split_whitespace()",
                ".split_whitespace() /* TODO: implement split_whitespace */");
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
}

impl Default for Nu2CppConverter {
    fn default() -> Self {
        Self::new()
    }
}
