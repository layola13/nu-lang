//! C++ AST Definitions for Nu2CPP Transpiler
//!
//! This module defines a lightweight C++ Abstract Syntax Tree (AST) to serve as an
//! intermediate representation between the Nu AST and the final C++ code output.
//!
//! ## Design Goals
//! - **Structured output**: Avoid string concatenation errors (e.g., `>>` vs `> >`)
//! - **Template support**: `template<typename T>` declarations are first-class nodes
//! - **Closure capture analysis**: Lambda captures are explicit in the AST
//! - **Statement vs Expression**: C++ is statement-based, so we model this explicitly

use std::fmt;

/// Represents a C++ type (used in declarations, return types, parameters)
#[derive(Debug, Clone, PartialEq)]
pub enum CppType {
    /// Void type
    Void,
    /// Primitive types: int32_t, uint64_t, float, double, bool, char32_t
    Primitive(String),
    /// Named type (e.g., std::string, MyClass)
    Named(String),
    /// Pointer type: T*
    Pointer(Box<CppType>),
    /// Reference type: T& or const T&
    Reference { inner: Box<CppType>, is_const: bool },
    /// Template instantiation: std::vector<T>, std::optional<T>
    Template {
        base: String,
        args: Vec<CppType>,
    },
    /// Auto type (for type inference)
    Auto,
    /// Decltype
    Decltype(String),
}

impl CppType {
    pub fn int32() -> Self { CppType::Primitive("int32_t".to_string()) }
    pub fn int64() -> Self { CppType::Primitive("int64_t".to_string()) }
    pub fn uint32() -> Self { CppType::Primitive("uint32_t".to_string()) }
    pub fn uint64() -> Self { CppType::Primitive("uint64_t".to_string()) }
    pub fn float32() -> Self { CppType::Primitive("float".to_string()) }
    pub fn float64() -> Self { CppType::Primitive("double".to_string()) }
    pub fn bool_type() -> Self { CppType::Primitive("bool".to_string()) }
    pub fn string() -> Self { CppType::Named("std::string".to_string()) }
    pub fn string_view() -> Self { CppType::Named("std::string_view".to_string()) }

    /// Create a std::vector<T>
    pub fn vector(inner: CppType) -> Self {
        CppType::Template {
            base: "std::vector".to_string(),
            args: vec![inner],
        }
    }

    /// Create a std::optional<T>
    pub fn optional(inner: CppType) -> Self {
        CppType::Template {
            base: "std::optional".to_string(),
            args: vec![inner],
        }
    }

    /// Create a std::unique_ptr<T>
    pub fn unique_ptr(inner: CppType) -> Self {
        CppType::Template {
            base: "std::unique_ptr".to_string(),
            args: vec![inner],
        }
    }

    /// Create a std::shared_ptr<T>
    pub fn shared_ptr(inner: CppType) -> Self {
        CppType::Template {
            base: "std::shared_ptr".to_string(),
            args: vec![inner],
        }
    }

    /// Create a const reference: const T&
    pub fn const_ref(inner: CppType) -> Self {
        CppType::Reference {
            inner: Box::new(inner),
            is_const: true,
        }
    }

    /// Create std::expected<T, E> (C++23) for Result type
    pub fn expected(ok_type: CppType, err_type: CppType) -> Self {
        CppType::Template {
            base: "std::expected".to_string(),
            args: vec![ok_type, err_type],
        }
    }

    /// Create std::monostate (for unit type in variant context)
    pub fn monostate() -> Self {
        CppType::Named("std::monostate".to_string())
    }

    /// Create std::jthread (C++20/23 auto-joining thread)
    pub fn jthread() -> Self {
        CppType::Named("std::jthread".to_string())
    }

    /// Create std::variant<Ts...>
    pub fn variant(types: Vec<CppType>) -> Self {
        CppType::Template {
            base: "std::variant".to_string(),
            args: types,
        }
    }
}

/// A function/method parameter
#[derive(Debug, Clone)]
pub struct CppParam {
    pub name: String,
    pub param_type: CppType,
    pub default_value: Option<String>,
}

/// Lambda capture mode
#[derive(Debug, Clone, PartialEq)]
pub enum CppCapture {
    /// No capture: []
    None,
    /// Copy all: [=]
    CopyAll,
    /// Reference all: [&]
    RefAll,
    /// Explicit captures: [x, &y, z = std::move(z)]
    Explicit(Vec<CppCaptureItem>),
}

/// A single capture item in a lambda
#[derive(Debug, Clone, PartialEq)]
pub struct CppCaptureItem {
    pub name: String,
    pub by_ref: bool,
    pub moved: bool,
}

/// A C++ expression
#[derive(Debug, Clone)]
pub enum CppExpr {
    /// Literal value: 42, "hello", true
    Literal(String),
    /// Variable reference
    Var(String),
    /// Binary operation: a + b, a && b
    BinOp {
        left: Box<CppExpr>,
        op: String,
        right: Box<CppExpr>,
    },
    /// Unary operation: !x, -x, *ptr
    UnaryOp {
        op: String,
        operand: Box<CppExpr>,
    },
    /// Function call: func(args...)
    Call {
        callee: Box<CppExpr>,
        args: Vec<CppExpr>,
    },
    /// Method call: obj.method(args...)
    MethodCall {
        object: Box<CppExpr>,
        method: String,
        args: Vec<CppExpr>,
    },
    /// Member access: obj.field
    MemberAccess {
        object: Box<CppExpr>,
        member: String,
    },
    /// Arrow access: ptr->field
    ArrowAccess {
        object: Box<CppExpr>,
        member: String,
    },
    /// Array/Vector indexing: arr[i]
    Index {
        object: Box<CppExpr>,
        index: Box<CppExpr>,
    },
    /// Ternary operator: cond ? a : b
    Ternary {
        condition: Box<CppExpr>,
        then_expr: Box<CppExpr>,
        else_expr: Box<CppExpr>,
    },
    /// Lambda expression: [capture](params) { body }
    Lambda {
        capture: CppCapture,
        params: Vec<CppParam>,
        return_type: Option<CppType>,
        body: Vec<CppStmt>,
    },
    /// Struct/Class initialization: MyStruct{a, b, c}
    BraceInit {
        type_name: String,
        fields: Vec<(Option<String>, CppExpr)>, // (name, value) - name is optional for positional
    },
    /// Cast expression: static_cast<T>(expr)
    Cast {
        cast_type: String, // "static_cast", "dynamic_cast", "reinterpret_cast", "const_cast"
        target_type: CppType,
        expr: Box<CppExpr>,
    },
    /// std::move(expr)
    Move(Box<CppExpr>),
    /// this pointer
    This,
    /// nullptr
    Nullptr,
    /// Raw C++ expression (escape hatch for complex cases)
    Raw(String),
}

/// A C++ statement
#[derive(Debug, Clone)]
pub enum CppStmt {
    /// Variable declaration: Type name = expr; or auto name = expr;
    VarDecl {
        name: String,
        var_type: CppType,
        init: Option<CppExpr>,
        is_const: bool,
    },
    /// Expression statement: expr;
    Expr(CppExpr),
    /// Return statement: return expr;
    Return(Option<CppExpr>),
    /// If statement: if (cond) { ... } else { ... }
    If {
        condition: CppExpr,
        then_block: Vec<CppStmt>,
        else_block: Option<Vec<CppStmt>>,
    },
    /// While loop: while (cond) { ... }
    While {
        condition: CppExpr,
        body: Vec<CppStmt>,
    },
    /// For loop: for (init; cond; update) { ... }
    For {
        init: Option<Box<CppStmt>>,
        condition: Option<CppExpr>,
        update: Option<CppExpr>,
        body: Vec<CppStmt>,
    },
    /// Range-based for: for (Type var : range) { ... }
    ForRange {
        var_name: String,
        var_type: CppType,
        range: CppExpr,
        body: Vec<CppStmt>,
    },
    /// For loop with enumerate: size_t index = 0; for (const auto& var : range) { body; index++; }
    ForEnumerate {
        index_var: String,
        value_var: String,
        collection: CppExpr,
        body: Vec<CppStmt>,
    },
    /// Switch statement
    Switch {
        expr: CppExpr,
        cases: Vec<(CppExpr, Vec<CppStmt>)>,
        default: Option<Vec<CppStmt>>,
    },
    /// Break statement
    Break,
    /// Continue statement
    Continue,
    /// Block: { ... }
    Block(Vec<CppStmt>),
    /// Comment
    Comment(String),
    /// Raw C++ code (escape hatch)
    Raw(String),
}

/// Visibility modifier
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CppVisibility {
    Public,
    Private,
    Protected,
}

/// A struct/class member (field)
#[derive(Debug, Clone)]
pub struct CppField {
    pub name: String,
    pub field_type: CppType,
    pub visibility: CppVisibility,
    pub default_value: Option<String>,
}

/// A C++ function or method
#[derive(Debug, Clone)]
pub struct CppFunction {
    pub name: String,
    pub template_params: Vec<String>, // <typename T, typename U>
    pub params: Vec<CppParam>,
    pub return_type: CppType,
    pub body: Option<Vec<CppStmt>>, // None for declarations only
    pub is_const: bool,    // for methods: void foo() const
    pub is_static: bool,
    pub is_virtual: bool,
    pub is_override: bool,
    pub is_noexcept: bool,
    pub visibility: CppVisibility,
}

/// A C++ struct or class
#[derive(Debug, Clone)]
pub struct CppClass {
    pub name: String,
    pub is_struct: bool, // struct vs class (affects default visibility)
    pub template_params: Vec<String>,
    pub base_classes: Vec<(String, CppVisibility)>, // (name, visibility)
    pub fields: Vec<CppField>,
    pub methods: Vec<CppFunction>,
    pub nested_types: Vec<CppItem>, // Nested structs, enums, etc.
    /// Derive traits: Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash
    pub derive_traits: Vec<String>,
    /// Conditional compilation: #if condition
    pub cfg_condition: Option<String>,
}

/// A C++ enum
#[derive(Debug, Clone)]
pub struct CppEnum {
    pub name: String,
    pub is_class: bool, // enum vs enum class
    pub underlying_type: Option<CppType>,
    pub variants: Vec<CppEnumVariant>,
}

/// A variant in a C++ enum
#[derive(Debug, Clone)]
pub struct CppEnumVariant {
    pub name: String,
    pub value: Option<String>,
    /// For Rust-like enums with associated data, we generate a struct
    pub associated_data: Option<Vec<CppField>>,
}

/// A type alias: using Name = Type;
#[derive(Debug, Clone)]
pub struct CppTypeAlias {
    pub name: String,
    pub template_params: Vec<String>,
    pub target_type: CppType,
}

/// A namespace
#[derive(Debug, Clone)]
pub struct CppNamespace {
    pub name: String,
    pub items: Vec<CppItem>,
}

/// A #include directive
#[derive(Debug, Clone)]
pub struct CppInclude {
    pub path: String,
    pub is_system: bool, // <header> vs "header"
}

/// Top-level C++ item
#[derive(Debug, Clone)]
pub enum CppItem {
    Include(CppInclude),
    Namespace(CppNamespace),
    Class(CppClass),
    Enum(CppEnum),
    Function(CppFunction),
    TypeAlias(CppTypeAlias),
    GlobalVar {
        name: String,
        var_type: CppType,
        init: Option<CppExpr>,
        is_const: bool,
        is_static: bool,
        is_constexpr: bool,
    },
    Comment(String),
    Raw(String),
}

/// A complete C++ translation unit (a single .cpp or .hpp file)
#[derive(Debug, Clone, Default)]
pub struct CppTranslationUnit {
    pub includes: Vec<CppInclude>,
    pub items: Vec<CppItem>,
}

impl CppTranslationUnit {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add standard includes needed for Nu runtime (C++23)
    pub fn add_standard_includes(&mut self) {
        // Core C++23 headers
        self.includes.push(CppInclude { path: "cstdint".to_string(), is_system: true });
        self.includes.push(CppInclude { path: "string".to_string(), is_system: true });
        self.includes.push(CppInclude { path: "string_view".to_string(), is_system: true });
        self.includes.push(CppInclude { path: "vector".to_string(), is_system: true });
        self.includes.push(CppInclude { path: "memory".to_string(), is_system: true });
        self.includes.push(CppInclude { path: "optional".to_string(), is_system: true });
        // C++23 additions
        self.includes.push(CppInclude { path: "expected".to_string(), is_system: true }); // std::expected<T,E>
        self.includes.push(CppInclude { path: "print".to_string(), is_system: true });    // std::println
        self.includes.push(CppInclude { path: "format".to_string(), is_system: true });   // std::format
        self.includes.push(CppInclude { path: "variant".to_string(), is_system: true });  // std::variant
        self.includes.push(CppInclude { path: "thread".to_string(), is_system: true });   // std::jthread
    }

    pub fn add_item(&mut self, item: CppItem) {
        self.items.push(item);
    }
}

// Display implementations for debugging
impl fmt::Display for CppType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CppType::Void => write!(f, "void"),
            CppType::Primitive(s) => write!(f, "{}", s),
            CppType::Named(s) => write!(f, "{}", s),
            CppType::Pointer(inner) => write!(f, "{}*", inner),
            CppType::Reference { inner, is_const } => {
                if *is_const {
                    write!(f, "const {}&", inner)
                } else {
                    write!(f, "{}&", inner)
                }
            }
            CppType::Template { base, args } => {
                write!(f, "{}<", base)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", arg)?;
                }
                write!(f, ">")
            }
            CppType::Auto => write!(f, "auto"),
            CppType::Decltype(expr) => write!(f, "decltype({})", expr),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpp_type_display() {
        assert_eq!(CppType::int32().to_string(), "int32_t");
        assert_eq!(CppType::vector(CppType::int32()).to_string(), "std::vector<int32_t>");
        assert_eq!(
            CppType::Template {
                base: "std::map".to_string(),
                args: vec![CppType::string(), CppType::int32()],
            }.to_string(),
            "std::map<std::string, int32_t>"
        );
    }

    #[test]
    fn test_nested_templates() {
        // Vec<Vec<i32>> -> std::vector<std::vector<int32_t>>
        let nested = CppType::vector(CppType::vector(CppType::int32()));
        assert_eq!(nested.to_string(), "std::vector<std::vector<int32_t>>");
    }
}
