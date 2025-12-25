// Nu2TS Codegen (完整版)
// 将 AST 转换为 TypeScript 代码

use super::ast::*;
use super::types::TsConfig;
use anyhow::Result;

pub struct TsCodegen {
    config: TsConfig,
    output: String,
    indent: usize,
    temp_counter: usize,
}

impl TsCodegen {
    pub fn new(config: TsConfig) -> Self {
        Self {
            config,
            output: String::new(),
            indent: 0,
            temp_counter: 0,
        }
    }

    /// 生成整个文件
    pub fn generate_file(&mut self, file: &NuFile) -> Result<String> {
        // 生成 runtime import
        self.emit_runtime_import();
        self.writeln("");

        // 生成所有项目
        for item in &file.items {
            self.emit_item(item)?;
            self.writeln("");
        }

        Ok(self.output.clone())
    }

    /// 向后兼容的 Stmt 列表生成
    pub fn generate(&mut self, stmts: &[Stmt]) -> Result<String> {
        self.emit_runtime_import();
        self.writeln("");

        for stmt in stmts {
            self.emit_stmt(stmt)?;
            self.writeln("");
        }

        Ok(self.output.clone())
    }

    fn emit_runtime_import(&mut self) {
        use super::runtime::generate_runtime_import;
        use super::types::RuntimeMode;

        if self.config.runtime_mode == RuntimeMode::Import {
            self.write(generate_runtime_import());
            self.writeln("");
        }
    }

    // ============ Item 生成 ============

    fn emit_item(&mut self, item: &Item) -> Result<()> {
        match item {
            Item::Use { path, .. } => {
                self.emit_use(path)?;
            }
            Item::Function(f) => {
                self.emit_function(f)?;
            }
            Item::Struct(s) => {
                self.emit_struct(s)?;
            }
            Item::Enum(e) => {
                self.emit_enum(e)?;
            }
            Item::Impl(i) => {
                self.emit_impl(i)?;
            }
            Item::Mod(m) => {
                self.emit_mod(m)?;
            }
            Item::Stmt(s) => {
                // 顶层语句（如Match表达式）
                self.emit_stmt(s)?;
            }
            Item::Raw(s) => {
                // 透传行：尽量转换
                self.emit_raw_line(s)?;
            }
        }
        Ok(())
    }

    fn emit_use(&mut self, path: &str) -> Result<()> {
        // use 声明 -> TypeScript import
        // 简化：注释掉 use
        self.writeln(&format!("// use {}", path));
        Ok(())
    }

    fn emit_function(&mut self, f: &FunctionDef) -> Result<()> {
        let export = if f.is_pub { "export " } else { "" };
        let asyncc = if f.is_async { "async " } else { "" };

        self.write(&format!("{}{}function {}(", export, asyncc, f.name));

        // 参数
        for (i, param) in f.params.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            let ref_prefix = if param.is_ref {
                if param.is_mut { "/* &mut */ " } else { "/* & */ " }
            } else {
                ""
            };
            self.write(&format!("{}{}: {}", ref_prefix, param.name, self.type_to_ts(&param.ty)));
        }

        self.write(")");

        // 返回类型
        if let Some(ret_ty) = &f.return_type {
            self.write(&format!(": {}", self.type_to_ts(ret_ty)));
        }

        self.writeln(" {");
        self.indent += 1;

        // 函数体
        self.emit_block_body(&f.body)?;

        self.indent -= 1;
        self.write_indent();
        self.writeln("}");

        Ok(())
    }

    fn emit_struct(&mut self, s: &StructDef) -> Result<()> {
        self.writeln(&format!("export interface {} {{", s.name));
        self.indent += 1;

        for field in &s.fields {
            self.write_indent();
            self.writeln(&format!("{}: {};", field.name, self.type_to_ts(&field.ty)));
        }

        self.indent -= 1;
        self.writeln("}");

        Ok(())
    }

    fn emit_enum(&mut self, e: &EnumDef) -> Result<()> {
        // TypeScript: 使用 tagged union
        self.writeln(&format!("// Enum: {}", e.name));

        // 生成每个变体的类型
        for variant in &e.variants {
            if let Some(_fields) = &variant.fields {
                self.writeln(&format!(
                    "export type {}_{} = {{ tag: '{}', value: any }};",
                    e.name, variant.name, variant.name.to_lowercase()
                ));
            } else {
                self.writeln(&format!(
                    "export type {}_{} = {{ tag: '{}' }};",
                    e.name, variant.name, variant.name.to_lowercase()
                ));
            }
        }

        // 生成联合类型
        let variant_types: Vec<String> = e.variants.iter()
            .map(|v| format!("{}_{}", e.name, v.name))
            .collect();
        self.writeln(&format!(
            "export type {} = {};",
            e.name,
            variant_types.join(" | ")
        ));

        // 生成构造函数
        for variant in &e.variants {
            if let Some(_fields) = &variant.fields {
                self.writeln(&format!(
                    "export const {}_{} = (value: any): {}_{} => ({{ tag: '{}', value }});",
                    e.name, variant.name, e.name, variant.name, variant.name.to_lowercase()
                ));
            } else {
                self.writeln(&format!(
                    "export const {}_{}: {}_{} = {{ tag: '{}' }};",
                    e.name, variant.name, e.name, variant.name, variant.name.to_lowercase()
                ));
            }
        }

        Ok(())
    }

    fn emit_impl(&mut self, i: &ImplDef) -> Result<()> {
        self.writeln(&format!("// impl {}", i.target));
        self.writeln(&format!("export namespace {} {{", i.target));
        self.indent += 1;

        for method in &i.methods {
            self.emit_function(method)?;
            self.writeln("");
        }

        self.indent -= 1;
        self.writeln("}");

        Ok(())
    }

    fn emit_mod(&mut self, m: &ModDef) -> Result<()> {
        self.writeln(&format!("// mod {}", m.name));
        self.writeln(&format!("export namespace {} {{", m.name));
        self.indent += 1;

        for item in &m.items {
            self.emit_item(item)?;
        }

        self.indent -= 1;
        self.writeln("}");

        Ok(())
    }

    fn emit_raw_line(&mut self, line: &str) -> Result<()> {
        // 尝试转换常见模式
        let trimmed = line.trim();

        // 跳过属性
        if trimmed.starts_with("#[") || trimmed.starts_with("#D") {
            return Ok(());
        }

        // 注释透传
        if trimmed.starts_with("//") {
            self.write_indent();
            self.writeln(trimmed);
            return Ok(());
        }

        // 其他行作为注释
        if !trimmed.is_empty() {
            self.write_indent();
            self.writeln(&format!("// RAW: {}", trimmed));
        }

        Ok(())
    }

    // ============ 语句生成 ============

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::Let { name, ty, value, is_mut } => {
                self.emit_let(name, ty, value, *is_mut)?;
            }
            Stmt::ExprStmt(expr) => {
                self.write_indent();
                self.emit_expr(expr)?;
                if !self.is_block_expr(expr) {
                    self.write(";");
                }
                self.writeln("");
            }
            Stmt::Raw(s) => {
                self.emit_raw_line(s)?;
            }
        }
        Ok(())
    }

    fn emit_let(&mut self, name: &str, ty: &Option<Type>, value: &Expr, _is_mut: bool) -> Result<()> {
        self.write_indent();
        if let Some(t) = ty {
            self.write(&format!("const {}: {} = ", name, self.type_to_ts(t)));
        } else {
            self.write(&format!("const {} = ", name));
        }
        self.emit_expr(value)?;
        self.writeln(";");
        Ok(())
    }

    // ============ 块体生成 ============

    fn emit_block_body(&mut self, expr: &Expr) -> Result<()> {
        match expr {
            Expr::Block { stmts, trailing_expr } => {
                for stmt in stmts {
                    self.emit_stmt(stmt)?;
                }
                if let Some(e) = trailing_expr {
                    self.write_indent();
                    self.emit_expr(e)?;
                    if !self.is_block_expr(e) {
                        self.write(";");
                    }
                    self.writeln("");
                }
            }
            _ => {
                self.write_indent();
                self.emit_expr(expr)?;
                if !self.is_block_expr(expr) {
                    self.write(";");
                }
                self.writeln("");
            }
        }
        Ok(())
    }

    fn is_block_expr(&self, expr: &Expr) -> bool {
        matches!(expr, Expr::Match { .. } | Expr::If { .. } | Expr::Loop { .. } | Expr::For { .. } | Expr::Block { .. })
    }

    // ============ 表达式生成 ============

    fn emit_expr(&mut self, expr: &Expr) -> Result<()> {
        match expr {
            Expr::Match { target, arms } => {
                self.emit_match(target, arms)?;
            }
            Expr::If { condition, then_body, else_body } => {
                self.emit_if(condition, then_body, else_body)?;
            }
            Expr::For { pattern, iterator, body } => {
                self.emit_for(pattern, iterator, body)?;
            }
            Expr::Loop { body } => {
                self.write("while (true) {");
                self.writeln("");
                self.indent += 1;
                self.emit_block_body(body)?;
                self.indent -= 1;
                self.write_indent();
                self.write("}");
            }
            Expr::Return(value) => {
                self.write("return");
                if let Some(val) = value {
                    self.write(" ");
                    self.emit_expr(val)?;
                }
            }
            Expr::Break => {
                self.write("break");
            }
            Expr::Continue => {
                self.write("continue");
            }
            Expr::TryOp { expr } => {
                self.write("$unwrap(");
                self.emit_expr(expr)?;
                self.write(")");
            }
            Expr::Call { func, args } => {
                self.emit_expr(func)?;
                self.write("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_expr(arg)?;
                }
                self.write(")");
            }
            Expr::MethodCall { object, method, args } => {
                self.emit_expr(object)?;
                self.write(&format!(".{}(", method));
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_expr(arg)?;
                }
                self.write(")");
            }
            Expr::Field { object, field } => {
                self.emit_expr(object)?;
                self.write(&format!(".{}", field));
            }
            Expr::Index { object, index } => {
                self.emit_expr(object)?;
                self.write("[");
                self.emit_expr(index)?;
                self.write("]");
            }
            Expr::Binary { left, op, right } => {
                self.emit_expr(left)?;
                self.write(&format!(" {} ", self.binop_to_ts(*op)));
                self.emit_expr(right)?;
            }
            Expr::Unary { op, expr } => {
                self.write(self.unop_to_ts(*op));
                self.emit_expr(expr)?;
            }
            Expr::Block { stmts, trailing_expr } => {
                for stmt in stmts {
                    self.emit_stmt(stmt)?;
                }
                if let Some(e) = trailing_expr {
                    self.emit_expr(e)?;
                }
            }
            Expr::Closure { params, return_type, body, is_move } => {
                if *is_move {
                    self.write("/* move */ ");
                }
                self.write("(");
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&format!("{}: {}", param.name, self.type_to_ts(&param.ty)));
                }
                self.write(")");
                if let Some(ret) = return_type {
                    self.write(&format!(": {}", self.type_to_ts(ret)));
                }
                self.write(" => ");
                self.emit_expr(body)?;
            }
            Expr::StructInit { name, fields } => {
                // 清理名称中的空格
                let clean_name = name.trim().replace(" ", "");
                self.write("{ ");
                for (i, (fname, fval)) in fields.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&format!("{}: ", fname.trim()));
                    self.emit_expr(fval)?;
                }
                self.write(" }");
            }
            Expr::EnumVariant { enum_name, variant, args } => {
                if let Some(arg_list) = args {
                    self.write(&format!("{}(", variant));
                    for (i, arg) in arg_list.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.emit_expr(arg)?;
                    }
                    self.write(")");
                } else {
                    self.write(&format!("{}_{}", enum_name, variant));
                }
            }
            Expr::Macro { name, args } => {
                self.emit_macro(name, args)?;
            }
            Expr::Path { segments } => {
                // 移除路径中的空格
                let path = segments.iter()
                    .map(|s| s.trim())
                    .collect::<Vec<_>>()
                    .join(".");
                self.write(&path);
            }
            Expr::Ident(name) => {
                // 清理空格，并尝试识别函数调用模式
                // TODO: fix clean_expr_string method signature mismatch
                // let cleaned = self.clean_expr_string(name);
                // self.write(&cleaned);
                self.write(name);
            }
            Expr::Literal(lit) => {
                self.write(&self.literal_to_ts(lit));
            }
            Expr::Tuple(exprs) => {
                self.write("[");
                for (i, e) in exprs.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_expr(e)?;
                }
                self.write("]");
            }
            Expr::Array(exprs) => {
                self.write("[");
                for (i, e) in exprs.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_expr(e)?;
                }
                self.write("]");
            }
            Expr::Raw(s) => {
                self.write(&format!("/* {} */", s));
            }
        }
        Ok(())
    }

    // ============ Match 生成 ============

    fn emit_match(&mut self, target: &Expr, arms: &[MatchArm]) -> Result<()> {
        let temp = format!("_m{}", self.temp_counter);
        self.temp_counter += 1;

        self.write(&format!("const {} = ", temp));
        self.emit_expr(target)?;
        self.writeln(";");

        for (i, arm) in arms.iter().enumerate() {
            let prefix = if i == 0 { "if" } else { "else if" };

            let condition = self.pattern_to_condition(&temp, &arm.pattern);
            self.write_indent();
            self.write(&format!("{} ({}) {{", prefix, condition));
            self.writeln("");
            self.indent += 1;

            // 变量绑定
            if let Some(binding) = self.pattern_binding(&temp, &arm.pattern) {
                self.write_indent();
                self.writeln(&binding);
            }

            // 分支体 - 添加 return
            self.write_indent();
            // 如果不是块表达式，添加 return
            if !self.is_block_expr(&arm.body) {
                self.write("return ");
            }
            self.emit_expr(&arm.body)?;
            self.writeln(";");

            self.indent -= 1;
            self.write_indent();
            self.writeln("}");
        }

        Ok(())
    }

    fn pattern_to_condition(&self, temp: &str, pattern: &Pattern) -> String {
        match pattern {
            Pattern::ResultOk(_) => format!("{}.tag === 'ok'", temp),
            Pattern::ResultErr(_) => format!("{}.tag === 'err'", temp),
            Pattern::OptionSome(_) => format!("{} !== null && {} !== undefined", temp, temp),
            Pattern::OptionNone => format!("{} === null || {} === undefined", temp, temp),
            Pattern::Wildcard => "true".to_string(),
            Pattern::Literal(lit) => format!("{} === {}", temp, self.literal_to_ts(lit)),
            Pattern::Ident(_) => "true".to_string(),
            Pattern::EnumVariant { path, .. } => {
                // 提取变体名
                let variant = path.split("::").last().unwrap_or(path);
                // 移除括号部分和空格
                let variant_name = variant.split('(').next().unwrap_or(variant).trim();
                format!("{}.tag === '{}'", temp, variant_name.to_lowercase().replace(" ", ""))
            }
        }
    }

    fn pattern_binding(&self, temp: &str, pattern: &Pattern) -> Option<String> {
        match pattern {
            Pattern::ResultOk(var) if var != "_" => Some(format!("const {} = {}.val;", var, temp)),
            Pattern::ResultErr(var) if var != "_" => Some(format!("const {} = {}.err;", var, temp)),
            Pattern::OptionSome(var) if var != "_" => Some(format!("const {} = {};", var, temp)),
            Pattern::Ident(var) if var != "_" => Some(format!("const {} = {};", var, temp)),
            Pattern::EnumVariant { path, bindings } if !bindings.is_empty() => {
                let bindings_str: Vec<String> = bindings.iter()
                    .filter(|b| *b != "_")
                    .enumerate()
                    .map(|(i, b)| format!("const {} = {}.value{};", b, temp, if bindings.len() > 1 { format!("[{}]", i) } else { "".to_string() }))
                    .collect();
                if bindings_str.is_empty() {
                    None
                } else {
                    Some(bindings_str.join(" "))
                }
            }
            _ => None,
        }
    }

    // ============ If 生成 ============

    fn emit_if(&mut self, condition: &Expr, then_body: &Expr, else_body: &Option<Box<Expr>>) -> Result<()> {
        self.write("if (");
        self.emit_expr(condition)?;
        self.writeln(") {");
        self.indent += 1;

        self.emit_block_body(then_body)?;

        self.indent -= 1;
        self.write_indent();
        self.write("}");

        if let Some(else_expr) = else_body {
            self.writeln(" else {");
            self.indent += 1;
            self.emit_block_body(else_expr)?;
            self.indent -= 1;
            self.write_indent();
            self.write("}");
        }

        Ok(())
    }

    // ============ For 生成 ============

    fn emit_for(&mut self, pattern: &str, iterator: &Expr, body: &Expr) -> Result<()> {
        self.write(&format!("for (const {} of ", pattern));
        self.emit_expr(iterator)?;
        self.writeln(") {");
        self.indent += 1;

        self.emit_block_body(body)?;

        self.indent -= 1;
        self.write_indent();
        self.write("}");

        Ok(())
    }

    // ============ 宏生成 ============

    fn emit_macro(&mut self, name: &str, args: &str) -> Result<()> {
        let name = name.trim();
        let args = self.clean_expr_string(args.trim());
        
        match name {
            "println" => {
                self.write(&format!("console.log({})", self.convert_format_string(&args)));
            }
            "print" => {
                self.write(&format!("process.stdout.write({})", self.convert_format_string(&args)));
            }
            "format" => {
                self.write(&format!("$fmt({})", args));
            }
            "vec" => {
                self.write(&format!("[{}]", args));
            }
            "assert" | "assert_eq" | "assert_ne" => {
                self.write(&format!("/* {}!({}) */", name, args));
            }
            _ => {
                self.write(&format!("/* {}!({}) */", name, args));
            }
        }
        Ok(())
    }

    fn convert_format_string(&self, args: &str) -> String {
        // 简化：直接透传
        // 实际应该将 "{}" 转换为模板字符串
        args.to_string()
    }

    // ============ 类型转换 ============

    fn type_to_ts(&self, ty: &Type) -> String {
        match ty {
            Type::Named(name) => {
                match name.as_str() {
                    "i8" | "i16" | "i32" | "i64" | "i128" | "isize" |
                    "u8" | "u16" | "u32" | "u64" | "u128" | "usize" |
                    "f32" | "f64" => "number".to_string(),
                    "String" | "str" | "&str" => "string".to_string(),
                    "bool" => "boolean".to_string(),
                    "()" => "void".to_string(),
                    "Self" => "this".to_string(),
                    // 缩写类型
                    "V" => "Array".to_string(),
                    "R" => "Result".to_string(),
                    _ => name.clone(),
                }
            }
            Type::Generic { base, params } => {
                let base_ts = match base.as_str() {
                    "Vec" | "V" => "Array",
                    "Option" => "Option",
                    "Result" | "R" => "Result",
                    "HashMap" => "Map",
                    "HashSet" => "Set",
                    _ => base,
                };
                let params_ts: Vec<String> = params.iter().map(|p| self.type_to_ts(p)).collect();
                format!("{}<{}>", base_ts, params_ts.join(", "))
            }
            Type::Tuple(types) => {
                let types_ts: Vec<String> = types.iter().map(|t| self.type_to_ts(t)).collect();
                format!("[{}]", types_ts.join(", "))
            }
            Type::Reference { inner, .. } => {
                self.type_to_ts(inner)
            }
            Type::Function { params, return_type } => {
                let params_ts: Vec<String> = params.iter().enumerate().map(|(i, p)| {
                    format!("arg{}: {}", i, self.type_to_ts(p))
                }).collect();
                let ret_ts = self.type_to_ts(return_type);
                format!("({}) => {}", params_ts.join(", "), ret_ts)
            }
        }
    }

    fn literal_to_ts(&self, lit: &Literal) -> String {
        match lit {
            Literal::Integer(n) => n.to_string(),
            Literal::Float(f) => f.to_string(),
            Literal::String(s) => format!("\"{}\"", s),
            Literal::Bool(b) => b.to_string(),
            Literal::Null => "null".to_string(),
        }
    }

    fn binop_to_ts(&self, op: BinOp) -> &'static str {
        match op {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Mod => "%",
            BinOp::And => "&&",
            BinOp::Or => "||",
            BinOp::Eq => "===",
            BinOp::Ne => "!==",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::Assign => "=",
            BinOp::Range => "/* .. */",
        }
    }

    fn unop_to_ts(&self, op: UnOp) -> &'static str {
        match op {
            UnOp::Not => "!",
            UnOp::Neg => "-",
            UnOp::Deref => "/* * */",
            UnOp::Ref => "/* & */",
            UnOp::RefMut => "/* &mut */",
        }
    }

    // ============ 辅助方法 ============

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn writeln(&mut self, s: &str) {
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
    }

    /// 清理表达式字符串中的空格
    fn clean_expr_string(&self, s: &str) -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();
        let mut in_string = false;
        let mut prev_char = ' ';

        while let Some(c) = chars.next() {
            // 字符串内部不处理
            if c == '"' && prev_char != '\\' {
                in_string = !in_string;
            }

            if in_string {
                result.push(c);
                prev_char = c;
                continue;
            }

            // 移除 ` (` 中的空格，变成 `(`
            if c == ' ' {
                if let Some(&next) = chars.peek() {
                    if next == '(' || next == ')' || next == ',' || next == '.' {
                        continue; // 跳过这个空格
                    }
                }
                // 移除 `. ` 和 `:: ` 前的空格
                if prev_char == '.' || prev_char == ':' {
                    continue;
                }
            }

            // 移除 `! (` 中的空格
            if c == ' ' && prev_char == '!' {
                if let Some(&next) = chars.peek() {
                    if next == '(' || next == '[' {
                        continue;
                    }
                }
            }

            result.push(c);
            prev_char = c;
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nu2ts::types::RuntimeMode;

    #[test]
    fn test_emit_match() {
        let mut codegen = TsCodegen::new(TsConfig {
            runtime_mode: RuntimeMode::Inline,
            ..Default::default()
        });

        let match_expr = Expr::Match {
            target: Box::new(Expr::Ident("x".to_string())),
            arms: vec![
                MatchArm {
                    pattern: Pattern::ResultOk("v".to_string()),
                    guard: None,
                    body: Box::new(Expr::Ident("v".to_string())),
                },
                MatchArm {
                    pattern: Pattern::ResultErr("_".to_string()),
                    guard: None,
                    body: Box::new(Expr::Literal(Literal::Integer(0))),
                },
            ],
        };

        codegen.emit_expr(&match_expr).unwrap();
        let output = codegen.output;

        assert!(output.contains("const _m0 = x;"));
        assert!(output.contains("if (_m0.tag === 'ok')"));
        assert!(output.contains("const v = _m0.val;"));
    }

    #[test]
    fn test_type_conversion() {
        let codegen = TsCodegen::new(TsConfig::default());

        assert_eq!(codegen.type_to_ts(&Type::Named("i32".to_string())), "number");
        assert_eq!(codegen.type_to_ts(&Type::Named("String".to_string())), "string");
        assert_eq!(codegen.type_to_ts(&Type::Named("bool".to_string())), "boolean");
    }
}
