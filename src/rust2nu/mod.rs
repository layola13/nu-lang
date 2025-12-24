// Rust to Nu Converter
// 将标准Rust代码压缩为Nu高密度语法

use anyhow::{Context, Result};
use quote::ToTokens;
use syn::{
    visit::Visit, Attribute, Block, Expr, File, FnArg, Item, ItemEnum, ItemFn, ItemImpl,
    ItemStruct, ItemTrait, ReturnType, Signature, Stmt, Type, Visibility,
};

pub struct Rust2NuConverter {
    output: String,
    indent_level: usize,
}

impl Rust2NuConverter {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent_level: 0,
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

        // 如果全是注释，直接返回
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

        // 泛型参数保持不变
        if !sig.generics.params.is_empty() {
            result.push_str(&sig.generics.to_token_stream().to_string());
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

    /// 转换类型 - 保留泛型参数中的类型标注
    fn convert_type(&self, ty: &Type) -> String {
        let type_str = ty.to_token_stream().to_string();

        // 替换常见类型，注意处理泛型参数
        type_str
            .replace("String", "Str")
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
            .replace(" >", ">")
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
            Stmt::Macro(mac) => {
                // v1.6: 宏语句原样保留（println!, vec!, assert!, etc.）
                // 移除to_token_stream()插入的空格（"println !" -> "println!"）
                self.write(&self.indent());
                let macro_str = mac
                    .mac
                    .to_token_stream()
                    .to_string()
                    .replace(" !", "!") // 修复宏名和!之间的空格
                    .replace(" (", "(") // 修复!和(之间的空格
                    .replace(" ,", ","); // 修复参数逗号前的空格
                self.write(&macro_str);
                if mac.semi_token.is_some() {
                    self.write(";");
                }
                self.write("\n");
            }
            Stmt::Expr(expr, semi) => {
                // 处理return
                if let Expr::Return(ret) = expr {
                    self.write(&self.indent());
                    self.write("< ");
                    if let Some(val) = &ret.expr {
                        self.write(&self.convert_expr(val));
                    }
                    self.write("\n");
                } else if let Expr::Macro(_mac) = expr {
                    // v1.6: 表达式宏原样保留（极少见，大多数宏是Stmt::Macro）
                    self.write(&self.indent());
                    let macro_str = expr
                        .to_token_stream()
                        .to_string()
                        .replace(" !", "!")
                        .replace(" (", "(")
                        .replace(" ,", ",");
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
                // match表达式保持换行结构
                let scrutinee = self.convert_expr(&match_expr.expr);
                let mut result = format!("match {} {{\n", scrutinee);
                for arm in &match_expr.arms {
                    result.push_str("        ");
                    result.push_str(&arm.pat.to_token_stream().to_string());
                    if let Some((_, guard)) = &arm.guard {
                        result.push_str(" if ");
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
                // if表达式保持换行
                let cond = self.convert_expr(&if_expr.cond);
                let mut result = format!("if {} {{\n", cond);
                // then分支保持单独行
                for stmt in &if_expr.then_branch.stmts {
                    result.push_str("        ");
                    result.push_str(&stmt.to_token_stream().to_string());
                    result.push('\n');
                }
                result.push_str("    }");

                // else分支
                if let Some((_, else_branch)) = &if_expr.else_branch {
                    result.push_str(" else ");
                    result.push_str(&self.convert_expr(else_branch));
                }
                self.convert_type_in_string(&result)
            }
            Expr::Block(block_expr) => {
                // 块表达式保持换行
                let mut result = String::from("{\n");
                for stmt in &block_expr.block.stmts {
                    result.push_str("        ");
                    result.push_str(&stmt.to_token_stream().to_string());
                    result.push('\n');
                }
                result.push_str("    }");
                self.convert_type_in_string(&result)
            }
            Expr::ForLoop(for_loop) => {
                // for循环保持换行
                let pat = for_loop.pat.to_token_stream().to_string();
                let iter = self.convert_expr(&for_loop.expr);
                let mut result = format!("for {} in {} {{\n", pat, iter);
                for stmt in &for_loop.body.stmts {
                    result.push_str("        ");
                    result.push_str(&stmt.to_token_stream().to_string());
                    result.push('\n');
                }
                result.push_str("    }");
                self.convert_type_in_string(&result)
            }
            Expr::While(while_expr) => {
                // while循环保持换行
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
                // loop保持换行
                let mut result = String::from("loop {\n");
                for stmt in &loop_expr.body.stmts {
                    result.push_str("        ");
                    result.push_str(&stmt.to_token_stream().to_string());
                    result.push('\n');
                }
                result.push_str("    }");
                self.convert_type_in_string(&result)
            }
            _ => {
                // 默认：保持原样但替换类型
                let expr_str = expr.to_token_stream().to_string();
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

        // 执行类型替换
        result = result
            .replace("String", "Str")
            .replace("Vec", "V")
            .replace("Option", "O")
            .replace("Result", "R")
            .replace("Arc", "A")
            .replace("Mutex", "X")
            .replace("Box", "B")
            .replace("& mut", "&!");

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
                // 处理module，特别是#[cfg(test)] mod tests
                for attr in &m.attrs {
                    self.writeln(&self.convert_attribute(attr));
                }

                // Nu v1.5.1: D=mod（移除了 M/m）
                // 可见性由标识符首字母决定（Go风格）
                self.write("D");
                self.write(" ");
                self.write(&m.ident.to_string());

                if let Some((_, items)) = &m.content {
                    self.writeln(" {");
                    self.indent_level += 1;
                    for item in items {
                        self.visit_item(item);
                        self.output.push('\n');
                    }
                    self.indent_level -= 1;
                    self.writeln("}");
                } else {
                    self.writeln(";");
                }
            }
            Item::Use(u) => {
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
                self.write("ST ");
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
            if let syn::ImplItem::Fn(method) = item {
                let sig_str = self.convert_fn_signature(&method.sig, &method.vis);
                self.write(&self.indent());
                self.write(&sig_str);
                self.convert_block(&method.block);
                self.output.push('\n');
            }
        }

        self.indent_level -= 1;
        self.writeln("}");
    }
}

impl Default for Rust2NuConverter {
    fn default() -> Self {
        Self::new()
    }
}
