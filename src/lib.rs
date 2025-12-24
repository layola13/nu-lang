// Nu Language Compiler Library
// Version: 1.6.2
// High-Density Rust Dialect Transpiler

pub mod ast;
pub mod codegen;
pub mod lexer;
pub mod module;
pub mod nu2rust;
pub mod nu2ts;
pub mod parser;
pub mod project;
pub mod rust2nu;
pub mod utils;

pub use ast::*;
pub use nu2rust::Nu2RustConverter;
pub use nu2ts::Nu2TsConverter;
pub use rust2nu::Rust2NuConverter;

use anyhow::Result;

/// 将Rust代码转换为Nu代码
pub fn rust_to_nu(rust_code: &str) -> Result<String> {
    let converter = Rust2NuConverter::new();
    converter.convert(rust_code)
}

/// 将Nu代码转换为Rust代码
pub fn nu_to_rust(nu_code: &str) -> Result<String> {
    let converter = Nu2RustConverter::new();
    converter.convert(nu_code)
}

/// 将Nu代码转换为TypeScript代码
pub fn nu_to_ts(nu_code: &str) -> Result<String> {
    let converter = Nu2TsConverter::with_default_config();
    converter.convert(nu_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_to_nu_conversion() {
        let rust_code = r#"
            pub fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        "#;

        let nu_code = rust_to_nu(rust_code).unwrap();
        assert!(nu_code.contains("F add"));
    }

    #[test]
    fn test_nu_to_rust_conversion() {
        let nu_code = r#"
F add(a: i32, b: i32) -> i32 {
    < a + b
}
        "#;

        let rust_code = nu_to_rust(nu_code).unwrap();
        assert!(rust_code.contains("pub fn add"));
        assert!(rust_code.contains("return a + b"));
    }

    #[test]
    fn test_round_trip_simple() {
        let original_rust = "pub fn test() -> i32 { return 42; }";

        // Rust -> Nu
        let nu_code = rust_to_nu(original_rust).unwrap();

        // Nu -> Rust
        let back_to_rust = nu_to_rust(&nu_code).unwrap();

        // 验证关键元素
        assert!(back_to_rust.contains("pub fn test"));
        assert!(back_to_rust.contains("return 42"));
    }
}
