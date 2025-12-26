// Nu2TS Parser (完整版)
// 递归下降解析器，将 Nu 代码解析为 AST
// 策略：精确解析核心语法（Match、函数签名），透传复杂结构

use super::ast::*;
use anyhow::Result;

// ============ Parser ============

pub struct Parser {
    lines: Vec<String>,
    current_line: usize,
}

impl Parser {
    pub fn new(input: &str) -> Self {
        let lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
        Self {
            lines,
            current_line: 0,
        }
    }

    /// 解析整个文件为 Item 列表
    pub fn parse_file(&mut self) -> Result<NuFile> {
        let mut items = vec![];

        while self.current_line < self.lines.len() {
            let line = self.current_line().trim().to_string();

            // 跳过空行和注释
            if line.is_empty() || line.starts_with("//") {
                self.advance();
                continue;
            }

            // 解析顶级项目
            if let Some(item) = self.parse_item()? {
                items.push(item);
            }

            self.advance();
        }

        Ok(NuFile { items })
    }

    /// 解析 Stmt 列表（向后兼容）
    pub fn parse(&mut self) -> Result<Vec<Stmt>> {
        let file = self.parse_file()?;

        // 将 Item 转换为 Stmt（用于旧的接口）
        let stmts: Vec<Stmt> = file
            .items
            .into_iter()
            .filter_map(|item| match item {
                Item::Function(f) => Some(Stmt::ExprStmt(Box::new(Expr::Block {
                    stmts: vec![],
                    trailing_expr: None,
                }))),
                Item::Raw(s) => Some(Stmt::Raw(s)),
                _ => None,
            })
            .collect();

        Ok(stmts)
    }

    fn parse_item(&mut self) -> Result<Option<Item>> {
        let line = self.current_line().trim().to_string();

        // use 声明: u path::{items}
        if line.starts_with("u ") {
            return Ok(Some(self.parse_use()?));
        }

        // 函数定义: F/f name(...)
        if line.starts_with("F ") || line.starts_with("f ") {
            return Ok(Some(Item::Function(self.parse_function()?)));
        }

        // 结构体: s/S Name { ... }
        if line.starts_with("s ") || line.starts_with("S ") {
            return Ok(Some(Item::Struct(self.parse_struct()?)));
        }

        // 枚举: E Name { ... }
        if line.starts_with("E ") {
            return Ok(Some(Item::Enum(self.parse_enum()?)));
        }

        // impl 块: I Type { ... }
        if line.starts_with("I ") {
            return Ok(Some(Item::Impl(self.parse_impl()?)));
        }

        // mod 块: D name { ... }
        if line.starts_with("D ") {
            return Ok(Some(Item::Mod(self.parse_mod()?)));
        }

        // Derive 宏: #D(...)
        if line.starts_with("#D") || line.starts_with("#[") {
            // 跳过属性行，它会被下一个项目消费
            return Ok(None);
        }

        // cfg test: #[cfg(test)]
        if line.starts_with("#[cfg(test)]") {
            return Ok(None);
        }

        // 独立的大括号
        if line == "{" || line == "}" {
            return Ok(None);
        }

        // 顶层变量声明: l/v name = value
        if line.starts_with("l ") || line.starts_with("v ") || line.starts_with("let ") {
            let stmt = self.parse_let()?;
            return Ok(Some(Item::Stmt(stmt)));
        }

        // 顶层 Match 表达式: M expr { ... }
        if line.starts_with("M ") {
            let match_expr = self.parse_match()?;
            // 包装为包含单个语句的函数
            return Ok(Some(Item::Stmt(Stmt::ExprStmt(Box::new(match_expr)))));
        }

        // 顶层 If 表达式: ? cond { ... }
        if line.starts_with("? ") {
            let if_expr = self.parse_if()?;
            return Ok(Some(Item::Stmt(Stmt::ExprStmt(Box::new(if_expr)))));
        }

        // 修复问题2：识别顶层println!宏调用
        if line.contains("println!") || line.contains("print!") {
            if let Ok(expr) = self.parse_expr_string(&line) {
                return Ok(Some(Item::Stmt(Stmt::ExprStmt(Box::new(expr)))));
            }
        }

        // 透传其他行
        Ok(Some(Item::Raw(line)))
    }

    fn parse_use(&mut self) -> Result<Item> {
        let line = self.current_line().trim();
        let content = &line[2..].trim(); // 跳过 "u "

        Ok(Item::Use {
            path: content.to_string(),
            items: vec![],
        })
    }

    fn parse_function(&mut self) -> Result<FunctionDef> {
        let line = self.current_line().trim().to_string();
        let is_pub = line.starts_with("F ");
        let content = &line[2..]; // 跳过 "F " 或 "f "

        // 解析函数签名
        let (name, params, return_type) = self.parse_function_signature(content)?;

        // 解析前必须推进到下一行，因为 parse_block_body 期望开始于内容行或大括号
        // 如果当前行包含 {，在 parse_block_body 中会处理但我们这里已经解析过签名
        // 然而 parse_block_body 检查当前行开始。
        // 如果签名行有 {，parse_block_body 会认为它是块开始。
        // 但 parse_function_signature 并没有消耗行。
        // 所以我们手动推进。
        self.advance();

        // 解析函数体
        let body_stmts_raw = self.parse_block_body()?;
        let (body_stmts, trailing_expr) = self.extract_trailing_expr(body_stmts_raw);

        Ok(FunctionDef {
            name,
            params,
            return_type,
            body: Box::new(Expr::Block {
                stmts: body_stmts,
                trailing_expr,
            }),
            is_pub,
            is_async: false,
            attributes: vec![],
        })
    }

    fn parse_struct(&mut self) -> Result<StructDef> {
        let line = self.current_line().trim().to_string();
        let content = &line[2..].trim(); // 跳过 "s " 或 "S "

        // 提取名称
        let name = content.split('{').next().unwrap_or("").trim().to_string();

        // 解析字段
        let fields_raw = self.collect_block()?;
        let mut fields = vec![];

        for field_line in &fields_raw {
            let field_line = field_line.trim();
            if field_line.is_empty() || field_line.starts_with("//") {
                continue;
            }

            // 格式: name: type ,
            let field_line = field_line.trim_end_matches(',').trim();
            if let Some(colon_pos) = field_line.find(':') {
                let fname = field_line[..colon_pos].trim().to_string();
                let ftype_str = field_line[colon_pos + 1..].trim();
                let ftype = self.parse_type(ftype_str);
                fields.push(Field {
                    name: fname,
                    ty: ftype,
                });
            }
        }

        Ok(StructDef {
            name,
            fields,
            derives: vec![],
            doc: None,
        })
    }

    fn parse_enum(&mut self) -> Result<EnumDef> {
        let line = self.current_line().trim().to_string();
        let content = &line[2..].trim(); // 跳过 "E "

        let name = content.split('{').next().unwrap_or("").trim().to_string();

        // 解析变体
        let variants = self.parse_enum_variants()?;

        Ok(EnumDef {
            name,
            variants,
            derives: vec![],
            doc: None,
        })
    }

    fn parse_enum_variants(&mut self) -> Result<Vec<EnumVariant>> {
        let mut variants = vec![];

        // 如果当前行包含 {，跳到下一行
        if self.current_line().contains("{") {
            self.advance();
        }

        while self.current_line < self.lines.len() {
            let line = self.current_line().trim().to_string();

            if line == "}" {
                break;
            }

            if line.is_empty() || line == "{" {
                self.advance();
                continue;
            }

            // 解析变体: Name 或 Name(Type)
            let variant_str = line.trim_end_matches(',').trim();
            if !variant_str.is_empty() {
                let (name, fields) = if variant_str.contains('(') {
                    let paren_pos = variant_str.find('(').unwrap();
                    let name = variant_str[..paren_pos].trim().to_string();
                    // 简化：不解析字段类型
                    (name, Some(vec![]))
                } else {
                    (variant_str.to_string(), None)
                };

                variants.push(EnumVariant { name, fields });
            }

            self.advance();
        }

        Ok(variants)
    }

    fn parse_impl(&mut self) -> Result<ImplDef> {
        let line = self.current_line().trim().to_string();
        let content = &line[2..].trim(); // 跳过 "I "

        // 提取目标类型
        let target = content.split('{').next().unwrap_or("").trim().to_string();

        // 解析方法
        let mut methods = vec![];

        if line.contains("{") {
            self.advance();
        }

        while self.current_line < self.lines.len() {
            let method_line = self.current_line().trim().to_string();

            // 检查impl块结束 - 顶层的单独}行
            if method_line == "}" {
                // 这是impl块的结束
                break;
            }

            if method_line.is_empty() {
                self.advance();
                continue;
            }

            // 解析方法
            if method_line.starts_with("f ") || method_line.starts_with("F ") {
                methods.push(self.parse_function()?);
                // parse_function已经处理了所有行推进（包括函数结束的}）
                // 当前行现在应该是函数}之后的下一行
                // 继续循环检查是否是impl块的}
            } else {
                // 非函数行，跳过
                self.advance();
            }
        }

        Ok(ImplDef {
            target,
            trait_name: None,
            methods,
        })
    }

    fn parse_mod(&mut self) -> Result<ModDef> {
        let line = self.current_line().trim().to_string();
        let content = &line[2..].trim(); // 跳过 "D "

        let name = content.split('{').next().unwrap_or("").trim().to_string();

        // 跳过整个 mod 块
        self.skip_block()?;

        Ok(ModDef {
            name,
            items: vec![],
        })
    }

    // ============ 函数签名解析 ============

    fn parse_function_signature(
        &self,
        content: &str,
    ) -> Result<(String, Vec<Param>, Option<Type>)> {
        let name = content.split('(').next().unwrap_or("").trim().to_string();

        let mut params = vec![];
        let mut return_type = None;

        // 提取参数部分
        if let Some(start) = content.find('(') {
            let end = self.find_matching_paren(content, start);
            if end > start {
                let params_str = &content[start + 1..end];
                if !params_str.trim().is_empty() {
                    for param_str in self.split_params(params_str) {
                        let param_str = param_str.trim();
                        if let Some(colon_pos) = param_str.find(':') {
                            let param_name = param_str[..colon_pos].trim();
                            let param_type_str = param_str[colon_pos + 1..].trim();

                            // 检查是否是引用或可变引用
                            let (is_ref, is_mut, type_str) = if param_name.starts_with("&!") {
                                (true, true, &param_name[2..])
                            } else if param_name.starts_with("&") {
                                (true, false, &param_name[1..])
                            } else {
                                (false, false, param_name)
                            };

                            params.push(Param {
                                name: type_str.trim().to_string(),
                                ty: self.parse_type(param_type_str),
                                is_ref,
                                is_mut,
                            });
                        }
                    }
                }
            }
        }

        // 解析返回类型
        if let Some(arrow_pos) = content.find("->") {
            let after_arrow = &content[arrow_pos + 2..];
            let type_str = after_arrow.split('{').next().unwrap_or("").trim();
            if !type_str.is_empty() {
                return_type = Some(self.parse_type(type_str));
            }
        }

        Ok((name, params, return_type))
    }

    // ============ 块解析 ============

    fn parse_block_body(&mut self) -> Result<Vec<Stmt>> {
        let mut stmts = vec![];
        let mut brace_depth = 0;

        // 检查当前行是否包含 {
        let current = self.current_line().to_string();
        let trimmed = current.trim();
        println!(
            "DEBUG: block start line='{}' depth={}",
            trimmed,
            brace_depth
        );
        
        // 如果当前行以M/? /L/l/v/c等开头，说明这是语句而不是block开始的{
        // 这些行应该被parse_stmt处理，不应该在这里consume
        let is_statement_with_brace = trimmed.starts_with("M ")
            || trimmed.starts_with("? ")
            || trimmed.starts_with("L ")
            || trimmed.starts_with("W ")
            || trimmed.starts_with("l ")
            || trimmed.starts_with("v ")
            || trimmed.starts_with("c ");
        
        if current.contains('{') && !is_statement_with_brace {
            brace_depth = 1;
            // 如果仅是 {，跳过到下一行
            if trimmed == "{" {
                self.advance();
            } else {
                // 函数签名行包含{但不只是{，立即推进到下一行避免重复计数
                self.advance();
            }
        }

        // 修复：继续解析所有语句直到真正的函数结束}
        while self.current_line < self.lines.len() {
            let line = self.current_line().trim().to_string();
            println!("DEBUG: line='{}' depth={}", line, brace_depth);

            // 跳过空行
            if line.is_empty() {
                self.advance();
                continue;
            }

            // 修复问题#2: 检测到新函数定义时立即break
            // 避免第二个函数被吸收进第一个函数体
            // 注意：由于Match等语句的{不计入depth，不能依赖brace_depth判断
            if line.starts_with("f ") || line.starts_with("F ") {
                // 这是另一个函数定义，当前函数体应该已经结束
                break;
            }

            // 检查大括号 - 单独的}行表示块结束
            if line == "}" {
                if brace_depth > 0 {
                    brace_depth -= 1;
                }
                if brace_depth == 0 {
                    // 到达块结束，推进到}之后
                    self.advance();
                    break;
                }
                self.advance();
                continue;
            }

            // 解析语句（return/break/continue等控制流语句应该被正常解析）
            let start_line = self.current_line;
            if let Some(stmt) = self.parse_stmt()? {
                stmts.push(stmt);
                // 如果 parse_stmt 没有推进行，强制推进
                if self.current_line == start_line {
                    self.advance();
                }
            } else {
                // parse_stmt返回None，推进行
                self.advance();
            }
        }

        Ok(stmts)
    }

    /// Helper to convert Vec<Stmt> to (Vec<Stmt>, Option<Box<Expr>>) for blocks
    fn extract_trailing_expr(&self, mut stmts: Vec<Stmt>) -> (Vec<Stmt>, Option<Box<Expr>>) {
        if let Some(last) = stmts.pop() {
            if let Stmt::ExprStmt(expr) = last {
                return (stmts, Some(expr));
            } else {
                stmts.push(last);
            }
        }
        (stmts, None)
    }

    fn parse_stmt(&mut self) -> Result<Option<Stmt>> {
        let line = self.current_line().trim().to_string();

        // 变量声明
        if line.starts_with("l ") || line.starts_with("v ") || line.starts_with("let ") {
            return Ok(Some(self.parse_let()?));
        }

        // Match 表达式
        if line.starts_with("M ") {
            let match_expr = self.parse_match()?;
            return Ok(Some(Stmt::ExprStmt(Box::new(match_expr))));
        }

        // ... existing match/if code ...

        // If 表达式
        if line.starts_with("? ") {
            let if_expr = self.parse_if()?;
            return Ok(Some(Stmt::ExprStmt(Box::new(if_expr))));
        }

        // Return
        if line.starts_with("< ") || line == "<" || line.starts_with("<") {
            let ret_expr = self.parse_return()?;
            return Ok(Some(Stmt::ExprStmt(Box::new(ret_expr))));
        }

        // Loop
        if line.starts_with("L ") || line == "L {" {
            let loop_expr = self.parse_loop()?;
            return Ok(Some(Stmt::ExprStmt(Box::new(loop_expr))));
        }

        // 修复问题6: while let 循环解析
        if line.starts_with("while let ") {
            let loop_expr = self.parse_while_let()?;
            return Ok(Some(Stmt::ExprStmt(Box::new(loop_expr))));
        }

        // 修复问题#3: for 循环解析
        if line.starts_with("for ") {
            let for_expr = self.parse_for()?;
            return Ok(Some(Stmt::ExprStmt(Box::new(for_expr))));
        }

        // 跳过大括号
        if line == "{" || line == "}" || line == "}," {
            return Ok(None);
        }

        // 结构体初始化: Name { field: value } - 必须是顶层的 Name { ... }
        // 排除：宏调用(!), 函数调用中的结构体({在(之后), 格式字符串("{}")
        let is_struct_init = line.contains('{')
            && line.contains(':')
            && !line.contains("=>")
            && !line.starts_with("//")
            && !line.contains("!(")
            && !line.contains("! (")
            && !line.contains("![")
            && !line.contains("\"{")
            && !line.contains("{}"); // 格式占位符

        if is_struct_init {
            // 进一步检查：{ 应该在函数括号之前，或者没有函数括号
            let brace_pos = line.find('{').unwrap_or(line.len());
            let paren_pos = line.find('(').unwrap_or(line.len());
            if brace_pos < paren_pos {
                if let Ok(expr) = self.parse_struct_init(&line) {
                    return Ok(Some(Stmt::ExprStmt(Box::new(expr))));
                }
            }
        }

        // 修复问题2：优先识别println!等宏调用
        if line.contains("println!") || line.contains("print!") || line.contains("format!") {
            if let Ok(expr) = self.parse_expr_string(&line) {
                return Ok(Some(Stmt::ExprStmt(Box::new(expr))));
            }
        }

        // 宏调用和一般表达式 - 尝试解析
        if line.contains("!(")
            || line.contains("! (")
            || line.contains("![")
            || line.contains("! [")
            || line.contains('(')
            || line.contains('.')
            || line.contains("::")
            || line.contains('+')
            || line.contains('-')
        {
            if let Ok(expr) = self.parse_expr_string(&line) {
                if !matches!(expr, Expr::Ident(ref s) if s == &line) {
                    return Ok(Some(Stmt::ExprStmt(Box::new(expr))));
                }
            }
        }

        // 透传其他行
        Ok(Some(Stmt::Raw(line)))
    }

    fn parse_let(&mut self) -> Result<Stmt> {
        let line = self.current_line().trim().to_string();
        let is_mut = line.starts_with("v ");
        let content = &line[2..];

        // 处理 let mut 形式
        let content = if content.trim().starts_with("mut ") {
            &content.trim()[4..]
        } else {
            content
        };

        // 分割 name = value
        let parts: Vec<&str> = content.splitn(2, '=').collect();
        let name_part = parts.get(0).unwrap_or(&"").trim();
        let value_str = parts.get(1).map(|s| s.trim()).unwrap_or("");

        // 提取变量名和类型
        let (name, ty) = if let Some(colon_pos) = name_part.find(':') {
            let n = name_part[..colon_pos].trim().to_string();
            let t = self.parse_type(&name_part[colon_pos + 1..]);
            (n, Some(t))
        } else {
            (name_part.to_string(), None)
        };

        // 解析值（如果有）
        let value = if value_str.is_empty() {
            Expr::Literal(Literal::Null)
        } else {
            let value_trimmed = value_str.trim_end_matches(';').trim();

            // 修复问题1: 检查是否是 Match 表达式: M expr { ... }
            if value_trimmed.starts_with("M ") && value_trimmed.contains('{') {
                // 使用parse_match_from_value方法处理let语句中的Match
                self.parse_match_from_value(value_trimmed)?
            }
            // 检查是否是闭包: |params| body 或 $|params| body
            else if (value_trimmed.starts_with('|') || value_trimmed.starts_with("$|"))
                && value_trimmed.contains("->")
            {
                // 这是带返回类型的闭包
                self.parse_closure_expr(value_trimmed)?
            }
            else if value_trimmed.starts_with('|') || value_trimmed.starts_with("$|") {
                // 简单闭包
                self.parse_closure_expr(value_trimmed)?
            }
            // 检查是否是结构体初始化: Name { field: value }
            else if value_trimmed.contains('{')
                && value_trimmed.contains(':')
                && !value_trimmed.contains("=>")
                && !value_trimmed.contains('|')
            {
                self.parse_struct_init(value_trimmed)?
            } else {
                self.parse_expr_string(value_trimmed)?
            }
        };

        Ok(Stmt::Let {
            name,
            ty,
            value: Box::new(value),
            is_mut,
        })
    }

    fn parse_struct_init(&self, s: &str) -> Result<Expr> {
        let trimmed = s.trim();

        // 格式: Name { field: value, ... }
        if let Some(brace_pos) = trimmed.find('{') {
            let name = trimmed[..brace_pos].trim().to_string();
            let fields_str = &trimmed[brace_pos + 1..]
                .trim_end_matches('}')
                .trim_end_matches(',');

            let mut fields = vec![];
            // 简化解析: 按逗号分割
            for field_def in self.split_params(fields_str) {
                let field_def = field_def.trim();
                if field_def.is_empty() {
                    continue;
                }

                if let Some(colon_pos) = field_def.find(':') {
                    let fname = field_def[..colon_pos].trim().to_string();
                    let fvalue_str = field_def[colon_pos + 1..].trim();
                    let fvalue = self.parse_expr_string(fvalue_str)?;
                    fields.push((fname, fvalue));
                }
            }

            return Ok(Expr::StructInit { name, fields });
        }

        // 回退
        self.parse_expr_string(trimmed)
    }

    fn collect_closure_value(&self, first_line: &str) -> Result<String> {
        // 如果闭包体在同一行结束，直接返回
        if first_line.ends_with("};") || first_line.ends_with('}') {
            return Ok(first_line.to_string());
        }

        // 找到当前行后收集需要的行
        // 简化：由于Parser是按行处理的，这里直接返回第一行
        // 多行闭包在实际使用中应该由parse_block_body处理

        // 检查是否有开始的 {
        if first_line.contains('{') {
            // 闭包体开始，需要收集到对应的 }
            let mut full = first_line.to_string();
            let mut brace_depth = first_line.matches('{').count() - first_line.matches('}').count();

            // 注意：这里我们不能前进Parser，只能返回当前行
            // 实际的多行收集需要在parse_let层面处理
            if brace_depth > 0 {
                // 返回不完整的闭包，后续行会被单独处理
                return Ok(full);
            }
            return Ok(full);
        }

        Ok(first_line.to_string())
    }

    fn parse_closure_expr(&self, s: &str) -> Result<Expr> {
        let trimmed = s.trim();
        println!("DEBUG: parse_closure_expr input='{}'", trimmed);

        // 检查是否是 move 闭包: $|params|
        let is_move = trimmed.starts_with("$|");
        let content = if is_move {
            &trimmed[2..] // 跳过 "$|"
        } else {
            &trimmed[1..] // 跳过 "|"
        };

        // 找到参数结束的 |
        let param_end = content.find('|').unwrap_or(0);
        let params_str = &content[..param_end];
        let rest = &content[param_end + 1..].trim();

        // 解析参数
        let mut params = vec![];
        for param_str in self.split_params(params_str) {
            let param_str = param_str.trim();
            if param_str.is_empty() {
                continue;
            }

            if let Some(colon_pos) = param_str.find(':') {
                let name = param_str[..colon_pos].trim().to_string();
                let ty = self.parse_type(&param_str[colon_pos + 1..]);
                params.push(Param {
                    name,
                    ty,
                    is_ref: false,
                    is_mut: false,
                });
            } else {
                params.push(Param {
                    name: param_str.to_string(),
                    ty: Type::Named("any".to_string()),
                    is_ref: false,
                    is_mut: false,
                });
            }
        }

        // 检查返回类型: -> type
        let (return_type, body_str) = if rest.starts_with("->") {
            let after_arrow = rest[2..].trim();
            // 找到 { 的位置
            if let Some(brace_pos) = after_arrow.find('{') {
                let ret_str = &after_arrow[..brace_pos].trim();
                let body = &after_arrow[brace_pos..];
                (Some(self.parse_type(ret_str)), body)
            } else {
                // 没有花括号，整个after_arrow是body
                (None, after_arrow)
            }
        } else {
            (None, *rest)
        };

        // 解析body - 修复问题1：正确处理多行闭包体
        let body = if body_str.starts_with('{') {
            // 块体 - 提取内部内容并解析为表达式
            let inner = body_str
                .trim_start_matches('{')
                .trim_end_matches('}')
                .trim()
                .trim_end_matches(';');
            
            if inner.is_empty() {
                // 空块
                Expr::Block {
                    stmts: vec![],
                    trailing_expr: None,
                }
            } else {
                // 将内部内容解析为表达式（作为trailing_expr）
                match self.parse_expr_string(inner) {
                    Ok(e) => Expr::Block {
                        stmts: vec![],
                        trailing_expr: Some(Box::new(e)),
                    },
                    Err(e) => {
                        println!(
                            "DEBUG: failed to parse block body expr inner='{}': {:?}",
                            inner, e
                        );
                        return Err(e);
                    }
                }
            }
        } else if body_str.is_empty() {
            // 空body
            Expr::Block {
                stmts: vec![],
                trailing_expr: None,
            }
        } else {
            // 表达式体（单行闭包）
            match self.parse_expr_string(body_str) {
                Ok(e) => e,
                Err(e) => {
                    println!(
                        "DEBUG: failed to parse expression body='{}': {:?}",
                        body_str, e
                    );
                    Expr::Raw(body_str.to_string())
                }
            }
        };

        Ok(Expr::Closure {
            params,
            return_type,
            body: Box::new(body),
            is_move,
        })
    }

    // ============ Match 解析（核心）============

    fn parse_match(&mut self) -> Result<Expr> {
        let line = self.current_line().trim().to_string();
        let content = &line[2..]; // 跳过 "M "

        // 提取目标表达式
        let target_str = if let Some(pos) = content.find('{') {
            content[..pos].trim()
        } else {
            content.trim()
        };

        let target = self.parse_expr_string(target_str)?;

        // 解析分支
        let arms = self.parse_match_arms()?;

        Ok(Expr::Match {
            target: Box::new(target),
            arms,
        })
    }

    fn parse_match_from_value(&mut self, value_str: &str) -> Result<Expr> {
        let content = &value_str[2..]; // 跳过 "M "

        // 提取目标表达式
        let target_str = if let Some(pos) = content.find('{') {
            content[..pos].trim()
        } else {
            content.trim()
        };

        let target = self.parse_expr_string(target_str)?;

        // 由于这是从let语句的value部分调用的，当前行还是let语句行
        // 需要推进到Match块开始，然后解析arms
        // parse_match_arms_inline不会再次推进
        self.advance();
        let arms = self.parse_match_arms_inline()?;

        Ok(Expr::Match {
            target: Box::new(target),
            arms,
        })
    }
    
    fn parse_match_arms_inline(&mut self) -> Result<Vec<MatchArm>> {
        let mut arms = vec![];

        // 不推进，直接从当前行开始
        while self.current_line < self.lines.len() {
            let line = self.current_line().trim().to_string();

            if line == "}" || line == "};" {
                // 不推进，让调用者处理
                break;
            }

            if line.is_empty() || line == "{" {
                self.advance();
                continue;
            }

            // 解析分支
            if line.contains("=>") {
                let arm = self.parse_match_arm_multiline()?;
                arms.push(arm);
                // parse_match_arm_multiline已经推进了
            } else {
                self.advance();
            }
        }

        Ok(arms)
    }

    fn parse_match_arms(&mut self) -> Result<Vec<MatchArm>> {
        let mut arms = vec![];

        // 跳过当前行
        self.advance();

        while self.current_line < self.lines.len() {
            let line = self.current_line().trim().to_string();

            if line == "}" || line == "};" {
                self.advance();
                break;
            }

            if line.is_empty() || line == "{" {
                self.advance();
                continue;
            }

            // 解析分支: pattern => body
            if line.contains("=>") {
                let arm = self.parse_match_arm_multiline()?;
                arms.push(arm);
            } else {
                self.advance();
            }
        }

        Ok(arms)
    }
    
    fn parse_match_arm_multiline(&mut self) -> Result<MatchArm> {
        let line = self.current_line().trim().to_string();
        
        // 分割 pattern => body
        let parts: Vec<&str> = line.splitn(2, "=>").collect();
        let pattern_str = parts[0].trim();
        let body_start = parts.get(1).map(|s| s.trim()).unwrap_or("");
        
        let pattern = self.parse_pattern(pattern_str)?;
        
        // 检查body是否是多行块: => {
        let body_expr = if body_start.trim_start().starts_with('{') && !body_start.trim().ends_with("},") && !body_start.trim().ends_with('}') {
            // 多行块体，需要收集完整内容
            let mut body_lines = vec![body_start.to_string()];
            let mut brace_depth = body_start.matches('{').count() - body_start.matches('}').count();
            
            self.advance();
            
            while self.current_line < self.lines.len() && brace_depth > 0 {
                let body_line = self.current_line().trim().to_string();
                brace_depth += body_line.matches('{').count();
                brace_depth -= body_line.matches('}').count();
                body_lines.push(body_line.clone());
                
                // 总是推进，不管depth
                self.advance();
            }
            
            // 合并所有行并解析
            let full_body = body_lines.join(" ");
            self.parse_arm_body(&full_body)?
        } else {
            // 单行body - 推进到下一行
            self.advance();
            self.parse_arm_body(body_start)?
        };
        
        Ok(MatchArm {
            pattern,
            guard: None,
            body: Box::new(body_expr),
        })
    }


    fn parse_arm_body(&self, body_str: &str) -> Result<Expr> {
        let trimmed = body_str.trim();

        // 清理括号和逗号
        let cleaned = if trimmed.starts_with('{') {
            let mut s = trimmed[1..].to_string();
            s = s.trim_end().to_string();
            if s.ends_with("},") {
                s = s[..s.len() - 2].to_string();
            } else if s.ends_with('}') {
                s = s[..s.len() - 1].to_string();
            }
            s
        } else {
            trimmed.trim_end_matches(',').to_string()
        };

        let body_trimmed = cleaned.trim();
        
        // 检查是否包含if语句(Match arm body中的if)
        if body_trimmed.starts_with("if ") {
            // 这是一个if表达式，需要特殊处理
            // 提取条件和then body
            if let Some(brace_pos) = body_trimmed.find('{') {
                let condition_part = body_trimmed[3..brace_pos].trim(); // "if " 之后到'{'之前
                let rest = &body_trimmed[brace_pos..];
                
                // 找到配对的}
                let mut depth = 0;
                let mut then_end = 0;
                for (i, ch) in rest.chars().enumerate() {
                    if ch == '{' { depth += 1; }
                    if ch == '}' {
                        depth -= 1;
                        if depth == 0 {
                            then_end = i;
                            break;
                        }
                    }
                }
                
                if then_end > 0 {
                    let then_body_str = &rest[1..then_end].trim();
                    let after_then = rest[then_end+1..].trim();
                    
                    // 解析条件
                    let condition = self.parse_expr_string(condition_part)?;
                    
                    // 解析then body - 可能是多个语句
                    let then_stmts = self.parse_stmts_from_string(then_body_str)?;
                    let (then_s, then_t) = self.extract_trailing_expr(then_stmts);
                    let then_expr = Expr::Block {
                        stmts: then_s,
                        trailing_expr: then_t,
                    };
                    
                    // 检查是否有else
                    let else_body = if after_then.starts_with("else") {
                        let else_part = after_then[4..].trim();
                        if else_part.starts_with('{') {
                            // 找到else body的结束
                            let else_stmts_str = else_part.trim_start_matches('{').trim_end_matches('}');
                            let else_stmts = self.parse_stmts_from_string(else_stmts_str)?;
                            let (else_s, else_t) = self.extract_trailing_expr(else_stmts);
                            Some(Box::new(Expr::Block {
                                stmts: else_s,
                                trailing_expr: else_t,
                            }))
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    
                    // 检查after_then是否还有trailing expression
                    let final_expr = if !after_then.is_empty() && !after_then.starts_with("else") {
                        // 有trailing expression，需要创建一个Block包含if和trailing
                        let if_expr = Expr::If {
                            condition: Box::new(condition),
                            then_body: Box::new(then_expr),
                            else_body,
                        };
                        let trailing = self.parse_expr_string(after_then)?;
                        Expr::Block {
                            stmts: vec![Stmt::ExprStmt(Box::new(if_expr))],
                            trailing_expr: Some(Box::new(trailing)),
                        }
                    } else {
                        Expr::If {
                            condition: Box::new(condition),
                            then_body: Box::new(then_expr),
                            else_body,
                        }
                    };
                    
                    return Ok(final_expr);
                }
            }
        }

        self.parse_expr_string(body_trimmed)
    }

    fn parse_pattern(&self, pattern_str: &str) -> Result<Pattern> {
        let trimmed = pattern_str.trim();

        // Ok(binding)
        if trimmed.starts_with("Ok(") || trimmed.starts_with("Ok (") {
            let start = trimmed.find('(').unwrap();
            let end = trimmed.rfind(')').unwrap_or(trimmed.len());
            let binding = trimmed[start + 1..end].trim().to_string();
            return Ok(Pattern::ResultOk(binding));
        }

        // Err(binding)
        if trimmed.starts_with("Err(") || trimmed.starts_with("Err (") {
            let start = trimmed.find('(').unwrap();
            let end = trimmed.rfind(')').unwrap_or(trimmed.len());
            let binding = trimmed[start + 1..end].trim().to_string();
            return Ok(Pattern::ResultErr(binding));
        }

        // Some(binding)
        if trimmed.starts_with("Some(") || trimmed.starts_with("Some (") {
            let start = trimmed.find('(').unwrap();
            let end = trimmed.rfind(')').unwrap_or(trimmed.len());
            let binding = trimmed[start + 1..end].trim().to_string();
            return Ok(Pattern::OptionSome(binding));
        }

        // None
        if trimmed == "None" {
            return Ok(Pattern::OptionNone);
        }

        // 通配符
        if trimmed == "_" {
            return Ok(Pattern::Wildcard);
        }

        // 枚举变体: Type::Variant 或 Type::Variant(bindings)
        if trimmed.contains("::") {
            let parts: Vec<&str> = trimmed.splitn(2, "::").collect();
            let path = format!("{}::{}", parts[0], parts.get(1).unwrap_or(&""));
            let bindings = if let Some(paren_start) = path.find('(') {
                if let Some(paren_end) = path.rfind(')') {
                    path[paren_start + 1..paren_end]
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect()
                } else {
                    vec![]
                }
            } else {
                vec![]
            };
            return Ok(Pattern::EnumVariant { path, bindings });
        }

        // 字符串字面量
        if trimmed.starts_with('"') && trimmed.ends_with('"') {
            return Ok(Pattern::Literal(Literal::String(
                trimmed[1..trimmed.len() - 1].to_string(),
            )));
        }

        // 整数字面量
        if let Ok(num) = trimmed.parse::<i64>() {
            return Ok(Pattern::Literal(Literal::Integer(num)));
        }

        // 标识符
        Ok(Pattern::Ident(trimmed.to_string()))
    }

    // ============ If 解析 ============

    fn parse_if(&mut self) -> Result<Expr> {
        let line = self.current_line().trim().to_string();
        // line starts with "? " or "?"
        let content_start = if line.starts_with("? ") { 2 } else { 1 };
        let content = &line[content_start..];

        // Find block start
        let brace_pos = content.find('{').unwrap_or(content.len());
        let condition_str = content[..brace_pos].trim();

        // Handle if let vs normal if
        let mut if_let_info = None;
        let mut condition_expr = None;

        // 修复问题6: 处理 if let 语法
        if condition_str.starts_with("let ") {
            let inner = condition_str[4..].trim();
            if let Some(eq_pos) = inner.find('=') {
                let pattern_str = inner[..eq_pos].trim();
                let expr_str = inner[eq_pos + 1..].trim();
                let pattern = self.parse_pattern(pattern_str)?;
                let expr = self.parse_expr_string(expr_str)?;
                if_let_info = Some((pattern, expr));
            } else {
                // Fallback for valid "let" variable usage? Unlikely in condition.
                condition_expr = Some(self.parse_expr_string(condition_str)?);
            }
        } else {
            condition_expr = Some(self.parse_expr_string(condition_str)?);
        }

        let mut then_body_stmts = vec![];
        let mut else_body = None;

        // Check for inline block: { ... }
        if brace_pos < content.len() {
            let rest = &content[brace_pos..];
            // 简单处理：如果行内包含 }，假设是行内块
            // 统计 braces
            let mut depth = 0;
            let mut close_pos = None;
            for (i, c) in rest.char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            close_pos = Some(i);
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if let Some(end) = close_pos {
                // Found closing brace on same line -> Inline Block
                let block_inner = &rest[1..end]; // Inside { }
                if !block_inner.trim().is_empty() {
                    then_body_stmts = self.parse_stmts_from_string(block_inner)?;
                }

                // Check for else on same line
                let after_block = &rest[end + 1..];
                if after_block.trim().starts_with("else") {
                    let else_rest = after_block.trim()[4..].trim(); // skip "else"
                    if else_rest.starts_with('{') {
                        if else_rest == "{" {
                            // else block starts here but continues next line
                            self.advance();
                            // Parse else body (multiline)
                            let else_stmts = self.parse_block_body()?;
                            let (else_s, else_t) = self.extract_trailing_expr(else_stmts);
                            else_body = Some(Box::new(Expr::Block {
                                stmts: else_s,
                                trailing_expr: else_t,
                            }));
                        } else {
                            // Inline else block
                            let inner = else_rest
                                .trim_start_matches('{')
                                .trim_end_matches('}')
                                .trim();
                            if !inner.is_empty() {
                                let else_stmts = self.parse_stmts_from_string(inner)?;
                                let (else_s, else_t) = self.extract_trailing_expr(else_stmts);
                                else_body = Some(Box::new(Expr::Block {
                                    stmts: else_s,
                                    trailing_expr: else_t,
                                }));
                            }
                        }
                    } else if else_rest.is_empty() {
                        // else on next line?
                        self.advance();
                        let current = self.current_line().trim();
                        if current.starts_with('{') {
                            // else block
                            let else_stmts = self.parse_block_body()?;
                            let (else_s, else_t) = self.extract_trailing_expr(else_stmts);
                            else_body = Some(Box::new(Expr::Block {
                                stmts: else_s,
                                trailing_expr: else_t,
                            }));
                        }
                    } else {
                        self.advance();
                    }
                } else {
                    // No else on this line
                    // Advance past this line
                    self.advance();
                }
            } else {
                // Multiline start
                self.advance();
                then_body_stmts = self.parse_block_body()?;
            }
        } else {
            // Assume multiline block follows
            self.advance();
            then_body_stmts = self.parse_block_body()?;
        }

        // extract trailing
        let (then_stmts, then_trailing) = self.extract_trailing_expr(then_body_stmts);
        let then_expr = Expr::Block {
            stmts: then_stmts,
            trailing_expr: then_trailing,
        };

        // If else_body not parsed yet, check lines
        if else_body.is_none() {
            let binding = self.current_line().clone().to_string(); // clone to avoid borrow check issues
            let current = binding.as_str().trim();
            if current.starts_with("else") {
                let after_else = current[4..].trim();
                if after_else == "{" || after_else.starts_with('{') {
                    // else {
                    if after_else == "{" {
                        self.advance(); // consume else {
                        let else_stmts_raw = self.parse_block_body()?;
                        let (else_s, else_t) = self.extract_trailing_expr(else_stmts_raw);
                        else_body = Some(Box::new(Expr::Block {
                            stmts: else_s,
                            trailing_expr: else_t,
                        }));
                    } else {
                        // inline else on new line?
                        // else { stmt }
                        let inner = after_else
                            .trim_start_matches('{')
                            .trim_end_matches('}')
                            .trim();
                        let else_stmts = self.parse_stmts_from_string(inner)?;
                        let (else_s, else_t) = self.extract_trailing_expr(else_stmts);
                        else_body = Some(Box::new(Expr::Block {
                            stmts: else_s,
                            trailing_expr: else_t,
                        }));
                        self.advance();
                    }
                }
            }
        }

        if let Some((pattern, target)) = if_let_info {
            // Desugar to Match
            let arms = vec![
                MatchArm {
                    pattern,
                    guard: None,
                    body: Box::new(then_expr),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: if let Some(e) = else_body {
                        e
                    } else {
                        Box::new(Expr::Block {
                            stmts: vec![],
                            trailing_expr: None,
                        })
                    },
                },
            ];
            Ok(Expr::Match {
                target: Box::new(target),
                arms,
            })
        } else {
            Ok(Expr::If {
                condition: Box::new(condition_expr.unwrap()),
                then_body: Box::new(then_expr),
                else_body,
            })
        }
    }

    fn parse_stmts_from_string(&self, s: &str) -> Result<Vec<Stmt>> {
        let mut stmts = vec![];
        // Split by ; but respect quotes/parens
        // 简化：split by ;
        for part in s.split(';') {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(expr) = self.parse_expr_string(trimmed) {
                stmts.push(Stmt::ExprStmt(Box::new(expr)));
            } else {
                stmts.push(Stmt::Raw(trimmed.to_string()));
            }
        }
        Ok(stmts)
    }

    // ============ 其他表达式 ============

    fn parse_return(&mut self) -> Result<Expr> {
        let line = self.current_line().trim();

        if line == "<" {
            return Ok(Expr::Return(None));
        }

        let content = line
            .trim_start_matches('<')
            .trim()
            .trim_end_matches(';')
            .trim();
        if content.is_empty() {
            return Ok(Expr::Return(None));
        }

        // 修复#8: 解析返回值时避免嵌套Return
        let value = self.parse_expr_string(content)?;
        // 如果解析出的表达式已经是Return，不要再次包装
        if matches!(value, Expr::Return(_)) {
            return Ok(value);
        }
        Ok(Expr::Return(Some(Box::new(value))))
    }

    fn parse_loop(&mut self) -> Result<Expr> {
        let line = self.current_line().trim().to_string();

        // 修复问题6: 处理 while let 语法 - 将其转换为无限循环+match+break
        if line.starts_with("L ") && line.contains("let ") && line.contains('=') {
            // while let 循环: L let pattern = expr { body }
            let content = &line[2..].trim(); // 跳过 "L "
            
            if content.starts_with("let ") {
                let inner = &content[4..];
                if let Some(eq_pos) = inner.find('=') {
                    let pattern_str = inner[..eq_pos].trim();
                    let rest = &inner[eq_pos + 1..];
                    let brace_pos = rest.find('{').unwrap_or(rest.len());
                    let expr_str = rest[..brace_pos].trim();
                    
                    let pattern = self.parse_pattern(pattern_str)?;
                    let expr = self.parse_expr_string(expr_str)?;
                    
                    // 解析循环体
                    self.advance();
                    let body_stmts = self.parse_block_body()?;
                    let (body_s, body_t) = self.extract_trailing_expr(body_stmts);
                    
                    // 转换为: loop { match expr { pattern => { body }, _ => break } }
                    let match_expr = Expr::Match {
                        target: Box::new(expr),
                        arms: vec![
                            MatchArm {
                                pattern,
                                guard: None,
                                body: Box::new(Expr::Block {
                                    stmts: body_s,
                                    trailing_expr: body_t,
                                }),
                            },
                            MatchArm {
                                pattern: Pattern::Wildcard,
                                guard: None,
                                body: Box::new(Expr::Break),
                            },
                        ],
                    };
                    
                    return Ok(Expr::Loop {
                        body: Box::new(Expr::Block {
                            stmts: vec![Stmt::ExprStmt(Box::new(match_expr))],
                            trailing_expr: None,
                        }),
                    });
                }
            }
        }

        // 修复问题2: 检查是否是复杂的单行for循环: L {? ... L x in iter { ... }}
        // 这种情况需要作为Raw表达式直接透传，由后期处理
        if line.starts_with("L {") && line.contains(" in ") {
            // 这是复杂的单行循环，透传为Raw
            return Ok(Expr::Raw(line.clone()));
        }

        // 检查是否是标准 for 循环: L pattern in iterator { body }
        if line.starts_with("L ") && line.contains(" in ") {
            // For 循环
            let content = &line[2..]; // 跳过 "L "

            // 找到 "in" 的位置
            if let Some(in_pos) = content.find(" in ") {
                let pattern = content[..in_pos].trim().to_string();

                // 提取迭代器表达式（在 "in" 和 "{" 之间）
                let after_in = &content[in_pos + 4..];
                let brace_pos = after_in.find('{').unwrap_or(after_in.len());
                let iterator_str = after_in[..brace_pos].trim();

                let iterator = self.parse_expr_string(iterator_str)?;

                // 解析循环体
                self.advance();
                let body_stmts = self.parse_block_body()?;
                let (stmts, trailing) = self.extract_trailing_expr(body_stmts);

                return Ok(Expr::For {
                    pattern,
                    iterator: Box::new(iterator),
                    body: Box::new(Expr::Block {
                        stmts,
                        trailing_expr: trailing,
                    }),
                });
            }
        }

        // 否则是无限循环: L { body }
        self.advance();
        let body_stmts = self.parse_block_body()?;
        let (stmts, trailing) = self.extract_trailing_expr(body_stmts);

        Ok(Expr::Loop {
            body: Box::new(Expr::Block {
                stmts,
                trailing_expr: trailing,
            }),
        })
    }

    fn parse_while_let(&mut self) -> Result<Expr> {
        let line = self.current_line().trim().to_string();
        let content = &line[10..].trim(); // 跳过 "while let "
        
        // 格式: while let pattern = expr { body }
        if let Some(eq_pos) = content.find('=') {
            let pattern_str = content[..eq_pos].trim();
            let rest = &content[eq_pos + 1..];
            let brace_pos = rest.find('{').unwrap_or(rest.len());
            let expr_str = rest[..brace_pos].trim();
            
            let pattern = self.parse_pattern(pattern_str)?;
            let expr = self.parse_expr_string(expr_str)?;
            
            // 解析循环体
            self.advance();
            let body_stmts = self.parse_block_body()?;
            let (body_s, body_t) = self.extract_trailing_expr(body_stmts);
            
            // 转换为: loop { match expr { pattern => { body }, _ => break } }
            let match_expr = Expr::Match {
                target: Box::new(expr),
                arms: vec![
                    MatchArm {
                        pattern,
                        guard: None,
                        body: Box::new(Expr::Block {
                            stmts: body_s,
                            trailing_expr: body_t,
                        }),
                    },
                    MatchArm {
                        pattern: Pattern::Wildcard,
                        guard: None,
                        body: Box::new(Expr::Break),
                    },
                ],
            };
            
            return Ok(Expr::Loop {
                body: Box::new(Expr::Block {
                    stmts: vec![Stmt::ExprStmt(Box::new(match_expr))],
                    trailing_expr: None,
                }),
            });
        }
        
        // 解析失败，返回Raw
        Ok(Expr::Raw(line.clone()))
    }

    fn parse_for(&mut self) -> Result<Expr> {
        let line = self.current_line().trim().to_string();
        let content = &line[4..]; // 跳过 "for "

        // 修复#4: for循环解析 - 提取 pattern in iterator
        if let Some(in_pos) = content.find(" in ") {
            let pattern = content[..in_pos].trim().to_string();
            
            // 提取迭代器表达式（在 "in" 和 "{" 之间）
            let after_in = &content[in_pos + 4..];
            let brace_pos = after_in.find('{').unwrap_or(after_in.len());
            let iterator_str = after_in[..brace_pos].trim();
            
            let iterator = self.parse_expr_string(iterator_str)?;
            
            // 解析循环体
            self.advance();
            let body_stmts = self.parse_block_body()?;
            let (stmts, trailing) = self.extract_trailing_expr(body_stmts);
            
            return Ok(Expr::For {
                pattern,
                iterator: Box::new(iterator),
                body: Box::new(Expr::Block {
                    stmts,
                    trailing_expr: trailing,
                }),
            });
        }

        // 如果解析失败，跳过块
        self.skip_block()?;
        Ok(Expr::Raw(format!("/* {} */", line)))
    }

    // ============ 表达式字符串解析 ============

    fn parse_expr_string(&self, expr_str: &str) -> Result<Expr> {
        println!("DEBUG: parse_expr_string input='{}'", expr_str);
        // 规范化：去除 :: 周围空格，去除函数调用前空格
        let normalized = expr_str
            .trim()
            .replace(" :: ", "::")
            .replace(" . ", ".")
            .replace(" (", "(")
            .replace("( ", "(")
            .replace(" )", ")")
            .replace(") ", ")");

        // 转换 Rust turbofish 语法: method::<Type>() -> method<Type>()
        let normalized = normalized.replace("::<", "<");

        let trimmed = normalized.trim().trim_end_matches(';').trim();

        if trimmed.is_empty() {
            return Ok(Expr::Literal(Literal::Null));
        }

        // Return 表达式
        if trimmed.starts_with("< ") || trimmed.starts_with("<") {
            let value_str = trimmed.trim_start_matches('<').trim();
            if value_str.is_empty() {
                return Ok(Expr::Return(None));
            }
            // 修复#8: 递归解析时避免重复生成Return
            let value = self.parse_expr_string(value_str)?;
            // 如果已经是Return，不要再次包装
            if matches!(value, Expr::Return(_)) {
                return Ok(value);
            }
            return Ok(Expr::Return(Some(Box::new(value))));
        }

        // Closure (High priority)
        if trimmed.starts_with('|') {
            if let Ok(closure) = self.parse_closure_expr(trimmed) {
                return Ok(closure);
            }
        }

        // If Expression (? ...)
        if trimmed.starts_with('?') {
            if let Ok(if_expr) = self.parse_if_expr_string(trimmed) {
                return Ok(if_expr);
            }
        }

        // 整数
        if let Ok(num) = trimmed.parse::<i64>() {
            return Ok(Expr::Literal(Literal::Integer(num)));
        }

        // 浮点数
        if trimmed.contains('.') && !trimmed.contains('(') && !trimmed.contains('"') {
            if let Ok(num) = trimmed.parse::<f64>() {
                return Ok(Expr::Literal(Literal::Float(num)));
            }
        }

        // 布尔
        if trimmed == "true" {
            return Ok(Expr::Literal(Literal::Bool(true)));
        }
        if trimmed == "false" {
            return Ok(Expr::Literal(Literal::Bool(false)));
        }

        // 字符串
        if trimmed.starts_with('"') && trimmed.ends_with('"') {
            return Ok(Expr::Literal(Literal::String(
                trimmed[1..trimmed.len() - 1].to_string(),
            )));
        }

        // 宏调用: name!(...) 或 name ! (...) 或 name![...] 或 name ! [...]
        if trimmed.contains("!(")
            || trimmed.contains("! (")
            || trimmed.contains("![")
            || trimmed.contains("! [")
            || trimmed.ends_with("!()")
        {
            // 找到 ! 的位置
            let exclaim_pos = trimmed.find('!').unwrap();
            let name = trimmed[..exclaim_pos].trim().to_string();
            // 找到参数开始位置
            let args = if let Some(paren_start) = trimmed[exclaim_pos..].find('(') {
                let start = exclaim_pos + paren_start;
                // 找到匹配的右括号
                let end = self.find_matching_paren(trimmed, start);
                if end > start + 1 {
                    trimmed[start + 1..end].to_string()
                } else {
                    String::new()
                }
            } else if let Some(bracket_start) = trimmed[exclaim_pos..].find('[') {
                let start = exclaim_pos + bracket_start;
                // 找到匹配的右括号
                let mut depth = 0;
                let mut byte_pos = start;
                for c in trimmed[start..].chars() {
                    match c {
                        '[' => depth += 1,
                        ']' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    byte_pos += c.len_utf8();
                }
                if byte_pos > start + 1 {
                    trimmed[start + 1..byte_pos].to_string()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            return Ok(Expr::Macro { name, args });
        }

        // 路径或枚举构造: Type::Variant(args) 或 path::item
        if trimmed.contains("::") {
            // 检查是否有参数
            if let Some(paren_pos) = trimmed.find('(') {
                let path_part = &trimmed[..paren_pos];
                let parts: Vec<&str> = path_part.rsplitn(2, "::").collect();
                if parts.len() == 2 {
                    let method_or_variant = parts[0].to_string();
                    let type_name = parts[1].to_string();

                    // 检查是否是构造函数/静态方法调用（如 BinaryHeap::new(), String::from()）
                    // 特征：方法名是小写开头（new, from, default等）或全小写
                    let is_static_method = method_or_variant
                        .chars()
                        .next()
                        .map(|c| c.is_lowercase())
                        .unwrap_or(false);

                    if is_static_method {
                        // 这是静态方法调用，解析为 Call 表达式
                        let args_str = &trimmed[paren_pos + 1..].trim_end_matches(')');
                        let args: Vec<Expr> = if args_str.is_empty() {
                            vec![]
                        } else {
                            self.split_args(args_str)
                                .into_iter()
                                .map(|a| self.parse_expr_string(a.trim()))
                                .collect::<Result<Vec<_>>>()?
                        };

                        // 构造 Type::method 路径作为函数
                        let func_path = Expr::Path {
                            segments: vec![
                                type_name.trim().to_string(),
                                method_or_variant.trim().to_string(),
                            ],
                        };

                        return Ok(Expr::Call {
                            func: Box::new(func_path),
                            args,
                        });
                    } else {
                        // 这是枚举变体构造
                        let args_str = &trimmed[paren_pos + 1..].trim_end_matches(')');
                        let args = if args_str.is_empty() {
                            Some(vec![])
                        } else {
                            let parsed_args: Result<Vec<Expr>> = self
                                .split_args(args_str)
                                .into_iter()
                                .map(|a| self.parse_expr_string(a.trim()))
                                .collect();
                            Some(parsed_args?)
                        };

                        return Ok(Expr::EnumVariant {
                            enum_name: type_name.trim().to_string(),
                            variant: method_or_variant.trim().to_string(),
                            args,
                        });
                    }
                }
            }

            // 纯路径 - 清理空格
            let segments: Vec<String> = trimmed.split("::").map(|s| s.trim().to_string()).collect();
            return Ok(Expr::Path { segments });
        }

        // 结构体初始化: TypeName { field: value, ... }
        // 必须在函数调用检测之前，因为函数调用也包含 '('
        if trimmed.contains('{') && trimmed.contains('}') {
            // 查找 '{' 前的部分
            if let Some(brace_pos) = trimmed.find('{') {
                let before_brace = trimmed[..brace_pos].trim();
                // 检查是否是大写开头的标识符（结构体名）
                if !before_brace.is_empty()
                    && before_brace.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                    && !before_brace.contains('(')  // 排除函数调用
                    && !before_brace.contains('.')
                // 排除方法调用
                {
                    // 提取字段列表
                    let end_brace = trimmed.rfind('}').unwrap();
                    let fields_str = &trimmed[brace_pos + 1..end_brace];

                    // 解析字段: field: value, field2: value2
                    let mut fields = Vec::new();
                    for field_str in fields_str.split(',') {
                        let field_str = field_str.trim();
                        if field_str.is_empty() {
                            continue;
                        }

                        if let Some(colon_pos) = field_str.find(':') {
                            let name = field_str[..colon_pos].trim().to_string();
                            let value_str = field_str[colon_pos + 1..].trim();
                            let value = self.parse_expr_string(value_str)?;
                            fields.push((name, value));
                        }
                    }

                    return Ok(Expr::StructInit {
                        name: before_brace.to_string(),
                        fields,
                    });
                }
            }
        }

        // 函数调用
        if trimmed.contains('(') && trimmed.ends_with(')') {
            // Find splitting paren (matching the last ')')
            let mut paren_pos = 0;
            let mut depth = 0;
            let mut found = false;
            // Scan backwards from character before the last ')'
            for (i, c) in trimmed[..trimmed.len() - 1].char_indices().rev() {
                match c {
                    ')' => depth += 1,
                    '(' => {
                        if depth == 0 {
                            paren_pos = i;
                            found = true;
                            break;
                        }
                        depth -= 1;
                    }
                    _ => {}
                }
            }

            if found {
                let func_str = trimmed[..paren_pos].trim();
                let args_str = &trimmed[paren_pos + 1..trimmed.len() - 1];

                // Recursively parse function part (supports chaining)
                let func_expr = self.parse_expr_string(func_str)?;

                let args: Result<Vec<Expr>> = if args_str.is_empty() {
                    Ok(vec![])
                } else {
                    self.split_args(args_str)
                        .into_iter()
                        .map(|a| self.parse_expr_string(a.trim()))
                        .collect()
                };

                return Ok(Expr::Call {
                    func: Box::new(func_expr),
                    args: args?,
                });
            }
        }

        // 二元操作（简化）
        for (op_str, op) in [
            ("==", BinOp::Eq),
            ("!=", BinOp::Ne),
            ("<=", BinOp::Le),
            (">=", BinOp::Ge),
            ("&&", BinOp::And),
            ("||", BinOp::Or),
            ("+", BinOp::Add),
            ("-", BinOp::Sub),
            ("*", BinOp::Mul),
            ("/", BinOp::Div),
            ("<", BinOp::Lt),
            (">", BinOp::Gt),
        ] {
            if let Some(pos) = trimmed.find(op_str) {
                // Ignore if at start (Unary)
                if pos > 0
                    && !self.is_inside_parens(trimmed, pos)
                    && !self.is_inside_string(trimmed, pos)
                {
                    let left_str = &trimmed[..pos];
                    let right_str = &trimmed[pos + op_str.len()..];
                    return Ok(Expr::Binary {
                        left: Box::new(self.parse_expr_string(left_str)?),
                        op,
                        right: Box::new(self.parse_expr_string(right_str)?),
                    });
                }
            }
        }

        // Unary Operators
        if trimmed.starts_with('!') {
            return Ok(Expr::Unary {
                op: UnOp::Not,
                expr: Box::new(self.parse_expr_string(&trimmed[1..])?),
            });
        }
        if trimmed.starts_with('&') {
            let inner = trimmed[1..].trim();
            if inner.starts_with("mut ") {
                return Ok(Expr::Unary {
                    op: UnOp::RefMut,
                    expr: Box::new(self.parse_expr_string(&inner[4..])?),
                });
            }
            return Ok(Expr::Unary {
                op: UnOp::Ref,
                expr: Box::new(self.parse_expr_string(inner)?),
            });
        }
        if trimmed.starts_with('*') {
            return Ok(Expr::Unary {
                op: UnOp::Deref,
                expr: Box::new(self.parse_expr_string(&trimmed[1..])?),
            });
        }
        if trimmed.starts_with('-') {
            return Ok(Expr::Unary {
                op: UnOp::Neg,
                expr: Box::new(self.parse_expr_string(&trimmed[1..])?),
            });
        }

        // Closure
        if trimmed.starts_with('|') {
            if let Ok(closure) = self.parse_closure_expr(trimmed) {
                return Ok(closure);
            }
        }

        // 在返回Ident前，尝试识别包含闭包的复杂表达式
        // 如果表达式包含 |...| 模式但没被识别为闭包，可能是方法链中的闭包参数
        if trimmed.contains('|') && trimmed.contains('(') {
            // 尝试作为Raw表达式保留原始语法，让codegen直接输出
            // 但这会导致无效的TypeScript
            // 更好的方案：检测 .method(|param| body) 模式并转换
            Ok(Expr::Raw(trimmed.to_string()))
        } else {
            // 标识符或透传
            Ok(Expr::Ident(trimmed.to_string()))
        }
    }

    // ============ 辅助方法 ============

    fn parse_if_expr_string(&self, s: &str) -> Result<Expr> {
        let content = s.trim().trim_start_matches('?').trim();

        let brace_pos = content.find('{').unwrap_or(content.len());
        let condition_str = content[..brace_pos].trim();
        let condition = self.parse_expr_string(condition_str)?;

        let mut then_body = Expr::Block {
            stmts: vec![],
            trailing_expr: None,
        };
        let mut else_body = None;

        if brace_pos < content.len() {
            let rest = &content[brace_pos..];
            // Find closing brace of then block
            let mut depth = 0;
            let mut then_close = None;
            for (i, c) in rest.char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            then_close = Some(i);
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if let Some(end) = then_close {
                let inner = rest[1..end].trim();
                let params = self.split_params(inner); // Misuse split_params? No, use parse_stmts logic.
                                                       // Assuming inline expression body for simplicity or semicolon separated
                let stmts = self.parse_stmts_from_string(inner)?;
                let (s, t) = self.extract_trailing_expr(stmts);
                then_body = Expr::Block {
                    stmts: s,
                    trailing_expr: t,
                };

                let after = rest[end + 1..].trim();
                if after.starts_with("else") {
                    let else_inner = after[4..].trim();
                    if else_inner.starts_with('{') {
                        let inner = else_inner
                            .trim_start_matches('{')
                            .trim_end_matches('}')
                            .trim();
                        let stmts = self.parse_stmts_from_string(inner)?;
                        let (s, t) = self.extract_trailing_expr(stmts);
                        else_body = Some(Box::new(Expr::Block {
                            stmts: s,
                            trailing_expr: t,
                        }));
                    }
                }
            }
        }

        Ok(Expr::If {
            condition: Box::new(condition),
            then_body: Box::new(then_body),
            else_body,
        })
    }

    fn parse_type(&self, type_str: &str) -> Type {
        let trimmed = type_str.trim();

        // 引用类型
        if trimmed.starts_with("&!") {
            return Type::Reference {
                is_mut: true,
                inner: Box::new(self.parse_type(&trimmed[2..])),
            };
        }
        if trimmed.starts_with("&") {
            return Type::Reference {
                is_mut: false,
                inner: Box::new(self.parse_type(&trimmed[1..])),
            };
        }

        // 元组类型: (Type1, Type2)
        if trimmed.starts_with('(') && trimmed.ends_with(')') {
            let inner = &trimmed[1..trimmed.len() - 1];
            let types: Vec<Type> = self
                .split_params(inner)
                .into_iter()
                .map(|s| self.parse_type(s.trim()))
                .collect();
            if types.len() > 1 {
                return Type::Tuple(types);
            }
        }

        // 泛型
        if let Some(lt_pos) = trimmed.find('<') {
            let gt_pos = self.find_matching_bracket(trimmed, lt_pos);
            if gt_pos > lt_pos {
                let base = trimmed[..lt_pos].to_string();
                let params_str = &trimmed[lt_pos + 1..gt_pos];
                let params: Vec<Type> = self
                    .split_params(params_str)
                    .into_iter()
                    .map(|s| self.parse_type(s.trim()))
                    .collect();
                return Type::Generic { base, params };
            }
        }

        Type::Named(trimmed.to_string())
    }

    fn find_matching_paren(&self, s: &str, start: usize) -> usize {
        let mut depth = 0;
        let mut in_string = false;
        let mut prev_char = ' ';
        let mut byte_pos = start;

        for c in s[start..].chars() {
            // 处理字符串字面量
            if c == '"' && prev_char != '\\' {
                in_string = !in_string;
            }

            if !in_string {
                match c {
                    '(' | '<' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            return byte_pos;
                        }
                    }
                    '>' => depth -= 1,
                    _ => {}
                }
            }
            prev_char = c;
            byte_pos += c.len_utf8();
        }
        s.len()
    }

    fn find_matching_bracket(&self, s: &str, start: usize) -> usize {
        let mut depth = 0;
        for (i, c) in s[start..].chars().enumerate() {
            match c {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        return start + i;
                    }
                }
                _ => {}
            }
        }
        s.len()
    }

    fn split_params<'a>(&self, params_str: &'a str) -> Vec<&'a str> {
        let mut result = vec![];
        let mut depth = 0;
        let mut in_closure = false;
        let mut start = 0;

        for (i, c) in params_str.char_indices() {
            match c {
                '|' => {
                    // 闭包边界：|param| body
                    in_closure = !in_closure;
                }
                '<' | '(' | '[' | '{' if !in_closure => depth += 1,
                '>' | ')' | ']' | '}' if !in_closure => depth -= 1,
                ',' if depth == 0 && !in_closure => {
                    result.push(&params_str[start..i]);
                    start = i + 1;
                }
                _ => {}
            }
        }

        if start < params_str.len() {
            result.push(&params_str[start..]);
        }

        result
    }

    fn split_args<'a>(&self, args_str: &'a str) -> Vec<&'a str> {
        self.split_params(args_str)
    }

    fn is_inside_parens(&self, s: &str, pos: usize) -> bool {
        let mut depth = 0;
        for (i, c) in s.chars().enumerate() {
            if i >= pos {
                break;
            }
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
        }
        depth > 0
    }

    fn is_inside_string(&self, s: &str, pos: usize) -> bool {
        let mut in_string = false;
        for (i, c) in s.chars().enumerate() {
            if i >= pos {
                break;
            }
            if c == '"' {
                in_string = !in_string;
            }
        }
        in_string
    }

    fn collect_block(&mut self) -> Result<Vec<String>> {
        let mut lines = vec![];
        let mut brace_depth = 0;

        let current = self.current_line().to_string();
        if current.contains('{') {
            brace_depth = 1;
            self.advance();
        }

        while self.current_line < self.lines.len() && brace_depth > 0 {
            let line = self.current_line().to_string();
            brace_depth += line.matches('{').count();
            brace_depth = brace_depth.saturating_sub(line.matches('}').count());
            if brace_depth > 0 {
                lines.push(line);
            }
            self.advance();
        }

        Ok(lines)
    }

    fn skip_block(&mut self) -> Result<()> {
        let mut brace_depth = 0;

        let current = self.current_line().to_string();
        if current.contains('{') {
            brace_depth = 1;
            self.advance();
        }

        while self.current_line < self.lines.len() && brace_depth > 0 {
            let line = self.current_line().to_string();
            brace_depth += line.matches('{').count();
            brace_depth = brace_depth.saturating_sub(line.matches('}').count());
            self.advance();
        }

        Ok(())
    }

    fn current_line(&self) -> &str {
        if self.current_line < self.lines.len() {
            &self.lines[self.current_line]
        } else {
            ""
        }
    }

    fn advance(&mut self) {
        self.current_line += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_match() {
        let input = r#"M x {
    Ok(v): { v },
    Err(_): { 0 }
}"#;

        let mut parser = Parser::new(input);
        let file = parser.parse_file().unwrap();

        assert_eq!(file.items.len(), 1);
    }

    #[test]
    fn test_parse_enum_pattern() {
        let parser = Parser::new("");

        let pat = parser.parse_pattern("Operator::Add").unwrap();
        assert!(matches!(pat, Pattern::EnumVariant { .. }));

        let pat = parser
            .parse_pattern("CalcError::InvalidOperator(op)")
            .unwrap();
        assert!(matches!(pat, Pattern::EnumVariant { .. }));
    }

    #[test]
    fn test_parse_function() {
        let input = r#"f test(x: i32) -> i32 {
    < x + 1
}"#;

        let mut parser = Parser::new(input);
        let file = parser.parse_file().unwrap();

        assert_eq!(file.items.len(), 1);
        if let Item::Function(f) = &file.items[0] {
            assert_eq!(f.name, "test");
            assert_eq!(f.params.len(), 1);
        } else {
            panic!("Expected Function");
        }
    }
}
