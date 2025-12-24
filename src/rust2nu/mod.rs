// Rust to Nu Converter
// 将标准Rust代码压缩为Nu高密度语法

use anyhow::{Context, Result};
use quote::ToTokens;
use std::collections::{HashMap, HashSet};
use syn::{
    visit::Visit, Attribute, Block, Expr, File, FnArg, Item, ItemEnum, ItemFn, ItemImpl,
    ItemStruct, ItemTrait, ReturnType, Signature, Stmt, Type, Visibility,
};

pub struct Rust2NuConverter {
    output: String,
    indent_level: usize,
    // 泛型作用域栈：跟踪当前作用域中的泛型参数名
    // 用于避免将泛型参数（如impl<S>中的S）误转换为类型缩写
    generic_scope_stack: Vec<HashSet<String>>,
}

impl Rust2NuConverter {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent_level: 0,
            generic_scope_stack: Vec::new(),
        }
    }

    pub fn convert(&self, rust_code: &str) -> Result<String> {
        // 策略：混合处理 - 保留注释行，转换代码行
        // 1. 先提取所有注释和它们的位置
        let lines: Vec<&str> = rust_code.lines().collect();
        let mut line_types = Vec::new(); // true = comment line, false = code line

        for line in &lines {
            let trimmed = line.trim();
            // 判断是否为纯注释行或空行
            let is_comment_or_empty = trimmed.is_empty()
                || trimmed.starts_with("//")
                || trimmed.starts_with("/*")
                || trimmed.starts_with("*");
            line_types.push(is_comment_or_empty);
        }

        // 2. 解析并转换代码（syn会忽略注释）
        let syntax_tree = syn::parse_file(rust_code).context("Failed to parse Rust code")?;

        let mut converter = Self::new();
        converter.visit_file(&syntax_tree);
        let converted_code = converter.output;

        // 3. 合并：在转换后的代码中插入注释
        // 简单策略：在文件开头保留所有前导注释，然后是转换后的代码
        let mut output = String::new();
        let mut found_code = false;

        for (i, line) in lines.iter().enumerate() {
            if line_types[i] {
                // 注释或空行
                if !found_code {
                    // 文件开头的注释，直接保留
                    output.push_str(line);
                    output.push('\n');
                }
            } else {
                // 遇到第一行代码
                if !found_code {
                    found_code = true;
                    output.push_str(&converted_code);
                }
                break;
            }
        }

        // 如果全是注释但转换后有内容，输出转换内容
        if !found_code && !converted_code.is_empty() {
            return Ok(converted_code);
        }
        
        // 如果全是注释且转换后也是空的，返回注释
        if !found_code {
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
        matches!(vis, Visibility::Public(_))
    }

    /// 检查名称是否是当前作用域中的泛型参数
    fn is_generic_param(&self, name: &str) -> bool {
        self.generic_scope_stack
            .iter()
            .any(|scope| scope.contains(name))
    }

    /// 进入泛型作用域，记录泛型参数名
    fn push_generic_scope(&mut self, generics: &syn::Generics) {
        let mut scope = HashSet::new();
        for param in &generics.params {
            if let syn::GenericParam::Type(type_param) = param {
                scope.insert(type_param.ident.to_string());
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

        // async函数用 ~ 前缀
        if sig.asyncness.is_some() {
            result.push('~');
        }

        // pub fn -> F, fn -> f
        result.push_str(if self.is_public(vis) { "F" } else { "f" });

        result.push(' ');
        result.push_str(&sig.ident.to_string());

        // 泛型参数保持不变，但需要清理空格
        if !sig.generics.params.is_empty() {
            let generics_str = sig.generics.to_token_stream().to_string();
            // 清理泛型中的多余空格
            let cleaned = generics_str
                .replace(" <", "<")
                .replace("< ", "<")
                .replace(" >", ">")
                .replace(" ,", ",");
            result.push_str(&cleaned);
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
                    if r.reference.is_some() {
                        result.push('&');
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
        if let Some(where_clause) = &sig.generics.where_clause {
            result.push_str(" wh ");
            result.push_str(
                &where_clause
                    .to_token_stream()
                    .to_string()
                    .replace("where", ""),
            );
        }

        result
    }

    /// 转换类型 - 保留泛型参数中的类型标注，避免将泛型参数误转换
    fn convert_type(&self, ty: &Type) -> String {
        let type_str = ty.to_token_stream().to_string();

        // 检查是否是单个泛型参数（如 "S", "D", "A" 等）
        // 如果是当前作用域中的泛型参数，则不进行转换
        let trimmed = type_str.trim();
        if trimmed.len() == 1 && self.is_generic_param(trimmed) {
            // 这是一个泛型参数，保持原样
            return trimmed.to_string();
        }

        // 检查是否包含泛型参数（如 "S::Error"）
        // 如果类型路径的第一段是泛型参数，则保持整个路径不转换
        if let Some(first_segment) = trimmed.split("::").next() {
            if self.is_generic_param(first_segment) {
                // 路径以泛型参数开头，保持原样
                return type_str.clone();
            }
        }

        // 替换常见类型，注意处理泛型参数
        // v1.7: String不再缩写为Str
        let result = type_str
            .replace("Vec <", "V<")
            .replace("Vec<", "V<")
            .replace("Option <", "O<")
            .replace("Option<", "O<")
            .replace("Result <", "R<")
            .replace("Result<", "R<")
            .replace("Arc <", "A<")
            .replace("Arc<", "A<")
            .replace("Mutex <", "X<")
            .replace("Mutex<", "X<")
            .replace("Box <", "B<")
            .replace("Box<", "B<")
            .replace("& mut", "&!")
            .replace(" mut", "!")
            .replace(" >", ">");
        
        // 清理多余空格: 移除 :: < > , 周围的空格
        result
            .replace(" :: ", "::")
            .replace(" ::", "::")
            .replace(":: ", "::")
            .replace(" <", "<")
            .replace("< ", "<")  // 移除 < 后的空格
            .replace(" >", ">")
            .replace(" ,", ",")
    }

    /// 转换语句
    fn convert_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Local(local) => {
                self.write(&self.indent());

                // let vs let mut
                let pat_str = local.pat.to_token_stream().to_string();
                let is_mut = pat_str.contains("mut");

                if local.init.is_some() {
                    self.write(if is_mut { "v " } else { "l " });

                    // 变量名（去掉mut）
                    let clean_pat = pat_str.replace("mut ", "");
                    self.write(&clean_pat);

                    self.write(" = ");
                    if let Some(init) = &local.init {
                        self.write(&self.convert_expr(&init.expr));
                    }
                }

                self.write(";\n");
            }
            Stmt::Expr(expr, semi) => {
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
                
                // 原有的return和macro处理...
                if let Expr::Return(ret) = expr {
                    self.write(&self.indent());
                    self.write("< ");
                    if let Some(val) = &ret.expr {
                        self.write(&self.convert_expr(val));
                    }
                    self.write("\n");
                } else if let Expr::Macro(_mac) = expr {
                    self.write(&self.indent());
                    let macro_str = expr
                        .to_token_stream()
                        .to_string()
                        .replace(" !", "!")
                        .replace(" (", "(")
                        .replace(" ,", ",")
                        .replace("vec!", "V!");  // vec! -> V!
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
                // 移除to_token_stream()插入的空格（"println !" -> "println!"）
                self.write(&self.indent());
                let macro_str = mac
                    .mac
                    .to_token_stream()
                    .to_string()
                    .replace(" !", "!") // 修复宏名和!之间的空格
                    .replace(" (", "(") // 修复!和(之间的空格
                    .replace(" ,", ",") // 修复参数逗号前的空格
                    .replace("vec!", "V!");  // vec! -> V!
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
                    turbo.to_token_stream().to_string()
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
                    .map(|p| p.to_token_stream().to_string())
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
                    result.push_str(&arm.pat.to_token_stream().to_string());
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
                            result.push_str(&stmt.to_token_stream().to_string());
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
                            if let Some(val) = &ret.expr {
                                format!("< {}", self.convert_expr(val))
                            } else {
                                String::from("<")
                            }
                        }
                        _ => stmt.to_token_stream().to_string(),
                    };
                    result.push_str(&stmt_str);
                    result.push('\n');
                }
                result.push_str("    }");
                self.convert_type_in_string(&result)
            }
            Expr::ForLoop(for_loop) => {
                // L = for
                let pat = for_loop.pat.to_token_stream().to_string();
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
                            let stmt_str = stmt.to_token_stream().to_string().replace("vec!", "V!");
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
                    result.push_str(&stmt.to_token_stream().to_string());
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
                            let stmt_str = stmt.to_token_stream().to_string().replace("vec!", "V!");
                            result.push_str(&stmt_str);
                            result.push(' ');
                        }
                    }
                }
                result.push('}');
                self.convert_type_in_string(&result)
            }
            Expr::Break(_) => {
                String::from("br")
            }
            Expr::Continue(_) => {
                String::from("ct")
            }
            _ => {
                // 默认：保持原样但替换类型和vec!宏
                let expr_str = expr.to_token_stream().to_string().replace("vec!", "V!");
                self.convert_type_in_string(&expr_str)
            }
        }
    }

    fn convert_type_in_string(&self, s: &str) -> String {
        // 智能类型替换：保护 turbofish 语法中的类型标注（如 collect::<String>()）
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

        // 执行类型替换和宏替换
        // v1.7: String不再缩写为Str
        result = result
            .replace("Vec", "V")
            .replace("Option", "O")
            .replace("Result", "R")
            .replace("Arc", "A")
            .replace("Mutex", "X")
            .replace("Box", "B")
            .replace("& mut", "&!")
            .replace("vec!", "V!");  // vec! -> V!

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

    fn convert_attribute(&self, attr: &Attribute) -> String {
        let path = attr.path().to_token_stream().to_string();
        let tokens = attr.meta.to_token_stream().to_string();

        if path == "derive" {
            format!("#D{}", tokens.trim_start_matches("derive"))
        } else {
            // 保持其他属性的完整格式，不要过度简化
            format!("#[{}]", tokens)
        }
    }
}

impl<'ast> Visit<'ast> for Rust2NuConverter {
    fn visit_file(&mut self, node: &'ast File) {
        // Nu v1.6.3: 优先输出文件级属性 #![...]
        for attr in &node.attrs {
            let attr_str = attr.to_token_stream().to_string();
            if attr_str.starts_with("#![") {
                self.writeln(&attr_str);
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
            Item::Mod(m) => {
                // Nu v1.6.3: 保留 #[cfg] 属性
                for attr in &m.attrs {
                    let attr_str = attr.to_token_stream().to_string();
                    // to_token_stream()会在#和[之间插入空格，需要移除
                    let cleaned_attr = attr_str.replace("# [", "#[").replace(" ]", "]");
                    if cleaned_attr.starts_with("#[cfg") {
                        self.writeln(&cleaned_attr);
                    }
                }

                // Nu v1.6.3: DM=pub mod, D=mod
                let keyword = if self.is_public(&m.vis) {
                    "DM"
                } else {
                    "D"
                };
                
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
                // Nu v1.6.3: 保留 #[cfg] 属性
                for attr in &u.attrs {
                    let attr_str = attr.to_token_stream().to_string();
                    // to_token_stream()会在#和[之间插入空格，需要移除
                    let cleaned_attr = attr_str.replace("# [", "#[").replace(" ]", "]");
                    if cleaned_attr.starts_with("#[cfg") {
                        self.writeln(&cleaned_attr);
                    }
                }
                
                let use_str = u.to_token_stream().to_string();
                let nu_use = if self.is_public(&u.vis) {
                    use_str.replace("pub use", "U").replace("use", "U")
                } else {
                    use_str.replace("use", "u")
                };
                self.writeln(&nu_use);
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
        
        // Nu v1.6.3: 保留 #[cfg] 属性
        for attr in &node.attrs {
            let attr_str = attr.to_token_stream().to_string();
            // to_token_stream()会在#和[之间插入空格，需要移除
            let cleaned_attr = attr_str.replace("# [", "#[").replace(" ]", "]");
            if cleaned_attr.starts_with("#[cfg") {
                self.writeln(&cleaned_attr);
            }
        }
        
        // Nu v1.5.1: 只有 S（移除了 s）
        // 可见性由标识符首字母决定（Go风格）
        self.write("S");
        self.write(" ");
        self.write(&node.ident.to_string());

        // 泛型
        if !node.generics.params.is_empty() {
            self.write(&node.generics.to_token_stream().to_string());
        }

        // 字段
        match &node.fields {
            syn::Fields::Named(fields) => {
                self.writeln(" {");
                self.indent_level += 1;
                for field in &fields.named {
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
            _ => {
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

        // Nu v1.5.1: 只有 E（移除了 e）
        // 可见性由标识符首字母决定（Go风格）
        self.write("E");
        self.write(" ");
        self.write(&node.ident.to_string());

        if !node.generics.params.is_empty() {
            self.write(&node.generics.to_token_stream().to_string());
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
        let keyword = if self.is_public(&node.vis) {
            "TR"
        } else {
            "tr"
        };

        self.write(keyword);
        self.write(" ");
        self.write(&node.ident.to_string());

        if !node.generics.params.is_empty() {
            self.write(&node.generics.to_token_stream().to_string());
        }

        self.writeln(" {");
        self.indent_level += 1;

        for item in &node.items {
            if let syn::TraitItem::Fn(method) = item {
                let sig_str = self.convert_fn_signature(&method.sig, &Visibility::Inherited);
                self.write(&self.indent());
                self.write(&sig_str);
                self.writeln(";");
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
            // to_token_stream()会在#和[之间插入空格，需要移除
            let cleaned_attr = attr_str.replace("# [", "#[").replace(" ]", "]");
            if cleaned_attr.starts_with("#[cfg") {
                self.writeln(&cleaned_attr);
            }
        }
        
        // Nu v1.6.3: U I = unsafe impl
        if node.unsafety.is_some() {
            self.write("U ");
        }
        
        self.write("I");

        // 泛型
        if !node.generics.params.is_empty() {
            self.write(&node.generics.to_token_stream().to_string());
        }

        self.write(" ");

        // trait实现
        if let Some((_, path, _)) = &node.trait_ {
            self.write(&path.to_token_stream().to_string());
            self.write(" for ");
        }

        self.write(&self.convert_type(&node.self_ty));

        self.writeln(" {");
        self.indent_level += 1;

        for item in &node.items {
            match item {
                syn::ImplItem::Fn(method) => {
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

