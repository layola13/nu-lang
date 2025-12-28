//! Nu to C++ AST Converter
//!
//! This module provides the conversion logic from Nu source code to C++ AST.
//! It replaces the string-based approach with structured AST generation.
//!
//! ## Architecture
//! 1. Parse Nu source (reuse existing lexer/patterns)
//! 2. Convert to CppAST nodes
//! 3. Use CppCodegen to generate C++ code

use super::cpp_ast::*;
use anyhow::Result;

/// Converter from Nu code to C++ AST
pub struct NuToCppAstConverter {
    /// Collected items for the translation unit
    items: Vec<CppItem>,
    /// Current struct being parsed (for multi-line struct definitions)
    current_struct: Option<CppClass>,
    /// Current function being parsed
    current_function: Option<CppFunction>,
    /// Template parameters for current item
    template_params: Vec<String>,
}

impl NuToCppAstConverter {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            current_struct: None,
            current_function: None,
            template_params: Vec::new(),
        }
    }

    /// Convert Nu code to a C++ translation unit
    pub fn convert(&mut self, nu_code: &str) -> Result<CppTranslationUnit> {
        let mut unit = CppTranslationUnit::new();
        unit.add_standard_includes();

        let lines: Vec<&str> = nu_code.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with("//") {
                i += 1;
                continue;
            }

            // Parse line and generate AST nodes
            if let Some(item) = self.parse_line(trimmed, &lines, &mut i)? {
                unit.add_item(item);
            }

            i += 1;
        }

        Ok(unit)
    }

    /// Parse a single line and return a CppItem if applicable
    fn parse_line(&mut self, line: &str, lines: &[&str], index: &mut usize) -> Result<Option<CppItem>> {
        // Priority 1: Loop (L) - must check before function since L looks like identifier
        if line.starts_with("L ") || line == "L {" || line.starts_with("L(") {
            // Loop/for is a statement, not a top-level item
            return Ok(None);
        }

        // Priority 2: Struct definition: S Name { ... }
        if line.starts_with("S ") || line.starts_with("s ") {
            return self.parse_struct(line, lines, index);
        }

        // Priority 3: Impl block: I Type { ... }
        if line.starts_with("I ") {
            return self.parse_impl(line, lines, index);
        }

        // Priority 4: Function definition: F name(...) or f name(...)
        if line.starts_with("F ") || line.starts_with("f ") {
            let after_marker = &line[2..];
            if after_marker.contains('(') && !after_marker.starts_with('(') {
                return self.parse_function(line, lines, index);
            }
        }

        // Priority 5: Enum definition: E Name { ... }
        if line.starts_with("E ") || line.starts_with("e ") {
            if !line.contains("=>") {
                return self.parse_enum(line, lines, index);
            }
        }

        // Priority 6: Trait definition: TR Name or tr Name
        if line.starts_with("TR ") || line.starts_with("tr ") {
            return self.parse_trait(line, lines, index);
        }

        // Priority 7: Module: D name or DM name
        if line.starts_with("D ") || line.starts_with("DM ") {
            return self.parse_module(line);
        }

        // Type alias: t Name = Type
        if line.starts_with("t ") {
            return self.parse_type_alias(line);
        }

        // Const: C NAME: Type = value
        if line.starts_with("C ") || line.starts_with("CP ") {
            return self.parse_const(line);
        }

        Ok(None)
    }

    /// Parse an impl block (methods for a type)
    fn parse_impl(&mut self, line: &str, lines: &[&str], index: &mut usize) -> Result<Option<CppItem>> {
        let content = &line[2..]; // Skip "I "

        // Check for trait implementation: I Trait for Type { ... }
        let (class_name, trait_name) = if content.contains(" for ") {
            let parts: Vec<&str> = content.split(" for ").collect();
            if parts.len() == 2 {
                let trait_n = parts[0].trim().to_string();
                let type_n = parts[1].trim().trim_end_matches(" {").trim_end_matches('{').trim().to_string();
                (type_n, Some(trait_n))
            } else {
                return Ok(None);
            }
        } else {
            // Regular impl: I Type { ... }
            let name = content.trim().trim_end_matches(" {").trim_end_matches('{').trim();
            let (type_name, _generics) = self.extract_name_and_generics(name)?;
            (type_name, None)
        };

        // Parse methods in impl block
        let mut methods = vec![];

        if content.ends_with('{') {
            let mut depth = 1;
            *index += 1;

            while *index < lines.len() && depth > 0 {
                let method_line = lines[*index].trim();

                if method_line.is_empty() || method_line.starts_with("//") {
                    *index += 1;
                    continue;
                }

                if method_line == "}" {
                    depth -= 1;
                    break;
                }

                // Check for nested braces
                let open = method_line.matches('{').count();
                let close = method_line.matches('}').count();
                depth += open as i32 - close as i32;

                // Parse function definition within impl
                if method_line.starts_with("F ") || method_line.starts_with("f ") {
                    if let Some(CppItem::Function(mut func)) = self.parse_function(method_line, lines, index)? {
                        // Mark methods with &self as non-static
                        if method_line.contains("&self") || method_line.contains("&mut self") {
                            func.is_static = false;
                        }
                        // Methods in impl blocks are associated with the class
                        methods.push(func);
                    }
                }

                *index += 1;
            }
        }

        // Generate a class with methods (or append to existing class)
        let cls = CppClass {
            name: class_name.clone(),
            is_struct: true,
            template_params: vec![],
            base_classes: if let Some(ref t) = trait_name {
                vec![(t.clone(), CppVisibility::Public)]
            } else {
                vec![]
            },
            fields: vec![],
            methods,
            nested_types: vec![],
            derive_traits: vec![],
            cfg_condition: None,
        };

        // Add comment about trait implementation
        if let Some(trait_n) = trait_name {
            Ok(Some(CppItem::Comment(format!("Implementation of {} for {}", trait_n, class_name))))
        } else if !cls.methods.is_empty() {
            Ok(Some(CppItem::Class(cls)))
        } else {
            Ok(Some(CppItem::Comment(format!("Implementation for {}", class_name))))
        }
    }

    /// Parse a trait definition
    fn parse_trait(&mut self, line: &str, lines: &[&str], index: &mut usize) -> Result<Option<CppItem>> {
        let is_pub = line.starts_with("TR ");
        let content = &line[3..]; // Skip "TR " or "tr "
        
        let name = content.trim().trim_end_matches(" {").trim_end_matches('{').trim().to_string();

        // Parse trait methods (as pure virtual)
        let mut methods = vec![];

        if content.ends_with('{') {
            let mut depth = 1;
            *index += 1;

            while *index < lines.len() && depth > 0 {
                let method_line = lines[*index].trim();

                if method_line.is_empty() || method_line.starts_with("//") {
                    *index += 1;
                    continue;
                }

                if method_line == "}" {
                    depth -= 1;
                    break;
                }

                // Parse function signature (no body = pure virtual in C++)
                if method_line.starts_with("F ") || method_line.starts_with("f ") {
                    if let Some(CppItem::Function(mut func)) = self.parse_function(method_line, lines, index)? {
                        func.is_virtual = true;
                        func.body = None; // Pure virtual = 0
                        methods.push(func);
                    }
                }

                *index += 1;
            }
        }

        let cls = CppClass {
            name,
            is_struct: false, // Traits become classes (not structs)
            template_params: vec![],
            base_classes: vec![],
            fields: vec![],
            methods,
            nested_types: vec![],
            derive_traits: vec![],
            cfg_condition: None,
        };

        Ok(Some(CppItem::Class(cls)))
    }

    /// Parse a module declaration
    fn parse_module(&self, line: &str) -> Result<Option<CppItem>> {
        let is_pub = line.starts_with("DM ");
        let content = if is_pub { &line[3..] } else { &line[2..] };
        
        let name = content.trim().trim_end_matches(';').to_string();
        
        // C++ doesn't have direct module equivalent, use namespace
        Ok(Some(CppItem::Namespace(CppNamespace {
            name,
            items: vec![],
        })))
    }

    /// Parse a struct definition
    fn parse_struct(&mut self, line: &str, lines: &[&str], index: &mut usize) -> Result<Option<CppItem>> {
        let is_pub = line.starts_with("S ");
        let content = &line[2..];

        // Extract struct name and template params
        let (name, template_params) = self.extract_name_and_generics(content)?;

        let mut cls = CppClass {
            name,
            is_struct: true,
            template_params,
            base_classes: vec![],
            fields: vec![],
            methods: vec![],
            nested_types: vec![],
            derive_traits: vec![],
            cfg_condition: None,
        };

        // Check if this is a multi-line definition
        if content.ends_with('{') {
            // Parse fields until we hit '}'
            let mut depth = 1;
            *index += 1;

            while *index < lines.len() && depth > 0 {
                let field_line = lines[*index].trim();

                if field_line.is_empty() || field_line.starts_with("//") {
                    *index += 1;
                    continue;
                }

                if field_line == "}" || field_line == "};" {
                    depth -= 1;
                    break;
                }

                if field_line.ends_with('{') {
                    depth += 1;
                }

                // Parse field: name: Type,
                if let Some(field) = self.parse_struct_field(field_line)? {
                    cls.fields.push(field);
                }

                *index += 1;
            }
        }

        Ok(Some(CppItem::Class(cls)))
    }

    /// Parse a single struct field
    fn parse_struct_field(&self, line: &str) -> Result<Option<CppField>> {
        let line = line.trim_end_matches(',').trim();

        if !line.contains(':') || line.contains("::") {
            return Ok(None);
        }

        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Ok(None);
        }

        let name = parts[0].trim().to_string();
        let type_str = parts[1].trim();
        let field_type = self.parse_type(type_str)?;

        Ok(Some(CppField {
            name,
            field_type,
            visibility: CppVisibility::Public,
            default_value: None,
        }))
    }

    /// Parse a function definition
    fn parse_function(&mut self, line: &str, _lines: &[&str], _index: &mut usize) -> Result<Option<CppItem>> {
        let is_pub = line.starts_with("F ");
        let content = &line[2..];

        // Extract function signature
        let paren_pos = content.find('(').unwrap_or(content.len());
        let name_part = &content[..paren_pos];

        // Check for generics in name
        let (name, template_params) = self.extract_name_and_generics(name_part)?;

        // Extract parameters
        let params = self.extract_function_params(content)?;

        // Extract return type
        let return_type = self.extract_return_type(content)?;

        // Check for body
        let has_body = content.contains('{');

        let func = CppFunction {
            name,
            template_params,
            params,
            return_type,
            body: if has_body { Some(vec![CppStmt::Comment("TODO: parse body".to_string())]) } else { None },
            is_const: false,
            is_static: !is_pub && !content.contains("&self"),
            is_virtual: false,
            is_override: false,
            is_noexcept: false,
            visibility: if is_pub { CppVisibility::Public } else { CppVisibility::Private },
        };

        Ok(Some(CppItem::Function(func)))
    }

    /// Parse an enum definition
    fn parse_enum(&mut self, line: &str, lines: &[&str], index: &mut usize) -> Result<Option<CppItem>> {
        let content = &line[2..];

        // Extract enum name
        let name = content.trim().trim_end_matches(" {").trim_end_matches('{').trim().to_string();

        let mut variants = vec![];

        // Check if multi-line
        if content.ends_with('{') {
            *index += 1;
            while *index < lines.len() {
                let variant_line = lines[*index].trim();

                if variant_line.is_empty() || variant_line.starts_with("//") {
                    *index += 1;
                    continue;
                }

                if variant_line == "}" || variant_line == "};" {
                    break;
                }

                // Parse variant
                let variant_name = variant_line.trim_end_matches(',').trim().to_string();
                if !variant_name.is_empty() {
                    variants.push(CppEnumVariant {
                        name: variant_name,
                        value: None,
                        associated_data: None,
                    });
                }

                *index += 1;
            }
        }

        Ok(Some(CppItem::Enum(CppEnum {
            name,
            is_class: true, // Use enum class for type safety
            underlying_type: None,
            variants,
        })))
    }

    /// Parse a type alias
    fn parse_type_alias(&self, line: &str) -> Result<Option<CppItem>> {
        let content = &line[2..];

        if let Some(eq_pos) = content.find('=') {
            let name = content[..eq_pos].trim().to_string();
            let target = content[eq_pos + 1..].trim().trim_end_matches(';');
            let target_type = self.parse_type(target)?;

            return Ok(Some(CppItem::TypeAlias(CppTypeAlias {
                name,
                template_params: vec![],
                target_type,
            })));
        }

        Ok(None)
    }

    /// Parse a const definition
    fn parse_const(&self, line: &str) -> Result<Option<CppItem>> {
        let is_pub = line.starts_with("CP ");
        let content = if is_pub { &line[3..] } else { &line[2..] };

        // Parse: NAME: Type = value
        if let Some(colon_pos) = content.find(':') {
            let name = content[..colon_pos].trim().to_string();
            let rest = &content[colon_pos + 1..];

            if let Some(eq_pos) = rest.find('=') {
                let type_str = rest[..eq_pos].trim();
                let value = rest[eq_pos + 1..].trim().trim_end_matches(';').to_string();

                let var_type = self.parse_type(type_str)?;

                return Ok(Some(CppItem::GlobalVar {
                    name,
                    var_type,
                    init: Some(CppExpr::Raw(value)),
                    is_const: true,
                    is_static: false,
                    is_constexpr: true,
                }));
            }
        }

        Ok(None)
    }

    /// Extract name and generic parameters from a string like "Foo<T, U>"
    fn extract_name_and_generics(&self, s: &str) -> Result<(String, Vec<String>)> {
        let s = s.trim().trim_end_matches(" {").trim_end_matches('{');

        if let Some(lt_pos) = s.find('<') {
            if let Some(gt_pos) = s.rfind('>') {
                let name = s[..lt_pos].trim().to_string();
                let generics_str = &s[lt_pos + 1..gt_pos];
                let generics: Vec<String> = generics_str
                    .split(',')
                    .map(|g| g.trim().to_string())
                    .filter(|g| !g.is_empty())
                    .collect();
                return Ok((name, generics));
            }
        }

        Ok((s.trim().to_string(), vec![]))
    }

    /// Extract function parameters
    fn extract_function_params(&self, content: &str) -> Result<Vec<CppParam>> {
        let mut params = vec![];

        if let Some(start) = content.find('(') {
            if let Some(end) = content.rfind(')') {
                let params_str = &content[start + 1..end];

                for param in params_str.split(',') {
                    let param = param.trim();
                    if param.is_empty() || param == "&self" || param == "&mut self" {
                        continue;
                    }

                    if let Some(colon_pos) = param.find(':') {
                        let name = param[..colon_pos].trim().to_string();
                        let type_str = param[colon_pos + 1..].trim();
                        let param_type = self.parse_type(type_str)?;

                        params.push(CppParam {
                            name,
                            param_type,
                            default_value: None,
                        });
                    }
                }
            }
        }

        Ok(params)
    }

    /// Extract return type from function signature
    fn extract_return_type(&self, content: &str) -> Result<CppType> {
        if let Some(arrow_pos) = content.find("->") {
            let after_arrow = &content[arrow_pos + 2..];
            let type_str = if let Some(brace_pos) = after_arrow.find('{') {
                after_arrow[..brace_pos].trim()
            } else {
                after_arrow.trim()
            };
            return self.parse_type(type_str);
        }

        Ok(CppType::Void)
    }

    /// Parse a Nu type string into CppType
    fn parse_type(&self, s: &str) -> Result<CppType> {
        let s = s.trim();

        // Handle references
        if s.starts_with('&') {
            let inner = if s.starts_with("&mut ") {
                &s[5..]
            } else {
                &s[1..]
            };
            let inner_type = self.parse_type(inner)?;
            return Ok(CppType::Reference {
                inner: Box::new(inner_type),
                is_const: !s.starts_with("&mut "),
            });
        }

        // Handle generics
        if let Some(lt_pos) = s.find('<') {
            if let Some(gt_pos) = s.rfind('>') {
                let base = &s[..lt_pos];
                let args_str = &s[lt_pos + 1..gt_pos];

                let cpp_base = match base {
                    "V" | "Vec" => "std::vector",
                    "Option" | "O" => "std::optional",
                    // Smart pointers (per NU2CPP23.md spec)
                    "Box" | "B" => "std::unique_ptr",
                    "Rc" => "std::shared_ptr",
                    "Arc" | "A" => "std::shared_ptr",
                    "Weak" | "W" => "std::weak_ptr",  // Weak reference
                    // Collections
                    "HashMap" | "HM" => "std::unordered_map",
                    "HashSet" | "HS" => "std::unordered_set",
                    // C++23 types
                    "Result" | "R" => "std::expected", // C++23 std::expected<T, E>
                    "Mutex" | "X" => "nu::Mutex",      // Custom wrapper for lock-guard semantics
                    _ => base,
                };

                // Parse generic arguments
                let args: Result<Vec<CppType>> = args_str
                    .split(',')
                    .map(|a| self.parse_type(a.trim()))
                    .collect();

                return Ok(CppType::Template {
                    base: cpp_base.to_string(),
                    args: args?,
                });
            }
        }

        // Primitive type mapping
        let cpp_type = match s {
            "()" => CppType::Void,
            "i8" => CppType::Primitive("int8_t".to_string()),
            "i16" => CppType::Primitive("int16_t".to_string()),
            "i32" => CppType::Primitive("int32_t".to_string()),
            "i64" => CppType::Primitive("int64_t".to_string()),
            "u8" => CppType::Primitive("uint8_t".to_string()),
            "u16" => CppType::Primitive("uint16_t".to_string()),
            "u32" => CppType::Primitive("uint32_t".to_string()),
            "u64" => CppType::Primitive("uint64_t".to_string()),
            "usize" => CppType::Primitive("size_t".to_string()),
            "isize" => CppType::Primitive("ptrdiff_t".to_string()),
            "f32" => CppType::Primitive("float".to_string()),
            "f64" => CppType::Primitive("double".to_string()),
            "bool" => CppType::Primitive("bool".to_string()),
            "char" => CppType::Primitive("char32_t".to_string()),
            "String" | "Str" => CppType::Named("std::string".to_string()),
            "str" => CppType::Named("std::string_view".to_string()),
            "Self" => CppType::Named("Self".to_string()), // Will be replaced later
            _ => CppType::Named(s.to_string()),
        };

        Ok(cpp_type)
    }
}

impl Default for NuToCppAstConverter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nu2cpp::cpp_codegen::CppCodegen;

    #[test]
    fn test_simple_struct_conversion() {
        let nu_code = r#"
S Point {
    x: i32,
    y: i32,
}
"#;

        let mut converter = NuToCppAstConverter::new();
        let unit = converter.convert(nu_code).unwrap();

        let mut codegen = CppCodegen::new();
        let output = codegen.generate(&unit);

        assert!(output.contains("struct Point {"));
        assert!(output.contains("int32_t x;"));
        assert!(output.contains("int32_t y;"));
    }

    #[test]
    fn test_generic_struct_conversion() {
        let nu_code = r#"
S Container<T> {
    value: T,
    count: i32,
}
"#;

        let mut converter = NuToCppAstConverter::new();
        let unit = converter.convert(nu_code).unwrap();

        let mut codegen = CppCodegen::new();
        let output = codegen.generate(&unit);

        assert!(output.contains("template<typename T>"));
        assert!(output.contains("struct Container {"));
        assert!(output.contains("T value;"));
    }

    #[test]
    fn test_function_conversion() {
        let nu_code = r#"
F add(a: i32, b: i32) -> i32 {
}
"#;

        let mut converter = NuToCppAstConverter::new();
        let unit = converter.convert(nu_code).unwrap();

        let mut codegen = CppCodegen::new();
        let output = codegen.generate(&unit);

        assert!(output.contains("int32_t add(int32_t a, int32_t b)"));
    }

    #[test]
    fn test_enum_conversion() {
        let nu_code = r#"
E Color {
    Red,
    Green,
    Blue,
}
"#;

        let mut converter = NuToCppAstConverter::new();
        let unit = converter.convert(nu_code).unwrap();

        let mut codegen = CppCodegen::new();
        let output = codegen.generate(&unit);

        assert!(output.contains("enum class Color {"));
        assert!(output.contains("Red"));
        assert!(output.contains("Green"));
        assert!(output.contains("Blue"));
    }

    #[test]
    fn test_type_alias() {
        let nu_code = "t MyInt = i32;";

        let mut converter = NuToCppAstConverter::new();
        let unit = converter.convert(nu_code).unwrap();

        let mut codegen = CppCodegen::new();
        let output = codegen.generate(&unit);

        assert!(output.contains("using MyInt = int32_t;"));
    }

    #[test]
    fn test_vector_type() {
        let nu_code = r#"
S Data {
    items: V<i32>,
}
"#;

        let mut converter = NuToCppAstConverter::new();
        let unit = converter.convert(nu_code).unwrap();

        let mut codegen = CppCodegen::new();
        let output = codegen.generate(&unit);

        assert!(output.contains("std::vector<int32_t> items;"));
    }

    #[test]
    fn test_impl_block() {
        let nu_code = r#"
I Point {
    F new(x: i32, y: i32) -> Point {
    }
    F distance(&self) -> f64 {
    }
}
"#;

        let mut converter = NuToCppAstConverter::new();
        let unit = converter.convert(nu_code).unwrap();

        let mut codegen = CppCodegen::new();
        let output = codegen.generate(&unit);

        // Impl block should generate a struct with methods
        assert!(output.contains("struct Point {"));
        assert!(output.contains("Point new(int32_t x, int32_t y)"));
        assert!(output.contains("double distance()"));
    }

    #[test]
    fn test_trait_definition() {
        let nu_code = r#"
TR Drawable {
    F draw(&self);
    F area(&self) -> f64;
}
"#;

        let mut converter = NuToCppAstConverter::new();
        let unit = converter.convert(nu_code).unwrap();

        let mut codegen = CppCodegen::new();
        let output = codegen.generate(&unit);

        // Trait should become a class with virtual methods
        assert!(output.contains("class Drawable {"));
        assert!(output.contains("virtual"));
    }

    #[test]
    fn test_module_parsing() {
        let nu_code = "DM utils;";

        let mut converter = NuToCppAstConverter::new();
        let unit = converter.convert(nu_code).unwrap();

        let mut codegen = CppCodegen::new();
        let output = codegen.generate(&unit);

        assert!(output.contains("namespace utils {"));
    }

    #[test]
    fn test_optional_type() {
        let nu_code = r#"
S Config {
    name: Str,
    timeout: O<i32>,
}
"#;

        let mut converter = NuToCppAstConverter::new();
        let unit = converter.convert(nu_code).unwrap();

        let mut codegen = CppCodegen::new();
        let output = codegen.generate(&unit);

        assert!(output.contains("std::string name;"));
        assert!(output.contains("std::optional<int32_t> timeout;"));
    }
}
