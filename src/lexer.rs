// Nu Language Lexer
// 使用logos进行词法分析

use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
pub enum Token {
    // 定义关键字
    #[token("S")]
    StructPub,
    #[token("s")]
    StructPriv,
    #[token("E")]
    EnumPub,
    #[token("e")]
    EnumPriv,
    #[token("F")]
    FnPub,
    #[token("f")]
    FnPriv,
    #[token("TR")]
    TraitPub,
    #[token("tr")]
    TraitPriv,
    #[token("I")]
    Impl,
    #[token("D")]
    Mod,

    // 原子关键字
    #[token("l")]
    Let,
    #[token("v")]
    LetMut,
    #[token("u")]
    Use,
    #[token("U")]
    PubUse,

    // 符号
    #[token("<")]
    LessThanOrReturn,
    #[token(">")]
    GreaterThanOrPrint,
    #[token("?")]
    If,
    #[token("M")]
    Match,
    #[token("L")]
    Loop,

    // 标识符
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    // 字面量
    #[regex(r#""([^"\\]|\\.)*""#, |lex| lex.slice().to_string())]
    StringLit(String),

    #[regex(r"\d+", |lex| lex.slice().parse::<i64>().ok())]
    IntLit(Option<i64>),

    // 空白和注释
    #[regex(r"[ \t\r\n]+", logos::skip)]
    #[regex(r"//[^\n]*", logos::skip)]
    Whitespace,
}

pub fn tokenize(input: &str) -> Vec<Token> {
    Token::lexer(input).filter_map(|t| t.ok()).collect()
}
