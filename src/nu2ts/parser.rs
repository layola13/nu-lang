// Nu2TS Parser (完整版)
// 递归下降解析器，将 Nu 代码解析为 AST
// 策略：精确解析核心语法（Match、函数签名），透传复杂结构

use super::ast::*;
use anyhow::{Result, bail};

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
        let stmts: Vec<Stmt> = file.items.into_iter().filter_map(|item| {
            match item {
                Item::Function(f) => Some(Stmt::ExprStmt(Box::new(Expr::Block {
                    stmts: vec![],
                    trailing_expr: None,
                }))),
                Item::Raw(s) => Some(Stmt::Raw(s)),
                _ => None,
            }
        }).collect();

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

        // 结构体: s Name { ... }
        if line.starts_with("s ") {
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

        // 解析函数体
        let body_stmts = self.parse_block_body()?;

        Ok(FunctionDef {
            name,
            params,
            return_type,
            body: Box::new(Expr::Block {
                stmts: body_stmts,
                trailing_expr: None,
            }),
            is_pub,
            is_async: false,
            attributes: vec![],
        })
    }

    fn parse_struct(&mut self) -> Result<StructDef> {
        let line = self.current_line().trim().to_string();
        let content = &line[2..].trim(); // 跳过 "s "

        // 提取名称
        let name = content.split('{').next().unwrap_or("").trim().to_string();

        // 解析字段（透传）
        let fields_raw = self.collect_block()?;

        Ok(StructDef {
            name,
            fields: vec![], // 透传到 codegen 处理
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

            if method_line == "}" {
                break;
            }

            if method_line.is_empty() {
                self.advance();
                continue;
            }

            // 解析方法
            if method_line.starts_with("f ") || method_line.starts_with("F ") {
                methods.push(self.parse_function()?);
            }

            self.advance();
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

    fn parse_function_signature(&self, content: &str) -> Result<(String, Vec<Param>, Option<Type>)> {
        let name = content.split('(').next().unwrap_or("").trim().to_string();

        let mut params = vec![];
        let mut return_type = None;

        // 提取参数部分
        if let Some(start) = content.find('(') {
            let end = self.find_matching_paren(content, start);
            if end > start {
                let params_str = &content[start+1..end];
                if !params_str.trim().is_empty() {
                    for param_str in self.split_params(params_str) {
                        let param_str = param_str.trim();
                        if let Some(colon_pos) = param_str.find(':') {
                            let param_name = param_str[..colon_pos].trim();
                            let param_type_str = param_str[colon_pos+1..].trim();

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
            let after_arrow = &content[arrow_pos+2..];
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
        if current.contains('{') {
            brace_depth = 1;
            self.advance();
        }

        while self.current_line < self.lines.len() && brace_depth > 0 {
            let line = self.current_line().trim().to_string();

            // 检查是否到达函数结束
            if line == "}" && brace_depth == 1 {
                break;
            }

            // 跳过空行和注释
            if line.is_empty() || line.starts_with("//") {
                self.advance();
                continue;
            }

            // Match 语句 - 需要精确解析
            if line.starts_with("M ") {
                if let Some(stmt) = self.parse_stmt()? {
                    stmts.push(stmt);
                }
                continue;
            }

            // If 语句
            if line.starts_with("? ") {
                if let Some(stmt) = self.parse_stmt()? {
                    stmts.push(stmt);
                }
                continue;
            }

            // 更新大括号深度
            brace_depth += line.matches('{').count();
            brace_depth = brace_depth.saturating_sub(line.matches('}').count());

            if brace_depth == 0 {
                break;
            }

            // 跳过单独的大括号
            if line == "{" {
                self.advance();
                continue;
            }

            // 解析语句
            if let Some(stmt) = self.parse_stmt()? {
                stmts.push(stmt);
            }

            self.advance();
        }

        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Option<Stmt>> {
        let line = self.current_line().trim().to_string();

        // 变量声明
        if line.starts_with("l ") || line.starts_with("v ") {
            return Ok(Some(self.parse_let()?));
        }

        // Match 表达式
        if line.starts_with("M ") {
            let match_expr = self.parse_match()?;
            return Ok(Some(Stmt::ExprStmt(Box::new(match_expr))));
        }

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

        // for 循环
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
        let is_struct_init = line.contains('{') && 
            line.contains(':') && 
            !line.contains("=>") && 
            !line.starts_with("//") &&
            !line.contains("!(") && 
            !line.contains("! (") &&
            !line.contains("![") &&
            !line.contains("\"{") && // 格式字符串
            !line.contains("{}");    // 格式占位符
        
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

        // 宏调用和一般表达式 - 尝试解析
        if line.contains("!(") || line.contains("! (") || 
           line.contains("![") || line.contains("! [") ||
           line.contains('(') || line.contains('.') ||
           line.contains("::") || line.contains('+') || line.contains('-') {
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
            let t = self.parse_type(&name_part[colon_pos+1..]);
            (n, Some(t))
        } else {
            (name_part.to_string(), None)
        };

        // 解析值（如果有）
        let value = if value_str.is_empty() {
            Expr::Literal(Literal::Null)
        } else {
            let value_trimmed = value_str.trim_end_matches(';').trim();
            
            // 检查是否是闭包: |params| body 或 $|params| body
            if value_trimmed.starts_with('|') || value_trimmed.starts_with("$|") {
                // 闭包可能跨越多行，需要收集完整的闭包体
                let full_closure = self.collect_closure_value(value_trimmed)?;
                self.parse_closure_expr(&full_closure)?
            }
            // 检查是否是 Match 表达式: M expr { ... }
            else if value_trimmed.starts_with("M ") {
                Expr::Raw(format!("(() => {{ {} }})()", value_trimmed))
            } 
            // 检查是否是结构体初始化: Name { field: value }
            else if value_trimmed.contains('{') && value_trimmed.contains(':') && !value_trimmed.contains("=>") && !value_trimmed.contains('|') {
                self.parse_struct_init(value_trimmed)?
            }
            else {
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
            let fields_str = &trimmed[brace_pos+1..].trim_end_matches('}').trim_end_matches(',');
            
            let mut fields = vec![];
            // 简化解析: 按逗号分割
            for field_def in self.split_params(fields_str) {
                let field_def = field_def.trim();
                if field_def.is_empty() { continue; }
                
                if let Some(colon_pos) = field_def.find(':') {
                    let fname = field_def[..colon_pos].trim().to_string();
                    let fvalue_str = field_def[colon_pos+1..].trim();
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
        let rest = &content[param_end+1..].trim();
        
        // 解析参数
        let mut params = vec![];
        for param_str in self.split_params(params_str) {
            let param_str = param_str.trim();
            if param_str.is_empty() { continue; }
            
            if let Some(colon_pos) = param_str.find(':') {
                let name = param_str[..colon_pos].trim().to_string();
                let ty = self.parse_type(&param_str[colon_pos+1..]);
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
        
        // 解析body
        let body = if body_str.starts_with('{') {
            // 块体
            let inner = body_str.trim_start_matches('{').trim_end_matches('}').trim();
            if inner.is_empty() {
                // 多行闭包：body内容在后续行，返回TODO注释
                Expr::Raw("/* TODO: multiline body */".to_string())
            } else {
                self.parse_expr_string(inner)?
            }
        } else if body_str.is_empty() {
            // 空body
            Expr::Raw("/* empty body */".to_string())
        } else {
            // 表达式体
            self.parse_expr_string(body_str)?
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
            if line.contains("=>") || line.contains(":") {
                let arm = self.parse_match_arm(&line)?;
                arms.push(arm);
            }

            self.advance();
        }

        Ok(arms)
    }

    fn parse_match_arm(&mut self, line: &str) -> Result<MatchArm> {
        // 支持 pattern => body 和 pattern: { body } 两种格式
        let (pattern_str, body_str) = if line.contains("=>") {
            let parts: Vec<&str> = line.splitn(2, "=>").collect();
            (parts[0].trim(), parts.get(1).map(|s| *s).unwrap_or(""))
        } else if line.contains(":") {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            (parts[0].trim(), parts.get(1).map(|s| *s).unwrap_or(""))
        } else {
            (line, "")
        };

        let pattern = self.parse_pattern(pattern_str)?;

        // 解析分支体
        let body = self.parse_arm_body(body_str.trim())?;

        Ok(MatchArm {
            pattern,
            guard: None,
            body: Box::new(body),
        })
    }

    fn parse_arm_body(&self, body_str: &str) -> Result<Expr> {
        let trimmed = body_str.trim();

        // 清理括号和逗号
        let cleaned = if trimmed.starts_with('{') {
            let mut s = trimmed[1..].to_string();
            s = s.trim_end().to_string();
            if s.ends_with("},") {
                s = s[..s.len()-2].to_string();
            } else if s.ends_with('}') {
                s = s[..s.len()-1].to_string();
            }
            s
        } else {
            trimmed.trim_end_matches(',').to_string()
        };

        self.parse_expr_string(cleaned.trim())
    }

    fn parse_pattern(&self, pattern_str: &str) -> Result<Pattern> {
        let trimmed = pattern_str.trim();

        // Ok(binding)
        if trimmed.starts_with("Ok(") || trimmed.starts_with("Ok (") {
            let start = trimmed.find('(').unwrap();
            let end = trimmed.rfind(')').unwrap_or(trimmed.len());
            let binding = trimmed[start+1..end].trim().to_string();
            return Ok(Pattern::ResultOk(binding));
        }

        // Err(binding)
        if trimmed.starts_with("Err(") || trimmed.starts_with("Err (") {
            let start = trimmed.find('(').unwrap();
            let end = trimmed.rfind(')').unwrap_or(trimmed.len());
            let binding = trimmed[start+1..end].trim().to_string();
            return Ok(Pattern::ResultErr(binding));
        }

        // Some(binding)
        if trimmed.starts_with("Some(") || trimmed.starts_with("Some (") {
            let start = trimmed.find('(').unwrap();
            let end = trimmed.rfind(')').unwrap_or(trimmed.len());
            let binding = trimmed[start+1..end].trim().to_string();
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
                    path[paren_start+1..paren_end]
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
                trimmed[1..trimmed.len()-1].to_string()
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
        let content = &line[2..]; // 跳过 "? "

        // 解析条件
        let condition_str = if let Some(pos) = content.find('{') {
            content[..pos].trim()
        } else {
            content.trim()
        };

        let condition = self.parse_expr_string(condition_str)?;

        // 解析 then 块
        let then_body = self.parse_if_block()?;

        // TODO: 解析 else 块
        let else_body = None;

        Ok(Expr::If {
            condition: Box::new(condition),
            then_body: Box::new(then_body),
            else_body,
        })
    }

    fn parse_if_block(&mut self) -> Result<Expr> {
        let mut stmts = vec![];
        let mut brace_depth = 0;

        let current = self.current_line().to_string();
        if current.contains('{') {
            brace_depth = 1;
            self.advance();
        }

        while self.current_line < self.lines.len() {
            let line = self.current_line().trim().to_string();

            if line == "}" && brace_depth == 1 {
                self.advance();
                break;
            }

            brace_depth += line.matches('{').count();
            brace_depth = brace_depth.saturating_sub(line.matches('}').count());

            if brace_depth == 0 {
                break;
            }

            if line.is_empty() || line == "{" {
                self.advance();
                continue;
            }

            if let Some(stmt) = self.parse_stmt()? {
                stmts.push(stmt);
            }

            self.advance();
        }

        Ok(Expr::Block {
            stmts,
            trailing_expr: None,
        })
    }

    // ============ 其他表达式 ============

    fn parse_return(&mut self) -> Result<Expr> {
        let line = self.current_line().trim();

        if line == "<" {
            return Ok(Expr::Return(None));
        }

        let content = line.trim_start_matches('<').trim().trim_end_matches(';').trim();
        if content.is_empty() {
            return Ok(Expr::Return(None));
        }

        let value = self.parse_expr_string(content)?;
        Ok(Expr::Return(Some(Box::new(value))))
    }

    fn parse_loop(&mut self) -> Result<Expr> {
        // 跳过循环块
        self.skip_block()?;
        Ok(Expr::Raw("/* loop */".to_string()))
    }

    fn parse_for(&mut self) -> Result<Expr> {
        let line = self.current_line().trim().to_string();
        let content = &line[4..]; // 跳过 "for "

        // 提取 pattern in iterator
        // 简化处理
        self.skip_block()?;

        Ok(Expr::Raw(format!("/* {} */", line)))
    }

    // ============ 表达式字符串解析 ============

    fn parse_expr_string(&self, expr_str: &str) -> Result<Expr> {
        // 规范化：去除 :: 周围空格，去除函数调用前空格
        let normalized = expr_str.trim()
            .replace(" :: ", "::")
            .replace(" . ", ".")
            .replace(" (", "(")
            .replace("( ", "(")
            .replace(" )", ")")
            .replace(") ", ")");
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
            let value = self.parse_expr_string(value_str)?;
            return Ok(Expr::Return(Some(Box::new(value))));
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
                trimmed[1..trimmed.len()-1].to_string()
            )));
        }

        // 宏调用: name!(...) 或 name ! (...) 或 name![...] 或 name ! [...]
        if trimmed.contains("!(") || trimmed.contains("! (") || 
           trimmed.contains("![") || trimmed.contains("! [") || 
           trimmed.ends_with("!()") {
            // 找到 ! 的位置
            let exclaim_pos = trimmed.find('!').unwrap();
            let name = trimmed[..exclaim_pos].trim().to_string();
            // 找到参数开始位置
            let args = if let Some(paren_start) = trimmed[exclaim_pos..].find('(') {
                let start = exclaim_pos + paren_start + 1;
                trimmed[start..].trim_end_matches(')').to_string()
            } else if let Some(bracket_start) = trimmed[exclaim_pos..].find('[') {
                let start = exclaim_pos + bracket_start + 1;
                trimmed[start..].trim_end_matches(']').to_string()
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
                    let variant = parts[0].to_string();
                    let enum_name = parts[1].to_string();
                    let args_str = &trimmed[paren_pos+1..].trim_end_matches(')');

                    let args = if args_str.is_empty() {
                        Some(vec![])
                    } else {
                        let parsed_args: Result<Vec<Expr>> = self.split_args(args_str)
                            .into_iter()
                            .map(|a| self.parse_expr_string(a.trim()))
                            .collect();
                        Some(parsed_args?)
                    };

                    return Ok(Expr::EnumVariant {
                        enum_name: enum_name.trim().to_string(),
                        variant: variant.trim().to_string(),
                        args,
                    });
                }
            }

            // 纯路径 - 清理空格
            let segments: Vec<String> = trimmed.split("::")
                .map(|s| s.trim().to_string())
                .collect();
            return Ok(Expr::Path { segments });
        }

        // 函数调用
        if trimmed.contains('(') && trimmed.ends_with(')') {
            let paren_pos = trimmed.find('(').unwrap();
            let func_name = trimmed[..paren_pos].trim(); // 清理空格
            let args_str = &trimmed[paren_pos+1..trimmed.len()-1];

            let args: Result<Vec<Expr>> = if args_str.is_empty() {
                Ok(vec![])
            } else {
                self.split_args(args_str)
                    .into_iter()
                    .map(|a| self.parse_expr_string(a.trim()))
                    .collect()
            };

            return Ok(Expr::Call {
                func: Box::new(Expr::Ident(func_name.to_string())),
                args: args?,
            });
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
                if !self.is_inside_parens(trimmed, pos) && !self.is_inside_string(trimmed, pos) {
                    let left_str = &trimmed[..pos];
                    let right_str = &trimmed[pos+op_str.len()..];
                    return Ok(Expr::Binary {
                        left: Box::new(self.parse_expr_string(left_str)?),
                        op,
                        right: Box::new(self.parse_expr_string(right_str)?),
                    });
                }
            }
        }

        // 标识符或透传
        Ok(Expr::Ident(trimmed.to_string()))
    }

    // ============ 辅助方法 ============

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

        // 泛型
        if let Some(lt_pos) = trimmed.find('<') {
            let gt_pos = self.find_matching_bracket(trimmed, lt_pos);
            if gt_pos > lt_pos {
                let base = trimmed[..lt_pos].to_string();
                let params_str = &trimmed[lt_pos+1..gt_pos];
                let params: Vec<Type> = self.split_params(params_str)
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
        for (i, c) in s[start..].chars().enumerate() {
            match c {
                '(' | '<' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return start + i;
                    }
                }
                '>' => depth -= 1,
                _ => {}
            }
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
        let mut start = 0;

        for (i, c) in params_str.chars().enumerate() {
            match c {
                '<' | '(' | '[' => depth += 1,
                '>' | ')' | ']' => depth -= 1,
                ',' if depth == 0 => {
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
            if i >= pos { break; }
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
            if i >= pos { break; }
            if c == '"' { in_string = !in_string; }
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

        let pat = parser.parse_pattern("CalcError::InvalidOperator(op)").unwrap();
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
