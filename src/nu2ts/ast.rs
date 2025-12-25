// Nu2TS AST 定义
// 用于表示 Nu 代码的抽象语法树

use std::fmt;

// ============ 语句 ============

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// 函数定义: F/f name(params) -> type { body }
    Function {
        name: String,
        params: Vec<Param>,
        return_type: Option<Type>,
        body: Box<Expr>,
        is_pub: bool,
        is_async: bool,
    },

    /// 变量声明: l/v name = expr
    Let {
        name: String,
        ty: Option<Type>,
        value: Box<Expr>,
        is_mut: bool,
    },

    /// 表达式语句
    ExprStmt(Box<Expr>),
}

// ============ 表达式 ============

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Match 表达式
    Match {
        target: Box<Expr>,
        arms: Vec<MatchArm>,
    },

    /// If 表达式: ? condition { then } else { else }
    If {
        condition: Box<Expr>,
        then_body: Box<Expr>,
        else_body: Option<Box<Expr>>,
    },

    /// Loop: L { body } 或 L var in iter { body }
    Loop {
        pattern: Option<String>,  // for 循环的变量
        iterator: Option<Box<Expr>>, // for 循环的迭代器
        body: Box<Expr>,
    },

    /// Return: < expr
    Return(Option<Box<Expr>>),

    /// Break: br
    Break,

    /// Continue: ct
    Continue,

    /// Try 操作符: expr?
    TryOp {
        expr: Box<Expr>,
    },

    /// 函数调用: func(args)
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },

    /// 字段访问: expr.field
    Field {
        object: Box<Expr>,
        field: String,
    },

    /// 方法调用: expr.method(args)
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },

    /// 二元操作: left op right
    Binary {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },

    /// 一元操作: op expr
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },

    /// 块表达式: { stmts }
    Block {
        stmts: Vec<Stmt>,
        trailing_expr: Option<Box<Expr>>,
    },

    /// 标识符
    Ident(String),

    /// 字面量
    Literal(Literal),

    /// 元组: (a, b, c)
    Tuple(Vec<Expr>),

    /// 数组: [a, b, c]
    Array(Vec<Expr>),
}

// ============ 辅助类型 ============

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Box<Expr>>,  // 守卫条件（暂不支持）
    pub body: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Ok(binding)
    ResultOk(String),
    /// Err(binding)
    ResultErr(String),
    /// Some(binding)
    OptionSome(String),
    /// None
    OptionNone,
    /// 字面量
    Literal(Literal),
    /// 通配符 _
    Wildcard,
    /// 标识符（变量绑定）
    Ident(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    /// 基础类型
    Named(String),
    /// 泛型: Vec<T>
    Generic {
        base: String,
        params: Vec<Type>,
    },
    /// 元组: (A, B)
    Tuple(Vec<Type>),
    /// 函数类型: fn(A) -> B
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    /// 引用: &T, &!T
    Reference {
        is_mut: bool,
        inner: Box<Type>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,      // +
    Sub,      // -
    Mul,      // *
    Div,      // /
    Mod,      // %
    And,      // &&
    Or,       // ||
    Eq,       // ==
    Ne,       // !=
    Lt,       // <
    Le,       // <=
    Gt,       // >
    Ge,       // >=
    Assign,   // =
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Not,      // !
    Neg,      // -
    Deref,    // *
    Ref,      // &
    RefMut,   // &!
}

// ============ Display 实现（用于调试）============

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expr::Ident(name) => write!(f, "{}", name),
            Expr::Literal(lit) => write!(f, "{:?}", lit),
            Expr::Match { target, arms } => {
                write!(f, "match {} {{ {} arms }}", target, arms.len())
            }
            Expr::If { .. } => write!(f, "if {{ ... }}"),
            Expr::Block { stmts, .. } => write!(f, "{{ {} stmts }}", stmts.len()),
            _ => write!(f, "<expr>"),
        }
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Pattern::ResultOk(v) => write!(f, "Ok({})", v),
            Pattern::ResultErr(v) => write!(f, "Err({})", v),
            Pattern::OptionSome(v) => write!(f, "Some({})", v),
            Pattern::OptionNone => write!(f, "None"),
            Pattern::Wildcard => write!(f, "_"),
            Pattern::Literal(lit) => write!(f, "{:?}", lit),
            Pattern::Ident(name) => write!(f, "{}", name),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Named(name) => write!(f, "{}", name),
            Type::Generic { base, params } => {
                write!(f, "{}<", base)?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ">")
            }
            Type::Tuple(types) => {
                write!(f, "(")?;
                for (i, t) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", t)?;
                }
                write!(f, ")")
            }
            Type::Reference { is_mut, inner } => {
                if *is_mut {
                    write!(f, "&mut {}", inner)
                } else {
                    write!(f, "&{}", inner)
                }
            }
            Type::Function { params, return_type } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", return_type)
            }
        }
    }
}
