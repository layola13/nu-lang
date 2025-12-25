// Nu2TS Codegen
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

    pub fn generate(&mut self, stmts: &[Stmt]) -> Result<String> {
        // 生成 runtime import
        self.emit_runtime_import();

        // 生成所有语句
        for stmt in stmts {
            self.visit_stmt(stmt)?;
            self.writeln("");
        }

        Ok(self.output.clone())
    }

    fn emit_runtime_import(&mut self) {
        use super::runtime::generate_runtime_import;
        use super::types::RuntimeMode;

        if self.config.runtime_mode == RuntimeMode::Import {
            self.write(generate_runtime_import());
        }
    }

    // ============ 语句生成 ============

    fn visit_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::Function { name, params, return_type, body, is_pub, is_async } => {
                self.emit_function(name, params, return_type, body, *is_pub, *is_async)?;
            }
            Stmt::Let { name, ty, value, is_mut } => {
                self.emit_let(name, ty, value, *is_mut)?;
            }
            Stmt::ExprStmt(expr) => {
                self.visit_expr(expr)?;
                self.writeln(";");
            }
        }
        Ok(())
    }

    fn emit_function(
        &mut self,
        name: &str,
        params: &[Param],
        return_type: &Option<Type>,
        body: &Expr,
        is_pub: bool,
        is_async: bool,
    ) -> Result<()> {
        // export function name(params): return_type
        let export = if is_pub { "export " } else { "" };
        let asyncc = if is_async { "async " } else { "" };

        self.write(&format!("{}{}function {}(", export, asyncc, name));

        // 参数
        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.write(&format!("{}: {}", param.name, self.type_to_ts(&param.ty)));
        }

        self.write(")");

        // 返回类型
        if let Some(ret_ty) = return_type {
            self.write(&format!(": {}", self.type_to_ts(ret_ty)));
        }

        self.writeln(" {");
        self.indent += 1;

        // 函数体
        self.visit_expr(body)?;

        self.indent -= 1;
        self.writeln("}");

        Ok(())
    }

    fn emit_let(&mut self, name: &str, _ty: &Option<Type>, value: &Expr, _is_mut: bool) -> Result<()> {
        self.write(&format!("const {} = ", name));
        self.visit_expr(value)?;
        self.writeln(";");
        Ok(())
    }

    // ============ 表达式生成 ============

    fn visit_expr(&mut self, expr: &Expr) -> Result<()> {
        match expr {
            Expr::Match { target, arms } => {
                self.emit_match(target, arms)?;
            }
            Expr::If { condition, then_body, else_body } => {
                self.emit_if(condition, then_body, else_body)?;
            }
            Expr::Return(value) => {
                self.write("return");
                if let Some(val) = value {
                    self.write(" ");
                    self.visit_expr(val)?;
                }
            }
            Expr::Break => {
                self.write("break");
            }
            Expr::Continue => {
                self.write("continue");
            }
            Expr::Call { func, args } => {
                self.visit_expr(func)?;
                self.write("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.visit_expr(arg)?;
                }
                self.write(")");
            }
            Expr::Field { object, field } => {
                self.visit_expr(object)?;
                self.write(&format!(".{}", field));
            }
            Expr::Binary { left, op, right } => {
                self.visit_expr(left)?;
                self.write(&format!(" {} ", self.binop_to_ts(*op)));
                self.visit_expr(right)?;
            }
            Expr::Block { stmts, trailing_expr } => {
                for stmt in stmts {
                    self.visit_stmt(stmt)?;
                }
                if let Some(expr) = trailing_expr {
                    self.visit_expr(expr)?;
                }
            }
            Expr::Ident(name) => {
                self.write(name);
            }
            Expr::Literal(lit) => {
                self.write(&self.literal_to_ts(lit));
            }
            _ => {
                // TODO: 其他表达式
                self.write("/* TODO */");
            }
        }
        Ok(())
    }

    // ============ Match 生成（核心）============

    fn emit_match(&mut self, target: &Expr, arms: &[MatchArm]) -> Result<()> {
        // 生成临时变量
        let temp = format!("_m{}", self.temp_counter);
        self.temp_counter += 1;

        self.write(&format!("const {} = ", temp));
        self.visit_expr(target)?;
        self.writeln(";");

        // 生成 if-chain
        for (i, arm) in arms.iter().enumerate() {
            let prefix = if i == 0 { "if" } else { "else if" };

            let condition = self.pattern_to_condition(&temp, &arm.pattern);
            self.write_indent();
            self.write(&format!("{} ({}) {{", prefix, condition));
            self.writeln("");
            self.indent += 1;

            // 生成变量绑定
            if let Some(binding) = self.pattern_binding(&temp, &arm.pattern) {
                self.write_indent();
                self.writeln(&binding);
            }

            // 生成分支体
            self.write_indent();
            self.visit_expr(&arm.body)?;
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
            Pattern::OptionSome(_) => format!("{} !== null", temp),
            Pattern::OptionNone => format!("{} === null", temp),
            Pattern::Wildcard => "true".to_string(),
            Pattern::Literal(lit) => format!("{} === {}", temp, self.literal_to_ts(lit)),
            Pattern::Ident(_) => "true".to_string(),  // 总是匹配
        }
    }

    fn pattern_binding(&self, temp: &str, pattern: &Pattern) -> Option<String> {
        match pattern {
            Pattern::ResultOk(var) => Some(format!("const {} = {}.val;", var, temp)),
            Pattern::ResultErr(var) => Some(format!("const {} = {}.err;", var, temp)),
            Pattern::OptionSome(var) => Some(format!("const {} = {};", var, temp)),
            Pattern::Ident(var) => Some(format!("const {} = {};", var, temp)),
            _ => None,
        }
    }

    fn emit_if(&mut self, condition: &Expr, then_body: &Expr, else_body: &Option<Box<Expr>>) -> Result<()> {
        self.write("if (");
        self.visit_expr(condition)?;
        self.writeln(") {");
        self.indent += 1;

        self.write_indent();
        self.visit_expr(then_body)?;
        self.writeln(";");

        self.indent -= 1;
        self.write_indent();
        self.write("}");

        if let Some(else_expr) = else_body {
            self.writeln(" else {");
            self.indent += 1;

            self.write_indent();
            self.visit_expr(else_expr)?;
            self.writeln(";");

            self.indent -= 1;
            self.write_indent();
            self.write("}");
        }

        Ok(())
    }

    // ============ 类型转换 ============

    fn type_to_ts(&self, ty: &Type) -> String {
        match ty {
            Type::Named(name) => {
                // Nu 类型 → TypeScript 类型
                match name.as_str() {
                    "i32" | "i64" | "u32" | "u64" | "f32" | "f64" | "usize" | "isize" => "number".to_string(),
                    "String" | "str" => "string".to_string(),
                    "bool" => "boolean".to_string(),
                    "()" => "void".to_string(),
                    _ => name.clone(),
                }
            }
            Type::Generic { base, params } => {
                let base_ts = self.type_to_ts(&Type::Named(base.clone()));
                let params_ts: Vec<String> = params.iter().map(|p| self.type_to_ts(p)).collect();
                format!("{}<{}>", base_ts, params_ts.join(", "))
            }
            Type::Tuple(types) => {
                let types_ts: Vec<String> = types.iter().map(|t| self.type_to_ts(t)).collect();
                format!("[{}]", types_ts.join(", "))
            }
            Type::Reference { inner, .. } => {
                // TypeScript 无引用概念，直接使用内部类型
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

        codegen.visit_expr(&match_expr).unwrap();
        let output = codegen.output;

        assert!(output.contains("const _m0 = x;"));
        assert!(output.contains("if (_m0.tag === 'ok')"));
        assert!(output.contains("const v = _m0.val;"));
        assert!(output.contains("else if (_m0.tag === 'err')"));
    }

    #[test]
    fn test_type_conversion() {
        let codegen = TsCodegen::new(TsConfig::default());

        assert_eq!(codegen.type_to_ts(&Type::Named("i32".to_string())), "number");
        assert_eq!(codegen.type_to_ts(&Type::Named("String".to_string())), "string");
        assert_eq!(codegen.type_to_ts(&Type::Named("bool".to_string())), "boolean");
    }
}
