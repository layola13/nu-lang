
//! Test for enum variant conversion

use crate::nu2cpp::ast_converter::NuToCppAstConverter;
use crate::nu2cpp::cpp_codegen::CppCodegen;

#[test]
fn test_enum_with_tuple_variant() {
    let nu_code = r#"
E CalcError {
    InvalidOperator(String),
    DivisionByZero,
    ParseError,
}
"#;

    let mut converter = NuToCppAstConverter::new();
    let unit = converter.convert(nu_code).unwrap();

    let mut codegen = CppCodegen::new();
    let output = codegen.generate(&unit);

    println!("Generated C++ code:\n{}", output);

    // Should generate struct for InvalidOperator with _0 field
    assert!(output.contains("struct InvalidOperator"), "Missing struct InvalidOperator");
    assert!(output.contains("std::string _0"), "Missing _0 field in InvalidOperator");
    
    // Should generate empty structs for unit variants
    assert!(output.contains("struct DivisionByZero"), "Missing struct DivisionByZero");
    assert!(output.contains("struct ParseError"), "Missing struct ParseError");
    
    // Should generate std::variant type alias
    assert!(output.contains("using CalcError = std::variant<"), "Missing std::variant type alias");
    assert!(output.contains("InvalidOperator"), "CalcError variant missing InvalidOperator");
    assert!(output.contains("DivisionByZero"), "CalcError variant missing DivisionByZero");
    assert!(output.contains("ParseError"), "CalcError variant missing ParseError");
}

#[test]
fn test_enum_with_struct_variant() {
    let nu_code = r#"
E Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
}
"#;

    let mut converter = NuToCppAstConverter::new();
    let unit = converter.convert(nu_code).unwrap();

    let mut codegen = CppCodegen::new();
    let output = codegen.generate(&unit);

    println!("Generated C++ code:\n{}", output);

    // Should generate struct for Move with named fields
    assert!(output.contains("struct Move"), "Missing struct Move");
    assert!(output.contains("int32_t x"), "Missing x field in Move");
    assert!(output.contains("int32_t y"), "Missing y field in Move");
    
    // Should generate struct for Write with _0 field
    assert!(output.contains("struct Write"), "Missing struct Write");
    assert!(output.contains("std::string _0"), "Missing _0 field in Write");
    
    // Should generate empty struct for Quit
    assert!(output.contains("struct Quit"), "Missing struct Quit");
    
    // Should generate std::variant type alias
    assert!(output.contains("using Message = std::variant<"), "Missing std::variant type alias");
}
