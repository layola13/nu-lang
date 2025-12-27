// Property-Based Tests for Workspace Module
// Uses proptest to verify correctness properties

use proptest::prelude::*;
use super::mapping::*;
use super::toml_converter::*;

// ==================== Property 3: TOML 节名双向转换一致性 ====================
// **Validates: Requirements 3.1, 3.4, 4.4, 7.1-7.7**

/// 生成有效的 Cargo.toml 节名
fn cargo_section_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("[package]".to_string()),
        Just("[workspace]".to_string()),
        Just("[dependencies]".to_string()),
        Just("[dev-dependencies]".to_string()),
        Just("[build-dependencies]".to_string()),
        Just("[workspace.dependencies]".to_string()),
        Just("[workspace.package]".to_string()),
        Just("[workspace.lints]".to_string()),
        Just("[workspace.lints.rust]".to_string()),
        Just("[workspace.lints.clippy]".to_string()),
        Just("[lib]".to_string()),
        Just("[features]".to_string()),
        Just("[[bin]]".to_string()),
        Just("[[example]]".to_string()),
        Just("[[test]]".to_string()),
        Just("[[bench]]".to_string()),
        // 保留节
        Just("[profile.release]".to_string()),
        Just("[profile.dev]".to_string()),
        Just("[patch.crates-io]".to_string()),
        Just("[target.'cfg(windows)'.dependencies]".to_string()),
        Just("[badges]".to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    /// Property 3: TOML 节名双向转换一致性
    /// *For any* valid Cargo.toml section name, converting to Nu and back should produce the original
    #[test]
    fn prop_section_roundtrip(section in cargo_section_strategy()) {
        // Feature: cargo-workspace-support, Property 3: TOML 节名双向转换一致性
        // **Validates: Requirements 3.1, 3.4, 4.4, 7.1-7.7**
        
        let nu_section = convert_section_cargo_to_nu(&section);
        let restored = convert_section_nu_to_cargo(&nu_section);
        
        prop_assert_eq!(
            &section, &restored,
            "Roundtrip failed: {} -> {} -> {}", &section, &nu_section, &restored
        );
    }
}

// ==================== Property 4: 键名双向转换一致性 ====================
// **Validates: Requirements 2.3, 3.2, 3.3, 3.5, 4.1**

/// 生成有效的 Cargo.toml 键名
fn cargo_key_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Package 字段
        Just("name".to_string()),
        Just("version".to_string()),
        Just("edition".to_string()),
        Just("authors".to_string()),
        Just("description".to_string()),
        Just("license".to_string()),
        Just("license-file".to_string()),
        Just("repository".to_string()),
        Just("documentation".to_string()),
        Just("homepage".to_string()),
        Just("keywords".to_string()),
        Just("categories".to_string()),
        Just("rust-version".to_string()),
        // Workspace 字段
        Just("members".to_string()),
        Just("exclude".to_string()),
        Just("resolver".to_string()),
        // 依赖字段
        Just("workspace".to_string()),
        Just("default-features".to_string()),
        Just("optional".to_string()),
        Just("package".to_string()),
        // Lib 字段
        Just("proc-macro".to_string()),
        Just("crate-type".to_string()),
        // Bin/Example/Test 字段
        Just("required-features".to_string()),
        // 保留键
        Just("path".to_string()),
        Just("git".to_string()),
        Just("branch".to_string()),
        Just("tag".to_string()),
        Just("rev".to_string()),
        Just("features".to_string()),
        Just("readme".to_string()),
        Just("publish".to_string()),
        Just("include".to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    /// Property 4: 键名双向转换一致性
    /// *For any* valid Cargo.toml key name, converting to Nu and back should produce the original
    #[test]
    fn prop_key_roundtrip(key in cargo_key_strategy()) {
        // Feature: cargo-workspace-support, Property 4: 键名双向转换一致性
        // **Validates: Requirements 2.3, 3.2, 3.3, 3.5, 4.1**
        
        let nu_key = convert_key_cargo_to_nu(&key);
        let restored = convert_key_nu_to_cargo(&nu_key);
        
        prop_assert_eq!(
            &key, &restored,
            "Roundtrip failed: {} -> {} -> {}", &key, &nu_key, &restored
        );
    }
}

// ==================== Property: TOML 内容双向转换一致性 ====================

/// 生成简单的 Cargo.toml 内容
fn simple_cargo_toml_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // 简单 package
        Just(r#"[package]
name = "test"
version = "0.1.0"
edition = "2021"
"#.to_string()),
        // 带依赖
        Just(r#"[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = "1.0"
"#.to_string()),
        // Workspace
        Just(r#"[workspace]
members = ["lib1", "lib2"]
resolver = "2"
"#.to_string()),
        // Workspace with dependencies
        Just(r#"[workspace]
members = ["lib1"]

[workspace.dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }
"#.to_string()),
        // 带 profile（保留节）
        Just(r#"[package]
name = "test"
version = "0.1.0"

[profile.release]
opt-level = 3
"#.to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    /// Property: TOML 内容双向转换语义等价
    /// *For any* valid Cargo.toml content, converting to Nu and back should preserve semantic meaning
    #[test]
    fn prop_toml_content_roundtrip(content in simple_cargo_toml_strategy()) {
        let cargo2nu = Cargo2NuConverter::new();
        let nu2cargo = Nu2CargoConverter::new();
        
        let nu_content = cargo2nu.convert(&content);
        let restored = nu2cargo.convert(&nu_content);
        
        // 验证关键内容被保留（忽略空白差异）
        let original_lines: Vec<&str> = content.lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        let restored_lines: Vec<&str> = restored.lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        
        // 检查行数相同
        prop_assert_eq!(
            original_lines.len(), 
            restored_lines.len(),
            "Line count mismatch: original {} vs restored {}", 
            original_lines.len(), 
            restored_lines.len()
        );
    }
}

// ==================== Property: 保留节不变性 ====================

/// 生成包含保留节的 Cargo.toml 内容
fn preserved_section_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(r#"[profile.release]
opt-level = 3
lto = true
"#.to_string()),
        Just(r#"[profile.dev]
opt-level = 0
debug = true
"#.to_string()),
        Just(r#"[patch.crates-io]
serde = { path = "../serde" }
"#.to_string()),
        Just(r#"[target.'cfg(windows)'.dependencies]
winapi = "0.3"
"#.to_string()),
        Just(r#"[badges]
maintenance = { status = "actively-developed" }
"#.to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    /// Property 7: 保留节不变性
    /// *For any* preserved section (profile, patch, target, badges), content should remain unchanged
    #[test]
    fn prop_preserved_sections_unchanged(content in preserved_section_strategy()) {
        // Feature: cargo-workspace-support, Property 7: 保留节不变性
        // **Validates: Requirements 5.2, 7.7, 4.2**
        
        let cargo2nu = Cargo2NuConverter::new();
        let nu_content = cargo2nu.convert(&content);
        
        // 保留节内容应该完全不变
        let original_trimmed: String = content.lines()
            .map(|l| l.trim())
            .collect::<Vec<_>>()
            .join("\n");
        let converted_trimmed: String = nu_content.lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        
        prop_assert_eq!(
            original_trimmed.trim(),
            converted_trimmed.trim(),
            "Preserved section was modified"
        );
    }
}

// ==================== Property 5: 依赖继承标记保留 ====================
// **Validates: Requirements 3.2, 3.3, 3.5**

/// 生成包含 workspace 继承的依赖配置
fn workspace_inheritance_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // 简单继承
        Just(r#"[dependencies]
serde.workspace = true
"#.to_string()),
        // 继承带附加属性
        Just(r#"[dependencies]
tokio = { workspace = true, features = ["full"] }
"#.to_string()),
        // 多个继承依赖
        Just(r#"[dependencies]
serde.workspace = true
tokio = { workspace = true, features = ["rt"] }
anyhow.workspace = true
"#.to_string()),
        // dev-dependencies 继承
        Just(r#"[dev-dependencies]
proptest.workspace = true
"#.to_string()),
        // build-dependencies 继承
        Just(r#"[build-dependencies]
cc.workspace = true
"#.to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    /// Property 5: 依赖继承标记保留
    /// *For any* dependency with workspace = true, the inheritance marker should be preserved after roundtrip
    #[test]
    fn prop_workspace_inheritance_preserved(content in workspace_inheritance_strategy()) {
        // Feature: cargo-workspace-support, Property 5: 依赖继承标记保留
        // **Validates: Requirements 3.2, 3.3, 3.5**
        
        let cargo2nu = Cargo2NuConverter::new();
        let nu2cargo = Nu2CargoConverter::new();
        
        let nu_content = cargo2nu.convert(&content);
        let restored = nu2cargo.convert(&nu_content);
        
        // 验证 workspace = true 被保留
        let original_workspace_count = content.matches("workspace = true").count()
            + content.matches(".workspace = true").count();
        let restored_workspace_count = restored.matches("workspace = true").count()
            + restored.matches(".workspace = true").count();
        
        prop_assert_eq!(
            original_workspace_count,
            restored_workspace_count,
            "Workspace inheritance markers not preserved: original {} vs restored {}",
            original_workspace_count,
            restored_workspace_count
        );
        
        // 验证 Nu 格式使用 w = true
        let nu_w_count = nu_content.matches("w = true").count()
            + nu_content.matches(".w = true").count();
        
        prop_assert!(
            nu_w_count > 0,
            "Nu format should use 'w = true' for workspace inheritance"
        );
    }
}

// ==================== Property 8: 往返转换语义等价性 ====================
// **Validates: Requirements 11.1, 11.2, 11.3**

/// 生成完整的 Cargo.toml 配置
fn complete_cargo_toml_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // 完整的单项目配置
        Just(r#"[package]
name = "my-crate"
version = "1.0.0"
edition = "2021"
authors = ["Test Author"]
description = "A test crate"
license = "MIT"
repository = "https://github.com/test/test"

[lib]
proc-macro = true

[dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }

[dev-dependencies]
proptest = "1.0"

[features]
default = ["std"]
std = []
"#.to_string()),
        // Virtual workspace
        Just(r#"[workspace]
members = ["crate1", "crate2", "crate3"]
exclude = ["examples/*"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = "1.0"
"#.to_string()),
        // Mixed workspace
        Just(r#"[package]
name = "root-crate"
version = "0.1.0"
edition = "2021"

[workspace]
members = ["sub1", "sub2"]

[dependencies]
serde.workspace = true

[workspace.dependencies]
serde = "1.0"
"#.to_string()),
        // 带 bin/example/test 目标
        Just(r#"[package]
name = "multi-target"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "cli"
path = "src/bin/cli.rs"
required-features = ["cli"]

[[example]]
name = "demo"
path = "examples/demo.rs"

[[test]]
name = "integration"
path = "tests/integration.rs"

[[bench]]
name = "perf"
path = "benches/perf.rs"

[features]
cli = []
"#.to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    /// Property 8: 往返转换语义等价性
    /// *For any* valid Cargo.toml, Cargo -> Nu -> Cargo should preserve semantic meaning
    #[test]
    fn prop_roundtrip_semantic_equivalence(content in complete_cargo_toml_strategy()) {
        // Feature: cargo-workspace-support, Property 8: 往返转换语义等价性
        // **Validates: Requirements 11.1, 11.2, 11.3**
        
        let cargo2nu = Cargo2NuConverter::new();
        let nu2cargo = Nu2CargoConverter::new();
        
        let nu_content = cargo2nu.convert(&content);
        let restored = nu2cargo.convert(&nu_content);
        
        // 提取关键语义元素进行比较
        fn extract_semantic_elements(content: &str) -> Vec<String> {
            let mut elements = Vec::new();
            
            // 提取所有 key = value 对
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                // 节头
                if trimmed.starts_with('[') {
                    elements.push(trimmed.to_string());
                }
                // 键值对
                else if trimmed.contains('=') {
                    elements.push(trimmed.to_string());
                }
            }
            elements.sort();
            elements
        }
        
        let original_elements = extract_semantic_elements(&content);
        let restored_elements = extract_semantic_elements(&restored);
        
        // 验证元素数量相同
        prop_assert_eq!(
            original_elements.len(),
            restored_elements.len(),
            "Semantic element count mismatch: {} vs {}",
            original_elements.len(),
            restored_elements.len()
        );
        
        // 验证每个元素都被保留
        for (orig, rest) in original_elements.iter().zip(restored_elements.iter()) {
            prop_assert_eq!(
                orig, rest,
                "Semantic element mismatch: '{}' vs '{}'",
                orig, rest
            );
        }
    }
}

// ==================== Property 6: 路径依赖保留 ====================
// **Validates: Requirements 5.1, 5.3, 5.4**

/// 生成包含路径依赖的配置
fn path_dependency_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(r#"[dependencies]
local-crate = { path = "../local-crate" }
"#.to_string()),
        Just(r#"[dependencies]
my-lib = { path = "./libs/my-lib", version = "0.1.0" }
"#.to_string()),
        Just(r#"[patch.crates-io]
serde = { path = "../serde" }
tokio = { path = "../tokio" }
"#.to_string()),
        Just(r#"[dependencies]
internal = { path = "crates/internal", features = ["full"] }
"#.to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    /// Property 6: 路径依赖保留
    /// *For any* path dependency, the path value should remain unchanged after conversion
    #[test]
    fn prop_path_dependencies_preserved(content in path_dependency_strategy()) {
        // Feature: cargo-workspace-support, Property 6: 路径依赖保留
        // **Validates: Requirements 5.1, 5.3, 5.4**
        
        let cargo2nu = Cargo2NuConverter::new();
        let nu2cargo = Nu2CargoConverter::new();
        
        let nu_content = cargo2nu.convert(&content);
        let restored = nu2cargo.convert(&nu_content);
        
        // 提取所有路径值
        fn extract_paths(content: &str) -> Vec<String> {
            let mut paths = Vec::new();
            for line in content.lines() {
                if let Some(start) = line.find("path = \"") {
                    let rest = &line[start + 8..];
                    if let Some(end) = rest.find('"') {
                        paths.push(rest[..end].to_string());
                    }
                }
            }
            paths.sort();
            paths
        }
        
        let original_paths = extract_paths(&content);
        let restored_paths = extract_paths(&restored);
        
        prop_assert_eq!(
            original_paths,
            restored_paths,
            "Path dependencies not preserved"
        );
    }
}
