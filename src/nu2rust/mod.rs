// Nu to Rust Converter
// 将Nu代码转换回标准Rust代码

use anyhow::Result;

// 导出 sourcemap 模块
pub mod sourcemap;
pub use sourcemap::LazySourceMap;

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
    in_trait_impl: bool,   // v1.6.4: 区分 impl Trait for Type (不能加pub)
    in_struct_block: bool, // v1.6.4: 追踪是否在struct定义块内
}

impl Nu2RustConverter {
    pub fn new() -> Self {
        Self {
            context: ConversionContext::default(),
        }
    }

    /// 转换 Nu 代码为 Rust 代码
    ///
    /// # Arguments
    /// * `nu_code` - Nu 源代码
    /// * `sourcemap` - 可选的 SourceMap，用于记录行号映射
    pub fn convert(&self, nu_code: &str) -> Result<String> {
        self.convert_with_sourcemap(nu_code, None)
    }
    
    /// 转换 Nu 代码为 Rust 代码，并记录行号映射
    ///
    /// # Arguments
    /// * `nu_code` - Nu 源代码
    /// * `sourcemap` - 可选的 SourceMap，用于记录行号映射
    pub fn convert_with_sourcemap(
        &self,
        nu_code: &str,
        mut sourcemap: Option<&mut LazySourceMap>,
    ) -> Result<String> {
        let mut output = String::new();
        let lines: Vec<&str> = nu_code.lines().collect();
        let mut context = ConversionContext::default();

        let mut i = 0;
        let mut rust_line = 1; // 跟踪当前生成的 Rust 行号（1-based）
        
        while i < lines.len() {
            // v1.8: 保留原始行（包含前导空格）用于输出
            // trimmed 仅用于模式检测
            let line = lines[i];
            let trimmed = line.trim();
            let nu_line = i + 1; // Nu 源代码行号（1-based）

            // 保留空行和注释（不跳过）
            if trimmed.is_empty() {
                output.push('\n');
                rust_line += 1;
                i += 1;
                continue;
            }

            // 保留注释行
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
                // 记录映射：注释行也映射
                if let Some(ref mut sm) = sourcemap {
                    sm.add_mapping(rust_line, nu_line);
                }
                output.push_str(lines[i]);
                output.push('\n');
                rust_line += 1;
                i += 1;
                continue;
            }

            // 处理属性标记
            if trimmed.starts_with("#D") {
                // 转换 #D(Debug) -> #[derive(Debug)]
                let attr_content = trimmed.trim_start_matches("#D");
                // 修复派生属性中的空格: #D (Debug) -> #[derive(Debug)]
                let fixed_content = attr_content
                    .replace(" (", "(")
                    .replace(" )", ")")
                    .replace(" ,", ",");
                // 记录映射
                if let Some(ref mut sm) = sourcemap {
                    sm.add_mapping(rust_line, nu_line);
                }
                output.push_str(&format!("#[derive{}]\n", fixed_content));
                rust_line += 1;
                i += 1;
                continue;
            }

            // 其他属性（如 #[test], #[cfg(test)], #![cfg(...)]）
            if (trimmed.starts_with("#[") && trimmed.ends_with("]"))
                || (trimmed.starts_with("#![") && trimmed.ends_with("]"))
                || (trimmed.starts_with("# [") && trimmed.ends_with("]"))
                || (trimmed.starts_with("# ![") && trimmed.ends_with("]"))
            {
                // 修复属性中的空格格式
                // "# [cfg (not (feature = "kv"))]" -> "#[cfg(not(feature = "kv"))]"
                let mut fixed_attr = line.to_string();
                fixed_attr = fixed_attr.replace("# [", "#[").replace("# ![", "#![");
                fixed_attr = fixed_attr.replace(" (", "(").replace(" )", ")");
                fixed_attr = fixed_attr.replace(" ,", ",");
                // 记录映射
                if let Some(ref mut sm) = sourcemap {
                    sm.add_mapping(rust_line, nu_line);
                }
                output.push_str(&fixed_attr);
                output.push('\n');
                rust_line += 1;
                i += 1;
                continue;
            }

            // 智能恢复丢失的cfg属性
            // 检测panic语句：当行包含panic!("key-value support时，在前面添加#[cfg(not(feature = "kv"))]
            // 注意：Nu转换可能在!后添加空格，所以检查 panic!( 和 panic ! (
            if (line.contains("panic!(") || line.contains("panic !"))
                && line.contains("key-value support")
            {
                output.push_str("#[cfg(not(feature = \"kv\"))]\n");
                rust_line += 1;
            }

            // 检测key_values调用：当行包含.key_values(时，在前面添加#[cfg(feature = "kv")]
            if trimmed.contains(".key_values(") {
                output.push_str("#[cfg(feature = \"kv\")]\n");
                rust_line += 1;
            }

            // 记录当前行的映射（在转换之前）
            if let Some(ref mut sm) = sourcemap {
                sm.add_mapping(rust_line, nu_line);
            }

            // 处理各种Nu语法
            if let Some(mut converted) = self.convert_line(line, &lines, &mut i, &mut context)? {
                // v1.6.4: 结构体字段默认添加pub（状态机精确控制）
                if context.in_struct_block {
                    let trimmed = line.trim();
                    // 检测是否是字段行（包含冒号，不是注释，不是已有pub，不是struct定义本身）
                    if !trimmed.is_empty()
                        && !trimmed.starts_with("//")
                        && !trimmed.starts_with("pub ")
                        && !trimmed.starts_with("S ")   // 排除 struct 定义行
                        && !trimmed.starts_with("s ")   // 排除 struct 定义行
                        && trimmed.contains(':')
                        && !trimmed.starts_with("fn ")
                        && !trimmed.starts_with('}')
                    {
                        // 在converted前添加pub
                        converted = format!("pub {}", converted);
                    }
                }
                // v1.8: 全局缩进保留 - 从原始行提取前导空格
                // 这是统一的解决方案，避免在每个转换函数中单独处理
                let leading_whitespace: String = line.chars().take_while(|c| c.is_whitespace()).collect();
                // 如果converted已经有缩进（从某些函数返回），先去除再添加正确的
                let trimmed_converted = converted.trim_start();
                output.push_str(&leading_whitespace);
                output.push_str(trimmed_converted);
                output.push('\n');
                rust_line += 1;
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
        // v1.7.5: 添加 L( 模式支持用于带模式匹配的 for 循环，如 L(i,(word, count)) in ...
        if trimmed.starts_with("L ") || trimmed == "L {" || trimmed.starts_with("L(") {
            return Ok(Some(self.convert_loop(trimmed)?));
        }

        // If not: ?!
        if trimmed.starts_with("?! ") {
            return Ok(Some(self.convert_if_not(trimmed)?));
        }

        // If: ? (also handle ?(condition) case)
        if trimmed.starts_with("? ") || trimmed.starts_with("?(") {
            return Ok(Some(self.convert_if(trimmed)?));
        }

        // Match: M (v1.7.6: 支持 M( 模式用于元组模式匹配，如 M(c1, c2) { ... })
        if trimmed.starts_with("M ") || trimmed.starts_with("M(") {
            return Ok(Some(self.convert_match(trimmed)?));
        }

        // Unsafe函数定义: 
        // v1.8: "unsafe F"/"unsafe f" (新格式，unsafe 不缩写)
        // 兼容: "U F"/"U f" (旧格式)
        if trimmed.starts_with("unsafe F ") || trimmed.starts_with("unsafe f ") {
            return Ok(Some(self.convert_unsafe_function_v18(trimmed, context)?));
        }
        if trimmed.starts_with("U F ") || trimmed.starts_with("U f ") {
            return Ok(Some(self.convert_unsafe_function(trimmed, context)?));
        }

        // v1.8: "const F"/"const f" (支持 const fn)
        if trimmed.starts_with("const F ") || trimmed.starts_with("const f ") {
            return Ok(Some(self.convert_const_function(trimmed, context)?));
        }

        // 函数定义: F/f (F=pub fn, f=fn)
        // 但要避免将函数调用误认为函数定义
        // 函数定义必须包含括号（参数列表）或返回类型
        if trimmed.starts_with("F ") || trimmed.starts_with("f ") {
            // 检查是否真的是函数定义（有参数列表或返回类型）
            // 函数定义: "f name(...)" 或 "f name<T>(...)" 或 "F main() {" 等
            // 函数调用: "f ()" 或 "f ();" - 这些不应该被转换
            let after_marker = &trimmed[2..]; // 跳过 "F " 或 "f "

            // 如果紧跟着 '(' 或 '()'，这是函数调用，不是定义
            if after_marker.starts_with('(') || after_marker.starts_with("();") {
                // 这是函数调用，不转换，作为普通表达式处理
                return Ok(Some(self.convert_expression(trimmed)?));
            }

            // 否则检查是否包含函数定义的特征（标识符后跟括号）
            // 例如: "f name(...)" 或 "F main() {" 等
            if after_marker.contains('(') {
                return Ok(Some(self.convert_function(trimmed, context)?));
            }

            // 如果没有括号，也不是函数定义，作为表达式处理
            return Ok(Some(self.convert_expression(trimmed)?));
        }

        // 异步函数: ~F/~f
        if trimmed.starts_with("~F ") || trimmed.starts_with("~f ") {
            return Ok(Some(self.convert_async_function(trimmed)?));
        }

        // v1.8: pub(crate)/pub(super) 前缀的结构体和枚举
        // 检测 "pub(crate) S " 或 "pub(super) S " 等模式
        // 使用 line 而不是 trimmed 来保留前导空格
        if trimmed.contains(") S ") || trimmed.contains(") s ") {
            if trimmed.starts_with("pub(") {
                if trimmed.ends_with('{') {
                    context.in_struct_block = true;
                }
                return Ok(Some(self.convert_struct_with_visibility(line)?));
            }
        }
        if trimmed.contains(") E ") || trimmed.contains(") e ") {
            if trimmed.starts_with("pub(") {
                return Ok(Some(self.convert_enum_with_visibility(line)?));
            }
        }

        // 结构体: S/s (S=pub struct, s=struct)
        // v1.8.2: 确保不会将 "s = expr" 误识别为 struct 定义
        // struct 定义格式必须是 "S Name" 或 "s Name"，其中 Name 是标识符
        if trimmed.starts_with("S ") || trimmed.starts_with("s ") {
            let after_keyword = if trimmed.starts_with("S ") { &trimmed[2..] } else { &trimmed[2..] };
            let first_char = after_keyword.chars().next();
            // 如果后面是标识符开头（字母或下划线），才是 struct 定义
            // 如果是 "=" 或 "." 或 "[" 等，则是变量赋值或表达式，不是 struct
            if let Some(c) = first_char {
                if c.is_alphabetic() || c == '_' {
                    // v1.6.4: 进入struct块
                    if trimmed.ends_with('{') {
                        context.in_struct_block = true;
                    }
                    return Ok(Some(self.convert_struct(trimmed, lines, index)?));
                }
            }
        }

        // 枚举: E/e (E=pub enum, e=enum)
        if trimmed.starts_with("E ") || trimmed.starts_with("e ") {
            // v1.8.2: 排除 match arm (E => ...)
            if !trimmed.contains("=>") {
                return Ok(Some(self.convert_enum(trimmed, lines, index)?));
            }
        }

        // v1.8.3: unsafe Trait: unsafe TR/unsafe tr
        if trimmed.starts_with("unsafe TR ") || trimmed.starts_with("unsafe tr ") {
            return Ok(Some(self.convert_unsafe_trait(trimmed, lines, index)?));
        }

        // Trait: TR/tr
        if trimmed.starts_with("TR ") || trimmed.starts_with("tr ") {
            return Ok(Some(self.convert_trait(trimmed, lines, index)?));
        }

        // Impl块: I 或 unsafe I (v1.8) 或 U I (旧格式)
        // v1.8: unsafe impl -> "unsafe I" (不缩写 unsafe)
        if trimmed.starts_with("unsafe I ") || trimmed.starts_with("unsafe I<") {
            context.in_impl = true;
            context.in_trait_impl = trimmed.contains(" for ");
            return Ok(Some(self.convert_unsafe_impl_v18(trimmed, lines, index)?));
        }
        // 兼容旧格式: U I
        if trimmed.starts_with("U I ") {
            context.in_impl = true;
            // 检测是否是 trait impl: "U I Trait for Type"
            context.in_trait_impl = trimmed.contains(" for ");
            return Ok(Some(self.convert_unsafe_impl(trimmed, lines, index)?));
        }

        if trimmed.starts_with("I ") {
            context.in_impl = true;
            // 检测是否是 trait impl: "I Trait for Type"
            context.in_trait_impl = trimmed.contains(" for ");
            return Ok(Some(self.convert_impl(trimmed, lines, index)?));
        }

        // 检测块结束
        if trimmed == "}" {
            if context.in_impl {
                context.in_impl = false;
                context.in_trait_impl = false;
            }
            if context.in_struct_block {
                context.in_struct_block = false;
            }
        }

        // 模块: DM=pub mod, D=mod (Nu v1.6.3)
        // v1.8: 支持受限可见性 pub(crate) DM, pub(super) DM, pub(in path) DM
        if trimmed.starts_with("pub(crate) DM ") {
            let content = &trimmed[14..]; // 跳过 "pub(crate) DM "
            let converted = self.convert_types_in_string(content);
            return Ok(Some(format!("pub(crate) mod {}", converted)));
        }
        if trimmed.starts_with("pub(super) DM ") {
            let content = &trimmed[14..]; // 跳过 "pub(super) DM "
            let converted = self.convert_types_in_string(content);
            return Ok(Some(format!("pub(super) mod {}", converted)));
        }
        // v1.8.4: 支持 pub(in path) DM 格式，如 "pub(in crate :: runtime) DM time_alt;"
        if trimmed.starts_with("pub(in ") && trimmed.contains(") DM ") {
            // 找到 ") DM " 的位置
            if let Some(dm_pos) = trimmed.find(") DM ") {
                let vis_part = &trimmed[..dm_pos + 1]; // "pub(in crate :: runtime)"
                let content = &trimmed[dm_pos + 5..]; // "time_alt;"
                // 清理可见性部分的空格: "pub(in crate :: runtime)" -> "pub(in crate::runtime)"
                let cleaned_vis = vis_part.replace(" :: ", "::").replace(" ::", "::").replace(":: ", "::");
                let converted = self.convert_types_in_string(content);
                return Ok(Some(format!("{} mod {}", cleaned_vis, converted)));
            }
        }
        if trimmed.starts_with("DM ") {
            return Ok(Some(self.convert_pub_module(trimmed)?));
        }

        if trimmed.starts_with("D ") {
            return Ok(Some(self.convert_module(trimmed)?));
        }

        // 变量声明: l/v
        // 注意：需要区分变量声明 "v name = ..." 和变量使用 "v.method()"
        if trimmed.starts_with("l ") {
            return Ok(Some(self.convert_let(trimmed)?));
        }
        if trimmed.starts_with("v ") {
            // 检查是否是变量声明而不是变量使用
            // 变量声明: "v name = ..." 或 "v name: Type = ..." 或 "v (a, b) = ..." (元组解构)
            // 变量使用: "v.method()" 或 "v + ..." 等
            let after_v = &trimmed[2..]; // 跳过 "v "
            // v1.8: 添加 '(' 支持元组解构模式
            let is_declaration = after_v
                .chars()
                .next()
                .map(|c| c.is_alphabetic() || c == '_' || c == '(')
                .unwrap_or(false)
                && !after_v.starts_with('.');

            if is_declaration {
                return Ok(Some(self.convert_let_mut(trimmed)?));
            }
            // 否则作为普通表达式处理
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

        // Unsafe块表达式: U { (必须在use之前检查)
        if trimmed.starts_with("U {") {
            return Ok(Some(self.convert_unsafe_block(trimmed)?));
        }

        // Use语句: u/U
        if trimmed.starts_with("u ") || trimmed.starts_with("U ") {
            return Ok(Some(self.convert_use(trimmed)?));
        }

        // Type alias: t (but not "t +=" or "t -=" which are variable assignments)
        if trimmed.starts_with("t ") {
            // v1.8.2: 检查是否是赋值操作 (t += ..., t -= ..., t = ...) 而不是类型别名 (t Name = ...)
            // 判断依据：如果 t 后面紧跟一个标识符（如 "Output"），则是类型别名
            // 如果 t 后面紧跟操作符（如 "+=", "-="），则是赋值
            let content = &trimmed[2..];
            let first_token_is_operator = content.starts_with("+=") 
                || content.starts_with("-=")
                || content.starts_with("*=")
                || content.starts_with("/=")
                || content.starts_with("=");  // 只检查开头是否直接是操作符
            if !first_token_is_operator {
                return Ok(Some(self.convert_type_alias(trimmed)?));
            }
        }

        // Const: C or CP (pub)
        if trimmed.starts_with("C ") {
            return Ok(Some(self.convert_const(trimmed, false)?));
        }
        if trimmed.starts_with("CP ") {
            return Ok(Some(self.convert_const(trimmed, true)?));
        }

        // Static: ST or SP (pub)
        if trimmed.starts_with("ST ") {
            return Ok(Some(self.convert_static(trimmed, false, false)?));
        }
        if trimmed.starts_with("SP ") {
            return Ok(Some(self.convert_static(trimmed, false, true)?));
        }

        // Static mut: SM or SMP (pub)
        if trimmed.starts_with("SM ") {
            return Ok(Some(self.convert_static(trimmed, true, false)?));
        }
        if trimmed.starts_with("SMP ") {
            return Ok(Some(self.convert_static(trimmed, true, true)?));
        }

        // 其他情况：转换类型和表达式
        Ok(Some(self.convert_expression(trimmed)?))
    }

    fn convert_function(&self, line: &str, context: &ConversionContext) -> Result<String> {
        let is_pub = line.starts_with("F ");
        let content = &line[2..];

        // v1.6.4 Hotfix: 智能处理impl块内的方法可见性
        let visibility = if context.in_trait {
            "" // trait定义中的方法不能有pub
        } else if context.in_trait_impl {
            "" // trait实现中的方法不能有pub (impl Trait for Type)
        } else if context.in_impl {
            "pub " // 固有impl中的方法默认pub (impl Type)
        } else if is_pub {
            "pub " // 顶层的F标记
        } else {
            "" // 顶层的f标记（私有函数）
        };

        let mut converted = self.convert_types_in_string(content);

        // 处理 !self -> mut self (按值接收的可变self)
        // v1.8.3: 处理更多模式
        converted = converted.replace("(!self)", "(mut self)");
        converted = converted.replace("(!self,", "(mut self,");
        converted = converted.replace("(!self", "(mut self");

        Ok(format!("{}fn {}", visibility, converted))
    }

    fn convert_unsafe_function(&self, line: &str, context: &ConversionContext) -> Result<String> {
        let is_pub = line.starts_with("U F ");
        let content = if is_pub { &line[4..] } else { &line[3..] }; // 跳过 "U F " 或 "U f "

        // v1.6.4: 与convert_function保持一致的可见性逻辑
        let visibility = if context.in_trait {
            "" // trait定义中的方法不能有pub
        } else if context.in_trait_impl {
            "" // trait实现中的方法不能有pub
        } else if context.in_impl {
            "pub " // 固有impl中的方法默认pub
        } else if is_pub {
            "pub " // 顶层的U F标记
        } else {
            "" // 顶层的U f标记（私有函数）
        };

        let mut converted = self.convert_types_in_string(content);

        // 处理 !self -> mut self (按值接收的可变self)
        // v1.8.3: 处理更多模式
        converted = converted.replace("(!self)", "(mut self)");
        converted = converted.replace("(!self,", "(mut self,");
        converted = converted.replace("(!self", "(mut self");

        Ok(format!("{}unsafe fn {}", visibility, converted))
    }

    // v1.8: 处理 const 函数 (const F/const f)
    fn convert_const_function(&self, line: &str, context: &ConversionContext) -> Result<String> {
        let is_pub = line.starts_with("const F ");
        let content = if is_pub { &line[8..] } else { &line[7..] }; // 跳过 "const F " 或 "const f "

        let visibility = if context.in_trait {
            ""
        } else if context.in_trait_impl {
            ""
        } else if context.in_impl {
            "pub "
        } else if is_pub {
            "pub "
        } else {
            ""
        };

        let mut converted = self.convert_types_in_string(content);
        // v1.8.3: 处理更多模式
        converted = converted.replace("(!self)", "(mut self)");
        converted = converted.replace("(!self,", "(mut self,");
        converted = converted.replace("(!self", "(mut self");

        Ok(format!("{}const fn {}", visibility, converted))
    }


    // v1.8: 处理新格式的 unsafe 函数 (unsafe F/unsafe f 代替 U F/U f)
    fn convert_unsafe_function_v18(&self, line: &str, context: &ConversionContext) -> Result<String> {
        let is_pub = line.starts_with("unsafe F ");
        let content = if is_pub { &line[9..] } else { &line[8..] }; // 跳过 "unsafe F " 或 "unsafe f "

        let visibility = if context.in_trait {
            ""
        } else if context.in_trait_impl {
            ""
        } else if context.in_impl {
            "pub "
        } else if is_pub {
            "pub "
        } else {
            ""
        };

        let mut converted = self.convert_types_in_string(content);
        // v1.8.3: 处理更多模式
        converted = converted.replace("(!self)", "(mut self)");
        converted = converted.replace("(!self,", "(mut self,");
        converted = converted.replace("(!self", "(mut self");

        Ok(format!("{}unsafe fn {}", visibility, converted))
    }

    fn convert_async_function(&self, line: &str) -> Result<String> {
        let is_pub = line.starts_with("~F ");
        let content = &line[3..];

        let visibility = if is_pub { "pub " } else { "" };
        let mut converted = self.convert_types_in_string(content);
        
        // v1.8.3: 处理 !self -> mut self
        converted = converted.replace("(!self)", "(mut self)");
        converted = converted.replace("(!self,", "(mut self,");
        converted = converted.replace("(!self", "(mut self");

        Ok(format!("{}async fn {}", visibility, converted))
    }

    fn convert_struct(&self, line: &str, _lines: &[&str], _index: &mut usize) -> Result<String> {
        // S = pub struct, s = struct
        let is_pub_marker = line.starts_with("S ");
        let content = &line[2..]; // 跳过 "S " 或 "s "

        // v1.7.5: 处理 tuple struct 字段的可见性
        // 检查是否是tuple struct: struct Name(Type1, Type2);
        let converted =
            if content.contains('(') && content.contains(')') && content.trim().ends_with(';') {
                // Tuple struct - 需要给字段添加pub
                let struct_parts: Vec<&str> = content.splitn(2, '(').collect();
                if struct_parts.len() == 2 {
                    let struct_name = struct_parts[0];
                    let rest = struct_parts[1]; // "Type1, Type2);"

                    // 提取字段类型部分
                    if let Some(close_paren_pos) = rest.find(')') {
                        let fields = &rest[..close_paren_pos]; // "Type1, Type2"
                        let suffix = &rest[close_paren_pos..]; // ");" 或 ")"

                        // 给每个字段添加pub前缀，并转换类型
                        let pub_fields: Vec<String> = fields
                            .split(',')
                            .map(|f| {
                                let trimmed = f.trim();
                                if trimmed.is_empty() {
                                    String::new()
                                } else if trimmed.starts_with("pub ") {
                                    // v1.8: 对字段类型也进行转换
                                    let type_part = &trimmed[4..];
                                    format!("pub {}", self.convert_types_in_string(type_part))
                                } else {
                                    // v1.8: 对字段类型进行转换
                                    format!("pub {}", self.convert_types_in_string(trimmed))
                                }
                            })
                            .filter(|s| !s.is_empty())
                            .collect();

                        let converted_name = self.convert_types_in_string(struct_name);
                        let converted_fields = pub_fields.join(", ");
                        // v1.8: suffix 是 ");" 或 ")"，提取 ) 后的部分作为真正的后缀
                        let real_suffix = if suffix.starts_with(')') { &suffix[1..] } else { suffix };
                        format!("{}({}){}", converted_name, converted_fields, real_suffix)
                    } else {
                        self.convert_types_in_string(content)
                    }
                } else {
                    self.convert_types_in_string(content)
                }
            } else {
                self.convert_types_in_string(content)
            };

        let visibility = if is_pub_marker { "pub " } else { "" };
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

    // v1.8: 处理带受限可见性的 struct (如 pub(crate) S)
    fn convert_struct_with_visibility(&self, line: &str) -> Result<String> {
        // DEBUG: 跟踪调用
        eprintln!("DEBUG convert_struct_with_visibility: line=[{}]", line);
        
        // 格式: "pub(crate) S Name" 或 "pub(super) S Name {...}"
        // 找到 ") S " 或 ") s " 的位置
        let s_pos = if let Some(pos) = line.find(") S ") {
            pos + 2 // 跳过 ") "
        } else if let Some(pos) = line.find(") s ") {
            pos + 2
        } else {
            return Err(anyhow::anyhow!("Invalid struct with visibility format"));
        };
        
        let visibility = &line[..s_pos]; // includes leading whitespace + "pub(crate)"
        let rest = &line[s_pos + 2..]; // "Name {...}"
        let converted = self.convert_types_in_string(rest);
        
        let result = format!("{} struct {}", visibility, converted);
        eprintln!("DEBUG convert_struct_with_visibility: result=[{}]", result);
        Ok(result)
    }

    // v1.8: 处理带受限可见性的 enum (如 pub(crate) E)
    fn convert_enum_with_visibility(&self, line: &str) -> Result<String> {
        // 格式: "pub(crate) E Name" 或 "pub(super) E Name {...}"
        let e_pos = if let Some(pos) = line.find(") E ") {
            pos + 2
        } else if let Some(pos) = line.find(") e ") {
            pos + 2
        } else {
            return Err(anyhow::anyhow!("Invalid enum with visibility format"));
        };
        
        let visibility = &line[..e_pos]; // "pub(crate)"
        let rest = &line[e_pos + 2..]; // "Name {...}"
        let converted = self.convert_types_in_string(rest);
        
        Ok(format!("{} enum {}", visibility, converted))
    }

    fn convert_trait(&self, line: &str, _lines: &[&str], _index: &mut usize) -> Result<String> {
        let is_pub = line.starts_with("TR ");
        let content = &line[3..];

        let visibility = if is_pub { "pub " } else { "" };
        let converted = self.convert_types_in_string(content);

        Ok(format!("{}trait {}", visibility, converted))
    }

    // v1.8.3: 处理 unsafe trait
    fn convert_unsafe_trait(&self, line: &str, _lines: &[&str], _index: &mut usize) -> Result<String> {
        let is_pub = line.starts_with("unsafe TR ");
        let content = if is_pub { &line[10..] } else { &line[9..] }; // 跳过 "unsafe TR " 或 "unsafe tr "

        let visibility = if is_pub { "pub " } else { "" };
        let converted = self.convert_types_in_string(content);

        Ok(format!("{}unsafe trait {}", visibility, converted))
    }

    fn convert_impl(&self, line: &str, _lines: &[&str], _index: &mut usize) -> Result<String> {
        // Nu v1.6.3: 检查是否是 "U I" 模式（unsafe impl）
        // 注意：在convert_line中，"U I"会被识别为以"I "开头的行
        // 但我们需要在这里检查原始行是否有前导的"U "
        let content = &line[2..]; // 跳过 "I "
        let converted = self.convert_types_in_string(content);
        Ok(format!("impl {}", converted))
    }

    fn convert_module(&self, line: &str) -> Result<String> {
        // Nu v1.6.3: D=mod (私有)
        let content = &line[2..]; // 跳过 "D "
        let converted = self.convert_types_in_string(content);
        Ok(format!("mod {}", converted))
    }

    fn convert_pub_module(&self, line: &str) -> Result<String> {
        // Nu v1.6.3: DM=pub mod (公有)
        let content = &line[3..]; // 跳过 "DM "
        let converted = self.convert_types_in_string(content);
        Ok(format!("pub mod {}", converted))
    }

    fn convert_let(&self, line: &str) -> Result<String> {
        let content = &line[2..];
        // 先转换关键字，再转换类型
        let converted = self.convert_inline_keywords(content)?;
        let converted = self.convert_types_in_string(&converted);
        Ok(format!("let {}", converted))
    }

    fn convert_let_mut(&self, line: &str) -> Result<String> {
        let content = &line[2..]; // 跳过 "v "

        // 检查是否是元组解构赋值模式: v (a, b) = ... 或 v (a , b) = ...
        // 处理空格：移除content开头的空格，检查是否以 ( 开头
        let trimmed = content.trim_start();
        if trimmed.starts_with('(') {
            // 元组解构：v (a, b) = ... -> let mut (a, b) = ...
            // 注意：需要保留原始content，因为可能有前导空格需要处理
            let converted = self.convert_inline_keywords(content)?;
            let converted = self.convert_types_in_string(&converted);
            Ok(format!("let mut {}", converted))
        } else {
            // 普通变量：v name = ... -> let mut name = ...
            let converted = self.convert_inline_keywords(content)?;
            let converted = self.convert_types_in_string(&converted);
            Ok(format!("let mut {}", converted))
        }
    }

    fn convert_return(&self, line: &str) -> Result<String> {
        if line == "<" {
            Ok("return;".to_string())
        } else {
            let content = &line[2..];
            let converted = self.convert_types_in_string(content);
            // v1.8: return 语句需要分号结尾
            Ok(format!("return {};", converted.trim_end_matches(';')))
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
            } else if content.contains(" in ") || content.contains(" in(") || content.contains(" in[") || content.contains(": ")
            {
                // v1.7.6: 检查是否是for循环（L var in iter 或 L var: iter 或 L var in(...) 或 L var in[...] ）
                // v1.8.2: 添加 " in[" 模式支持数组字面量
                // 支持 " in " 和 " in(" 和 " in[" 模式
                let brace_pos = content.find('{');
                let in_space_pos = content.find(" in ");
                let in_paren_pos = content.find(" in(");
                let in_bracket_pos = content.find(" in["); // v1.8.2
                let colon_pos = content.find(": ");

                // 找到最早出现的 in 位置（空格、括号或方括号形式）
                let in_pos = [in_space_pos, in_paren_pos, in_bracket_pos]
                    .iter()
                    .filter_map(|&x| x)
                    .min();

                // 找到最早出现的分隔符位置
                let separator_pos = match (in_pos, colon_pos) {
                    (Some(in_p), Some(c_p)) => Some(in_p.min(c_p)),
                    (Some(in_p), None) => Some(in_p),
                    (None, Some(c_p)) => Some(c_p),
                    _ => None,
                };

                let is_for_loop = match (separator_pos, brace_pos) {
                    (Some(sep_p), Some(brace_p)) => sep_p < brace_p, // 分隔符在 { 之前
                    (Some(_), None) => true,                         // 有分隔符但没有 {
                    _ => false,
                };

                if is_for_loop {
                    // for 循环: L var in iter { ... } 或 L var: iter { ... } 或 L var in(...) { ... }
                    // 只替换for循环变量和迭代器之间的第一个 ": " 为 " in "
                    // 不能使用全局replace，否则会破坏循环体内的类型标注、结构体字段等
                    let converted_content = if content.contains(": ")
                        && !content.contains(" in ")
                        && !content.contains(" in(")
                    {
                        // 找到第一个 ": " 的位置
                        if let Some(colon_pos) = content.find(": ") {
                            // 只替换第一个 ": "
                            let (before, after) = content.split_at(colon_pos);
                            format!("{} in {}", before, &after[2..]) // 跳过 ": "
                        } else {
                            content.to_string()
                        }
                    } else if let Some(pos) = content.find(" in(") {
                        // v1.7.6: 处理 " in(" 模式，添加空格: " in(" -> " in ("
                        let (before, after) = content.split_at(pos + 3); // 包含 " in"
                        format!("{} {}", before, after) // 在 in 和 ( 之间添加空格
                    } else if let Some(pos) = content.find(" in[") {
                        // v1.8.2: 处理 " in[" 模式，添加空格: " in[" -> " in ["
                        let (before, after) = content.split_at(pos + 3); // 包含 " in"
                        format!("{} {}", before, after) // 在 in 和 [ 之间添加空格
                    } else {
                        content.to_string()
                    };
                    let converted = self.convert_inline_keywords(&converted_content)?;
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
        } else if let Some(content) = line.strip_prefix("L(") {
            // L(pattern) in iter { ... } - 带模式匹配的 for 循环
            // 例如: L(i,(word, count)) in freq.iter().take(top_n).enumerate()
            let converted = self.convert_inline_keywords(content)?;
            let converted = self.convert_types_in_string(&converted);
            Ok(format!("for ({}", converted))
        } else {
            Ok(line.to_string())
        }
    }

    fn convert_if(&self, line: &str) -> Result<String> {
        // v1.8.2: 处理 "? " 和 "?(" 两种模式
        let content = if line.starts_with("?(") {
            &line[1..] // 跳过 "?", 保留 "("
        } else {
            &line[2..] // 跳过 "? "
        };

        // v1.8.3: 先处理 "else ?" 模式，将其转换为 "else if"
        let content = content.replace("} else ? ", "} else if ");
        let content = content.replace("} else ?", "} else if ");

        // 检查是否是三元表达式: ? condition { value } else { value }
        // 这种情况下不应该添加 "if"，而是直接转换为 "if condition { value } else { value }"
        let trimmed = content.trim();
        if !trimmed.starts_with("let ") && !trimmed.starts_with("if ") {
            // 可能是三元表达式，检查是否有 { ... } else { ... } 模式
            if trimmed.contains('{') && trimmed.contains("} else {") {
                // 三元表达式，直接转换
                let converted = self.convert_inline_keywords(&content)?;
                let converted = self.convert_types_in_string(&converted);
                return Ok(format!("if {}", converted));
            }
        }

        // 递归处理if语句内容
        let converted = self.convert_inline_keywords(&content)?;
        let converted = self.convert_types_in_string(&converted);
        Ok(format!("if {}", converted))
    }

    fn convert_if_not(&self, line: &str) -> Result<String> {
        let content = &line[3..]; // 跳过 "?! "

        // 递归处理if not语句内容
        let converted = self.convert_inline_keywords(content)?;
        let converted = self.convert_types_in_string(&converted);
        Ok(format!("if !{}", converted))
    }

    fn convert_match(&self, line: &str) -> Result<String> {
        // v1.7.6: 支持 M( 模式（元组匹配）和 M 模式（普通匹配）
        let content = if line.starts_with("M(") {
            &line[1..] // 只跳过 "M"，保留 "(..."
        } else {
            &line[2..] // 跳过 "M "
        };

        // 检查是否是简单的 match 变量 (如 "u {" 或 "value {")
        // 这种情况下，变量名不应被转换为关键字
        let trimmed = content.trim();

        // 找到第一个 { 之前的内容
        let before_brace = if let Some(pos) = trimmed.find('{') {
            trimmed[..pos].trim()
        } else {
            trimmed
        };

        // 判断是否是简单标识符（只包含字母数字和下划线，没有空格或特殊字符）
        let is_simple_identifier = !before_brace.is_empty()
            && before_brace
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_')
            && !before_brace.contains("::");

        let converted = if is_simple_identifier {
            // 简单变量名，不转换关键字，只转换类型
            self.convert_types_in_string(content)
        } else {
            // 复杂表达式，需要正常转换
            let temp = self.convert_inline_keywords(content)?;
            self.convert_types_in_string(&temp)
        };

        Ok(format!("match {}", converted))
    }

    fn convert_print(&self, line: &str) -> Result<String> {
        let content = &line[2..];
        let converted = self.convert_types_in_string(content);
        Ok(format!("println!{}", converted))
    }

    fn convert_use(&self, line: &str) -> Result<String> {
        let is_pub = line.starts_with("U ");
        let mut content = line[2..].to_string();

        // 修复 use 语句中花括号内的空格格式
        // "use crate::{ a, b }" -> "use crate::{a, b}"
        if content.contains("{ ") || content.contains(" }") {
            content = content.replace("{ ", "{").replace(" }", "}");
        }

        // 修复逗号前的空格: "a , b" -> "a, b"
        content = content.replace(" ,", ",");

        let visibility = if is_pub { "pub " } else { "" };
        Ok(format!("{}use {}", visibility, content))
    }

    fn convert_const(&self, line: &str, is_pub: bool) -> Result<String> {
        let content = if is_pub { &line[3..] } else { &line[2..] };
        let converted = self.convert_types_in_string(content);
        let visibility = if is_pub { "pub " } else { "" };
        Ok(format!("{}const {}", visibility, converted))
    }

    fn convert_static(&self, line: &str, is_mut: bool, is_pub: bool) -> Result<String> {
        // Nu v1.6.3: ST = static, SM = static mut
        // v1.8: SP = pub static, SMP = pub static mut
        let content = if line.starts_with("ST ") {
            &line[3..]
        } else if line.starts_with("SP ") {
            &line[3..]
        } else if line.starts_with("SM ") {
            &line[3..]
        } else if line.starts_with("SMP ") {
            &line[4..]
        } else {
            &line[3..]
        };

        let converted = self.convert_types_in_string(content);
        let visibility = if is_pub { "pub " } else { "" };
        let mut_str = if is_mut { "mut " } else { "" };

        Ok(format!("{}static {}{}", visibility, mut_str, converted))
    }

    fn convert_unsafe_impl(
        &self,
        line: &str,
        _lines: &[&str],
        _index: &mut usize,
    ) -> Result<String> {
        // Nu v1.6.3: U I = unsafe impl (旧格式)
        let content = &line[4..]; // 跳过 "U I "
        let converted = self.convert_types_in_string(content);
        Ok(format!("unsafe impl {}", converted))
    }

    // v1.8: 处理新格式的 unsafe impl (unsafe I 代替 U I)
    fn convert_unsafe_impl_v18(
        &self,
        line: &str,
        _lines: &[&str],
        _index: &mut usize,
    ) -> Result<String> {
        // "unsafe I " 或 "unsafe I<"
        let content = if line.starts_with("unsafe I ") {
            &line[9..] // 跳过 "unsafe I "
        } else {
            &line[8..] // 跳过 "unsafe I" (紧跟 <)
        };
        let converted = self.convert_types_in_string(content);
        // 对于 "unsafe I<T>"，converted 以 "<" 开头，不需要额外空格
        // 对于 "unsafe I Trait"，converted 以字母开头，需要空格
        if converted.starts_with('<') || converted.starts_with(' ') {
            Ok(format!("unsafe impl{}", converted))
        } else {
            Ok(format!("unsafe impl {}", converted))
        }
    }

    fn convert_type_alias(&self, line: &str) -> Result<String> {
        let content = &line[2..];
        let mut converted = self.convert_types_in_string(content);
        // 修复类型别名中的格式问题
        // "Value < 'a > =" -> "Value<'a> ="
        converted = converted.replace(" < ", "<").replace(" >", ">");
        converted = converted.replace("= & ", "= &");
        Ok(format!("type {}", converted))
    }

    fn convert_unsafe_block(&self, line: &str) -> Result<String> {
        // U { ... } -> unsafe { ... }
        let content = &line[2..]; // 跳过 "U "
        let converted = self.convert_inline_keywords(content)?;
        let converted = self.convert_types_in_string(&converted);
        Ok(format!("unsafe {}", converted))
    }

    fn convert_expression(&self, line: &str) -> Result<String> {
        // v1.6.4 Hotfix: 结构体字段默认pub（基于状态机精确检测）
        // 此函数会被convert_line调用，但没有context参数
        // 因此需要在convert_line中处理字段的pub前缀

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
            // v1.8.2: 跳过字符串字面量，不对其中的内容进行转换
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

            // break: br 或 br; 或 br, (使用双字母避免与变量b冲突)
            if remaining.starts_with("br;")
                || remaining.starts_with("br,")
                || remaining.starts_with("br ")
                || remaining == "br"
            {
                let is_start_boundary =
                    i == 0 || (!chars[i - 1].is_alphanumeric() && chars[i - 1] != '_');
                if is_start_boundary {
                    if remaining.starts_with("br;") {
                        result.push_str("break;");
                        i += 3;
                    } else if remaining.starts_with("br,") {
                        result.push_str("break,");
                        i += 3;
                    } else if remaining.starts_with("br ") || remaining == "br" {
                        result.push_str("break");
                        i += 2;
                    }
                    continue;
                }
            }

            // continue: ct 或 ct; 或 ct, (使用双字母避免与变量c冲突)
            if remaining.starts_with("ct;")
                || remaining.starts_with("ct,")
                || remaining.starts_with("ct ")
                || remaining == "ct"
            {
                let is_start_boundary =
                    i == 0 || (!chars[i - 1].is_alphanumeric() && chars[i - 1] != '_');
                if is_start_boundary {
                    if remaining.starts_with("ct;") {
                        result.push_str("continue;");
                        i += 3;
                    } else if remaining.starts_with("ct,") {
                        result.push_str("continue,");
                        i += 3;
                    } else if remaining.starts_with("ct ") || remaining == "ct" {
                        result.push_str("continue");
                        i += 2;
                    }
                    continue;
                }
            }

            // if not: ?! (必须在 ? 之前检查)
            // 但要确保不会误匹配 "? expr!=" 中的 "?!"
            if i + 1 < chars.len() && chars[i] == '?' && chars[i + 1] == '!' {
                // 检查是否是 "?! " 模式（if not语句）
                // 必须是 "?! " 且 ! 后面紧跟空格，或者是 "?!" 后面紧跟非 = 的字符
                let is_if_not = if i + 2 < chars.len() {
                    chars[i + 2] == ' ' || chars[i + 2] != '='
                } else {
                    true // 行尾的 "?!"
                };

                if is_if_not {
                    result.push_str("if !");
                    i += 2; // 跳过 "?!"
                            // 如果后面是空格，也跳过
                    if i < chars.len() && chars[i] == ' ' {
                        i += 1;
                    }
                    continue;
                }
            }

            // v1.8.2: 错误传播运算符 ! -> ? 和 !! -> ??
            // 需要区分：
            // 1. 后缀 ! (错误传播): expr!
            // 2. 后缀 !! (双错误传播): expr!!
            // 3. 前缀 ! (mut): &!Type, *!ptr
            // 4. 不等于: !=
            if chars[i] == '!' {
                // 检查是否是 !!
                if i + 1 < chars.len() && chars[i+1] == '!' {
                    // 检查是否是 postfix !!
                    let is_postfix = i > 0 && (chars[i-1].is_alphanumeric() || chars[i-1] == ')' || chars[i-1] == ']' || chars[i-1] == '}');
                    if is_postfix {
                        result.push_str("??");
                        i += 2;
                        continue;
                    }
                }
                
                // 检查是否是 postfix !
                let is_postfix = i > 0 && (chars[i-1].is_alphanumeric() || chars[i-1] == ')' || chars[i-1] == ']' || chars[i-1] == '}');
                let is_not_equal = i + 1 < chars.len() && chars[i+1] == '=';
                
                if is_postfix && !is_not_equal {
                    // v1.8.2: 排除宏调用，防止被误转为 ? (宏调用通常后跟 ( [ { 或者其名称是 macro_rules)
                    let is_macro_rules = if i >= 11 {
                        let prev_11: String = chars[i - 11..i].iter().collect();
                        prev_11 == "macro_rules"
                    } else {
                        false
                    };
                    
                    // v1.8.3: 排除 if!, else!, while!, thread_local! 等关键字后的 !
                    let is_keyword_macro = {
                        let is_if = i >= 2 && {
                            let prev_2: String = chars[i-2..i].iter().collect();
                            prev_2 == "if"
                        };
                        let is_else = i >= 4 && {
                            let prev_4: String = chars[i-4..i].iter().collect();
                            prev_4 == "else"
                        };
                        let is_while = i >= 5 && {
                            let prev_5: String = chars[i-5..i].iter().collect();
                            prev_5 == "while"
                        };
                        let is_thread_local = i >= 12 && {
                            let prev_12: String = chars[i-12..i].iter().collect();
                            prev_12 == "thread_local"
                        };
                        is_if || is_else || is_while || is_thread_local
                    };

                    // v1.8.3: 改进宏调用检测，支持 ! 后有空格的情况 (如 quote! { ... })
                    let is_macro_call = if i + 1 < chars.len() {
                        let next_char = chars[i + 1];
                        if next_char == '(' || next_char == '[' || next_char == '{' {
                            true
                        } else if next_char == ' ' {
                            // 检查空格后是否是 { [ (
                            let mut j = i + 2;
                            while j < chars.len() && chars[j] == ' ' {
                                j += 1;
                            }
                            j < chars.len() && (chars[j] == '(' || chars[j] == '[' || chars[j] == '{')
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if !is_macro_rules && !is_keyword_macro && !is_macro_call {
                        result.push('?');
                        i += 1;
                        continue;
                    }
                }
            }

            // if: ? (检查是否是三元表达式或模式守卫的开始)
            // 但要避免：
            // 1. 在宏规则中转换（宏规则中 ? 是可选项标记）
            // 2. 转换 ? Sized（trait约束）
            // 3. 错误传播运算符 expr? （后面跟 ; , ) } 等）
            if chars[i] == '?' {
                // 首先检查是否是错误传播运算符
                // 需要检测: ?; ?, ?) ?} ?. 以及 ? . (空格后的点)
                let mut is_error_propagation = false;
                if i + 1 < chars.len() {
                    let next_char = chars[i + 1];
                    if next_char == ';'
                        || next_char == ','
                        || next_char == ')'
                        || next_char == '}'
                        || next_char == '.'
                    {
                        is_error_propagation = true;
                    } else if next_char == ' ' {
                        // v1.6.11: 检查空格后是否是 . , ; ) } (错误传播运算符: ? . 或 ? , 等)
                        let mut j = i + 2;
                        while j < chars.len() && chars[j] == ' ' {
                            j += 1;
                        }
                        if j < chars.len() {
                            let char_after_spaces = chars[j];
                            if char_after_spaces == '.'
                                || char_after_spaces == ','
                                || char_after_spaces == ';'
                                || char_after_spaces == ')'
                                || char_after_spaces == '}'
                            {
                                is_error_propagation = true;
                            }
                        }
                    }
                }

                if is_error_propagation {
                    // 保持原样，这是错误传播运算符
                    result.push(chars[i]);
                    i += 1;
                    continue;
                }

                // 只有后面有空格时才考虑转换为if
                if i + 1 < chars.len() && chars[i + 1] == ' ' {
                    // 检查是否是 ? Sized 模式
                    let mut is_sized_trait = false;
                    if i + 7 < chars.len() {
                        let next_6: String = chars[i + 2..i + 8].iter().collect();
                        if next_6 == "Sized "
                            || next_6 == "Sized+"
                            || next_6 == "Sized,"
                            || next_6 == "Sized>"
                        {
                            is_sized_trait = true;
                        }
                    }

                    // 检查后面是否紧跟 $（宏变量），如果是则不转换
                    let mut is_macro_optional = false;
                    if i + 2 < chars.len() && chars[i + 2] == '$' {
                        is_macro_optional = true;
                    }

                    // 检查是否在宏规则上下文中
                    let mut is_in_macro = false;
                    if !is_macro_optional {
                        // v1.8: 检查整行是否包含宏模式特征
                        // 宏规则行通常包含 $( 和 $... 模式
                        let full_line: String = chars.iter().collect();
                        let has_macro_patterns = full_line.contains("$(") 
                            || full_line.contains("$stack") 
                            || full_line.contains("$bail")
                            || full_line.contains("$fuel")
                            || full_line.contains("$rest")
                            || full_line.contains("$parse")
                            || full_line.contains("$buf")
                            || (full_line.matches('$').count() > 3);
                        
                        if has_macro_patterns {
                            // 进一步检查前面的非空白字符
                            let mut j = i;
                            while j > 0 {
                                j -= 1;
                                if !chars[j].is_whitespace() {
                                    // v1.8: 在宏规则中，) ? 或 * ? 或 + ? 或 ] ? 或 ? ? 是可选项标记
                                    is_in_macro = chars[j] == ')'
                                        || chars[j] == '*'
                                        || chars[j] == '+'
                                        || chars[j] == ']'
                                        || chars[j] == '?';  // v1.8: 处理 )?? 模式
                                    break;
                                }
                            }
                        }
                    }

                    // v1.8.2: 检查是否是 expr? { ... } 模式（条件执行块）
                    // 如果 ? 后面紧跟 { 或 空格+{，这是条件执行块，不是 if 语句
                    let mut is_conditional_block = false;
                    if i + 2 < chars.len() {
                        if chars[i + 2] == '{' {
                            is_conditional_block = true;
                        } else if chars[i + 2].is_whitespace() {
                            // 继续检查空格后是否有 {
                            let mut k = i + 2;
                            while k < chars.len() && chars[k].is_whitespace() {
                                k += 1;
                            }
                            if k < chars.len() && chars[k] == '{' {
                                is_conditional_block = true;
                            }
                        }
                    }

                    if !is_in_macro && !is_macro_optional && !is_sized_trait && !is_conditional_block {
                        result.push_str("if ");
                        i += 2;
                        continue;
                    }
                }
            }

            // match: M (需要确保不是其他标识符的一部分，也不在泛型参数中)
            // v1.6.8: 只处理 "M " 和特定的 "M&" 模式（match &expr）
            // v1.7.4: 增强where子句保护，避免误转换泛型参数
            // v1.7.6: 添加 M( 模式支持用于元组匹配
            // 单字母M很可能是泛型参数，需要非常保守地转换
            if remaining.starts_with("M ")
                || remaining.starts_with("M&")
                || remaining.starts_with("M(")
            {
                let has_start_boundary =
                    i == 0 || (!chars[i - 1].is_alphanumeric() && chars[i - 1] != '_');

                // v1.7.4: 检查是否在where子句中（向前查找 "wh " 或 "where "）
                let mut is_in_where_clause = false;
                if i >= 3 {
                    let context_start = if i > 50 { i - 50 } else { 0 };
                    let context: String = chars[context_start..i].iter().collect();
                    // 检查最近是否有 "wh " 或 "where "，且之后没有 { 或 ;
                    if let Some(wh_pos) = context.rfind("wh ") {
                        let after_wh = &context[wh_pos + 3..];
                        if !after_wh.contains('{') && !after_wh.contains(';') {
                            is_in_where_clause = true;
                        }
                    } else if let Some(where_pos) = context.rfind("where ") {
                        let after_where = &context[where_pos + 6..];
                        if !after_where.contains('{') && !after_where.contains(';') {
                            is_in_where_clause = true;
                        }
                    }
                }

                // 检查前面的上下文，避免在泛型/类型位置转换
                let mut is_in_generic_or_type = false;
                if i > 0 {
                    // 前面是 < , : 表示在泛型/类型上下文中
                    if chars[i - 1] == '<' || chars[i - 1] == ',' || chars[i - 1] == ':' {
                        is_in_generic_or_type = true;
                        
                        // v1.8.2: 特殊处理函数调用中的 match。
                        // 如果 M 后面紧跟着一个标识符和 {，那它极度可能是 match 而不是泛型参数 M。
                        // 例如 Self::new(vec![], M rule { ... })
                        let mut j = i + 1;
                        if remaining.starts_with("M ") { j = i + 2; }
                        else if remaining.starts_with("M&") { j = i + 2; }
                        else if remaining.starts_with("M(") { j = i + 1; }
                        
                        while j < chars.len() && chars[j].is_whitespace() { j += 1; }
                        let word_start = j;
                        while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') { j += 1; }
                        if j > word_start {
                            while j < chars.len() && chars[j].is_whitespace() { j += 1; }
                            if j < chars.len() && chars[j] == '{' {
                                is_in_generic_or_type = false;
                            }
                        }
                    } else if chars[i - 1].is_whitespace() {
                        // 如果前面是空格，向前查找最近的非空格字符
                        let mut j = i - 1;
                        while j > 0 && chars[j].is_whitespace() {
                            j -= 1;
                        }
                        // 检查空格前的字符是否是泛型分隔符
                        if chars[j] == '<' || chars[j] == ',' || chars[j] == ':' {
                            is_in_generic_or_type = true;
                            
                            // v1.8.2: 同样处理空格后的 match
                            let mut k = i + 1;
                            if remaining.starts_with("M ") { k = i + 2; }
                            else if remaining.starts_with("M&") { k = i + 2; }
                            else if remaining.starts_with("M(") { k = i + 1; }
                            
                            while k < chars.len() && chars[k].is_whitespace() { k += 1; }
                            let word_start = k;
                            while k < chars.len() && (chars[k].is_alphanumeric() || chars[k] == '_') { k += 1; }
                            if k > word_start {
                                while k < chars.len() && chars[k].is_whitespace() { k += 1; }
                                if k < chars.len() && chars[k] == '{' {
                                    is_in_generic_or_type = false;
                                }
                            }
                        }
                    }
                }

                // 检查后面的上下文，避免在泛型位置转换（如 M >）
                if !is_in_generic_or_type && i + 2 < chars.len() {
                    let next_char = chars[i + 1]; // M后第一个字符
                    if next_char == ' ' && i + 2 < chars.len() {
                        let char_after_space = chars[i + 2];
                        // M > 或 M , 或 M ) 表示泛型参数
                        if char_after_space == '>'
                            || char_after_space == ','
                            || char_after_space == ')'
                        {
                            is_in_generic_or_type = true;
                        }
                    }
                }

                // 只在以下情况转换为match：
                // 1. 有边界
                // 2. 后面跟空格（M ）或&（M&，match引用）或(（M(，元组匹配）
                // 3. 不在泛型/类型上下文中
                // 4. 不在where子句中
                if has_start_boundary && !is_in_generic_or_type && !is_in_where_clause {
                    if remaining.starts_with("M ") {
                        result.push_str("match ");
                        i += 2;
                        continue;
                    } else if remaining.starts_with("M&") {
                        // 特殊处理 M& -> match &
                        result.push_str("match &");
                        i += 2;
                        continue;
                    } else if remaining.starts_with("M(") {
                        // v1.7.6: 特殊处理 M( -> match (
                        result.push_str("match ");
                        i += 1; // 只跳过 M，保留 (
                        continue;
                    }
                }
            }

            // v1.6.11: 修复 Rust2Nu 错误转换的 if, if; if) if} -> ?, ?; ?) ?}
            // 当 Rust2Nu 错误地将 ? 转换为 if 时，会产生 if, if; 等非法语法
            // 这里检测并修复这些模式
            if remaining.starts_with("if,")
                || remaining.starts_with("if;")
                || remaining.starts_with("if)")
                || remaining.starts_with("if}")
                || remaining.starts_with("if.")
            {
                // 检查前面是否是表达式结尾（闭括号、宏调用等）
                // 需要跳过空格向前查找
                let mut should_convert = false;
                let mut j = i;
                while j > 0 {
                    j -= 1;
                    if !chars[j].is_whitespace() {
                        // ) ! } ] 后面的 if, 应该是错误传播的 ?
                        if chars[j] == ')' || chars[j] == '!' || chars[j] == '}' || chars[j] == ']'
                        {
                            should_convert = true;
                        }
                        break;
                    }
                }

                if should_convert {
                    result.push('?');
                    i += 2; // 跳过 "if"
                    continue;
                }
            }

            // use: u or U (for inline use statements like in cfg blocks)
            // 但要避免转换表达式中的变量名 u
            // 还要避免转换泛型参数 U（单字母大写通常是泛型）
            if remaining.starts_with("u ") || remaining.starts_with("U ") {
                // 检查是否在关键字之后（match, if, for, while等）
                // 如果在这些关键字后，u 应该是变量名而不是 use
                let mut is_after_keyword = false;
                if i >= 6 {
                    let prev_6: String = chars[i - 6..i].iter().collect();
                    if prev_6 == "match " || prev_6.ends_with("match ") {
                        is_after_keyword = true;
                    }
                }
                if i >= 3 && !is_after_keyword {
                    let prev_3: String = chars[i - 3..i].iter().collect();
                    if prev_3 == "if " || prev_3.ends_with("if ") {
                        is_after_keyword = true;
                    }
                }
                if i >= 4 && !is_after_keyword {
                    let prev_4: String = chars[i - 4..i].iter().collect();
                    if prev_4 == "for " || prev_4.ends_with("for ") {
                        is_after_keyword = true;
                    }
                }

                // 检查是否在泛型/类型位置（U 很可能是泛型参数）
                let mut is_generic_param = false;
                if remaining.starts_with("U ") && i > 0 {
                    // 检查前面的字符
                    let mut j = i - 1;
                    while j > 0 && chars[j].is_whitespace() {
                        j -= 1;
                    }
                    // 前面是 < , : -> ( 表示在泛型/类型位置
                    if chars[j] == '<'
                        || chars[j] == ','
                        || chars[j] == ':'
                        || chars[j] == '>'
                        || chars[j] == '('
                    {
                        is_generic_param = true;
                    }
                    // v1.8.3: 检查是否在 *mut 或 *const 后面（指针类型）
                    if !is_generic_param && result.ends_with("*mut ") {
                        is_generic_param = true;
                    }
                    if !is_generic_param && result.ends_with("*const ") {
                        is_generic_param = true;
                    }
                    // 检查后面的字符
                    if !is_generic_param && i + 2 < chars.len() {
                        let char_after_space = chars[i + 2];
                        // U > 或 U , 或 U ) 或 U : 表示泛型参数
                        if char_after_space == '>'
                            || char_after_space == ','
                            || char_after_space == ')'
                            || char_after_space == ':'
                        {
                            is_generic_param = true;
                        }
                    }
                }

                if !is_after_keyword && !is_generic_param {
                    let has_start_boundary =
                        i == 0 || (!chars[i - 1].is_alphanumeric() && chars[i - 1] != '_');
                    if has_start_boundary {
                        // v1.8: U = pub use, u = use
                        // 但如果前面已经有 pub(crate) / pub(super) 等可见性，不要再加 pub
                        let already_has_visibility = result.ends_with(") ");
                        if remaining.starts_with("U ") && !already_has_visibility {
                            result.push_str("pub use ");
                        } else {
                            result.push_str("use ");
                        }
                        i += 2;
                        continue;
                    }
                }
            }

            // mod: D (for inline mod statements)
            // 需要避免：
            // 1. 在关键字后转换
            // 2. 在泛型参数/类型位置转换（如 &mut D, Option<D>）
            // 3. 单字符大写标识符很可能是泛型参数（D, T, E等），不应转换
            // 关键修复：只转换 "D " 开头且确实是mod语句的情况
            // 单字符泛型参数（D, T, E, R等）绝对不应该被转换
            // v1.8.2: 进一步增强保护，检查是否在行首（允许前面的空白）
            // 只有在行首的 "D " 才是 mod declaration
            if remaining.starts_with("D ")
                || remaining.starts_with("D>")
                || remaining.starts_with("D,")
                || remaining.starts_with("D)")
            {
                let mut is_generic_param = false;
                
                // 检查是否在行首（允许空白）
                let is_start_of_line = i == 0 || result[..i].trim().is_empty() || result[..i].ends_with(';') || result[..i].ends_with('{');
                
                if !is_start_of_line && (remaining.starts_with("D>") || remaining.starts_with("D,") || remaining.starts_with("D)")) {
                    // 泛型参数 D
                    is_generic_param = true;
                }
                
                // 检查是否是单独的 "D " 模式（后面跟空格）
                let is_d_space = remaining.starts_with("D ");

                // 检查后面的字符，如果是 >, , ) 则必定是泛型参数
                if !is_start_of_line && is_d_space && i + 2 < chars.len() {
                    let next_char = chars[i + 2];
                    if next_char == '>' || next_char == ',' || next_char == ')' {
                        is_generic_param = true;
                    }
                }

                if is_d_space && !is_generic_param {
                    // Double check: if it's "D Name", it's mod
                    // If it's "D :", it's generic param
                    let mut j = i + 2;
                    while j < chars.len() && chars[j].is_whitespace() {
                        j += 1;
                    }
                    if j < chars.len() && (chars[j] == ':' || chars[j] == '=') {
                         is_generic_param = true;
                    } else if is_start_of_line { 
                         // v1.8.2: Ensure D is followed by an identifier (not implicit type D)
                         // 如果后面没有字符，或者是 EOF，则可能是 Type D
                         if j < chars.len() && (chars[j].is_alphabetic() || chars[j] == '_') {
                             result.push_str("mod ");
                             i += 2;
                             continue;
                         }
                    }
                }
                
                let next_char_is_generic_delimiter = remaining.starts_with("D>")
                    || remaining.starts_with("D,")
                    || remaining.starts_with("D)");

                if next_char_is_generic_delimiter {
                    // 确定是泛型参数，直接复制不转换
                    result.push(chars[i]);
                    i += 1;
                    continue;
                }

                // 只处理 "D " 的情况
                if !is_d_space {
                    result.push(chars[i]);
                    i += 1;
                    continue;
                }

                let mut is_after_keyword = false;
                    if i >= 6 {
                        let prev_6: String = chars[i - 6..i].iter().collect();
                        if prev_6 == "match " || prev_6.ends_with("match ") {
                            is_after_keyword = true;
                        }
                    }

                    // 检查是否在类型位置：前面是 mut、<、,、:、!、& 等
                    let mut is_in_type_position = false;
                if i > 0 {
                    // 关键修复：直接检查D前面的字符，不跳过空格
                    // 因为 "!D" 或 "! D" 都应该被识别为类型位置
                    let j = i - 1;

                    // 首先检查紧邻的前一个字符
                    if chars[j] == '!'
                        || chars[j] == '&'
                        || chars[j] == '<'
                        || chars[j] == '>'
                        || chars[j] == ','
                        || chars[j] == ':'
                        || chars[j].is_alphanumeric()  // v1.8.2: 生命周期名结束（如 'a 中的 a）
                    {
                        is_in_type_position = true;
                    } else if chars[j].is_whitespace() {
                        // 如果前面是空格，向前查找最近的非空格字符
                        let mut k = j;
                        while k > 0 && chars[k].is_whitespace() {
                            k -= 1;
                        }
                        if k < chars.len() {
                            // 检查空格前的字符
                            if chars[k] == '!'
                                || chars[k] == '&'
                                || chars[k] == '<'
                                || chars[k] == '>'
                                || chars[k] == ','
                                || chars[k] == ':'
                                || chars[k].is_alphanumeric()  // v1.8.2: 生命周期名结束
                            {
                                is_in_type_position = true;
                            } else {
                                // 检查是否是 "mut" 关键字
                                let prev_chars: String = if k >= 2 {
                                    chars[k - 2..=k].iter().collect()
                                } else {
                                    chars[0..=k].iter().collect()
                                };
                                is_in_type_position = prev_chars.ends_with("mut");
                            }
                        }
                    } else {
                        // 检查是否是 "mut" 等关键字的结尾
                        let prev_chars: String = if j >= 2 {
                            chars[j - 2..=j].iter().collect()
                        } else {
                            chars[0..=j].iter().collect()
                        };
                        is_in_type_position = prev_chars.ends_with("mut");
                    }
                    }

                    // 关键修复：检查后面是否紧跟逗号、>、)、空格等，这些都表示D是泛型参数
                    if i + 1 < chars.len() {
                        let next_char = chars[i + 1]; // D后面的字符
                        // 如果后面跟着空格，再检查空格后的字符
                        if next_char == ' ' && i + 2 < chars.len() {
                            let char_after_space = chars[i + 2];
                            is_generic_param = char_after_space == '>'
                                || char_after_space == ','
                                || char_after_space == ')';
                        }
                    }

                    if !is_after_keyword && !is_in_type_position && !is_generic_param {
                        let has_start_boundary =
                            i == 0 || (!chars[i - 1].is_alphanumeric() && chars[i - 1] != '_');
                        if has_start_boundary {
                            result.push_str("mod ");
                            i += 2;
                            continue;
                        }
                    }
                }

            // loop: L { (必须在 for 之前检查)
            // v1.8: 添加边界检查，避免替换 MAX_OL 中的 L
            if remaining.starts_with("L {") || remaining.starts_with("L ") {
                let is_start_boundary =
                    i == 0 || (!chars[i - 1].is_alphanumeric() && chars[i - 1] != '_');
                
                if is_start_boundary {
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
                            // v1.8.2: 支持 " in " 和 " in[" 和 " in(" 模式
                            if j + 2 < chars.len()
                                && chars[j] == ' '
                                && chars[j + 1] == 'i'
                                && chars[j + 2] == 'n'
                            {
                                // 检查 in 后面是空格、[ 或 (
                                if j + 3 < chars.len() {
                                    let next_char = chars[j + 3];
                                    if next_char == ' ' || next_char == '[' || next_char == '(' {
                                        found_in = true;
                                        break;
                                    }
                                }
                            }
                            j += 1;
                        }

                        if found_in {
                            result.push_str("for ");
                            i += 2;
                            continue;
                        }
                    }
                }
            }

            // 默认情况：复制字符
            result.push(chars[i]);
            i += 1;
        }

        Ok(result)
    }

    /// v1.8.1: 带边界检查的类型替换辅助函数
    /// 只在单字母类型缩写前后是非字母数字时才替换
    /// 例如: "R <" -> "Result<" 但 "YEAR <" 保持不变
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
        // 注意：只转换明确的类型缩写（如Vec<），不转换可能是泛型参数的单字母（如 V ）
        result = result
            // 自定义类型名称修复（优先级最高，防止被标准库替换影响）
            .replace("MyV ", "MyVec ")
            .replace("MyV<", "MyVec<")
            .replace("MyV{", "MyVec{")
            .replace("MyV(", "MyVec(")
            .replace("MyV::", "MyVec::")
            .replace("MyB ", "MyBox ")
            .replace("MyB<", "MyBox<")
            .replace("MyB{", "MyBox{")
            .replace("MyB(", "MyBox(")
            .replace("MyB::", "MyBox::")
            .replace("VecDeque<", "VecDeque<") // 保持VecDeque<不变
            .replace("VecDeque::", "VecDeque::") // 保持VecDeque::不变
            .replace("VDeque :: ", "VecDeque::") // VDeque :: -> VecDeque:: (带空格，优先处理)
            .replace("VDeque::", "VecDeque::") // VDeque:: -> VecDeque:: (优先处理，避免被V::转换)
            .replace("VDeque <", "VecDeque<") // VDeque < -> VecDeque< (带空格)
            .replace("VDeque<", "VecDeque<") // VDeque< -> VecDeque< (rust2nu错误缩写)
            .replace("Vec<", "Vec<") // 保持Vec<不变，它已经是完整的
            .replace("V :: ", "Vec::") // V :: -> Vec:: (Vec的关联函数，带空格)
            .replace("V::", "Vec::"); // V:: -> Vec:: (Vec的关联函数，无空格)
        
        // v1.8.1: 使用边界检查的替换，避免 YEAR < 被替换成 YEAResult<
        result = Self::replace_type_with_boundary(&result, "V <", "Vec<");
        result = Self::replace_type_with_boundary(&result, "V<", "Vec<");
        result = Self::replace_type_with_boundary(&result, "O <", "Option<");
        result = Self::replace_type_with_boundary(&result, "O<", "Option<");
        result = Self::replace_type_with_boundary(&result, "O::", "Option::");
        result = Self::replace_type_with_boundary(&result, "R <", "Result<");
        result = Self::replace_type_with_boundary(&result, "R<", "Result<");
        result = Self::replace_type_with_boundary(&result, "R::", "Result::");
        result = result
            .replace("::R ", "::Result ") // ::R  -> ::Result (模块路径后的Result，带空格)
            .replace("::R<", "::Result<") // ::R< -> ::Result< (模块路径后的Result泛型)
            // v1.8.3: 处理 io::R< 模式（io模块的Result类型）
            .replace("io::R<", "io::Result<")
            .replace("BedError", "BoxedError") // Bed -> Boxed (类型缩写，anyhow库)
            .replace("B :: ", "Box::") // B :: -> Box:: (Box的关联函数，带空格)
            .replace("B::", "Box::"); // B:: -> Box:: (Box的关联函数，无空格) - 修复regex库错误
        
        // v1.8.1: A/X/B 也使用边界检查
        result = Self::replace_type_with_boundary(&result, "A<", "Arc<");
        result = Self::replace_type_with_boundary(&result, "A::", "Arc::");
        result = Self::replace_type_with_boundary(&result, "X<", "Mutex<");
        result = Self::replace_type_with_boundary(&result, "X::", "Mutex::");
        result = Self::replace_type_with_boundary(&result, "B <", "Box<");
        result = Self::replace_type_with_boundary(&result, "B<", "Box<");
        result = result
            // v1.8.3: 先处理 .~! -> .await? (await with try operator)
            .replace(".~!", ".await?")
            .replace(".~", ".await")
            .replace("$|", "move |")
            .replace(" wh ", " where ")
            .replace("wh ", "where ")
            .replace(" I<", " impl<")
            .replace("I<", "impl<")
            .replace(" U I<", " unsafe impl<")
            .replace("\nU I<", "\nunsafe impl<")
            // v1.7.5: 智能替换 )! -> )? 但保留 )!= (不等于操作符)
            // 不能使用简单的 .replace(")!", ")?") 因为它会把 )!= 也替换成 )?=
            .replace(")!!;", ")??;") // v1.8.2: 双错误传播 )!! -> )??
            .replace(")!!)", "))??") // v1.8.2: 双错误传播 )!!) -> )??
            .replace(")!!", ")??") // v1.8.2: 双错误传播 )!! -> )?? (通用)
            .replace(")!;", ")?;")
            .replace(")!,", ")?,")
            .replace(")!)", ")?)")
            .replace(")!}", ")?}")
            .replace(")!.", ")?.") // 链式调用 foo()!.bar()
            .replace(")! ", ")? ") // )! 后跟空格但不是 !=
            .replace("? Sized", "?Sized"); // 修复 ?Sized trait约束（Nu中为 "? Sized"，还原为 "?Sized"）

        // v1.6.7: 智能处理 &! -> &mut 和 *! -> *mut
        // 1. &!Type -> &mut Type
        // 2. &'a!Type -> &'a mut Type
        // 3. &'a !Type -> &'a mut Type (关键：处理空格)
        // 4. *! Type -> *mut Type
        // 5. * const Type -> *const Type (保持不变)
        let chars: Vec<char> = result.chars().collect();
        let mut i = 0;
        let mut new_result = String::new();

        while i < chars.len() {
            if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '!' {
                // 找到 *! 模式 -> *mut
                new_result.push_str("*mut");
                i += 2;
                // 跳过后面的空格
                while i < chars.len() && chars[i].is_whitespace() {
                    i += 1;
                }
                // 确保类型名前有空格
                if i < chars.len() && !chars[i].is_whitespace() {
                    new_result.push(' ');
                }
            } else if i + 1 < chars.len() && chars[i] == '&' && chars[i + 1] == '!' {
                // v1.7.6: 检查是否是 &&! 模式（逻辑与 + 逻辑非），不应转换为 &&mut
                // 只有单独的 &! 模式才转换为 &mut
                let prev_is_ampersand = i > 0 && chars[i - 1] == '&';
                
                // v1.8.3: 改进按位与非检测，排除关键字和运算符后的 &!
                // 如果 &! 前面是 identifier, ), ]，则可能是按位运算
                // 但如果前面是关键字 (in, =, (, {, [, ,, ;, :, for) 则是引用
                let mut k = i;
                if k > 0 { k -= 1; }
                while k > 0 && chars[k].is_whitespace() {
                    k -= 1;
                }
                let prev_char = chars[k];
                
                // v1.8.3: 检查前面是否是关键字或运算符（这些后面的 &! 应该是 &mut）
                let is_after_keyword_or_operator = prev_char == '=' 
                    || prev_char == '(' 
                    || prev_char == '{' 
                    || prev_char == '[' 
                    || prev_char == ',' 
                    || prev_char == ';' 
                    || prev_char == ':'
                    || prev_char == '>';  // v1.8.3: 添加 > 用于泛型约束后的 &!
                
                // v1.8.3: 检查前面是否是 "in" 或 "for" 关键字
                let is_after_in_keyword = if k >= 1 {
                    let prev_word: String = chars[k.saturating_sub(1)..=k].iter().collect();
                    prev_word == "in"
                } else {
                    false
                };
                
                // v1.8.3: 检查前面是否是 "for" 关键字
                let is_after_for_keyword = if k >= 2 {
                    let prev_word: String = chars[k.saturating_sub(2)..=k].iter().collect();
                    prev_word == "for"
                } else {
                    false
                };
                
                // 如果 prev_char 是空格（k=0且是空格），则说明开始就是空格
                let is_bitwise_and_not = !prev_char.is_whitespace() 
                    && !is_after_keyword_or_operator 
                    && !is_after_in_keyword
                    && !is_after_for_keyword
                    && (prev_char.is_alphanumeric() || prev_char == ')' || prev_char == ']' || prev_char == '_' || prev_char == '}');

                if prev_is_ampersand || is_bitwise_and_not {
                    // &&! 是逻辑运算符，或者 expr & !expr 是按位运算，保持原样(但插入空格使语义清晰)
                    new_result.push_str("& !");
                    i += 2;
                } else {
                    // 找到 &! 模式 -> &mut (只有在类型位置或引用位置)
                    new_result.push_str("&mut ");
                    i += 2;
                    // 跳过后面的空格
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                }
            } else if i + 3 < chars.len() && chars[i] == '&' && chars[i + 1] == '\'' {
                // 检查 &'a! 或 &'a ! 模式
                let mut j = i + 2;
                // 找到生命周期名称的结束
                while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                    j += 1;
                }
                // 检查生命周期后是否有 ! 或 空格+!
                let has_exclaim = if j < chars.len() && chars[j] == '!' {
                    true
                } else if j < chars.len() && chars[j].is_whitespace() {
                    // 跳过空格查找 !
                    let mut k = j;
                    while k < chars.len() && chars[k].is_whitespace() {
                        k += 1;
                    }
                    if k < chars.len() && chars[k] == '!' {
                        j = k; // 更新j指向!的位置
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };

                if has_exclaim {
                    // 找到 &'lifetime! 或 &'lifetime ! 模式
                    new_result.push('&');
                    new_result.push('\'');
                    for k in i + 2..j {
                        if !chars[k].is_whitespace() {
                            // 只复制非空格字符（生命周期名）
                            new_result.push(chars[k]);
                        }
                    }
                    new_result.push_str(" mut "); // 添加空格后缀
                    i = j + 1; // 跳过!
                               // 跳过!后面的空格
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                } else {
                    new_result.push(chars[i]);
                    i += 1;
                }
            } else {
                new_result.push(chars[i]);
                i += 1;
            }
        }
        result = new_result;

        // V! -> vec! (宏调用)
        result = result.replace("V![", "vec![").replace("V! [", "vec![");

        // v1.8.3: 恢复闭包参数，但先转换其中的类型缩写
        for (idx, closure) in protected_closures.iter().enumerate() {
            // 转换闭包中的类型缩写
            let converted_closure = closure
                .replace("io::R<", "io::Result<")
                .replace("R<", "Result<")
                .replace("O<", "Option<")
                .replace("V<", "Vec<")
                .replace("A<", "Arc<")
                .replace("X<", "Mutex<")
                .replace("B<", "Box<");
            result = result.replace(&format!("__CLOSURE_PARAMS_{}__", idx), &converted_closure);
        }

        // 移除函数调用中的多余空格，但保留方法调用: ") (" -> ")("
        // 注意：不要移除 "变量 . 方法" 模式中的空格
        result = result.replace(") (", ")(");

        // 移除方法调用中的多余空格: " . " -> "."
        // 但要小心不要破坏其他模式
        result = result.replace(" . ", ".");

        // *** 修复空格格式化问题 ***
        // v1.8.3: 保护 ": ::" 模式（类型注解后跟绝对路径），如 "me: ::core::ptr"
        result = result.replace(": ::", "__COLON_SPACE_DOUBLE_COLON__");
        
        // 移除 :: 周围的多余空格，但保护 ::< 模式
        result = result.replace(" :: ", "::");
        result = result.replace(" ::", "::");
        result = result.replace(":: ", "::");
        
        // v1.8.3: 恢复 ": ::" 模式
        result = result.replace("__COLON_SPACE_DOUBLE_COLON__", ": ::");

        // 修复错误的 : in 模式（这些都是convert_loop bug产生的）
        // ": in " -> "::" (用于各种路径和turbofish)
        // 注意：这是修复旧bug产生的错误，新代码已在convert_loop中修复
        result = result.replace(": in ", "::");
        result = result.replace(": in<", "::<");

        // 移除 < > 周围的多余空格（容错手写Nu代码的空格）
        result = result.replace(" < ", "<");
        result = result.replace(" <", "<");
        result = result.replace("< ", "<"); // 移除 < 后的空格
        result = result.replace(" >", ">"); // 移除 > 前的空格
                                            // 注意：不要移除 > 后的空格，否则会导致 Vec<i32>>= vec![] 的问题

        // 移除逗号前的空格
        result = result.replace(" ,", ",");
        result = result.replace(", ", ", "); // 确保逗号后有一个空格

        // 修复错误的 if ; 模式（应该是完整的if表达式）
        // 这个问题来自于错误的.unwrap_or_else转换
        result = result.replace("if ;", "?;");

        // 修复 serde_core -> serde
        // 在路径和类型中替换，但保留 cfg 属性
        if !result.contains("#![cfg") && !result.contains("#[cfg") {
            result = result.replace("serde_core", "serde");
        } else {
            // 只替换非cfg上下文中的serde_core
            result = result.replace("use serde_core", "use serde");
            result = result.replace("serde_core::", "serde::");
        }

        // 修复 FudIterator -> FusedIterator
        result = result.replace("FudIterator", "FusedIterator");

        // 修复错误的 ::custom 枚举语法（E::custom被错误转换为enum ::custom）
        result = result.replace("enum ::", "E::");

        // 修复 <- 运算符（应该是 < -）
        result = result.replace("<-", "< -");

        // 注意：不要替换 "fn (" -> "f("，因为这会破坏函数类型签名
        // 例如：fn(i32) -> i32 会被错误转换为 f(i32) -> i32
        // 如果闭包中确实有 fn (variable) 这种模式，应该在 rust2nu 转换时就避免

        // 修复 Weekday : in -> Weekday::
        result = result.replace("Weekday : in ", "Weekday::");
        result = result.replace("WeekdaySet : in ", "WeekdaySet::");
        result = result.replace("Month : in ", "Month::");
        result = result.replace("NaiveDate : in ", "NaiveDate::");

        // 修复 from_str : in<T> -> from_str::<T>
        result = result.replace("from_str : in<", "from_str::<");

        result
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
