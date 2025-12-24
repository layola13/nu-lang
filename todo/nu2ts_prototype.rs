// Nu2TS Quick-Start Prototype
// Demonstrates SWC AST-based code generation approach
// 
// This is a minimal working example showing how to:
// 1. Parse Nu/Rust code with syn
// 2. Build TypeScript AST with swc_ecma_ast
// 3. Emit TypeScript code with swc_ecma_codegen

use anyhow::Result;
use swc_common::{sync::Lrc, SourceMap, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_codegen::{text_writer::JsWriter, Emitter, Config};

/// TypeScript code generator
pub struct TsCodegen {
    source_map: Lrc<SourceMap>,
}

impl TsCodegen {
    pub fn new() -> Self {
        Self {
            source_map: Lrc::new(SourceMap::default()),
        }
    }

    /// Example: Convert Rust function to TypeScript function
    /// 
    /// Input (Rust/Nu):
    /// ```rust
    /// pub fn add(a: i32, b: i32) -> i32 {
    ///     a + b
    /// }
    /// ```
    /// 
    /// Output (TypeScript):
    /// ```typescript
    /// export function add(a: number, b: number): number {
    ///     return a + b;
    /// }
    /// ```
    pub fn convert_function(&self, rust_fn: &syn::ItemFn) -> ModuleItem {
        let name = rust_fn.sig.ident.to_string();
        let is_public = matches!(rust_fn.vis, syn::Visibility::Public(_));
        
        // Step 1: Build parameters
        let params = rust_fn.sig.inputs.iter()
            .filter_map(|arg| self.convert_param(arg))
            .collect::<Vec<_>>();
        
        // Step 2: Build return type
        let return_type = self.convert_return_type(&rust_fn.sig.output);
        
        // Step 3: Build function body
        let body = self.convert_block(&rust_fn.block);
        
        // Step 4: Construct SWC AST node
        let fn_decl = FnDecl {
            ident: Ident::new(name.into(), DUMMY_SP),
            declare: false,
            function: Box::new(Function {
                params,
                decorators: vec![],
                span: DUMMY_SP,
                body: Some(body),
                is_generator: false,
                is_async: rust_fn.sig.asyncness.is_some(),
                type_params: None,
                return_type: Some(Box::new(return_type)),
            }),
        };
        
        // Step 5: Wrap in export if public
        if is_public {
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
                span: DUMMY_SP,
                decl: Decl::Fn(fn_decl),
            }))
        } else {
            ModuleItem::Stmt(Stmt::Decl(Decl::Fn(fn_decl)))
        }
    }

    /// Example: Convert Rust struct to TypeScript interface
    /// 
    /// Input (Rust/Nu):
    /// ```rust
    /// pub struct Person {
    ///     name: String,
    ///     age: u32,
    /// }
    /// ```
    /// 
    /// Output (TypeScript):
    /// ```typescript
    /// export interface Person {
    ///     name: string;
    ///     age: number;
    /// }
    /// ```
    pub fn convert_struct(&self, rust_struct: &syn::ItemStruct) -> ModuleItem {
        let name = rust_struct.ident.to_string();
        let is_public = matches!(rust_struct.vis, syn::Visibility::Public(_));
        
        // Build interface members
        let members = match &rust_struct.fields {
            syn::Fields::Named(fields) => {
                fields.named.iter()
                    .filter_map(|field| {
                        let field_name = field.ident.as_ref()?.to_string();
                        let ts_type = self.convert_type(&field.ty);
                        
                        Some(TsTypeElement::TsPropertySignature(TsPropertySignature {
                            span: DUMMY_SP,
                            readonly: false,
                            key: Box::new(Expr::Ident(Ident::new(field_name.into(), DUMMY_SP))),
                            computed: false,
                            optional: false,
                            type_ann: Some(Box::new(TsTypeAnn {
                                span: DUMMY_SP,
                                type_ann: Box::new(ts_type),
                            })),
                            init: None,
                        }))
                    })
                    .collect()
            }
            _ => vec![],
        };
        
        // Create TypeScript interface
        let ts_interface = TsInterfaceDecl {
            span: DUMMY_SP,
            id: Ident::new(name.into(), DUMMY_SP),
            declare: false,
            type_params: None,
            extends: vec![],
            body: TsInterfaceBody {
                span: DUMMY_SP,
                body: members,
            },
        };
        
        // Wrap in export if public
        if is_public {
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
                span: DUMMY_SP,
                decl: Decl::TsInterface(Box::new(ts_interface)),
            }))
        } else {
            ModuleItem::Stmt(Stmt::Decl(Decl::TsInterface(Box::new(ts_interface))))
        }
    }

    // --- Helper methods ---

    fn convert_param(&self, param: &syn::FnArg) -> Option<Param> {
        match param {
            syn::FnArg::Typed(pat_type) => {
                let name = self.extract_param_name(&pat_type.pat)?;
                let ts_type = self.convert_type(&pat_type.ty);
                
                Some(Param {
                    span: DUMMY_SP,
                    decorators: vec![],
                    pat: Pat::Ident(BindingIdent {
                        id: Ident::new(name.into(), DUMMY_SP),
                        type_ann: Some(Box::new(TsTypeAnn {
                            span: DUMMY_SP,
                            type_ann: Box::new(ts_type),
                        })),
                    }),
                })
            }
            syn::FnArg::Receiver(_) => {
                // Handle 'self' - convert to 'this'
                Some(Param {
                    span: DUMMY_SP,
                    decorators: vec![],
                    pat: Pat::Ident(BindingIdent {
                        id: Ident::new("this".into(), DUMMY_SP),
                        type_ann: None,
                    }),
                })
            }
        }
    }

    fn extract_param_name(&self, pat: &syn::Pat) -> Option<String> {
        match pat {
            syn::Pat::Ident(ident) => Some(ident.ident.to_string()),
            _ => None,
        }
    }

    fn convert_return_type(&self, ret: &syn::ReturnType) -> TsTypeAnn {
        let ts_type = match ret {
            syn::ReturnType::Default => TsType::TsKeywordType(TsKeywordType {
                span: DUMMY_SP,
                kind: TsKeywordTypeKind::TsVoidKeyword,
            }),
            syn::ReturnType::Type(_, ty) => self.convert_type(ty),
        };
        
        TsTypeAnn {
            span: DUMMY_SP,
            type_ann: Box::new(ts_type),
        }
    }

    /// Core type mapping: Rust types → TypeScript types
    fn convert_type(&self, ty: &syn::Type) -> TsType {
        use quote::ToTokens;
        
        match ty {
            syn::Type::Path(type_path) => {
                let segment = type_path.path.segments.last().unwrap();
                let type_name = segment.ident.to_string();
                
                match type_name.as_str() {
                    // Primitive numbers
                    "i8" | "i16" | "i32" | "i64" | "i128" |
                    "u8" | "u16" | "u32" | "u64" | "u128" |
                    "f32" | "f64" | "isize" | "usize" => {
                        TsType::TsKeywordType(TsKeywordType {
                            span: DUMMY_SP,
                            kind: TsKeywordTypeKind::TsNumberKeyword,
                        })
                    }
                    
                    // Boolean
                    "bool" => {
                        TsType::TsKeywordType(TsKeywordType {
                            span: DUMMY_SP,
                            kind: TsKeywordTypeKind::TsBooleanKeyword,
                        })
                    }
                    
                    // String types
                    "String" | "Str" => {
                        TsType::TsKeywordType(TsKeywordType {
                            span: DUMMY_SP,
                            kind: TsKeywordTypeKind::TsStringKeyword,
                        })
                    }
                    
                    // Vec<T> → Array<T>
                    "Vec" | "V" => {
                        let elem_type = self.extract_generic_arg(segment);
                        TsType::TsArrayType(TsArrayType {
                            span: DUMMY_SP,
                            elem_type: Box::new(elem_type),
                        })
                    }
                    
                    // Option<T> → T | null
                    "Option" | "O" => {
                        let inner_type = self.extract_generic_arg(segment);
                        TsType::TsUnionOrIntersectionType(
                            TsUnionOrIntersectionType::TsUnionType(TsUnionType {
                                span: DUMMY_SP,
                                types: vec![
                                    Box::new(inner_type),
                                    Box::new(TsType::TsKeywordType(TsKeywordType {
                                        span: DUMMY_SP,
                                        kind: TsKeywordTypeKind::TsNullKeyword,
                                    })),
                                ],
                            })
                        )
                    }
                    
                    // Result<T, E> → T | E (simplified)
                    // TODO: Better strategy with discriminated union
                    "Result" | "R" => {
                        let ok_type = self.extract_generic_arg(segment);
                        // For now, just return the Ok type
                        ok_type
                    }
                    
                    // User-defined types
                    _ => {
                        TsType::TsTypeRef(TsTypeRef {
                            span: DUMMY_SP,
                            type_name: TsEntityName::Ident(Ident::new(
                                type_name.into(),
                                DUMMY_SP,
                            )),
                            type_params: None,
                        })
                    }
                }
            }
            
            // Reference types: &T → T (ignore borrow)
            syn::Type::Reference(type_ref) => {
                self.convert_type(&type_ref.elem)
            }
            
            // Tuple: (A, B) → [A, B]
            syn::Type::Tuple(tuple) => {
                let elem_types = tuple.elems.iter()
                    .map(|t| Box::new(self.convert_type(t)))
                    .collect();
                
                TsType::TsTupleType(TsTupleType {
                    span: DUMMY_SP,
                    elem_types,
                })
            }
            
            // Default: use 'any' type
            _ => {
                TsType::TsKeywordType(TsKeywordType {
                    span: DUMMY_SP,
                    kind: TsKeywordTypeKind::TsAnyKeyword,
                })
            }
        }
    }

    fn extract_generic_arg(&self, segment: &syn::PathSegment) -> TsType {
        use quote::ToTokens;
        
        match &segment.arguments {
            syn::PathArguments::AngleBracketed(args) => {
                if let Some(syn::GenericArgument::Type(ty)) = args.args.first() {
                    return self.convert_type(ty);
                }
            }
            _ => {}
        }
        
        // Fallback to 'any'
        TsType::TsKeywordType(TsKeywordType {
            span: DUMMY_SP,
            kind: TsKeywordTypeKind::TsAnyKeyword,
        })
    }

    fn convert_block(&self, block: &syn::Block) -> BlockStmt {
        let stmts = block.stmts.iter()
            .filter_map(|stmt| self.convert_stmt(stmt))
            .collect();
        
        BlockStmt {
            span: DUMMY_SP,
            stmts,
        }
    }

    fn convert_stmt(&self, stmt: &syn::Stmt) -> Option<Stmt> {
        use quote::ToTokens;
        
        match stmt {
            // let x = expr → const x = expr
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    let name = self.extract_local_name(&local.pat)?;
                    let is_mut = self.is_mutable_local(&local.pat);
                    
                    let var_decl = VarDecl {
                        span: DUMMY_SP,
                        kind: if is_mut { 
                            VarDeclKind::Let 
                        } else { 
                            VarDeclKind::Const 
                        },
                        declare: false,
                        decls: vec![VarDeclarator {
                            span: DUMMY_SP,
                            name: Pat::Ident(BindingIdent {
                                id: Ident::new(name.into(), DUMMY_SP),
                                type_ann: None,
                            }),
                            init: Some(Box::new(self.convert_expr(&init.expr))),
                            definite: false,
                        }],
                    };
                    
                    return Some(Stmt::Decl(Decl::Var(Box::new(var_decl))));
                }
                None
            }
            
            // Expression statements
            syn::Stmt::Expr(expr, _) => {
                Some(Stmt::Expr(ExprStmt {
                    span: DUMMY_SP,
                    expr: Box::new(self.convert_expr(expr)),
                }))
            }
            
            _ => None,
        }
    }

    fn extract_local_name(&self, pat: &syn::Pat) -> Option<String> {
        match pat {
            syn::Pat::Ident(ident) => Some(ident.ident.to_string()),
            syn::Pat::Type(pat_type) => self.extract_local_name(&pat_type.pat),
            _ => None,
        }
    }

    fn is_mutable_local(&self, pat: &syn::Pat) -> bool {
        match pat {
            syn::Pat::Ident(ident) => ident.mutability.is_some(),
            syn::Pat::Type(pat_type) => self.is_mutable_local(&pat_type.pat),
            _ => false,
        }
    }

    fn convert_expr(&self, expr: &syn::Expr) -> Expr {
        use quote::ToTokens;
        
        match expr {
            // Binary operations
            syn::Expr::Binary(bin_expr) => {
                Expr::Bin(BinExpr {
                    span: DUMMY_SP,
                    op: self.convert_bin_op(&bin_expr.op),
                    left: Box::new(self.convert_expr(&bin_expr.left)),
                    right: Box::new(self.convert_expr(&bin_expr.right)),
                })
            }
            
            // Return expression
            syn::Expr::Return(ret_expr) => {
                Expr::Return(ReturnStmt {
                    span: DUMMY_SP,
                    arg: ret_expr.expr.as_ref().map(|e| Box::new(self.convert_expr(e))),
                })
            }
            
            // Identifiers
            syn::Expr::Path(path_expr) => {
                let name = path_expr.path.segments.last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();
                
                Expr::Ident(Ident::new(name.into(), DUMMY_SP))
            }
            
            // Literals
            syn::Expr::Lit(lit_expr) => {
                self.convert_literal(&lit_expr.lit)
            }
            
            // Default: create placeholder
            _ => {
                Expr::Ident(Ident::new("undefined".into(), DUMMY_SP))
            }
        }
    }

    fn convert_bin_op(&self, op: &syn::BinOp) -> BinaryOp {
        match op {
            syn::BinOp::Add(_) => BinaryOp::Add,
            syn::BinOp::Sub(_) => BinaryOp::Sub,
            syn::BinOp::Mul(_) => BinaryOp::Mul,
            syn::BinOp::Div(_) => BinaryOp::Div,
            syn::BinOp::Eq(_) => BinaryOp::EqEq,
            syn::BinOp::Ne(_) => BinaryOp::NotEq,
            syn::BinOp::Lt(_) => BinaryOp::Lt,
            syn::BinOp::Gt(_) => BinaryOp::Gt,
            syn::BinOp::Le(_) => BinaryOp::LtEq,
            syn::BinOp::Ge(_) => BinaryOp::GtEq,
            _ => BinaryOp::Add, // Fallback
        }
    }

    fn convert_literal(&self, lit: &syn::Lit) -> Expr {
        match lit {
            syn::Lit::Int(int_lit) => {
                let value = int_lit.base10_parse::<f64>().unwrap_or(0.0);
                Expr::Lit(Lit::Num(Number {
                    span: DUMMY_SP,
                    value,
                    raw: None,
                }))
            }
            syn::Lit::Str(str_lit) => {
                Expr::Lit(Lit::Str(Str {
                    span: DUMMY_SP,
                    value: str_lit.value().into(),
                    raw: None,
                }))
            }
            syn::Lit::Bool(bool_lit) => {
                Expr::Lit(Lit::Bool(Bool {
                    span: DUMMY_SP,
                    value: bool_lit.value,
                }))
            }
            _ => {
                Expr::Ident(Ident::new("undefined".into(), DUMMY_SP))
            }
        }
    }

    /// Emit TypeScript code from SWC Module
    pub fn emit(&self, module: Module) -> Result<String> {
        let mut buf = vec![];
        
        {
            let writer = JsWriter::new(
                self.source_map.clone(),
                "\n",
                &mut buf,
                None,
            );
            
            let mut emitter = Emitter {
                cfg: Config::default(),
                cm: self.source_map.clone(),
                comments: None,
                wr: writer,
            };
            
            emitter.emit_module(&module)?;
        }
        
        let code = String::from_utf8(buf)?;
        Ok(code)
    }
}

// --- Example Usage ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_function() {
        let rust_code = r#"
            pub fn add(a: i32, b: i32) -> i32 {
                return a + b;
            }
        "#;
        
        let syntax = syn::parse_file(rust_code).unwrap();
        let func = match &syntax.items[0] {
            syn::Item::Fn(f) => f,
            _ => panic!("Expected function"),
        };
        
        let codegen = TsCodegen::new();
        let module_item = codegen.convert_function(func);
        
        let module = Module {
            span: DUMMY_SP,
            body: vec![module_item],
            shebang: None,
        };
        
        let ts_code = codegen.emit(module).unwrap();
        
        println!("Generated TypeScript:\n{}", ts_code);
        assert!(ts_code.contains("export function add"));
        assert!(ts_code.contains("a: number"));
        assert!(ts_code.contains("b: number"));
        assert!(ts_code.contains("number"));
    }

    #[test]
    fn test_simple_struct() {
        let rust_code = r#"
            pub struct Person {
                name: String,
                age: u32,
            }
        "#;
        
        let syntax = syn::parse_file(rust_code).unwrap();
        let struct_item = match &syntax.items[0] {
            syn::Item::Struct(s) => s,
            _ => panic!("Expected struct"),
        };
        
        let codegen = TsCodegen::new();
        let module_item = codegen.convert_struct(struct_item);
        
        let module = Module {
            span: DUMMY_SP,
            body: vec![module_item],
            shebang: None,
        };
        
        let ts_code = codegen.emit(module).unwrap();
        
        println!("Generated TypeScript:\n{}", ts_code);
        assert!(ts_code.contains("export interface Person"));
        assert!(ts_code.contains("name: string"));
        assert!(ts_code.contains("age: number"));
    }
}
