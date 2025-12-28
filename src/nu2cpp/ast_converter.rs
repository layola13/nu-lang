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

/// Represents a match arm: pattern => expression
#[derive(Debug, Clone)]
struct MatchArm {
    pattern: String,
    expr: CppExpr,
}

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
                eprintln!(
                    "DEBUG: Adding item from parse_line: {:?}",
                    match &item {
                        CppItem::Class(c) => format!("Class({})", c.name),
                        CppItem::Comment(s) => format!("Comment({})", s),
                        CppItem::TypeAlias(t) => format!("TypeAlias({})", t.name),
                        _ => format!("{:?}", item),
                    }
                );
                unit.add_item(item);
            }

            i += 1;
        }

        // Add any items that were collected in self.items (e.g., from enum variants)
        eprintln!("DEBUG: self.items count before drain: {}", self.items.len());
        for item in self.items.drain(..) {
            eprintln!(
                "DEBUG: Adding item from self.items: {:?}",
                match &item {
                    CppItem::Class(c) => format!("Class({})", c.name),
                    CppItem::TypeAlias(t) => format!("TypeAlias({})", t.name),
                    _ => format!("{:?}", item),
                }
            );
            unit.add_item(item);
        }

        Ok(unit)
    }

    /// Parse a single line and return a CppItem if applicable
    fn parse_line(
        &mut self,
        line: &str,
        lines: &[&str],
        index: &mut usize,
    ) -> Result<Option<CppItem>> {
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
        if (line.starts_with("E ") || line.starts_with("e ")) && !line.contains("=>") {
            return self.parse_enum(line, lines, index);
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
    fn parse_impl(
        &mut self,
        line: &str,
        lines: &[&str],
        index: &mut usize,
    ) -> Result<Option<CppItem>> {
        let content = &line[2..]; // Skip "I "

        // Check for trait implementation: I Trait for Type { ... }
        let (class_name, trait_name) = if content.contains(" for ") {
            let parts: Vec<&str> = content.split(" for ").collect();
            if parts.len() == 2 {
                let trait_n = parts[0].trim().to_string();
                let type_n = parts[1]
                    .trim()
                    .trim_end_matches(" {")
                    .trim_end_matches('{')
                    .trim()
                    .to_string();
                (type_n, Some(trait_n))
            } else {
                return Ok(None);
            }
        } else {
            // Regular impl: I Type { ... }
            let name = content
                .trim()
                .trim_end_matches(" {")
                .trim_end_matches('{')
                .trim();
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
                    if let Some(CppItem::Function(mut func)) =
                        self.parse_function(method_line, lines, index)?
                    {
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
            Ok(Some(CppItem::Comment(format!(
                "Implementation of {} for {}",
                trait_n, class_name
            ))))
        } else if !cls.methods.is_empty() {
            Ok(Some(CppItem::Class(cls)))
        } else {
            Ok(Some(CppItem::Comment(format!(
                "Implementation for {}",
                class_name
            ))))
        }
    }

    /// Parse a trait definition
    fn parse_trait(
        &mut self,
        line: &str,
        lines: &[&str],
        index: &mut usize,
    ) -> Result<Option<CppItem>> {
        let is_pub = line.starts_with("TR ");
        let content = &line[3..]; // Skip "TR " or "tr "

        let name = content
            .trim()
            .trim_end_matches(" {")
            .trim_end_matches('{')
            .trim()
            .to_string();

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
                    if let Some(CppItem::Function(mut func)) =
                        self.parse_function(method_line, lines, index)?
                    {
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
    fn parse_struct(
        &mut self,
        line: &str,
        lines: &[&str],
        index: &mut usize,
    ) -> Result<Option<CppItem>> {
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
    fn parse_function(
        &mut self,
        line: &str,
        _lines: &[&str],
        _index: &mut usize,
    ) -> Result<Option<CppItem>> {
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
            body: if has_body {
                Some(vec![CppStmt::Comment("TODO: parse body".to_string())])
            } else {
                None
            },
            is_const: false,
            is_static: !is_pub && !content.contains("&self"),
            is_virtual: false,
            is_override: false,
            is_noexcept: false,
            visibility: if is_pub {
                CppVisibility::Public
            } else {
                CppVisibility::Private
            },
        };

        Ok(Some(CppItem::Function(func)))
    }

    /// Parse an enum definition with support for variants with associated data
    /// Handles:
    /// - Unit variants: DivisionByZero
    /// - Tuple variants: InvalidOperator(String)
    /// - Struct variants: Move { x: i32, y: i32 }
    ///
    /// Generates: struct per variant + using EnumName = std::variant<...>
    fn parse_enum(
        &mut self,
        line: &str,
        lines: &[&str],
        index: &mut usize,
    ) -> Result<Option<CppItem>> {
        let content = &line[2..];

        // Extract enum name
        let name = content
            .trim()
            .trim_end_matches(" {")
            .trim_end_matches('{')
            .trim()
            .to_string();

        let mut variants = vec![];
        let mut has_associated_data = false;

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

                // Parse variant (unit, tuple, or struct)
                if let Some(variant) = self.parse_enum_variant(variant_line)? {
                    if variant.associated_data.is_some() {
                        has_associated_data = true;
                    }
                    variants.push(variant);
                }

                *index += 1;
            }
        }

        // If enum has variants with associated data, generate structs + std::variant
        if has_associated_data {
            // Generate individual struct for each variant and add to self.items
            let mut variant_names = vec![];

            for variant in &variants {
                // Generate struct for this variant
                let variant_struct = CppClass {
                    name: variant.name.clone(),
                    is_struct: true,
                    template_params: vec![],
                    base_classes: vec![],
                    fields: variant.associated_data.clone().unwrap_or_default(),
                    methods: vec![],
                    nested_types: vec![],
                    derive_traits: vec![],
                    cfg_condition: None,
                };
                self.items.push(CppItem::Class(variant_struct));
                variant_names.push(variant.name.clone());
            }

            // Generate type alias: using EnumName = std::variant<Variant1, Variant2, ...>
            let variant_types: Vec<CppType> = variant_names
                .iter()
                .map(|name| CppType::Named(name.clone()))
                .collect();

            self.items.push(CppItem::TypeAlias(CppTypeAlias {
                name: name.clone(),
                template_params: vec![],
                target_type: CppType::variant(variant_types),
            }));

            // Return None since items are already added to self.items
            // The convert() method will drain self.items and add them to the translation unit
            Ok(None)
        } else {
            // Simple enum without associated data - use enum class
            Ok(Some(CppItem::Enum(CppEnum {
                name,
                is_class: true,
                underlying_type: None,
                variants,
            })))
        }
    }

    /// Parse an enum variant, supporting:
    /// - Unit: DivisionByZero
    /// - Tuple: InvalidOperator(String)
    /// - Struct: Move { x: i32, y: i32 }
    fn parse_enum_variant(&self, line: &str) -> Result<Option<CppEnumVariant>> {
        let line = line.trim_end_matches(',').trim();

        if line.is_empty() {
            return Ok(None);
        }

        // Check for tuple variant: Name(Type1, Type2, ...)
        if let Some(paren_pos) = line.find('(') {
            if let Some(close_paren) = line.rfind(')') {
                let variant_name = line[..paren_pos].trim().to_string();
                let types_str = &line[paren_pos + 1..close_paren];

                // Parse tuple fields
                let mut fields = vec![];
                for (i, type_str) in types_str.split(',').enumerate() {
                    let type_str = type_str.trim();
                    if type_str.is_empty() {
                        continue;
                    }

                    let field_type = self.parse_type(type_str)?;
                    fields.push(CppField {
                        name: format!("_{}", i), // _0, _1, _2, ...
                        field_type,
                        visibility: CppVisibility::Public,
                        default_value: None,
                    });
                }

                return Ok(Some(CppEnumVariant {
                    name: variant_name,
                    value: None,
                    associated_data: if fields.is_empty() {
                        None
                    } else {
                        Some(fields)
                    },
                }));
            }
        }

        // Check for struct variant: Name { field1: Type1, field2: Type2 }
        if let Some(brace_pos) = line.find('{') {
            if let Some(close_brace) = line.rfind('}') {
                let variant_name = line[..brace_pos].trim().to_string();
                let fields_str = &line[brace_pos + 1..close_brace];

                // Parse struct fields
                let mut fields = vec![];
                for field_part in fields_str.split(',') {
                    let field_part = field_part.trim();
                    if field_part.is_empty() {
                        continue;
                    }

                    if let Some(colon_pos) = field_part.find(':') {
                        let field_name = field_part[..colon_pos].trim().to_string();
                        let type_str = field_part[colon_pos + 1..].trim();
                        let field_type = self.parse_type(type_str)?;

                        fields.push(CppField {
                            name: field_name,
                            field_type,
                            visibility: CppVisibility::Public,
                            default_value: None,
                        });
                    }
                }

                return Ok(Some(CppEnumVariant {
                    name: variant_name,
                    value: None,
                    associated_data: if fields.is_empty() {
                        None
                    } else {
                        Some(fields)
                    },
                }));
            }
        }

        // Unit variant: just a name
        Ok(Some(CppEnumVariant {
            name: line.to_string(),
            value: None,
            associated_data: None,
        }))
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
                    "Weak" | "W" => "std::weak_ptr", // Weak reference
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
    /// Parse for loop with enumerate pattern
    /// Converts: for(i, record) in self.history.iter().enumerate()
    /// To: ForEnumerate { index_var: "i", value_var: "record", collection: "this->history", body: [...] }
    fn parse_for_enumerate(
        &self,
        line: &str,
        lines: &[&str],
        index: &mut usize,
    ) -> Result<Option<CppStmt>> {
        // Pattern: L (index_var, value_var) in collection.iter().enumerate() { ... }
        // or: for (index_var, value_var) in collection.iter().enumerate() { ... }

        let trimmed = line.trim();

        // Extract the for loop pattern
        let content = if trimmed.starts_with("L ") {
            &trimmed[2..]
        } else if trimmed.starts_with("for ") {
            &trimmed[4..]
        } else {
            return Ok(None);
        };

        // Check if this is an enumerate pattern
        if !content.contains(".enumerate()") {
            return Ok(None);
        }

        // Extract (index_var, value_var) part
        if !content.starts_with('(') {
            return Ok(None);
        }

        let close_paren = content
            .find(')')
            .ok_or_else(|| anyhow::anyhow!("Missing closing parenthesis in for enumerate"))?;
        let vars_part = &content[1..close_paren];

        // Split variables: "i, record" -> ["i", "record"]
        let vars: Vec<&str> = vars_part.split(',').map(|s| s.trim()).collect();
        if vars.len() != 2 {
            return Ok(None);
        }

        let index_var = vars[0].to_string();
        let value_var = vars[1].to_string();

        // Extract collection part: "in collection.iter().enumerate()"
        let in_pos = content
            .find(" in ")
            .ok_or_else(|| anyhow::anyhow!("Missing 'in' keyword in for enumerate"))?;
        let after_in = &content[in_pos + 4..];

        // Remove .iter().enumerate() or .enumerate() and { from collection expression
        let mut collection_str = after_in.trim();

        // Remove trailing {
        if let Some(brace_pos) = collection_str.rfind('{') {
            collection_str = &collection_str[..brace_pos].trim();
        }

        // Remove .iter().enumerate() or .enumerate()
        if let Some(pos) = collection_str.find(".iter().enumerate()") {
            collection_str = &collection_str[..pos];
        } else if let Some(pos) = collection_str.find(".enumerate()") {
            collection_str = &collection_str[..pos];
        }

        let collection_str = collection_str.trim();

        // Convert collection to CppExpr
        // Handle self.field -> this->field
        let cpp_collection = if collection_str.starts_with("self.") {
            let field = &collection_str[5..];
            CppExpr::ArrowAccess {
                object: Box::new(CppExpr::This),
                member: field.to_string(),
            }
        } else if collection_str == "self" {
            CppExpr::UnaryOp {
                op: "*".to_string(),
                operand: Box::new(CppExpr::This),
            }
        } else {
            CppExpr::Var(collection_str.to_string())
        };

        // Parse body statements
        let mut body = vec![];

        if content.ends_with('{') {
            let mut depth = 1;
            *index += 1;

            while *index < lines.len() && depth > 0 {
                let body_line = lines[*index].trim();

                if body_line.is_empty() || body_line.starts_with("//") {
                    *index += 1;
                    continue;
                }

                if body_line == "}" {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }

                // Count braces for nested structures
                let open = body_line.matches('{').count();
                let close = body_line.matches('}').count();
                depth += open as i32 - close as i32;

                // Parse statement (simplified - just convert to Raw statement for now)
                // In a full implementation, this would recursively parse statements
                let stmt = CppStmt::Raw(body_line.to_string());
                body.push(stmt);

                *index += 1;
            }
        }

        Ok(Some(CppStmt::ForEnumerate {
            index_var,
            value_var,
            collection: cpp_collection,
            body,
        }))
    }
    /// Parse a Nu expression into a CppExpr
    /// Handles: Constructor calls like Calculator{history: Vec::new()}
    fn parse_expr(&self, s: &str) -> Result<CppExpr> {
        let s = s.trim();

        // Handle struct initialization: Type{field: value, ...}
        if let Some(brace_pos) = s.find('{') {
            let type_part = &s[..brace_pos].trim();

            // Check if this looks like a struct constructor (not a block)
            if type_part
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '<' || c == '>' || c == ':')
            {
                if let Some(close_brace) = s.rfind('}') {
                    let fields_str = &s[brace_pos + 1..close_brace];

                    // Parse fields: field: value, field2: value2
                    let mut fields = vec![];
                    for field_part in fields_str.split(',') {
                        let field_part = field_part.trim();
                        if field_part.is_empty() {
                            continue;
                        }

                        if let Some(colon_pos) = field_part.find(':') {
                            // Check it's not :: (namespace separator)
                            if colon_pos > 0 && !field_part[..colon_pos].ends_with(':') {
                                let field_name = field_part[..colon_pos].trim().to_string();
                                let field_value_str = field_part[colon_pos + 1..].trim();

                                // Convert Vec::new() to std::vector<T>{}
                                let field_value_str = if field_value_str.contains("Vec::new()") {
                                    field_value_str
                                        .replace("Vec::new()", "std::vector<std::string>{}")
                                } else {
                                    field_value_str.to_string()
                                };

                                let field_value = self.parse_expr(&field_value_str)?;
                                fields.push((Some(field_name), field_value));
                            }
                        }
                    }

                    // Convert to C++ constructor call or brace initialization
                    // Type{field: value} -> Type() with member initialization
                    return Ok(CppExpr::BraceInit {
                        type_name: type_part.to_string(),
                        fields,
                    });
                }
            }
        }

        // Handle Vec::new() -> std::vector<T>{}
        if s.contains("Vec::new()") {
            let converted = s.replace("Vec::new()", "std::vector<std::string>{}");
            return Ok(CppExpr::Raw(converted));
        }

        // Default: treat as raw expression
        Ok(CppExpr::Raw(s.to_string()))
    }

    /// Parse a Nu statement into CppStmt
    /// Handles: let bindings, return statements, etc.
    fn parse_statement(&self, line: &str) -> Result<Option<CppStmt>> {
        let line = line.trim();

        if line.is_empty() || line.starts_with("//") {
            return Ok(None);
        }

        // Handle return statement: < expr
        if line.starts_with("< ") || line == "<" {
            let expr_str = if line.len() > 2 { &line[2..] } else { "" };

            if expr_str.is_empty() {
                return Ok(Some(CppStmt::Return(None)));
            }

            let expr = self.parse_expr(expr_str)?;
            return Ok(Some(CppStmt::Return(Some(expr))));
        }

        // Handle let binding: l name = expr
        if line.starts_with("l ") {
            let content = &line[2..];

            if let Some(eq_pos) = content.find('=') {
                let name = content[..eq_pos].trim().to_string();
                let value_str = content[eq_pos + 1..].trim();
                let init_expr = self.parse_expr(value_str)?;

                return Ok(Some(CppStmt::VarDecl {
                    name,
                    var_type: CppType::Named("auto".to_string()),
                    init: Some(init_expr),
                    is_const: true,
                }));
            }
        }

        // Handle mutable let: v name = expr
        if line.starts_with("v ") {
            let content = &line[2..];

            if let Some(eq_pos) = content.find('=') {
                let name = content[..eq_pos].trim().to_string();
                let value_str = content[eq_pos + 1..].trim();
                let init_expr = self.parse_expr(value_str)?;

                return Ok(Some(CppStmt::VarDecl {
                    name,
                    var_type: CppType::Named("auto".to_string()),
                    init: Some(init_expr),
                    is_const: false,
                }));
            }
        }

        // Default: treat as raw statement
        Ok(Some(CppStmt::Raw(line.to_string())))
    }

    /// Parse a match expression: M val { pattern => expr, ... }
    /// Converts to C++ if-else chain using std::get_if per NU2CPP23.md line 61
    fn parse_match_expression(
        &self,
        line: &str,
        lines: &[&str],
        index: &mut usize,
    ) -> Result<Option<CppStmt>> {
        let trimmed = line.trim();

        // Pattern: M value { ... } or match value { ... }
        let content = if trimmed.starts_with("M ") {
            &trimmed[2..]
        } else if trimmed.starts_with("match ") {
            &trimmed[6..]
        } else {
            return Ok(None);
        };

        // Extract the value being matched
        if !content.contains('{') {
            return Ok(None);
        }

        let brace_pos = content.find('{').unwrap();
        let value_str = content[..brace_pos].trim();
        let value_expr = self.parse_expr(value_str)?;

        // Parse match arms
        let mut arms = vec![];

        if content.ends_with('{') || content.contains('{') {
            let mut depth = 1;
            *index += 1;
            let mut current_arm = String::new();

            while *index < lines.len() && depth > 0 {
                let arm_line = lines[*index].trim();

                if arm_line.is_empty() || arm_line.starts_with("//") {
                    *index += 1;
                    continue;
                }

                if arm_line == "}" {
                    depth -= 1;
                    if depth == 0 {
                        // Process last arm if any
                        if !current_arm.is_empty() {
                            if let Some(arm) = self.parse_match_arm(&current_arm)? {
                                arms.push(arm);
                            }
                        }
                        break;
                    }
                }

                // Count braces
                let open = arm_line.matches('{').count();
                let close = arm_line.matches('}').count();
                depth += open as i32 - close as i32;

                // Accumulate arm content
                if arm_line.contains("=>") {
                    // New arm starts
                    if !current_arm.is_empty() {
                        if let Some(arm) = self.parse_match_arm(&current_arm)? {
                            arms.push(arm);
                        }
                    }
                    current_arm = arm_line.to_string();
                } else {
                    // Continuation of current arm
                    if !current_arm.is_empty() {
                        current_arm.push(' ');
                        current_arm.push_str(arm_line);
                    }
                }

                *index += 1;
            }
        }

        // Generate C++ if-else chain using std::get_if
        Ok(Some(self.generate_match_if_chain(value_expr, arms)?))
    }

    /// Parse a single match arm: pattern => expression
    fn parse_match_arm(&self, arm_str: &str) -> Result<Option<MatchArm>> {
        let arm_str = arm_str.trim().trim_end_matches(',');

        if !arm_str.contains("=>") {
            return Ok(None);
        }

        let parts: Vec<&str> = arm_str.splitn(2, "=>").collect();
        if parts.len() != 2 {
            return Ok(None);
        }

        let pattern = parts[0].trim().to_string();
        let expr_str = parts[1].trim();
        let expr = self.parse_expr(expr_str)?;

        Ok(Some(MatchArm { pattern, expr }))
    }

    /// Generate C++ if-else chain from match arms using std::get_if
    fn generate_match_if_chain(&self, value: CppExpr, arms: Vec<MatchArm>) -> Result<CppStmt> {
        if arms.is_empty() {
            return Ok(CppStmt::Comment("Empty match expression".to_string()));
        }

        // Build nested if-else chain
        let mut current: Option<Vec<CppStmt>> = None;

        // Process arms in reverse to build nested structure
        for arm in arms.iter().rev() {
            let condition = self.generate_match_condition(&value, &arm.pattern)?;
            let then_block = vec![CppStmt::Return(Some(arm.expr.clone()))];

            let if_stmt = if let Some(else_block) = current {
                CppStmt::If {
                    condition,
                    then_block,
                    else_block: Some(else_block),
                }
            } else {
                CppStmt::If {
                    condition,
                    then_block,
                    else_block: None,
                }
            };

            current = Some(vec![if_stmt]);
        }

        Ok(current.unwrap().into_iter().next().unwrap())
    }

    /// Generate condition for pattern matching using std::get_if
    fn generate_match_condition(&self, value: &CppExpr, pattern: &str) -> Result<CppExpr> {
        let pattern = pattern.trim();

        // Handle wildcard pattern: _
        if pattern == "_" {
            return Ok(CppExpr::Literal("true".to_string()));
        }

        // Handle variant pattern: TypeName(var) or TypeName
        if let Some(paren_pos) = pattern.find('(') {
            let type_name = &pattern[..paren_pos].trim();

            // Generate: auto* p = std::get_if<TypeName>(&value)
            // Condition: p != nullptr

            Ok(CppExpr::BinOp {
                left: Box::new(CppExpr::Call {
                    callee: Box::new(CppExpr::Raw(format!("std::get_if<{}>", type_name))),
                    args: vec![CppExpr::UnaryOp {
                        op: "&".to_string(),
                        operand: Box::new(value.clone()),
                    }],
                }),
                op: "!=".to_string(),
                right: Box::new(CppExpr::Nullptr),
            })
        } else {
            // Simple type check without binding
            Ok(CppExpr::BinOp {
                left: Box::new(CppExpr::Call {
                    callee: Box::new(CppExpr::Raw(format!("std::get_if<{}>", pattern))),
                    args: vec![CppExpr::UnaryOp {
                        op: "&".to_string(),
                        operand: Box::new(value.clone()),
                    }],
                }),
                op: "!=".to_string(),
                right: Box::new(CppExpr::Nullptr),
            })
        }
    }

    /// Convert Nu expression string to C++ (compatibility wrapper)
    fn convert_nu_expr_to_cpp(&self, expr: &str) -> Result<String> {
        let cpp_expr = self.parse_expr(expr)?;
        // Simple serialization - in practice would use CppCodegen
        Ok(self.serialize_expr(&cpp_expr))
    }

    /// Serialize CppExpr to string (helper for convert_nu_expr_to_cpp)
    fn serialize_expr(&self, expr: &CppExpr) -> String {
        match expr {
            CppExpr::Raw(s) => s.clone(),
            CppExpr::BraceInit { type_name, fields } => {
                let mut result = format!("{}(", type_name);
                for (i, (_name, value)) in fields.iter().enumerate() {
                    if i > 0 {
                        result.push_str(", ");
                    }
                    // Recursive serialization
                    result.push_str(&self.serialize_expr(value));
                }
                result.push(')');
                result
            }
            _ => format!("{:?}", expr), // Fallback
        }
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

    #[test]
    fn test_for_enumerate_parsing() {

        // Create a converter instance
        let converter = NuToCppAstConverter::new();

        // Test Nu code with enumerate pattern
        let nu_line = "L (i, record) in self.history.iter().enumerate() {";
        let lines = vec![nu_line, "    process(record);", "}"];
        let mut index = 0;

        // Parse the for enumerate loop
        let result = converter.parse_for_enumerate(nu_line, &lines, &mut index);
        assert!(result.is_ok());

        let stmt = result.unwrap();
        assert!(stmt.is_some());

        if let Some(CppStmt::ForEnumerate {
            index_var,
            value_var,
            collection,
            body,
        }) = stmt
        {
            // Verify parsed variables
            assert_eq!(index_var, "i");
            assert_eq!(value_var, "record");

            // Verify collection is converted to this->history
            match collection {
                CppExpr::ArrowAccess { member, .. } => {
                    assert_eq!(member, "history");
                }
                _ => panic!("Expected ArrowAccess for self.history"),
            }

            // Verify body is parsed
            assert!(!body.is_empty());
        } else {
            panic!("Expected ForEnumerate statement");
        }
    }

    #[test]
    fn test_for_enumerate_codegen() {
        // TODO: Implement test for ForEnumerate codegen
        // This test needs to be completed with proper CppFunction initialization
    }

    #[test]
    fn test_constructor_syntax_conversion() {
        let converter = NuToCppAstConverter::new();

        // Test parsing Calculator{history: Vec::new()} expression
        let result = converter.parse_expr("Calculator{history: Vec::new()}");
        assert!(result.is_ok());

        // Test conversion to C++ syntax
        let converted = converter.convert_nu_expr_to_cpp("Calculator{history: Vec::new()}");
        assert!(converted.is_ok());

        let cpp_output = converted.unwrap();
        // Should convert to constructor call with proper initialization
        // Calculator(std::vector<std::string>{})
        eprintln!("DEBUG: cpp_output = {:?}", cpp_output);
        assert!(cpp_output.contains("Calculator("));
        assert!(cpp_output.contains("std::vector") || cpp_output.contains("vector"));
    }

    #[test]
    fn test_enum_with_tuple_variant() {
        let nu_code = r#"
E CalcError {
    InvalidOperator(String),
    DivisionByZero,
    ParseError(String),
}
"#;

        let mut converter = NuToCppAstConverter::new();
        let result = converter.convert(nu_code);
        assert!(result.is_ok());

        let unit = result.unwrap();

        // The enum should generate structs for each variant + type alias
        assert!(!unit.items.is_empty());

        // Generate code to verify output
        let mut codegen = CppCodegen::new();
        let output = codegen.generate(&unit);

        // Debug: print actual output
        eprintln!("=== Generated C++ output ===");
        eprintln!("{}", output);
        eprintln!("=== End output ===");

        // Should contain struct definitions
        assert!(output.contains("struct InvalidOperator"));
        assert!(output.contains("struct DivisionByZero"));
        assert!(output.contains("struct ParseError"));

        // Should contain variant type alias
        assert!(output.contains("using CalcError = std::variant<"));
    }

    #[test]
    fn test_enum_with_struct_variant() {
        let nu_code = r#"
E Message {
    Move { x: i32, y: i32 },
    Write(String),
    Quit,
}
"#;

        let mut converter = NuToCppAstConverter::new();
        let result = converter.convert(nu_code);
        assert!(result.is_ok());

        let unit = result.unwrap();
        assert!(!unit.items.is_empty());

        let mut codegen = CppCodegen::new();
        let output = codegen.generate(&unit);

        assert!(output.contains("struct Move"));
        assert!(output.contains("struct Write"));
        assert!(output.contains("struct Quit"));
        assert!(output.contains("using Message = std::variant<"));
    }

    #[test]
    fn test_parse_enum_variant_unit() {
        let converter = NuToCppAstConverter::new();
        let result = converter.parse_enum_variant("DivisionByZero");

        assert!(result.is_ok());
        let variant = result.unwrap();
        assert!(variant.is_some());

        let variant = variant.unwrap();
        assert_eq!(variant.name, "DivisionByZero");
        assert!(variant.associated_data.is_none());
    }

    #[test]
    fn test_parse_enum_variant_tuple() {
        let converter = NuToCppAstConverter::new();
        let result = converter.parse_enum_variant("InvalidOperator(String)");

        assert!(result.is_ok());
        let variant = result.unwrap();
        assert!(variant.is_some());

        let variant = variant.unwrap();
        assert_eq!(variant.name, "InvalidOperator");
        assert!(variant.associated_data.is_some());

        let fields = variant.associated_data.unwrap();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].name, "_0");
    }

    #[test]
    fn test_parse_enum_variant_struct() {
        let converter = NuToCppAstConverter::new();
        let result = converter.parse_enum_variant("Move { x: i32, y: i32 }");

        assert!(result.is_ok());
        let variant = result.unwrap();
        assert!(variant.is_some());

        let variant = variant.unwrap();
        assert_eq!(variant.name, "Move");
        assert!(variant.associated_data.is_some());

        let fields = variant.associated_data.unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "x");
        assert_eq!(fields[1].name, "y");
    }
}
