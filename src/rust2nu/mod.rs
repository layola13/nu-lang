// Rust to Nu Converter
// 将标准Rust代码压缩为Nu高密度语法

use anyhow::{Context, Result};
use quote::ToTokens;
use std::collections::HashSet;
use syn::{
    visit::Visit, Attribute, Block, Expr, File, FnArg, Item, ItemEnum, ItemFn, ItemImpl,
    ItemStruct, ItemTrait, ReturnType, Signature, Stmt, Type, Visibility,
    spanned::Spanned,
};

pub struct Rust2NuConverter {
    output: String,
    indent_level: usize,
    // 泛型作用域栈：跟踪当前作用域中的泛型参数名
    // 用于避免将泛型参数（如impl<S>中的S）误转换为类型缩写
    generic_scope_stack: Vec<HashSet<String>>,
    // v1.8: 保存原始源代码，用于提取宏的原始格式
    source_code: String,
}

impl Rust2NuConverter {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent_level: 0,
            generic_scope_stack: Vec::new(),
            source_code: String::new(),
        }
    }

    pub fn new_with_source(source: &str) -> Self {
        Self {
            output: String::new(),
            indent_level: 0,
            generic_scope_stack: Vec::new(),
            source_code: source.to_string(),
        }
    }

    pub fn convert(&self, rust_code: &str) -> Result<String> {
        // 策略：混合处理 - 保留注释行，转换代码行
        // 1. 先提取所有注释和它们的位置
        let lines: Vec<&str> = rust_code.lines().collect();
        let mut line_types = Vec::new(); // true = comment line, false = code line
        let mut in_block_comment = false;
        let mut in_inner_doc = false; // 跟踪是否在 /*! ... */ 块中

        for line in &lines {
            let trimmed = line.trim();

            // 检测 /*! 开始的inner doc注释块（syn会将其转换为#![doc]属性）
            if trimmed.starts_with("/*!") {
                in_inner_doc = true;
                in_block_comment = true;
                line_types.push(false); // 标记为非注释，让syn处理
                continue;
            }

            // 如果在inner doc块中，检测结束
            if in_inner_doc {
                if trimmed.contains("*/") {
                    in_inner_doc = false;
                    in_block_comment = false;
                }
                line_types.push(false); // 标记为非注释，让syn处理
                continue;
            }

            // 检测普通块注释
            if trimmed.starts_with("/*") && !trimmed.starts_with("/*!") {
                in_block_comment = true;
            }
            if in_block_comment && trimmed.contains("*/") {
                in_block_comment = false;
                line_types.push(true); // 普通块注释保留
                continue;
            }

            // 判断是否为纯注释行或空行
            // 注意：属性（#[...] 和 #![...]）不算注释，会被syn处理并在converted_code中输出
            let is_comment_or_empty = trimmed.is_empty()
                || trimmed.starts_with("//")
                || in_block_comment
                || (trimmed.starts_with("*") && !trimmed.starts_with("*/"));
            line_types.push(is_comment_or_empty);
        }

        // 2. 解析并转换代码（syn会忽略注释）
        let syntax_tree = syn::parse_file(rust_code).context("Failed to parse Rust code")?;

        // v1.8: 使用包含源代码的转换器，以便提取宏的原始格式
        let mut converter = Self::new_with_source(rust_code);
        converter.visit_file(&syntax_tree);
        let converted_code = converter.output;

        // 3. 合并：在转换后的代码中插入注释
        // 策略：保留文件开头的纯注释行，然后输出转换后的代码
        let mut output = String::new();
        let mut found_non_comment = false;

        for (i, line) in lines.iter().enumerate() {
            if line_types[i] {
                // 注释或空行
                if !found_non_comment {
                    // 文件开头的纯注释，直接保留
                    output.push_str(line);
                    output.push('\n');
                }
            } else {
                // 遇到第一行非注释代码（可能是属性、use、fn等）
                if !found_non_comment {
                    found_non_comment = true;
                    // 追加完整的转换后代码（包含属性、use、fn等）
                    output.push_str(&converted_code);
                }
                break;
            }
        }

        // 如果全是注释但转换后有内容，直接返回转换内容
        if !found_non_comment && !converted_code.is_empty() {
            return Ok(converted_code);
        }

        // 如果全是注释且转换后也是空的，返回注释
        if !found_non_comment {
            return Ok(output);
        }

        Ok(output)
    }

    fn indent(&self) -> String {
        "    ".repeat(self.indent_level)
    }

    fn writeln(&mut self, text: &str) {
        self.output.push_str(&self.indent());
        self.output.push_str(text);
        self.output.push('\n');
    }

    fn write(&mut self, text: &str) {
        self.output.push_str(text);
    }

    /// 判断是否是pub
    fn is_public(&self, vis: &Visibility) -> bool {
        // v1.7.2: 将 pub(crate) 和 pub(in path) 也视为 public
        // 原因：Nu 不支持细粒度的模块可见性，宁可从宽（避免私有访问错误）
        matches!(vis, Visibility::Public(_) | Visibility::Restricted(_))
    }

    /// 检查名称是否是当前作用域中的泛型参数
    fn is_generic_param(&self, name: &str) -> bool {
        self.generic_scope_stack
            .iter()
            .any(|scope| scope.contains(name))
    }

    /// 进入泛型作用域，记录泛型参数名和生命周期参数名
    fn push_generic_scope(&mut self, generics: &syn::Generics) {
        let mut scope = HashSet::new();
        for param in &generics.params {
            match param {
                syn::GenericParam::Type(type_param) => {
                    scope.insert(type_param.ident.to_string());
                }
                syn::GenericParam::Lifetime(lifetime_param) => {
                    // 也记录生命周期参数（如'a）以便识别
                    scope.insert(format!("'{}", lifetime_param.lifetime.ident));
                }
                _ => {}
            }
        }
        self.generic_scope_stack.push(scope);
    }

    /// 退出泛型作用域
    fn pop_generic_scope(&mut self) {
        self.generic_scope_stack.pop();
    }

    /// 转换函数签名
    fn convert_fn_signature(&self, sig: &Signature, vis: &Visibility) -> String {
        let mut result = String::new();

        // v1.8: unsafe 保持不变（不缩写为 U，因为太重要且易与 use 混淆）
        if sig.unsafety.is_some() {
            result.push_str("unsafe ");
        }

        // async函数用 ~ 前缀
        if sig.asyncness.is_some() {
            result.push('~');
        }

        // pub fn -> F, fn -> f
        result.push_str(if self.is_public(vis) { "F" } else { "f" });

        result.push(' ');
        result.push_str(&sig.ident.to_string());

        // v1.6.5: 完整保留泛型参数（包括生命周期）
        if !sig.generics.params.is_empty() {
            result.push_str(&self.convert_generics(&sig.generics));
        }

        // 参数列表
        result.push('(');
        let mut first = true;
        for input in &sig.inputs {
            if !first {
                result.push_str(", ");
            }
            first = false;

            match input {
                FnArg::Receiver(r) => {
                    // 检查是否有显式self类型 (如 self: &Rc<Self>)
                    // Receiver的reference和mutability只在没有显式类型时有效
                    // 如果有显式类型(r.colon_token存在)，则使用完整的类型信息
                    if r.colon_token.is_some() {
                        // 显式类型：输出完整的 self: Type
                        result.push_str("self: ");
                        result.push_str(&self.convert_type(&r.ty));
                    } else if let Some((_, lifetime)) = &r.reference {
                        // v1.8: 保留 &'a self 中的生命周期
                        result.push('&');
                        if let Some(lt) = lifetime {
                            result.push_str(&lt.to_string());
                            result.push(' ');
                        }
                        if r.mutability.is_some() {
                            result.push('!'); // &mut -> &!
                        }
                        result.push_str("self");
                    } else {
                        // 按值接收的self
                        if r.mutability.is_some() {
                            result.push('!'); // mut self -> !self
                        }
                        result.push_str("self");
                    }
                }
                FnArg::Typed(pt) => {
                    result.push_str(&pt.pat.to_token_stream().to_string());
                    result.push_str(": ");
                    result.push_str(&self.convert_type(&pt.ty));
                }
            }
        }
        result.push(')');

        // 返回类型
        if let ReturnType::Type(_, ty) = &sig.output {
            result.push_str(" -> ");
            result.push_str(&self.convert_type(ty));
        }

        // where子句 - 使用 wh 而不是 w（避免与单字母变量冲突）
        // v1.7.4: 保护泛型参数名，避免被误替换
        if let Some(where_clause) = &sig.generics.where_clause {
            result.push_str(" wh ");
            let where_str = where_clause
                .to_token_stream()
                .to_string()
                .replace("where", "");
            // 不对where子句内容进行类型转换，保持泛型参数原样
            result.push_str(&where_str);
        }

        result
    }

    /// v1.6.5: 转换泛型参数（完整保留生命周期）
    fn convert_generics(&self, generics: &syn::Generics) -> String {
        if generics.params.is_empty() {
            return String::new();
        }

        let params: Vec<String> = generics
            .params
            .iter()
            .map(|param| {
                match param {
                    // 1. 生命周期参数：完整保留
                    syn::GenericParam::Lifetime(l) => {
                        let lifetime_str = format!("'{}", l.lifetime.ident);
                        // 处理生命周期约束 'a: 'b
                        if !l.bounds.is_empty() {
                            let bounds: Vec<String> =
                                l.bounds.iter().map(|b| format!("'{}", b.ident)).collect();
                            format!("{}: {}", lifetime_str, bounds.join(" + "))
                        } else {
                            lifetime_str
                        }
                    }
                    // 2. 类型参数
                    syn::GenericParam::Type(t) => {
                        let name = &t.ident;
                        // 处理类型约束 T: Display + Debug
                        let bounds = if t.bounds.is_empty() {
                            String::new()
                        } else {
                            format!(": {}", self.convert_type_param_bounds(&t.bounds))
                        };
                        // v1.7.4: 处理泛型默认值 E = ()
                        let default = if let Some(default_ty) = &t.default {
                            format!(" = {}", self.convert_type(default_ty))
                        } else {
                            String::new()
                        };
                        format!("{}{}{}", name, bounds, default)
                    }
                    // 3. 常量泛型参数
                    syn::GenericParam::Const(c) => {
                        format!("const {}: {}", c.ident, self.convert_type(&c.ty))
                    }
                }
            })
            .collect();

        format!("<{}>", params.join(", "))
    }

    /// v1.6.5: 转换类型参数约束
    /// v1.7.5: 修复 ?Sized 约束支持（核心修复！）
    fn convert_type_param_bounds(
        &self,
        bounds: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::token::Plus>,
    ) -> String {
        bounds
            .iter()
            .map(|bound| {
                match bound {
                    syn::TypeParamBound::Trait(trait_bound) => {
                        // 🔑 关键修复：处理 TraitBoundModifier::Maybe（即 ?Sized）
                        let modifier = match trait_bound.modifier {
                            syn::TraitBoundModifier::None => "",
                            syn::TraitBoundModifier::Maybe(_) => "?", // 保留 ?Sized 的 ? 前缀
                        };
                        let path_str = trait_bound.path.to_token_stream().to_string();
                        format!("{}{}", modifier, path_str)
                    }
                    syn::TypeParamBound::Lifetime(lifetime) => {
                        format!("'{}", lifetime.ident)
                    }
                    _ => bound.to_token_stream().to_string(),
                }
            })
            .collect::<Vec<_>>()
            .join(" + ")
    }

    /// v1.6.5: 转换类型 - 完整保留生命周期信息
    fn convert_type(&self, ty: &Type) -> String {
        match ty {
            // 引用类型：完整保留生命周期
            Type::Reference(type_ref) => {
                let lifetime = if let Some(l) = &type_ref.lifetime {
                    // v1.6.5: 'static 可选缩写为 'S（但为了兼容性暂时保持完整）
                    if l.ident == "static" {
                        "'static ".to_string()
                    } else {
                        format!("'{} ", l.ident)
                    }
                } else {
                    String::new()
                };

                let mutability = if type_ref.mutability.is_some() {
                    "!"
                } else {
                    ""
                };
                let inner = self.convert_type(&type_ref.elem);

                format!("&{}{}{}", lifetime, mutability, inner)
            }
            // 裸指针类型：*const T 或 *mut T
            Type::Ptr(type_ptr) => {
                let mutability = if type_ptr.mutability.is_some() {
                    "mut"
                } else {
                    "const"
                };
                let inner = self.convert_type(&type_ptr.elem);
                format!("*{} {}", mutability, inner)
            }
            // 路径类型：处理泛型参数中的生命周期
            Type::Path(type_path) => self.convert_type_path(type_path),
            // 其他类型：使用默认处理
            _ => {
                let type_str = ty.to_token_stream().to_string();
                self.convert_type_string(&type_str)
            }
        }
    }

    /// v1.6.5: 转换类型路径（处理泛型参数中的生命周期）
    fn convert_type_path(&self, type_path: &syn::TypePath) -> String {
        let mut result = String::new();

        for (i, segment) in type_path.path.segments.iter().enumerate() {
            if i > 0 {
                result.push_str("::");
            }

            let seg_name = segment.ident.to_string();

            // 检查是否是当前作用域中的泛型参数
            if self.is_generic_param(&seg_name) {
                result.push_str(&seg_name);
            } else {
                // 应用类型缩写
                let abbreviated = match seg_name.as_str() {
                    "Vec" => "V",
                    "Option" => "O",
                    "Result" => "R",
                    "Arc" => "A",
                    "Mutex" => "X",
                    "Box" => "B",
                    _ => &seg_name,
                };
                result.push_str(abbreviated);
            }

            // 处理泛型参数
            match &segment.arguments {
                syn::PathArguments::AngleBracketed(args) => {
                    result.push('<');
                    let arg_strs: Vec<String> = args
                        .args
                        .iter()
                        .map(|arg| {
                            match arg {
                                // 生命周期参数
                                syn::GenericArgument::Lifetime(l) => {
                                    format!("'{}", l.ident)
                                }
                                // 类型参数
                                syn::GenericArgument::Type(t) => self.convert_type(t),
                                // 约束
                                syn::GenericArgument::Constraint(c) => {
                                    format!(
                                        "{}: {}",
                                        c.ident,
                                        self.convert_type_param_bounds(&c.bounds)
                                    )
                                }
                                // 常量
                                syn::GenericArgument::Const(c) => c.to_token_stream().to_string(),
                                _ => arg.to_token_stream().to_string(),
                            }
                        })
                        .collect();
                    result.push_str(&arg_strs.join(", "));
                    result.push('>');
                }
                syn::PathArguments::Parenthesized(args) => {
                    result.push('(');
                    let input_strs: Vec<String> =
                        args.inputs.iter().map(|t| self.convert_type(t)).collect();
                    result.push_str(&input_strs.join(", "));
                    result.push(')');
                    if let syn::ReturnType::Type(_, ty) = &args.output {
                        result.push_str(" -> ");
                        result.push_str(&self.convert_type(ty));
                    }
                }
                syn::PathArguments::None => {}
            }
        }

        result
    }

    /// v1.6.5: 转换类型字符串（向后兼容旧逻辑）
    fn convert_type_string(&self, type_str: &str) -> String {
        // 🔑 首先清理 to_token_stream() 产生的多余空格
        let type_str = self.clean_token_spaces(type_str);

        // 检查是否是单个泛型参数
        let trimmed = type_str.trim();
        if trimmed.len() == 1 && self.is_generic_param(trimmed) {
            return trimmed.to_string();
        }

        // 检查是否包含泛型参数路径
        if let Some(first_segment) = trimmed.split("::").next() {
            if self.is_generic_param(first_segment) {
                return type_str.to_string();
            }
        }

        // 应用类型缩写
        type_str
            .replace("Vec<", "V<")
            .replace("Option<", "O<")
            .replace("Result<", "R<")
            .replace("Arc<", "A<")
            .replace("Mutex<", "X<")
            .replace("Box<", "B<")
            .replace("&mut", "&!")
            .replace("*mut", "*mut") // 保持裸指针的mut关键字
            .replace("*const", "*const") // 保持裸指针的const关键字
    }

    /// 转换语句
    fn convert_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Local(local) => {
                // 先处理语句级别的属性（如 #[cfg]）
                for attr in &local.attrs {
                    self.write(&self.indent());
                    self.write(&self.convert_attribute(attr));
                    self.write("\n");
                }

                self.write(&self.indent());

                // let vs let mut
                let pat_str = local.pat.to_token_stream().to_string();
                let is_mut = pat_str.contains("mut");

                // 变量声明（无论是否有初始化值）
                self.write(if is_mut { "v " } else { "l " });

                // 先转换类型（保护裸指针的mut关键字），再去掉变量名前的mut
                let converted_pat = self.convert_type_in_string(&pat_str);
                // 只删除开头的 "mut "（变量名前的mut）
                let clean_pat = if converted_pat.starts_with("mut ") {
                    &converted_pat[4..] // 跳过 "mut "
                } else {
                    &converted_pat
                };
                self.write(clean_pat);

                // 如果有初始化值，输出赋值部分
                if let Some(init) = &local.init {
                    self.write(" = ");
                    self.write(&self.convert_expr(&init.expr));
                    
                    // v1.8: 处理 let-else 语法 (Rust 1.65+)
                    // let Some(x) = expr else { return; }
                    if let Some((_, diverge)) = &init.diverge {
                        self.write(" else ");
                        self.write(&self.convert_expr(diverge));
                    }
                }

                self.write(";\n");
            }
            Stmt::Expr(expr, semi) => {
                // Stmt::Expr本身不包含attrs字段，但如果代码中有 #[cfg] if expr { }这样的形式，
                // syn会将其解析为特殊的结构。实际上在Rust中，属性后面跟的语句会被包装。
                // 但对于我们当前遇到的情况，cfg属性+if语句在syn中可能被解析为其他形式。
                // 这里我们先处理常规的表达式语句。

                // 处理unsafe块（包括嵌套在其他表达式中的unsafe块）
                if let Expr::Unsafe(unsafe_expr) = expr {
                    self.write(&self.indent());
                    self.write("U { ");
                    // 转换unsafe块内的语句
                    for inner_stmt in &unsafe_expr.block.stmts {
                        // 简化处理：直接输出赋值语句
                        if let Stmt::Expr(Expr::Assign(assign), _) = inner_stmt {
                            let left = assign.left.to_token_stream().to_string();
                            let right = self.convert_expr(&assign.right);
                            self.write(&format!("{} = {}; ", left, right));
                        } else {
                            self.write(&inner_stmt.to_token_stream().to_string());
                            self.write(" ");
                        }
                    }
                    self.write("}");
                    if semi.is_some() {
                        self.write(";");
                    }
                    self.write("\n");
                    return;
                }

                // 处理break和continue (使用br和ct)
                if let Expr::Break(_) = expr {
                    self.write(&self.indent());
                    self.write("br");
                    if semi.is_some() {
                        self.write(";");
                    }
                    self.write("\n");
                    return;
                } else if let Expr::Continue(_) = expr {
                    self.write(&self.indent());
                    self.write("ct");
                    if semi.is_some() {
                        self.write(";");
                    }
                    self.write("\n");
                    return;
                }

                // 原有的return和macro处理...(v1.8: 添加attrs支持)
                if let Expr::Return(ret) = expr {
                    // v1.8: 输出语句级别的 #[cfg] 等属性
                    for attr in &ret.attrs {
                        self.write(&self.indent());
                        self.writeln(&self.convert_attribute(attr));
                    }
                    self.write(&self.indent());
                    self.write("< ");
                    if let Some(val) = &ret.expr {
                        self.write(&self.convert_expr(val));
                    }
                    self.write("\n");
                } else if let Expr::Macro(_mac) = expr {
                    self.write(&self.indent());
                    let macro_str = self
                        .clean_token_spaces(&expr.to_token_stream().to_string())
                        .replace("vec!", "V!"); // vec! -> V!
                    self.write(&macro_str);
                    if semi.is_some() {
                        self.write(";");
                    }
                    self.write("\n");
                } else {
                    let expr_str = self.convert_expr(expr);
                    self.write(&self.indent());
                    self.write(&expr_str);
                    if semi.is_some() {
                        self.write(";");
                    }
                    self.write("\n");
                }
            }
            Stmt::Macro(mac) => {
                // v1.6: 宏语句，vec!转换为V!，其他保留（println!, assert!, etc.）
                // 使用 clean_token_spaces 移除 to_token_stream() 插入的空格
                self.write(&self.indent());
                let macro_str = self
                    .clean_token_spaces(&mac.mac.to_token_stream().to_string())
                    .replace("vec!", "V!"); // vec! -> V!
                self.write(&macro_str);
                if mac.semi_token.is_some() {
                    self.write(";");
                }
                self.write("\n");
            }
            Stmt::Item(item) => {
                self.visit_item(item);
            }
        }
    }

    /// 转换表达式，保持适当的换行
    fn convert_expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::Await(await_expr) => {
                format!("{}.~", self.convert_expr(&await_expr.base))
            }
            Expr::Try(try_expr) => {
                format!("{}!", self.convert_expr(&try_expr.expr))
            }
            Expr::MethodCall(call) => {
                let receiver = self.convert_expr(&call.receiver);
                let method = call.method.to_string();

                // v1.6: 保留Turbofish泛型参数 ::<Type>
                let turbofish = if let Some(turbo) = &call.turbofish {
                    self.clean_token_spaces(&turbo.to_token_stream().to_string())
                } else {
                    String::new()
                };

                let args = call
                    .args
                    .iter()
                    .map(|arg| self.convert_expr(arg))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}.{}{}({})", receiver, method, turbofish, args)
            }
            Expr::Return(_ret) => {
                // return语句在语句级别处理，在表达式中不应该转换
                // 保持原样以避免在match分支中错误转换
                expr.to_token_stream().to_string()
            }
            Expr::Closure(closure) => {
                let move_kw = if closure.capture.is_some() { "$" } else { "" };
                let inputs = closure
                    .inputs
                    .iter()
                    .map(|p| self.clean_token_spaces(&p.to_token_stream().to_string()))
                    .collect::<Vec<_>>()
                    .join(", ");

                // v1.6: 支持闭包返回类型 |x: i32| -> i32 { }
                let return_type = match &closure.output {
                    syn::ReturnType::Default => String::new(),
                    syn::ReturnType::Type(_, ty) => {
                        let ty_str = self.convert_type_in_string(&ty.to_token_stream().to_string());
                        format!(" -> {}", ty_str)
                    }
                };

                let body = self.convert_expr(&closure.body);
                format!("{}|{}|{} {}", move_kw, inputs, return_type, body)
            }
            Expr::Match(match_expr) => {
                // M = match
                let scrutinee = self.convert_expr(&match_expr.expr);
                let mut result = format!("M {} {{\n", scrutinee);
                for arm in &match_expr.arms {
                    result.push_str("        ");
                    result
                        .push_str(&self.clean_token_spaces(&arm.pat.to_token_stream().to_string()));
                    if let Some((_, guard)) = &arm.guard {
                        result.push_str(" ? ");
                        result.push_str(&self.convert_expr(guard));
                    }
                    result.push_str(" => ");
                    result.push_str(&self.convert_expr(&arm.body));
                    result.push_str(",\n");
                }
                result.push_str("    }");
                self.convert_type_in_string(&result)
            }
            Expr::If(if_expr) => {
                // ? = if
                let cond = self.convert_expr(&if_expr.cond);
                let mut result = format!("? {} {{ ", cond);
                // 递归转换then分支中的语句
                for stmt in &if_expr.then_branch.stmts {
                    match stmt {
                        Stmt::Expr(Expr::Break(_), _) => result.push_str("br; "),
                        Stmt::Expr(Expr::Continue(_), _) => result.push_str("ct; "),
                        _ => {
                            result.push_str(
                                &self.clean_token_spaces(&stmt.to_token_stream().to_string()),
                            );
                            result.push(' ');
                        }
                    }
                }
                result.push('}');

                if let Some((_, else_branch)) = &if_expr.else_branch {
                    result.push_str(" else ");
                    result.push_str(&self.convert_expr(else_branch));
                }
                self.convert_type_in_string(&result)
            }
            Expr::Block(block_expr) => {
                // 块表达式：递归转换内部语句
                let mut result = String::from("{\n");
                for stmt in &block_expr.block.stmts {
                    result.push_str("        ");
                    // 递归转换语句以处理内部的if/match/break/continue
                    let stmt_str = match stmt {
                        Stmt::Expr(Expr::Break(_), _) => String::from("br"),
                        Stmt::Expr(Expr::Continue(_), _) => String::from("ct"),
                        Stmt::Expr(Expr::Return(ret), _) => {
                            // v1.8: 处理return语句的 #[cfg] 等属性
                            let mut attr_prefix = String::new();
                            for attr in &ret.attrs {
                                attr_prefix.push_str(&self.convert_attribute(attr));
                                attr_prefix.push('\n');
                                attr_prefix.push_str("        ");
                            }
                            if let Some(val) = &ret.expr {
                                format!("{}< {}", attr_prefix, self.convert_expr(val))
                            } else {
                                format!("{}<", attr_prefix)
                            }
                        }
                        _ => self.clean_token_spaces(&stmt.to_token_stream().to_string()),
                    };
                    result.push_str(&stmt_str);
                    result.push('\n');
                }
                result.push_str("    }");
                self.convert_type_in_string(&result)
            }
            Expr::ForLoop(for_loop) => {
                // L = for
                let pat = self.clean_token_spaces(&for_loop.pat.to_token_stream().to_string());
                let iter = self.convert_expr(&for_loop.expr);
                let mut result = format!("L {} in {} {{ ", pat, iter);
                // 递归转换循环体中的语句
                for stmt in &for_loop.body.stmts {
                    match stmt {
                        Stmt::Expr(Expr::Break(_), _) => result.push_str("br; "),
                        Stmt::Expr(Expr::Continue(_), _) => result.push_str("ct; "),
                        Stmt::Expr(Expr::If(if_expr), semi) => {
                            result.push_str(&self.convert_expr(&Expr::If(if_expr.clone())));
                            if semi.is_some() {
                                result.push_str("; ");
                            } else {
                                result.push(' ');
                            }
                        }
                        Stmt::Expr(Expr::Match(match_expr), semi) => {
                            result.push_str(&self.convert_expr(&Expr::Match(match_expr.clone())));
                            if semi.is_some() {
                                result.push_str("; ");
                            } else {
                                result.push(' ');
                            }
                        }
                        _ => {
                            let stmt_str = self
                                .clean_token_spaces(&stmt.to_token_stream().to_string())
                                .replace("vec!", "V!");
                            result.push_str(&stmt_str);
                            result.push(' ');
                        }
                    }
                }
                result.push('}');
                self.convert_type_in_string(&result)
            }
            Expr::While(while_expr) => {
                // while暂时保持不变（nu没有while的简写）
                let cond = self.convert_expr(&while_expr.cond);
                let mut result = format!("while {} {{\n", cond);
                for stmt in &while_expr.body.stmts {
                    result.push_str("        ");
                    result.push_str(&self.clean_token_spaces(&stmt.to_token_stream().to_string()));
                    result.push('\n');
                }
                result.push_str("    }");
                self.convert_type_in_string(&result)
            }
            Expr::Loop(loop_expr) => {
                // L = loop
                let mut result = String::from("L { ");
                // 递归转换循环体中的语句
                for stmt in &loop_expr.body.stmts {
                    match stmt {
                        Stmt::Expr(Expr::Break(_), _) => result.push_str("br; "),
                        Stmt::Expr(Expr::Continue(_), _) => result.push_str("ct; "),
                        Stmt::Expr(Expr::If(if_expr), semi) => {
                            result.push_str(&self.convert_expr(&Expr::If(if_expr.clone())));
                            if semi.is_some() {
                                result.push_str("; ");
                            } else {
                                result.push(' ');
                            }
                        }
                        Stmt::Expr(Expr::ForLoop(for_loop), semi) => {
                            result.push_str(&self.convert_expr(&Expr::ForLoop(for_loop.clone())));
                            if semi.is_some() {
                                result.push_str("; ");
                            } else {
                                result.push(' ');
                            }
                        }
                        _ => {
                            let stmt_str = self
                                .clean_token_spaces(&stmt.to_token_stream().to_string())
                                .replace("vec!", "V!");
                            result.push_str(&stmt_str);
                            result.push(' ');
                        }
                    }
                }
                result.push('}');
                self.convert_type_in_string(&result)
            }
            Expr::Break(_) => String::from("br"),
            Expr::Continue(_) => String::from("ct"),
            // v1.8: 处理引用表达式，递归处理内部表达式
            // 这样 &StructLiteral{} 可以正确格式化结构体字面量
            Expr::Reference(ref_expr) => {
                let mutability = if ref_expr.mutability.is_some() { "&mut " } else { "& " };
                format!("{}{}", mutability, self.convert_expr(&ref_expr.expr))
            }
            // v1.8: 处理结构体表达式，保留字段上的 #[cfg] 属性并换行输出
            Expr::Struct(struct_expr) => {
                let path = self.clean_token_spaces(&struct_expr.path.to_token_stream().to_string());
                let mut result = format!("{}{{", path);
                
                for field in &struct_expr.fields {
                    // 处理字段上的 #[cfg] 等属性 - 每个属性独立一行
                    for attr in &field.attrs {
                        let attr_str = attr.to_token_stream().to_string();
                        let cleaned_attr = attr_str
                            .replace("# [", "#[")
                            .replace(" [", "[")
                            .replace(" ]", "]")
                            .replace(" (", "(")
                            .replace(" )", ")")
                            .replace(" ,", ",");
                        result.push_str(&cleaned_attr);
                        result.push('\n');  // 属性后换行
                    }
                    
                    // 字段名: 值
                    let member = field.member.to_token_stream().to_string();
                    let value = self.convert_expr(&field.expr);
                    result.push_str(&format!("{}: {},\n", member, value));
                }
                
                // 处理 .. 表达式（结构体更新语法）
                if let Some(rest) = &struct_expr.rest {
                    result.push_str(&format!("..{}", self.convert_expr(rest)));
                }
                
                result.push('}');
                self.convert_type_in_string(&result)
            }
            _ => {
                // 默认：保持原样但替换类型和vec!宏
                let expr_str = self
                    .clean_token_spaces(&expr.to_token_stream().to_string())
                    .replace("vec!", "V!");
                self.convert_type_in_string(&expr_str)
            }
        }
    }

    /// 清理 to_token_stream() 产生的多余空格
    /// 例如: "V < i32 >" -> "V<i32>", "vec ! []" -> "vec![]", "x . method()" -> "x.method()"
    fn clean_token_spaces(&self, s: &str) -> String {
        let mut result = s.to_string();

        // 移除 < > 周围的空格（用于泛型如 Vec< i32 > -> Vec<i32>）
        // v1.8: 智能处理 - 只在同一行有成对 <> 时才清理空格（泛型上下文）
        // 如果只有 < 没有 >，是 return 语句，保留空格
        let mut cleaned_lines = Vec::new();
        for line in result.lines() {
            let has_open = line.contains('<');
            let has_close = line.contains('>');
            let mut cleaned_line = line.to_string();
            
            // 只有同时存在 < 和 > 才是泛型，需要清理空格
            if has_open && has_close {
                cleaned_line = cleaned_line.replace(" < ", "<");
                cleaned_line = cleaned_line.replace(" <", "<");
                cleaned_line = cleaned_line.replace("< ", "<");
                cleaned_line = cleaned_line.replace(" > ", "> ");
                cleaned_line = cleaned_line.replace(" >", ">");
            }
            // 如果只有 < 没有 >，是 return 语句，保持原样
            
            cleaned_lines.push(cleaned_line);
        }
        result = cleaned_lines.join("\n");

        // 移除 :: 周围的空格
        result = result.replace(" :: ", "::");
        result = result.replace(" ::", "::");
        result = result.replace(":: ", "::");

        // 移除 ! 前的空格（用于宏调用如 vec ! [] -> vec![]）
        result = result.replace(" !", "!");

        // 移除 [ ] ( ) { } 周围的空格
        result = result.replace(" [", "[");
        result = result.replace("[ ", "[");
        result = result.replace(" ]", "]");
        result = result.replace(" (", "(");
        result = result.replace("( ", "(");
        result = result.replace(" )", ")");
        result = result.replace("{ ", "{");
        result = result.replace(" }", "}");

        // 移除逗号前的空格，保留逗号后的空格
        result = result.replace(" ,", ",");

        // 移除分号前的空格
        result = result.replace(" ;", ";");

        // 移除方法调用中 . 周围的空格（如 "x . method()" -> "x.method()"）
        result = result.replace(" . ", ".");
        result = result.replace(" .", ".");
        result = result.replace(". ", ".");

        // 但是要特别处理浮点数 - "1. 0" 不应该变成 "1.0"（syn不会这样输出，所以这里不需要特别处理）

        // 移除类型注解冒号后面的多余空格（但保留一个空格）
        // "x : Type" -> "x: Type" (保持 ": " 的格式)
        result = result.replace(" : ", ": ");

        // 修复 "identifier :(" -> "identifier: (" 的格式（元组类型注解）
        // 需要在冒号后添加空格
        result = result.replace(": (", ": ("); // 已经正确了
        result = result.replace(":(", ": ("); // 修复紧贴的情况

        // 修复空闭包管道: "| |" -> "||"
        result = result.replace("| |", "||");

        // 修复 *= += -= 等复合赋值运算符周围的空格
        result = result.replace("* ", "*"); // 解引用符后不需要空格

        result
    }

    fn convert_type_in_string(&self, s: &str) -> String {
        // v1.7.3: 智能类型替换，避免将泛型参数误替换为关键字
        // 例如：where M: Display 不应该变成 where match: Display

        // 🔑 首先清理 to_token_stream() 产生的多余空格
        let s = self.clean_token_spaces(s);

        // 先检查是否包含单字母泛型参数（如 <M>、<T>、where M:）
        // 这些情况下不进行类型名称的替换
        let has_generic_param_context =
            s.contains("where ") || s.contains("impl<") || s.contains("impl <");

        let mut result = s.to_string();
        let mut protected_parts = Vec::new();

        // 查找并保护所有的 turbofish 模式 (::<...>)
        let chars: Vec<char> = result.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            // 检测 ::< 模式
            if i + 2 < chars.len() && chars[i] == ':' && chars[i + 1] == ':' && chars[i + 2] == '<'
            {
                let start = i;
                i += 3;
                let mut depth = 1;

                // 找到匹配的 >
                while i < chars.len() && depth > 0 {
                    if chars[i] == '<' {
                        depth += 1;
                    } else if chars[i] == '>' {
                        depth -= 1;
                    }
                    i += 1;
                }

                // 提取 turbofish 部分
                let turbofish: String = chars[start..i].iter().collect();
                protected_parts.push(turbofish);
            } else {
                i += 1;
            }
        }

        // 用占位符替换 turbofish
        for (idx, part) in protected_parts.iter().enumerate() {
            result = result.replacen(part, &format!("__TURBOFISH_PLACEHOLDER_{}__", idx), 1);
        }

        // v1.7.3: 如果在泛型参数上下文中（where子句、impl<T>等），不进行类型替换
        if !has_generic_param_context {
            // 执行类型替换和宏替换
            // v1.7: String不再缩写为Str
            // v1.7.1: 保护类型路径前缀（Result::Ok等）不被替换
            // 注意：to_token_stream()会输出带空格的 "Result :: Ok"，需要同时保护
            // 先保护路径前缀（带空格和不带空格两种形式）
            result = result
                .replace("Vec :: ", "__VEC_PATH_SP__")
                .replace("Vec::", "__VEC_PATH__")
                .replace("Option :: ", "__OPTION_PATH_SP__")
                .replace("Option::", "__OPTION_PATH__")
                .replace("Result :: ", "__RESULT_PATH_SP__")
                .replace("Result::", "__RESULT_PATH__")
                .replace("Arc :: ", "__ARC_PATH_SP__")
                .replace("Arc::", "__ARC_PATH__")
                .replace("Mutex :: ", "__MUTEX_PATH_SP__")
                .replace("Mutex::", "__MUTEX_PATH__")
                .replace("Box :: ", "__BOX_PATH_SP__")
                .replace("Box::", "__BOX_PATH__");

            // v1.8: 先保护完整的标识符（如 Boxed, VecDeque）以防止被错误替换
            result = result
                .replace("Boxed", "__BOXED_IDENT__")
                .replace("VecDeque", "__VECDEQUE_IDENT__")
                .replace("ResultCode", "__RESULTCODE_IDENT__")
                .replace("OptionSet", "__OPTIONSET_IDENT__");

            // 执行类型名替换
            result = result
                .replace("Vec", "V")
                .replace("Option", "O")
                .replace("Result", "R")
                .replace("Arc", "A")
                .replace("Mutex", "X")
                .replace("Box", "B")
                .replace("& mut", "&!")
                .replace("vec!", "V!"); // vec! -> V!
            
            // 恢复被保护的标识符
            result = result
                .replace("__BOXED_IDENT__", "Boxed")
                .replace("__VECDEQUE_IDENT__", "VecDeque")
                .replace("__RESULTCODE_IDENT__", "ResultCode")
                .replace("__OPTIONSET_IDENT__", "OptionSet");

            // 恢复路径前缀（保持完整类型名）
            result = result
                .replace("__VEC_PATH_SP__", "Vec::")
                .replace("__VEC_PATH__", "Vec::")
                .replace("__OPTION_PATH_SP__", "Option::")
                .replace("__OPTION_PATH__", "Option::")
                .replace("__RESULT_PATH_SP__", "Result::")
                .replace("__RESULT_PATH__", "Result::")
                .replace("__ARC_PATH_SP__", "Arc::")
                .replace("__ARC_PATH__", "Arc::")
                .replace("__MUTEX_PATH_SP__", "Mutex::")
                .replace("__MUTEX_PATH__", "Mutex::")
                .replace("__BOX_PATH_SP__", "Box::")
                .replace("__BOX_PATH__", "Box::");
        }

        // 恢复 turbofish（保持原样，不进行类型替换）
        for (idx, part) in protected_parts.iter().enumerate() {
            result = result.replace(&format!("__TURBOFISH_PLACEHOLDER_{}__", idx), part);
        }

        result
    }

    /// 转换函数体
    fn convert_block(&mut self, block: &Block) {
        self.writeln(" {");
        self.indent_level += 1;

        for stmt in &block.stmts {
            self.convert_stmt(stmt);
        }

        self.indent_level -= 1;
        self.writeln("}");
    }

    /// 递归检测表达式中是否包含嵌套的unsafe块
    fn contains_nested_unsafe(expr: &Expr) -> bool {
        match expr {
            Expr::Unsafe(_) => true,
            Expr::Match(expr_match) => {
                // 检查match的每个分支
                expr_match
                    .arms
                    .iter()
                    .any(|arm| Self::contains_nested_unsafe(&arm.body))
            }
            Expr::If(expr_if) => {
                // 检查if的then分支
                let then_has_unsafe = expr_if.then_branch.stmts.iter().any(|stmt| {
                    if let Stmt::Expr(e, _) = stmt {
                        Self::contains_nested_unsafe(e)
                    } else {
                        false
                    }
                });
                // 检查else分支
                let else_has_unsafe = expr_if
                    .else_branch
                    .as_ref()
                    .map_or(false, |(_, e)| Self::contains_nested_unsafe(e));
                then_has_unsafe || else_has_unsafe
            }
            Expr::Block(expr_block) => {
                // 检查块中的语句
                expr_block.block.stmts.iter().any(|stmt| {
                    if let Stmt::Expr(e, _) = stmt {
                        Self::contains_nested_unsafe(e)
                    } else {
                        false
                    }
                })
            }
            Expr::Loop(loop_expr) => loop_expr.body.stmts.iter().any(|stmt| {
                if let Stmt::Expr(e, _) = stmt {
                    Self::contains_nested_unsafe(e)
                } else {
                    false
                }
            }),
            Expr::ForLoop(for_loop) => for_loop.body.stmts.iter().any(|stmt| {
                if let Stmt::Expr(e, _) = stmt {
                    Self::contains_nested_unsafe(e)
                } else {
                    false
                }
            }),
            Expr::While(while_expr) => while_expr.body.stmts.iter().any(|stmt| {
                if let Stmt::Expr(e, _) = stmt {
                    Self::contains_nested_unsafe(e)
                } else {
                    false
                }
            }),
            _ => false,
        }
    }

    /// 检查块是否包含unsafe代码（如unsafe块或unsafe static赋值）
    fn block_contains_unsafe(&self, block: &Block) -> bool {
        for stmt in &block.stmts {
            if let Stmt::Expr(expr, _) = stmt {
                if Self::contains_nested_unsafe(expr) {
                    return true;
                }
                // 检查赋值语句是否涉及static变量
                if let Expr::Assign(assign) = expr {
                    let left_str = assign.left.to_token_stream().to_string();
                    if left_str.to_uppercase() == left_str
                        && left_str.chars().all(|c| c.is_alphanumeric() || c == '_')
                    {
                        // 可能是LOGGER这样的static变量
                        return true;
                    }
                }
            }
        }
        false
    }

    fn convert_attribute(&self, attr: &Attribute) -> String {
        let path = attr.path().to_token_stream().to_string();
        let tokens = attr.meta.to_token_stream().to_string();

        if path == "derive" {
            format!("#D{}", tokens.trim_start_matches("derive"))
        } else {
            // 保持其他属性的完整格式，并清理多余空格
            let cleaned_tokens = tokens
                .replace(" (", "(")
                .replace(" )", ")")
                .replace(" ,", ",");
            format!("#[{}]", cleaned_tokens)
        }
    }
}

impl<'ast> Visit<'ast> for Rust2NuConverter {
    fn visit_file(&mut self, node: &'ast File) {
        // Nu v1.6.3: 优先输出文件级属性 #![...]
        for attr in &node.attrs {
            let attr_str = attr.to_token_stream().to_string();
            // to_token_stream()会在#!、[、]周围插入空格，需要移除
            let cleaned_attr = attr_str
                .replace("# !", "#!")
                .replace("#! ", "#!")
                .replace(" [", "[")
                .replace(" ]", "]")
                .replace(" (", "(")
                .replace(" )", ")");
            if cleaned_attr.starts_with("#![") {
                self.writeln(&cleaned_attr);
            }
        }

        for item in &node.items {
            self.visit_item(item);
            self.output.push('\n');
        }
    }

    fn visit_item(&mut self, node: &'ast Item) {
        match node {
            Item::Fn(func) => self.visit_item_fn(func),
            Item::Struct(s) => self.visit_item_struct(s),
            Item::Enum(e) => self.visit_item_enum(e),
            Item::Trait(t) => self.visit_item_trait(t),
            Item::Impl(i) => self.visit_item_impl(i),
            Item::Macro(m) => {
                // v1.8: 使用span提取原始宏文本，保留1:1换行格式
                // 如果有source_code，尝试从中提取原始文本
                if !self.source_code.is_empty() {
                    let span = m.span();
                    let start = span.start();
                    let end = span.end();
                    
                    // 按行分割源代码
                    let lines: Vec<&str> = self.source_code.lines().collect();
                    
                    // 提取从start到end的所有行（行号从1开始，转为0索引）
                    if start.line > 0 && end.line <= lines.len() {
                        let start_line = start.line - 1;
                        let end_line = end.line; // end.line是包含的，不需要-1
                        
                        // 提取原始宏文本
                        let original_macro: String = lines[start_line..end_line].join("\n");
                        self.writeln(&original_macro);
                        return;
                    }
                }
                
                // 回退方案：使用to_token_stream()并清理空格
                let macro_str = m.to_token_stream().to_string();
                let cleaned_macro = macro_str
                    .replace("# [", "#[")
                    .replace("# !", "#!")
                    .replace(" [", "[")
                    .replace(" ]", "]")
                    .replace(" (", "(")
                    .replace(" )", ")")
                    .replace(" ,", ",")
                    .replace(" ;", ";")
                    .replace("! {", "! {")
                    .replace("macro_rules!", "macro_rules!")
                    .replace("} ;", "};\n   ")
                    .replace("=> {", "=> {\n        ");
                self.writeln(&cleaned_macro);
            }
            Item::Mod(m) => {
                // v1.8: 保留 #[cfg] 和 #[macro_use] 属性
                for attr in &m.attrs {
                    let attr_str = attr.to_token_stream().to_string();
                    // to_token_stream()会在#、[、(、)周围插入空格，需要移除
                    let cleaned_attr = attr_str
                        .replace("# [", "#[")
                        .replace(" [", "[")
                        .replace(" ]", "]")
                        .replace(" (", "(")
                        .replace(" )", ")")
                        .replace(" ,", ",");
                    // v1.8: 保留 #[cfg] 和 #[macro_use] 属性（macro_use 对于宏可见性至关重要）
                    if cleaned_attr.starts_with("#[cfg") || cleaned_attr.starts_with("#[macro_use") {
                        self.writeln(&cleaned_attr);
                    }
                }

                // Nu v1.6.3: DM=pub mod, D=mod
                let keyword = if self.is_public(&m.vis) { "DM" } else { "D" };

                if let Some((_, items)) = &m.content {
                    // 内联模块：mod name { ... }
                    self.write(keyword);
                    self.write(" ");
                    self.write(&m.ident.to_string());
                    self.writeln(" {");
                    self.indent_level += 1;
                    for item in items {
                        self.visit_item(item);
                        self.output.push('\n');
                    }
                    self.indent_level -= 1;
                    self.writeln("}");
                } else {
                    // 模块声明：mod name;
                    self.writeln(&format!("{} {};", keyword, m.ident));
                }
            }
            Item::Use(u) => {
                // v1.8: 先单独输出属性（每个属性一行），避免合并到 use 语句行
                for attr in &u.attrs {
                    let attr_str = attr.to_token_stream().to_string();
                    let cleaned_attr = attr_str
                        .replace("# [", "#[")
                        .replace(" [", "[")
                        .replace(" ]", "]")
                        .replace(" (", "(")
                        .replace(" )", ")")
                        .replace(" ,", ",");
                    self.writeln(&cleaned_attr);
                }
                
                // 输出 use 语句本身（不含属性）
                // v1.8: 保留受限可见性 pub(crate)/pub(super)
                let vis_prefix = if let syn::Visibility::Restricted(vis_restricted) = &u.vis {
                    let vis_str = vis_restricted.to_token_stream().to_string();
                    let cleaned = vis_str.replace("pub (", "pub(").replace("( ", "(").replace(" )", ")");
                    format!("{} U ", cleaned)
                } else if self.is_public(&u.vis) {
                    "U ".to_string()
                } else {
                    "u ".to_string()
                };
                let tree_str = u.tree.to_token_stream().to_string();
                let cleaned_tree = self.clean_token_spaces(&tree_str);
                self.writeln(&format!("{}{};", vis_prefix, cleaned_tree));
            }
            Item::Const(c) => {
                self.write("C ");
                self.write(&c.ident.to_string());
                self.write(": ");
                self.write(&self.convert_type(&c.ty));
                self.write(" = ");
                self.write(&c.expr.to_token_stream().to_string());
                self.writeln(";");
            }
            Item::Static(s) => {
                // Nu v1.6.3: SM = static mut, ST = static
                let keyword = if matches!(s.mutability, syn::StaticMutability::Mut(_)) {
                    "SM"
                } else {
                    "ST"
                };
                self.write(keyword);
                self.write(" ");
                self.write(&s.ident.to_string());
                self.write(": ");
                self.write(&self.convert_type(&s.ty));
                self.write(" = ");
                self.write(&s.expr.to_token_stream().to_string());
                self.writeln(";");
            }
            _ => {
                // 其他项保持原样
                self.writeln(&node.to_token_stream().to_string());
            }
        }
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        // 属性
        for attr in &node.attrs {
            self.writeln(&self.convert_attribute(attr));
        }

        // 函数签名
        let sig_str = self.convert_fn_signature(&node.sig, &node.vis);
        self.write(&sig_str);

        // 函数体
        self.convert_block(&node.block);
    }

    fn visit_item_struct(&mut self, node: &'ast ItemStruct) {
        // 进入泛型作用域
        self.push_generic_scope(&node.generics);

        // Nu v1.6.3: 输出所有属性（derive、cfg等）
        for attr in &node.attrs {
            self.writeln(&self.convert_attribute(attr));
        }

        // v1.8: 保留 pub(crate)/pub(super) 等受限可见性
        // v1.8: 输出缩进（确保模块内的项正确缩进）
        self.write(&self.indent());
        if let syn::Visibility::Restricted(vis_restricted) = &node.vis {
            let vis_str = vis_restricted.to_token_stream().to_string();
            let cleaned_vis = vis_str
                .replace("pub (", "pub(")
                .replace("( ", "(")
                .replace(" )", ")");
            self.write(&cleaned_vis);
            self.write(" ");
        }

        // Nu v1.5.1: 只有 S（移除了 s）
        // 可见性由标识符首字母决定（Go风格）
        // 根据可见性决定使用 S 或 s (v1.7.2: pub(crate)也视为public)
        if self.is_public(&node.vis) {
            self.write("S");
        } else {
            self.write("s");
        }
        self.write(" ");
        self.write(&node.ident.to_string());

        // v1.6.5: 泛型（完整保留生命周期）
        if !node.generics.params.is_empty() {
            self.write(&self.convert_generics(&node.generics));
        }

        // v1.7.5: 结构体的 where 子句支持（关键修复！）
        if let Some(where_clause) = &node.generics.where_clause {
            self.write(" wh ");
            self.write(
                &where_clause
                    .to_token_stream()
                    .to_string()
                    .replace("where", "")
                    .trim(),
            );
        }

        // 字段
        match &node.fields {
            syn::Fields::Named(fields) => {
                self.writeln(" {");
                self.indent_level += 1;
                for field in &fields.named {
                    // 输出字段的 #[cfg] 属性
                    for attr in &field.attrs {
                        let attr_str = attr.to_token_stream().to_string();
                        // to_token_stream()会在#、[、(、)周围插入空格，需要移除
                        let cleaned_attr = attr_str
                            .replace("# [", "#[")
                            .replace(" [", "[")
                            .replace(" ]", "]")
                            .replace(" (", "(")
                            .replace(" )", ")")
                            .replace(" ,", ",");
                        if cleaned_attr.starts_with("#[cfg") {
                            self.write(&self.indent());
                            self.writeln(&cleaned_attr);
                        }
                    }

                    self.write(&self.indent());
                    if let Some(ident) = &field.ident {
                        self.write(&ident.to_string());
                        self.write(": ");
                        self.write(&self.convert_type(&field.ty));
                        self.writeln(",");
                    }
                }
                self.indent_level -= 1;
                self.writeln("}");
            }
            syn::Fields::Unnamed(fields) => {
                // Tuple struct: pub struct ParseLevelError(());
                self.write("(");
                let type_strs: Vec<String> = fields
                    .unnamed
                    .iter()
                    .map(|f| self.convert_type(&f.ty))
                    .collect();
                self.write(&type_strs.join(", "));
                self.writeln(");");
            }
            syn::Fields::Unit => {
                // Unit struct: pub struct UnitStruct;
                self.writeln(";");
            }
        }

        // 退出泛型作用域
        self.pop_generic_scope();
    }

    fn visit_item_enum(&mut self, node: &'ast ItemEnum) {
        // 处理属性
        for attr in &node.attrs {
            self.writeln(&self.convert_attribute(attr));
        }

        // v1.8: 保留 pub(crate)/pub(super) 等受限可见性
        // E 只处理简单的 pub/private，受限可见性需要显式保留
        // v1.8: 输出缩进（确保模块内的项正确缩进）
        self.write(&self.indent());
        if let syn::Visibility::Restricted(vis_restricted) = &node.vis {
            let vis_str = vis_restricted.to_token_stream().to_string();
            // 清理空格
            let cleaned_vis = vis_str
                .replace("pub (", "pub(")
                .replace("( ", "(")
                .replace(" )", ")");
            self.write(&cleaned_vis);
            self.write(" ");
        }

        // Nu v1.5.1: 只有 E（移除了 e）
        // 可见性由标识符首字母决定（Go风格）
        self.write("E");
        self.write(" ");
        self.write(&node.ident.to_string());

        // v1.6.5: 泛型（完整保留生命周期）
        if !node.generics.params.is_empty() {
            self.write(&self.convert_generics(&node.generics));
        }

        self.writeln(" {");
        self.indent_level += 1;

        for variant in &node.variants {
            self.write(&self.indent());
            self.write(&variant.ident.to_string());

            match &variant.fields {
                syn::Fields::Named(fields) => {
                    self.write(" { ");
                    let field_strs: Vec<String> = fields
                        .named
                        .iter()
                        .filter_map(|f| {
                            f.ident
                                .as_ref()
                                .map(|i| format!("{}: {}", i, self.convert_type(&f.ty)))
                        })
                        .collect();
                    self.write(&field_strs.join(", "));
                    self.write(" }");
                }
                syn::Fields::Unnamed(fields) => {
                    self.write("(");
                    let type_strs: Vec<String> = fields
                        .unnamed
                        .iter()
                        .map(|f| self.convert_type(&f.ty))
                        .collect();
                    self.write(&type_strs.join(", "));
                    self.write(")");
                }
                syn::Fields::Unit => {}
            }

            self.writeln(",");
        }

        self.indent_level -= 1;
        self.writeln("}");
    }

    fn visit_item_trait(&mut self, node: &'ast ItemTrait) {
        // v1.8: 保留 #[cfg] 属性
        for attr in &node.attrs {
            let attr_str = attr.to_token_stream().to_string();
            let cleaned_attr = attr_str
                .replace("# [", "#[")
                .replace(" [", "[")
                .replace(" ]", "]")
                .replace(" (", "(")
                .replace(" )", ")")
                .replace(" ,", ",");
            if cleaned_attr.starts_with("#[cfg") {
                self.writeln(&cleaned_attr);
            }
        }
        
        let keyword = if self.is_public(&node.vis) {
            "TR"
        } else {
            "tr"
        };

        self.write(keyword);
        self.write(" ");
        self.write(&node.ident.to_string());

        // v1.6.5: 泛型（完整保留生命周期）
        if !node.generics.params.is_empty() {
            self.write(&self.convert_generics(&node.generics));
        }

        self.writeln(" {");
        self.indent_level += 1;

        for item in &node.items {
            match item {
                syn::TraitItem::Fn(method) => {
                    // 处理方法的属性（如 #[allow(dead_code)]）
                    for attr in &method.attrs {
                        self.write(&self.indent());
                        self.write(&self.convert_attribute(attr));
                        self.write("\n");
                    }

                    let sig_str = self.convert_fn_signature(&method.sig, &Visibility::Inherited);
                    self.write(&self.indent());
                    self.write(&sig_str);

                    // 检查是否有默认实现（方法体）
                    if let Some(block) = &method.default {
                        // 有默认实现：输出函数体
                        self.convert_block(block);
                        self.output.push('\n');
                    } else {
                        // 无实现：只输出签名+分号
                        self.writeln(";");
                    }
                }
                syn::TraitItem::Type(assoc_type) => {
                    // 关联类型: type Output: 'a; → t Output: 'a;
                    self.write(&self.indent());
                    self.write("t ");
                    self.write(&assoc_type.ident.to_string());

                    // 处理类型约束 (如 : 'a)
                    if !assoc_type.bounds.is_empty() {
                        self.write(": ");
                        let bounds_str = self.convert_type_param_bounds(&assoc_type.bounds);
                        self.write(&bounds_str);
                    }

                    self.writeln(";");
                }
                syn::TraitItem::Const(const_item) => {
                    // Trait关联常量: const PI: f64 = 3.14159;
                    self.write(&self.indent());
                    self.write("C ");
                    self.write(&const_item.ident.to_string());
                    self.write(": ");
                    self.write(&self.convert_type(&const_item.ty));

                    // 检查是否有默认值
                    if let Some((_, expr)) = &const_item.default {
                        self.write(" = ");
                        self.write(&expr.to_token_stream().to_string());
                    }

                    self.writeln(";");
                }
                _ => {
                    // 忽略其他trait item类型
                }
            }
        }

        self.indent_level -= 1;
        self.writeln("}");
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        // 进入泛型作用域，记录impl的泛型参数
        self.push_generic_scope(&node.generics);

        // Nu v1.6.3: 保留 #[cfg] 属性
        for attr in &node.attrs {
            let attr_str = attr.to_token_stream().to_string();
            // to_token_stream()会在#、[、(、)周围插入空格，需要移除
            let cleaned_attr = attr_str
                .replace("# [", "#[")
                .replace(" [", "[")
                .replace(" ]", "]")
                .replace(" (", "(")
                .replace(" )", ")")
                .replace(" ,", ",");
            if cleaned_attr.starts_with("#[cfg") {
                self.writeln(&cleaned_attr);
            }
        }

        // v1.8: unsafe impl -> unsafe I (不缩写 unsafe，因为太重要且易与 use 混淆)
        if node.unsafety.is_some() {
            self.write("unsafe ");
        }

        // v1.7.6: impl -> I (per README.md spec)
        self.write("I");

        // v1.6.5: 泛型（完整保留生命周期）
        if !node.generics.params.is_empty() {
            self.write(&self.convert_generics(&node.generics));
        }

        self.write(" ");

        // trait实现
        if let Some((_, path, _)) = &node.trait_ {
            self.write(&path.to_token_stream().to_string());
            self.write(" for ");
        }

        self.write(&self.convert_type(&node.self_ty));

        // where子句 - 保留trait约束
        if let Some(where_clause) = &node.generics.where_clause {
            self.write(" wh ");
            self.write(
                &where_clause
                    .to_token_stream()
                    .to_string()
                    .replace("where", "")
                    .trim(),
            );
        }

        self.writeln(" {");
        self.indent_level += 1;

        for item in &node.items {
            match item {
                syn::ImplItem::Fn(method) => {
                    // 输出方法的 #[cfg] 属性
                    for attr in &method.attrs {
                        let attr_str = attr.to_token_stream().to_string();
                        // to_token_stream()会在#、[、(、)周围插入空格，需要移除
                        let cleaned_attr = attr_str
                            .replace("# [", "#[")
                            .replace(" [", "[")
                            .replace(" ]", "]")
                            .replace(" (", "(")
                            .replace(" )", ")")
                            .replace(" ,", ",");
                        if cleaned_attr.starts_with("#[cfg") {
                            self.write(&self.indent());
                            self.writeln(&cleaned_attr);
                        }
                    }

                    let sig_str = self.convert_fn_signature(&method.sig, &method.vis);
                    self.write(&self.indent());
                    self.write(&sig_str);
                    self.convert_block(&method.block);
                    self.output.push('\n');
                }
                syn::ImplItem::Type(type_item) => {
                    // 转换关联类型: type Value = Level; → t Value = Level;
                    self.write(&self.indent());
                    self.write("t ");
                    self.write(&type_item.ident.to_string());
                    self.write(" = ");
                    self.write(&self.convert_type(&type_item.ty));
                    self.writeln(";");
                }
                syn::ImplItem::Const(const_item) => {
                    // 处理 const 声明
                    self.write(&self.indent());
                    self.write("C ");
                    self.write(&const_item.ident.to_string());
                    self.write(": ");
                    self.write(&self.convert_type(&const_item.ty));
                    self.write(" = ");
                    self.write(&const_item.expr.to_token_stream().to_string());
                    self.writeln(";");
                }
                _ => {
                    // 其他类型的impl item暂时保持原样
                }
            }
        }

        self.indent_level -= 1;
        self.writeln("}");

        // 退出泛型作用域
        self.pop_generic_scope();
    }
}

impl Default for Rust2NuConverter {
    fn default() -> Self {
        Self::new()
    }
}
