// Nu2TS AST 定义 (完整版)
// 用于表示 Nu 代码的抽象语法树

use std::fmt;

// ============ 顶级项目 ============

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// use 声明: u std::io::{self, Write}
    Use { path: String, items: Vec<String> },

    /// 函数定义: F/f name(params) -> type { body }
    Function(FunctionDef),

    /// 结构体: s Name { fields }
    Struct(StructDef),

    /// 枚举: E Name { variants }
    Enum(EnumDef),

    /// impl 块: I Type { methods }
    Impl(ImplDef),

    /// mod 块: D tests { ... }
    Mod(ModDef),

    /// 顶层语句（表达式语句）
    Stmt(Stmt),

    /// 原始行（透传）
    Raw(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Box<Expr>,
    pub is_pub: bool,
    pub is_async: bool,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<Field>,
    pub derives: Vec<String>,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub derives: Vec<String>,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Option<Vec<Type>>, // 元组变体的字段类型
    pub struct_fields: Option<Vec<Field>>, // 结构体式变体的字段
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImplDef {
    pub target: String,
    pub trait_name: Option<String>,
    pub methods: Vec<FunctionDef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModDef {
    pub name: String,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub args: Option<String>,
}

// ============ 语句 ============

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// 变量声明: l/v name = expr
    Let {
        name: String,
        ty: Option<Type>,
        value: Box<Expr>,
        is_mut: bool,
    },

    /// 表达式语句
    ExprStmt(Box<Expr>),

    /// 原始语句（透传）
    Raw(String),
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

    /// Loop: L { body }
    Loop { body: Box<Expr> },

    /// For 循环: for (i, item) in iter { body }
    For {
        pattern: String,
        iterator: Box<Expr>,
        body: Box<Expr>,
    },

    /// Return: < expr
    Return(Option<Box<Expr>>),

    /// Break: br
    Break,

    /// Continue: ct
    Continue,

    /// Try 操作符: expr!
    TryOp { expr: Box<Expr> },

    /// 函数调用: func(args)
    Call { func: Box<Expr>, args: Vec<Expr> },

    /// 方法调用: expr.method(args)
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },

    /// 字段访问: expr.field
    Field { object: Box<Expr>, field: String },

    /// 索引访问: expr[index]
    Index { object: Box<Expr>, index: Box<Expr> },

    /// 二元操作: left op right
    Binary {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },

    /// 一元操作: op expr
    Unary { op: UnOp, expr: Box<Expr> },

    /// 块表达式: { stmts }
    Block {
        stmts: Vec<Stmt>,
        trailing_expr: Option<Box<Expr>>,
    },

    /// 闭包: |args| body 或 $|args| body (move)
    Closure {
        params: Vec<Param>,
        return_type: Option<Type>,
        body: Box<Expr>,
        is_move: bool,
    },

    /// 结构体构造: Name { field: value }
    StructInit {
        name: String,
        fields: Vec<(String, Expr)>,
    },

    /// 枚举变体: Enum::Variant 或 Enum::Variant(args)
    EnumVariant {
        enum_name: String,
        variant: String,
        args: Option<Vec<Expr>>,
    },

    /// 宏调用: name!(args)
    Macro { name: String, args: String },

    /// 路径表达式: Type::method
    Path { segments: Vec<String> },

    /// 标识符
    Ident(String),

    /// 字面量
    Literal(Literal),

    /// 元组: (a, b, c)
    Tuple(Vec<Expr>),

    /// 数组: [a, b, c]
    Array(Vec<Expr>),

    /// 数组重复: [value; count] - Rust 语法
    ArrayRepeat { value: Box<Expr>, count: Box<Expr> },

    /// 原始表达式（透传）
    Raw(String),
}

// ============ 辅助类型 ============

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Box<Expr>>,
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
    /// 枚举变体: Enum::Variant(bindings)
    EnumVariant { path: String, bindings: Vec<String> },
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
    pub is_ref: bool,
    pub is_mut: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    /// 基础类型
    Named(String),
    /// 泛型: Vec<T>
    Generic { base: String, params: Vec<Type> },
    /// 元组: (A, B)
    Tuple(Vec<Type>),
    /// 函数类型: fn(A) -> B
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    /// 引用: &T, &!T
    Reference { is_mut: bool, inner: Box<Type> },
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
    Add,       // +
    Sub,       // -
    Mul,       // *
    Div,       // /
    Mod,       // %
    And,       // &&
    Or,        // ||
    Eq,        // ==
    Ne,        // !=
    Lt,        // <
    Le,        // <=
    Gt,        // >
    Ge,        // >=
    Assign,    // =
    AddAssign, // +=
    SubAssign, // -=
    MulAssign, // *=
    DivAssign, // /=
    ModAssign, // %=
    Range,          // ..
    RangeInclusive, // ..=
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Not,    // !
    Neg,    // -
    Deref,  // *
    Ref,    // &
    RefMut, // &!
}

// ============ 文件 ============

#[derive(Debug, Clone, PartialEq)]
pub struct NuFile {
    pub items: Vec<Item>,
}

// ============ Display 实现 ============

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
            Expr::Macro { name, .. } => write!(f, "{}!(...)", name),
            Expr::EnumVariant {
                enum_name, variant, ..
            } => write!(f, "{}::{}", enum_name, variant),
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
            Pattern::EnumVariant { path, bindings } => {
                if bindings.is_empty() {
                    write!(f, "{}", path)
                } else {
                    write!(f, "{}({})", path, bindings.join(", "))
                }
            }
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
            Type::Function {
                params,
                return_type,
            } => {
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

// ============ 便捷构造方法 ============

impl Param {
    pub fn new(name: impl Into<String>, ty: Type) -> Self {
        Self {
            name: name.into(),
            ty,
            is_ref: false,
            is_mut: false,
        }
    }

    pub fn with_ref(mut self) -> Self {
        self.is_ref = true;
        self
    }

    pub fn with_mut(mut self) -> Self {
        self.is_mut = true;
        self
    }
}
