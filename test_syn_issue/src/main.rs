use syn::{parse_file, Item};
use quote::ToTokens;

fn main() {
    // 读取真实的 ensure.rs 文件
    let code = include_str!("../../examples_project/opensource_libs/anyhow/src/ensure.rs");
    
    println!("=== 源文件分析 ===");
    println!("源文件总字符数: {}", code.len());
    
    match parse_file(code) {
        Ok(file) => {
            println!("syn 解析成功! 找到 {} 个 items", file.items.len());
            println!("\n=== Item 列表 ===");
            
            for (i, item) in file.items.iter().enumerate() {
                match item {
                    Item::Macro(m) => {
                        let name = m.ident.as_ref().map(|id| id.to_string()).unwrap_or_else(|| "无名宏".to_string());
                        let tokens = m.to_token_stream().to_string();
                        println!("[{}] Macro '{}': {} chars", i, name, tokens.len());
                        
                        // 如果是 __parse_ensure，打印前100个字符确认内容
                        if name == "__parse_ensure" {
                            println!("    前100字符: {}...", &tokens[..tokens.len().min(100)]);
                        }
                    },
                    Item::Fn(f) => {
                        println!("[{}] Function '{}'", i, f.sig.ident);
                    },
                    Item::Struct(s) => {
                        println!("[{}] Struct '{}'", i, s.ident);
                    },
                    Item::Impl(imp) => {
                        let ty = imp.self_ty.to_token_stream().to_string();
                        let trait_name = imp.trait_.as_ref()
                            .map(|(_, path, _)| path.to_token_stream().to_string())
                            .unwrap_or_default();
                        if trait_name.is_empty() {
                            println!("[{}] Impl for {}", i, ty);
                        } else {
                            println!("[{}] Impl {} for {}", i, trait_name, ty);
                        }
                    },
                    Item::Trait(t) => {
                        println!("[{}] Trait '{}'", i, t.ident);
                    },
                    Item::Use(u) => {
                        println!("[{}] Use statement", i);
                    },
                    _ => {
                        println!("[{}] Other item", i);
                    }
                }
            }
        },
        Err(e) => {
            println!("syn 解析错误: {}", e);
        }
    }
}
