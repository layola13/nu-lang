// Nu2TS Parser
// 递归下降解析器，将 Nu 代码解析为 AST

use super::ast::*;
use anyhow::{Result, bail};

// ============ Token 定义 ============

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // 关键字
    Function(bool),   // F (true=pub), f (false)
    Let(bool),        // l (false), v (true=mut)
    Match,            // M
    If,               // ?
    Return,           // <
    Break,            // br
    Continue,         // ct
    Loop,             // L

    // 标识符和字面量
    Ident(String),
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),

    // 符号
    LParen,           // (
    RParen,           // )
    LBrace,           // {
    RBrace,           // }
    LBracket,         // [
    RBracket,         // ]
    Colon,            // :
    Comma,            // ,
    Arrow,            // ->
    QuestionMark,     // ?
    Semicolon,        // ;

    // 操作符
    Eq,               // =
    EqEq,             // ==
    Ne,               // !=
    Lt,               // <
    Le,               // <=
    Gt,               // >
    Ge,               // >=
    Plus,             // +
    Minus,            // -
    Star,             // *
    Slash,            // /
    Percent,          // %
    And,              // &&
    Or,               // ||
    Not,              // !
    Ampersand,        // &

    // 特殊
    Newline,
    Eof,
}

// ============ Tokenizer ============

pub struct Tokenizer {
    input: String,
    lines: Vec<String>,
    line_idx: usize,
    pos: usize,
}

impl Tokenizer {
    pub fn new(input: &str) -> Self {
        let lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
        Self {
            input: input.to_string(),
            lines,
            line_idx: 0,
            pos: 0,
        }
    }

    pub fn tokenize_line(&mut self, line: &str) -> Result<Vec<Token>> {
        let mut tokens = vec![];
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with("//") {
            return Ok(vec![Token::Newline]);
        }

        // 检测行首关键字
        if let Some(first_token) = self.detect_line_keyword(trimmed)? {
            tokens.push(first_token);
        }

        // TODO: 完整的词法分析（当前简化版本）
        tokens.push(Token::Newline);
        Ok(tokens)
    }

    fn detect_line_keyword(&self, line: &str) -> Result<Option<Token>> {
        if line.starts_with("F ") {
            return Ok(Some(Token::Function(true)));
        }
        if line.starts_with("f ") {
            return Ok(Some(Token::Function(false)));
        }
        if line.starts_with("l ") {
            return Ok(Some(Token::Let(false)));
        }
        if line.starts_with("v ") {
            return Ok(Some(Token::Let(true)));
        }
        if line.starts_with("M ") {
            return Ok(Some(Token::Match));
        }
        if line.starts_with("? ") {
            return Ok(Some(Token::If));
        }
        if line.starts_with("< ") || line == "<" {
            return Ok(Some(Token::Return));
        }
        if line.starts_with("br") {
            return Ok(Some(Token::Break));
        }
        if line.starts_with("ct") {
            return Ok(Some(Token::Continue));
        }
        if line.starts_with("L ") || line == "L {" {
            return Ok(Some(Token::Loop));
        }

        Ok(None)
    }
}

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

    pub fn parse(&mut self) -> Result<Vec<Stmt>> {
        let mut stmts = vec![];

        while self.current_line < self.lines.len() {
            let line = self.current_line().trim().to_string();

            // 跳过空行和注释
            if line.is_empty() || line.starts_with("//") {
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
        let line = self.current_line().trim();

        // 函数定义
        if line.starts_with("F ") || line.starts_with("f ") {
            return Ok(Some(self.parse_function()?));
        }

        // 变量声明
        if line.starts_with("l ") || line.starts_with("v ") {
            return Ok(Some(self.parse_let()?));
        }

        // 表达式语句（包括 Match, If, Return 等）
        if let Some(expr) = self.parse_expr()? {
            return Ok(Some(Stmt::ExprStmt(Box::new(expr))));
        }

        Ok(None)
    }

    fn parse_function(&mut self) -> Result<Stmt> {
        let line = self.current_line().trim();
        let is_pub = line.starts_with("F ");
        let content = &line[2..];  // 跳过 "F " 或 "f "

        // 简化解析：name(params) -> type {
        // TODO: 实现完整的函数签名解析
        let name = content.split('(').next().unwrap_or("").trim().to_string();

        // 解析函数体（多行）
        let body = self.parse_block_expr()?;

        Ok(Stmt::Function {
            name,
            params: vec![],  // TODO
            return_type: None,  // TODO
            body: Box::new(body),
            is_pub,
            is_async: false,
        })
    }

    fn parse_let(&mut self) -> Result<Stmt> {
        let line = self.current_line().trim();
        let is_mut = line.starts_with("v ");
        let content = &line[2..];  // 跳过 "l " 或 "v "

        // 简化解析：name = value
        let parts: Vec<&str> = content.splitn(2, '=').collect();
        let name = parts.get(0).unwrap_or(&"").trim().to_string();
        let value_str = parts.get(1).unwrap_or(&"").trim();

        // 解析值表达式
        let value = self.parse_simple_expr(value_str)?;

        Ok(Stmt::Let {
            name,
            ty: None,  // TODO: 解析类型注解
            value: Box::new(value),
            is_mut,
        })
    }

    fn parse_expr(&mut self) -> Result<Option<Expr>> {
        let line = self.current_line().trim();

        // Match 表达式
        if line.starts_with("M ") {
            return Ok(Some(self.parse_match()?));
        }

        // If 表达式
        if line.starts_with("? ") {
            return Ok(Some(self.parse_if()?));
        }

        // Return
        if line.starts_with("< ") || line == "<" {
            return Ok(Some(self.parse_return()?));
        }

        // Break
        if line.starts_with("br") {
            return Ok(Some(Expr::Break));
        }

        // Continue
        if line.starts_with("ct") {
            return Ok(Some(Expr::Continue));
        }

        // 简单表达式
        Ok(Some(self.parse_simple_expr(line)?))
    }

    fn parse_match(&mut self) -> Result<Expr> {
        let line = self.current_line().trim();
        let content = &line[2..];  // 跳过 "M "

        // 提取目标表达式
        let target_str = if let Some(pos) = content.find('{') {
            &content[..pos].trim()
        } else {
            content.trim()
        };

        let target = self.parse_simple_expr(target_str)?;

        // 解析分支
        let arms = self.parse_match_arms()?;

        Ok(Expr::Match {
            target: Box::new(target),
            arms,
        })
    }

    fn parse_match_arms(&mut self) -> Result<Vec<MatchArm>> {
        let mut arms = vec![];

        // 跳过当前行（含 M ...）
        self.advance();

        while self.current_line < self.lines.len() {
            let line = self.current_line().trim();

            // 遇到 } 结束
            if line == "}" {
                break;
            }

            // 解析分支：pattern: { body }
            if line.contains(":") {
                let arm = self.parse_match_arm()?;
                arms.push(arm);
            }

            self.advance();
        }

        Ok(arms)
    }

    fn parse_match_arm(&mut self) -> Result<MatchArm> {
        let line = self.current_line().trim();

        // 分离模式和分支体
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        let pattern_str = parts.get(0).unwrap_or(&"").trim();

        // 解析模式
        let pattern = self.parse_pattern(pattern_str)?;

        // 解析分支体
        let body = if parts.len() > 1 && parts[1].trim().starts_with('{') {
            // 单行形式: Ok(v): { expr }
            let body_str = parts[1].trim().trim_start_matches('{').trim_end_matches('}').trim();
            self.parse_simple_expr(body_str)?
        } else {
            // 多行形式（TODO）
            Expr::Block {
                stmts: vec![],
                trailing_expr: None,
            }
        };

        Ok(MatchArm {
            pattern,
            guard: None,
            body: Box::new(body),
        })
    }

    fn parse_pattern(&self, pattern_str: &str) -> Result<Pattern> {
        let trimmed = pattern_str.trim();

        if trimmed.starts_with("Ok(") && trimmed.ends_with(')') {
            let binding = trimmed[3..trimmed.len()-1].trim().to_string();
            return Ok(Pattern::ResultOk(binding));
        }

        if trimmed.starts_with("Err(") && trimmed.ends_with(')') {
            let binding = trimmed[4..trimmed.len()-1].trim().to_string();
            return Ok(Pattern::ResultErr(binding));
        }

        if trimmed.starts_with("Some(") && trimmed.ends_with(')') {
            let binding = trimmed[5..trimmed.len()-1].trim().to_string();
            return Ok(Pattern::OptionSome(binding));
        }

        if trimmed == "None" {
            return Ok(Pattern::OptionNone);
        }

        if trimmed == "_" {
            return Ok(Pattern::Wildcard);
        }

        // 字面量或标识符
        if let Ok(num) = trimmed.parse::<i64>() {
            return Ok(Pattern::Literal(Literal::Integer(num)));
        }

        Ok(Pattern::Ident(trimmed.to_string()))
    }

    fn parse_if(&mut self) -> Result<Expr> {
        let line = self.current_line().trim();
        let content = &line[2..];  // 跳过 "? "

        // 简化：condition { then } else { else }
        let condition = self.parse_simple_expr(content)?;

        // TODO: 解析 then/else 块
        Ok(Expr::If {
            condition: Box::new(condition),
            then_body: Box::new(Expr::Block { stmts: vec![], trailing_expr: None }),
            else_body: None,
        })
    }

    fn parse_return(&mut self) -> Result<Expr> {
        let line = self.current_line().trim();

        if line == "<" {
            return Ok(Expr::Return(None));
        }

        let content = &line[2..];  // 跳过 "< "
        let value = self.parse_simple_expr(content)?;

        Ok(Expr::Return(Some(Box::new(value))))
    }

    fn parse_block_expr(&mut self) -> Result<Expr> {
        // TODO: 解析块表达式
        Ok(Expr::Block {
            stmts: vec![],
            trailing_expr: None,
        })
    }

    fn parse_simple_expr(&self, expr_str: &str) -> Result<Expr> {
        let trimmed = expr_str.trim();

        // 字面量
        if let Ok(num) = trimmed.parse::<i64>() {
            return Ok(Expr::Literal(Literal::Integer(num)));
        }

        if let Ok(num) = trimmed.parse::<f64>() {
            return Ok(Expr::Literal(Literal::Float(num)));
        }

        if trimmed == "true" {
            return Ok(Expr::Literal(Literal::Bool(true)));
        }

        if trimmed == "false" {
            return Ok(Expr::Literal(Literal::Bool(false)));
        }

        // 函数调用
        if trimmed.contains('(') && trimmed.ends_with(')') {
            let func_name = trimmed.split('(').next().unwrap_or("").trim();
            return Ok(Expr::Call {
                func: Box::new(Expr::Ident(func_name.to_string())),
                args: vec![],  // TODO: 解析参数
            });
        }

        // 标识符
        Ok(Expr::Ident(trimmed.to_string()))
    }

    // 辅助方法
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
        let stmts = parser.parse().unwrap();

        assert_eq!(stmts.len(), 1);
        if let Stmt::ExprStmt(expr) = &stmts[0] {
            if let Expr::Match { arms, .. } = expr.as_ref() {
                assert_eq!(arms.len(), 2);
                assert!(matches!(arms[0].pattern, Pattern::ResultOk(_)));
                assert!(matches!(arms[1].pattern, Pattern::ResultErr(_)));
            } else {
                panic!("Expected Match expression");
            }
        } else {
            panic!("Expected ExprStmt");
        }
    }

    #[test]
    fn test_parse_pattern() {
        let parser = Parser::new("");

        assert!(matches!(
            parser.parse_pattern("Ok(val)").unwrap(),
            Pattern::ResultOk(_)
        ));

        assert!(matches!(
            parser.parse_pattern("Err(e)").unwrap(),
            Pattern::ResultErr(_)
        ));

        assert!(matches!(
            parser.parse_pattern("_").unwrap(),
            Pattern::Wildcard
        ));
    }
}
