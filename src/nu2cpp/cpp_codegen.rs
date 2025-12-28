//! C++ Code Generator - Prints CppAST to valid C++ source code
//!
//! This module is responsible for converting the structured CppAST into
//! properly formatted C++ source code with correct:
//! - Indentation
//! - Semicolons
//! - Template declarations
//! - Brace placement

use super::cpp_ast::*;

/// Configuration for C++ code generation
#[derive(Debug, Clone)]
pub struct CppCodegenConfig {
    /// Indentation string (default: 4 spaces)
    pub indent: String,
    /// Use C++20 features (concepts, ranges, etc.)
    pub cpp20: bool,
    /// Generate header guards for .hpp files
    pub header_guard: Option<String>,
}

impl Default for CppCodegenConfig {
    fn default() -> Self {
        Self {
            indent: "    ".to_string(),
            cpp20: false,
            header_guard: None,
        }
    }
}

/// C++ Code Generator
pub struct CppCodegen {
    config: CppCodegenConfig,
    output: String,
    indent_level: usize,
}

impl CppCodegen {
    pub fn new() -> Self {
        Self::with_config(CppCodegenConfig::default())
    }

    pub fn with_config(config: CppCodegenConfig) -> Self {
        Self {
            config,
            output: String::new(),
            indent_level: 0,
        }
    }

    /// Generate C++ code from a translation unit
    pub fn generate(&mut self, unit: &CppTranslationUnit) -> String {
        self.output.clear();
        self.indent_level = 0;

        // Header guard (if configured) - clone to avoid borrow conflict
        let header_guard = self.config.header_guard.clone();
        if let Some(ref guard) = header_guard {
            self.writeln(&format!("#ifndef {}", guard));
            self.writeln(&format!("#define {}", guard));
            self.writeln("");
        }

        // Includes
        for include in &unit.includes {
            self.gen_include(include);
        }
        if !unit.includes.is_empty() {
            self.writeln("");
        }

        // Items
        for item in &unit.items {
            self.gen_item(item);
            self.writeln("");
        }

        // Close header guard
        if let Some(ref guard) = header_guard {
            self.writeln(&format!("#endif // {}", guard));
        }

        self.output.clone()
    }

    fn gen_include(&mut self, include: &CppInclude) {
        if include.is_system {
            self.writeln(&format!("#include <{}>", include.path));
        } else {
            self.writeln(&format!("#include \"{}\"", include.path));
        }
    }

    fn gen_item(&mut self, item: &CppItem) {
        match item {
            CppItem::Include(inc) => self.gen_include(inc),
            CppItem::Namespace(ns) => self.gen_namespace(ns),
            CppItem::Class(cls) => self.gen_class(cls),
            CppItem::Enum(e) => self.gen_enum(e),
            CppItem::Function(f) => self.gen_function(f, None),
            CppItem::TypeAlias(ta) => self.gen_type_alias(ta),
            CppItem::GlobalVar { name, var_type, init, is_const, is_static, is_constexpr } => {
                self.gen_global_var(name, var_type, init.as_ref(), *is_const, *is_static, *is_constexpr);
            }
            CppItem::Comment(c) => self.writeln(&format!("// {}", c)),
            CppItem::Raw(s) => self.writeln(s),
        }
    }

    fn gen_namespace(&mut self, ns: &CppNamespace) {
        self.writeln(&format!("namespace {} {{", ns.name));
        self.indent_level += 1;
        for item in &ns.items {
            self.gen_item(item);
        }
        self.indent_level -= 1;
        self.writeln(&format!("}} // namespace {}", ns.name));
    }

    fn gen_class(&mut self, cls: &CppClass) {
        // Template declaration
        if !cls.template_params.is_empty() {
            self.write_indent();
            let params: Vec<String> = cls.template_params.iter()
                .map(|p| format!("typename {}", p))
                .collect();
            self.output.push_str(&format!("template<{}>\n", params.join(", ")));
        }

        // Class/struct declaration
        self.write_indent();
        let keyword = if cls.is_struct { "struct" } else { "class" };
        self.output.push_str(&format!("{} {}", keyword, cls.name));

        // Base classes
        if !cls.base_classes.is_empty() {
            self.output.push_str(" : ");
            let bases: Vec<String> = cls.base_classes.iter()
                .map(|(name, vis)| {
                    let vis_str = match vis {
                        CppVisibility::Public => "public",
                        CppVisibility::Private => "private",
                        CppVisibility::Protected => "protected",
                    };
                    format!("{} {}", vis_str, name)
                })
                .collect();
            self.output.push_str(&bases.join(", "));
        }

        self.output.push_str(" {\n");
        self.indent_level += 1;

        // Group by visibility
        let default_vis = if cls.is_struct { CppVisibility::Public } else { CppVisibility::Private };
        let mut current_vis = default_vis;

        // Fields
        for field in &cls.fields {
            if field.visibility != current_vis {
                self.indent_level -= 1;
                self.writeln(&format!("{}:", visibility_str(field.visibility)));
                self.indent_level += 1;
                current_vis = field.visibility;
            }
            self.gen_field(field);
        }

        // Methods
        for method in &cls.methods {
            if method.visibility != current_vis {
                self.indent_level -= 1;
                self.writeln(&format!("{}:", visibility_str(method.visibility)));
                self.indent_level += 1;
                current_vis = method.visibility;
            }
            self.gen_function(method, Some(&cls.name));
        }

        // Nested types
        for nested in &cls.nested_types {
            self.gen_item(nested);
        }

        // Generate derive trait implementations (C++20 default operators)
        if !cls.derive_traits.is_empty() {
            self.writeln("");
            self.writeln("// Derived traits");
            
            for trait_name in &cls.derive_traits {
                match trait_name.as_str() {
                    "PartialEq" | "Eq" => {
                        // C++20: bool operator==(const T&) const = default;
                        self.writeln(&format!("bool operator==(const {}&) const = default;", cls.name));
                    }
                    "PartialOrd" | "Ord" => {
                        // C++20: auto operator<=>(const T&) const = default;
                        self.writeln(&format!("auto operator<=>(const {}&) const = default;", cls.name));
                    }
                    "Clone" | "Copy" => {
                        // C++ default copy constructor
                        self.writeln(&format!("{}(const {}&) = default;", cls.name, cls.name));
                        self.writeln(&format!("{}& operator=(const {}&) = default;", cls.name, cls.name));
                    }
                    "Default" => {
                        // C++ default constructor
                        self.writeln(&format!("{}() = default;", cls.name));
                    }
                    _ => {
                        // Other traits: add comment
                        self.writeln(&format!("// TODO: derive({})", trait_name));
                    }
                }
            }
        }

        self.indent_level -= 1;
        self.writeln("};");
    }

    fn gen_field(&mut self, field: &CppField) {
        self.write_indent();
        self.output.push_str(&format!("{} {}", field.field_type, field.name));
        if let Some(ref val) = field.default_value {
            self.output.push_str(&format!(" = {}", val));
        }
        self.output.push_str(";\n");
    }

    fn gen_function(&mut self, func: &CppFunction, class_name: Option<&str>) {
        // Template declaration
        if !func.template_params.is_empty() {
            self.write_indent();
            let params: Vec<String> = func.template_params.iter()
                .map(|p| format!("typename {}", p))
                .collect();
            self.output.push_str(&format!("template<{}>\n", params.join(", ")));
        }

        self.write_indent();

        // Static/virtual
        if func.is_static { self.output.push_str("static "); }
        if func.is_virtual { self.output.push_str("virtual "); }

        // Return type
        self.output.push_str(&format!("{} ", func.return_type));

        // Name (with class prefix for out-of-class definitions)
        if let Some(cls) = class_name {
            if func.body.is_some() && !func.template_params.is_empty() {
                // Inline in header
                self.output.push_str(&func.name);
            } else {
                self.output.push_str(&func.name);
            }
        } else {
            self.output.push_str(&func.name);
        }

        // Parameters
        self.output.push('(');
        let params: Vec<String> = func.params.iter()
            .map(|p| {
                let mut s = format!("{} {}", p.param_type, p.name);
                if let Some(ref def) = p.default_value {
                    s.push_str(&format!(" = {}", def));
                }
                s
            })
            .collect();
        self.output.push_str(&params.join(", "));
        self.output.push(')');

        // Const
        if func.is_const { self.output.push_str(" const"); }

        // Noexcept
        if func.is_noexcept { self.output.push_str(" noexcept"); }

        // Override
        if func.is_override { self.output.push_str(" override"); }

        // Body or declaration
        match &func.body {
            Some(stmts) => {
                self.output.push_str(" {\n");
                self.indent_level += 1;
                for stmt in stmts {
                    self.gen_stmt(stmt);
                }
                self.indent_level -= 1;
                self.writeln("}");
            }
            None => {
                self.output.push_str(";\n");
            }
        }
    }

    fn gen_enum(&mut self, e: &CppEnum) {
        // Note: ast_converter already handles Rust-style enums with data by generating
        // separate structs + std::variant type alias. This function only handles
        // simple C-style enums (enum class) without associated data.
        
        self.write_indent();
        if e.is_class {
            self.output.push_str("enum class ");
        } else {
            self.output.push_str("enum ");
        }
        self.output.push_str(&e.name);
        if let Some(ref ut) = e.underlying_type {
            self.output.push_str(&format!(" : {}", ut));
        }
        self.output.push_str(" {\n");
        self.indent_level += 1;

        for (i, variant) in e.variants.iter().enumerate() {
            self.write_indent();
            self.output.push_str(&variant.name);
            if let Some(ref val) = variant.value {
                self.output.push_str(&format!(" = {}", val));
            }
            if i < e.variants.len() - 1 {
                self.output.push(',');
            }
            self.output.push('\n');
        }

        self.indent_level -= 1;
        self.writeln("};");
    }

    fn gen_type_alias(&mut self, ta: &CppTypeAlias) {
        self.write_indent();
        if !ta.template_params.is_empty() {
            let params: Vec<String> = ta.template_params.iter()
                .map(|p| format!("typename {}", p))
                .collect();
            self.output.push_str(&format!("template<{}>\n", params.join(", ")));
            self.write_indent();
        }
        self.output.push_str(&format!("using {} = {};\n", ta.name, ta.target_type));
    }

    fn gen_global_var(&mut self, name: &str, var_type: &CppType, init: Option<&CppExpr>,
                      is_const: bool, is_static: bool, is_constexpr: bool) {
        self.write_indent();
        if is_constexpr {
            self.output.push_str("constexpr ");
        } else {
            if is_static { self.output.push_str("static "); }
            if is_const { self.output.push_str("const "); }
        }
        self.output.push_str(&format!("{} {}", var_type, name));
        if let Some(expr) = init {
            self.output.push_str(" = ");
            self.gen_expr(expr);
        }
        self.output.push_str(";\n");
    }

    fn gen_stmt(&mut self, stmt: &CppStmt) {
        match stmt {
            CppStmt::VarDecl { name, var_type, init, is_const } => {
                self.write_indent();
                if *is_const { self.output.push_str("const "); }
                self.output.push_str(&format!("{} {}", var_type, name));
                if let Some(expr) = init {
                    self.output.push_str(" = ");
                    self.gen_expr(expr);
                }
                self.output.push_str(";\n");
            }
            CppStmt::Expr(expr) => {
                self.write_indent();
                self.gen_expr(expr);
                self.output.push_str(";\n");
            }
            CppStmt::Return(expr) => {
                self.write_indent();
                self.output.push_str("return");
                if let Some(e) = expr {
                    self.output.push(' ');
                    self.gen_expr(e);
                }
                self.output.push_str(";\n");
            }
            CppStmt::If { condition, then_block, else_block } => {
                self.write_indent();
                self.output.push_str("if (");
                self.gen_expr(condition);
                self.output.push_str(") {\n");
                self.indent_level += 1;
                for s in then_block {
                    self.gen_stmt(s);
                }
                self.indent_level -= 1;
                if let Some(else_stmts) = else_block {
                    self.writeln("} else {");
                    self.indent_level += 1;
                    for s in else_stmts {
                        self.gen_stmt(s);
                    }
                    self.indent_level -= 1;
                }
                self.writeln("}");
            }
            CppStmt::While { condition, body } => {
                self.write_indent();
                self.output.push_str("while (");
                self.gen_expr(condition);
                self.output.push_str(") {\n");
                self.indent_level += 1;
                for s in body {
                    self.gen_stmt(s);
                }
                self.indent_level -= 1;
                self.writeln("}");
            }
            CppStmt::For { init, condition, update, body } => {
                self.write_indent();
                self.output.push_str("for (");
                if let Some(i) = init {
                    // Special handling for var decl without semicolon
                    match i.as_ref() {
                        CppStmt::VarDecl { name, var_type, init, is_const } => {
                            if *is_const { self.output.push_str("const "); }
                            self.output.push_str(&format!("{} {}", var_type, name));
                            if let Some(expr) = init {
                                self.output.push_str(" = ");
                                self.gen_expr(expr);
                            }
                        }
                        _ => {}
                    }
                }
                self.output.push_str("; ");
                if let Some(c) = condition {
                    self.gen_expr(c);
                }
                self.output.push_str("; ");
                if let Some(u) = update {
                    self.gen_expr(u);
                }
                self.output.push_str(") {\n");
                self.indent_level += 1;
                for s in body {
                    self.gen_stmt(s);
                }
                self.indent_level -= 1;
                self.writeln("}");
            }
            CppStmt::ForRange { var_name, var_type, range, body } => {
                self.write_indent();
                // C++23 range-based for loop with proper syntax
                self.output.push_str(&format!("for ({} {} : ", var_type, var_name));
                self.gen_expr(range);
                self.output.push_str(") {\n");
                self.indent_level += 1;
                for s in body {
                    self.gen_stmt(s);
                }
                self.indent_level -= 1;
                self.writeln("}");
            }
            CppStmt::ForEnumerate { index_var, value_var, collection, body } => {
                // Generate: size_t index_var = 0; for (const auto& value_var : collection) { body; index_var++; }
                self.write_indent();
                self.output.push_str(&format!("size_t {} = 0;\n", index_var));
                self.write_indent();
                self.output.push_str(&format!("for (const auto& {} : ", value_var));
                self.gen_expr(collection);
                self.output.push_str(") {\n");
                self.indent_level += 1;
                
                // Generate body statements
                for s in body {
                    self.gen_stmt(s);
                }
                
                // Add index increment at the end
                self.write_indent();
                self.output.push_str(&format!("{}++;\n", index_var));
                
                self.indent_level -= 1;
                self.writeln("}");
            }
            CppStmt::Switch { expr, cases, default } => {
                self.write_indent();
                self.output.push_str("switch (");
                self.gen_expr(expr);
                self.output.push_str(") {\n");
                self.indent_level += 1;
                for (case_expr, stmts) in cases {
                    self.write_indent();
                    self.output.push_str("case ");
                    self.gen_expr(case_expr);
                    self.output.push_str(":\n");
                    self.indent_level += 1;
                    for s in stmts {
                        self.gen_stmt(s);
                    }
                    self.writeln("break;");
                    self.indent_level -= 1;
                }
                if let Some(def) = default {
                    self.writeln("default:");
                    self.indent_level += 1;
                    for s in def {
                        self.gen_stmt(s);
                    }
                    self.writeln("break;");
                    self.indent_level -= 1;
                }
                self.indent_level -= 1;
                self.writeln("}");
            }
            CppStmt::Break => self.writeln("break;"),
            CppStmt::Continue => self.writeln("continue;"),
            CppStmt::Block(stmts) => {
                self.writeln("{");
                self.indent_level += 1;
                for s in stmts {
                    self.gen_stmt(s);
                }
                self.indent_level -= 1;
                self.writeln("}");
            }
            CppStmt::Comment(c) => self.writeln(&format!("// {}", c)),
            CppStmt::Raw(s) => {
                // Handle Raw statements - convert Nu syntax to C++
                let converted = self.convert_raw_stmt(s);
                self.writeln(&converted);
            }
        }
    }

    /// Convert raw Nu statement syntax to C++ (helper for Raw statements)
    fn convert_raw_stmt(&self, s: &str) -> String {
        let mut result = s.to_string();
        
        // Convert self to this-> or *this
        result = regex::Regex::new(r"\bself\.")
            .unwrap()
            .replace_all(&result, "this->")
            .to_string();
        result = regex::Regex::new(r"\bself\b")
            .unwrap()
            .replace_all(&result, "(*this)")
            .to_string();
        
        // Convert Vec::new() to std::vector<auto>{}
        result = result.replace("Vec::new()", "std::vector<auto>{}");
        result = result.replace("Vec()", "std::vector<auto>{}");
        
        // Convert .clone() (for shared_ptr, just copy; for unique_ptr should use std::move)
        result = result.replace(".clone()", "");
        
        result
    }

    fn gen_expr(&mut self, expr: &CppExpr) {
        match expr {
            CppExpr::Literal(s) => self.output.push_str(s),
            CppExpr::Var(name) => {
                // Convert 'self' references in variable names
                if name == "self" {
                    self.output.push_str("(*this)");
                } else {
                    self.output.push_str(name);
                }
            }
            CppExpr::BinOp { left, op, right } => {
                self.output.push('(');
                self.gen_expr(left);
                self.output.push_str(&format!(" {} ", op));
                self.gen_expr(right);
                self.output.push(')');
            }
            CppExpr::UnaryOp { op, operand } => {
                self.output.push_str(op);
                self.gen_expr(operand);
            }
            CppExpr::Call { callee, args } => {
                self.gen_expr(callee);
                self.output.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { self.output.push_str(", "); }
                    self.gen_expr(arg);
                }
                self.output.push(')');
            }
            CppExpr::MethodCall { object, method, args } => {
                self.gen_expr(object);
                self.output.push('.');
                self.output.push_str(method);
                self.output.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { self.output.push_str(", "); }
                    self.gen_expr(arg);
                }
                self.output.push(')');
            }
            CppExpr::MemberAccess { object, member } => {
                self.gen_expr(object);
                self.output.push('.');
                self.output.push_str(member);
            }
            CppExpr::ArrowAccess { object, member } => {
                self.gen_expr(object);
                self.output.push_str("->");
                self.output.push_str(member);
            }
            CppExpr::Index { object, index } => {
                self.gen_expr(object);
                self.output.push('[');
                self.gen_expr(index);
                self.output.push(']');
            }
            CppExpr::Ternary { condition, then_expr, else_expr } => {
                self.output.push('(');
                self.gen_expr(condition);
                self.output.push_str(" ? ");
                self.gen_expr(then_expr);
                self.output.push_str(" : ");
                self.gen_expr(else_expr);
                self.output.push(')');
            }
            CppExpr::Lambda { capture, params, return_type, body } => {
                // Capture
                self.output.push('[');
                match capture {
                    CppCapture::None => {}
                    CppCapture::CopyAll => self.output.push('='),
                    CppCapture::RefAll => self.output.push('&'),
                    CppCapture::Explicit(items) => {
                        let caps: Vec<String> = items.iter().map(|item| {
                            if item.moved {
                                format!("{} = std::move({})", item.name, item.name)
                            } else if item.by_ref {
                                format!("&{}", item.name)
                            } else {
                                item.name.clone()
                            }
                        }).collect();
                        self.output.push_str(&caps.join(", "));
                    }
                }
                self.output.push(']');

                // Parameters
                self.output.push('(');
                let params_str: Vec<String> = params.iter()
                    .map(|p| format!("{} {}", p.param_type, p.name))
                    .collect();
                self.output.push_str(&params_str.join(", "));
                self.output.push(')');

                // Return type
                if let Some(ref rt) = return_type {
                    self.output.push_str(&format!(" -> {}", rt));
                }

                // Body
                self.output.push_str(" { ");
                for (i, stmt) in body.iter().enumerate() {
                    if i > 0 { self.output.push(' '); }
                    // Inline generation for lambda body
                    match stmt {
                        CppStmt::Return(Some(e)) => {
                            self.output.push_str("return ");
                            self.gen_expr(e);
                            self.output.push(';');
                        }
                        CppStmt::Expr(e) => {
                            self.gen_expr(e);
                            self.output.push(';');
                        }
                        _ => {
                            // For complex statements, use normal generation
                            let old_indent = self.indent_level;
                            self.indent_level = 0;
                            self.gen_stmt(stmt);
                            self.indent_level = old_indent;
                        }
                    }
                }
                self.output.push_str(" }");
            }
            CppExpr::BraceInit { type_name, fields } => {
                self.output.push_str(type_name);
                self.output.push('{');
                for (i, (name, val)) in fields.iter().enumerate() {
                    if i > 0 { self.output.push_str(", "); }
                    if let Some(n) = name {
                        self.output.push_str(&format!(".{} = ", n));
                    }
                    self.gen_expr(val);
                }
                self.output.push('}');
            }
            CppExpr::Cast { cast_type, target_type, expr } => {
                self.output.push_str(&format!("{}<{}>(", cast_type, target_type));
                self.gen_expr(expr);
                self.output.push(')');
            }
            CppExpr::Move(inner) => {
                self.output.push_str("std::move(");
                self.gen_expr(inner);
                self.output.push(')');
            }
            CppExpr::This => self.output.push_str("this"),
            CppExpr::Nullptr => self.output.push_str("nullptr"),
            CppExpr::Raw(s) => {
                // Convert raw Nu expressions to C++
                let converted = self.convert_raw_expr(s);
                self.output.push_str(&converted);
            }
        }
    }

    /// Convert raw Nu expression syntax to C++ (helper for Raw expressions)
    fn convert_raw_expr(&self, s: &str) -> String {
        let mut result = s.to_string();
        
        // Convert self to this-> or *this
        result = regex::Regex::new(r"\bself\.")
            .unwrap()
            .replace_all(&result, "this->")
            .to_string();
        result = regex::Regex::new(r"\bself\b")
            .unwrap()
            .replace_all(&result, "(*this)")
            .to_string();
        
        // Convert closures: |x| expr -> [&](auto x) { return expr; }
        if let Some(pipe_start) = result.find('|') {
            if let Some(pipe_end) = result[pipe_start + 1..].find('|') {
                let pipe_end = pipe_start + 1 + pipe_end;
                let params = &result[pipe_start + 1..pipe_end];
                let body = result[pipe_end + 1..].trim();
                
                // Handle special case: |_| -> no parameters
                let cpp_params = if params.trim() == "_" {
                    String::new()
                } else {
                    let param_vec: Vec<String> = params.split(',')
                        .map(|p| {
                            let p = p.trim();
                            if p == "_" {
                                "auto".to_string()
                            } else {
                                format!("auto {}", p)
                            }
                        })
                        .collect();
                    param_vec.join(", ")
                };
                
                // Build C++ lambda
                result = format!("[&]({}) {{ return {}; }}", cpp_params, body);
            }
        }
        
        // Convert Vec::new() to std::vector<auto>{}
        result = result.replace("Vec::new()", "std::vector<auto>{}");
        result = result.replace("Vec()", "std::vector<auto>{}");
        
        // Convert .clone() method calls
        result = result.replace(".clone()", "");
        
        // Convert format! and println! macros
        result = result.replace("format!", "std::format");
        result = result.replace("println!", "std::println");
        
        // Convert string literal methods: "text".to_string() -> std::string("text")
        result = regex::Regex::new(r#""([^"]+)"\s*\.\s*to_string\s*\(\s*\)"#)
            .unwrap()
            .replace_all(&result, r#"std::string("$1")"#)
            .to_string();
        
        result
    }

    // Helper methods
    fn write_indent(&mut self) {
        for _ in 0..self.indent_level {
            self.output.push_str(&self.config.indent);
        }
    }

    fn writeln(&mut self, s: &str) {
        self.write_indent();
        self.output.push_str(s);
        self.output.push('\n');
    }
}

fn visibility_str(vis: CppVisibility) -> &'static str {
    match vis {
        CppVisibility::Public => "public",
        CppVisibility::Private => "private",
        CppVisibility::Protected => "protected",
    }
}

impl Default for CppCodegen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_struct() {
        let cls = CppClass {
            name: "Point".to_string(),
            is_struct: true,
            template_params: vec![],
            base_classes: vec![],
            fields: vec![
                CppField {
                    name: "x".to_string(),
                    field_type: CppType::int32(),
                    visibility: CppVisibility::Public,
                    default_value: None,
                },
                CppField {
                    name: "y".to_string(),
                    field_type: CppType::int32(),
                    visibility: CppVisibility::Public,
                    default_value: None,
                },
            ],
            methods: vec![],
            nested_types: vec![],
            derive_traits: vec![],
            cfg_condition: None,
        };

        let mut unit = CppTranslationUnit::new();
        unit.add_item(CppItem::Class(cls));

        let mut codegen = CppCodegen::new();
        let output = codegen.generate(&unit);

        assert!(output.contains("struct Point {"));
        assert!(output.contains("int32_t x;"));
        assert!(output.contains("int32_t y;"));
    }

    #[test]
    fn test_template_struct() {
        let cls = CppClass {
            name: "Container".to_string(),
            is_struct: true,
            template_params: vec!["T".to_string()],
            base_classes: vec![],
            fields: vec![
                CppField {
                    name: "value".to_string(),
                    field_type: CppType::Named("T".to_string()),
                    visibility: CppVisibility::Public,
                    default_value: None,
                },
            ],
            methods: vec![],
            nested_types: vec![],
            derive_traits: vec![],
            cfg_condition: None,
        };

        let mut unit = CppTranslationUnit::new();
        unit.add_item(CppItem::Class(cls));

        let mut codegen = CppCodegen::new();
        let output = codegen.generate(&unit);

        assert!(output.contains("template<typename T>"));
        assert!(output.contains("struct Container {"));
        assert!(output.contains("T value;"));
    }

    #[test]
    fn test_lambda_with_capture() {
        let lambda = CppExpr::Lambda {
            capture: CppCapture::Explicit(vec![
                CppCaptureItem { name: "x".to_string(), by_ref: false, moved: false },
                CppCaptureItem { name: "y".to_string(), by_ref: true, moved: false },
            ]),
            params: vec![
                CppParam {
                    name: "z".to_string(),
                    param_type: CppType::int32(),
                    default_value: None,
                },
            ],
            return_type: Some(CppType::int32()),
            body: vec![
                CppStmt::Return(Some(CppExpr::BinOp {
                    left: Box::new(CppExpr::BinOp {
                        left: Box::new(CppExpr::Var("x".to_string())),
                        op: "+".to_string(),
                        right: Box::new(CppExpr::Var("y".to_string())),
                    }),
                    op: "+".to_string(),
                    right: Box::new(CppExpr::Var("z".to_string())),
                })),
            ],
        };

        let mut codegen = CppCodegen::new();
        codegen.gen_expr(&lambda);
        let output = codegen.output;

        assert!(output.contains("[x, &y]"));
        assert!(output.contains("(int32_t z)"));
        assert!(output.contains("-> int32_t"));
    }
}
