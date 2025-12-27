// TOML Converter
// Provides bidirectional conversion between Cargo.toml and Nu.toml formats
// This module is used by cargo2nu.rs and nu2cargo.rs binaries

use super::mapping::*;

/// Cargo.toml -> Nu.toml 转换器
pub struct Cargo2NuConverter {
    preserve_comments: bool,
}

impl Default for Cargo2NuConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl Cargo2NuConverter {
    pub fn new() -> Self {
        Self { preserve_comments: true }
    }

    /// 转换 Cargo.toml 内容为 Nu.toml 格式
    pub fn convert(&self, content: &str) -> String {
        let mut result = String::new();
        let mut in_preserved_section = false;
        let mut current_section = String::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // 处理注释
            if trimmed.starts_with('#') {
                if self.preserve_comments {
                    result.push_str(line);
                    result.push('\n');
                }
                continue;
            }

            // 处理空行
            if trimmed.is_empty() {
                result.push('\n');
                continue;
            }

            // 处理节头
            if trimmed.starts_with('[') {
                let section = extract_section(trimmed);
                current_section = section.clone();
                in_preserved_section = is_preserved_section(&section);

                if in_preserved_section {
                    result.push_str(line);
                } else {
                    let converted = convert_section_cargo_to_nu(&section);
                    let indent = &line[..line.len() - line.trim_start().len()];
                    result.push_str(indent);
                    result.push_str(&converted);
                }
                result.push('\n');
                continue;
            }

            // 处理键值对
            if in_preserved_section {
                result.push_str(line);
            } else {
                let converted = convert_kv_line_cargo_to_nu(line, &current_section);
                result.push_str(&converted);
            }
            result.push('\n');
        }

        result
    }
}

/// Nu.toml -> Cargo.toml 转换器
pub struct Nu2CargoConverter {
    preserve_comments: bool,
}

impl Default for Nu2CargoConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl Nu2CargoConverter {
    pub fn new() -> Self {
        Self { preserve_comments: true }
    }

    /// 转换 Nu.toml 内容为 Cargo.toml 格式
    pub fn convert(&self, content: &str) -> String {
        let mut result = String::new();
        let mut in_preserved_section = false;
        let mut current_section = String::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // 处理注释
            if trimmed.starts_with('#') {
                if self.preserve_comments {
                    result.push_str(line);
                    result.push('\n');
                }
                continue;
            }

            // 处理空行
            if trimmed.is_empty() {
                result.push('\n');
                continue;
            }

            // 处理节头
            if trimmed.starts_with('[') {
                let section = extract_section(trimmed);
                current_section = section.clone();
                in_preserved_section = is_preserved_section(&section);

                if in_preserved_section {
                    result.push_str(line);
                } else {
                    let converted = convert_section_nu_to_cargo(&section);
                    let indent = &line[..line.len() - line.trim_start().len()];
                    result.push_str(indent);
                    result.push_str(&converted);
                }
                result.push('\n');
                continue;
            }

            // 处理键值对
            if in_preserved_section {
                result.push_str(line);
            } else {
                let converted = convert_kv_line_nu_to_cargo(line, &current_section);
                result.push_str(&converted);
            }
            result.push('\n');
        }

        result
    }
}

/// 提取节名（包括方括号）
fn extract_section(line: &str) -> String {
    let trimmed = line.trim();
    if let Some(end) = trimmed.rfind(']') {
        return trimmed[..=end].to_string();
    }
    trimmed.to_string()
}

/// 转换键值对行：Cargo -> Nu
fn convert_kv_line_cargo_to_nu(line: &str, _current_section: &str) -> String {
    let trimmed = line.trim();
    
    if let Some(eq_pos) = trimmed.find('=') {
        let key = trimmed[..eq_pos].trim();
        let value = trimmed[eq_pos + 1..].trim();
        let indent = &line[..line.len() - line.trim_start().len()];
        
        // 处理点号语法（dotted key），如 serde.workspace = true
        if key.contains('.') {
            let parts: Vec<&str> = key.splitn(2, '.').collect();
            if parts.len() == 2 {
                let dep_name = parts[0];
                let sub_key = parts[1];
                let converted_sub_key = convert_key_cargo_to_nu(sub_key);
                return format!("{}{}.{} = {}", indent, dep_name, converted_sub_key, value);
            }
        }
        
        let converted_key = convert_key_cargo_to_nu(key);
        
        // 转换内联表格中的键名
        let converted_value = if value.starts_with('{') && value.ends_with('}') {
            convert_inline_table_cargo_to_nu(value)
        } else {
            value.to_string()
        };
        
        return format!("{}{} = {}", indent, converted_key, converted_value);
    }
    
    line.to_string()
}

/// 转换键值对行：Nu -> Cargo
fn convert_kv_line_nu_to_cargo(line: &str, _current_section: &str) -> String {
    let trimmed = line.trim();
    
    if let Some(eq_pos) = trimmed.find('=') {
        let key = trimmed[..eq_pos].trim();
        let value = trimmed[eq_pos + 1..].trim();
        let indent = &line[..line.len() - line.trim_start().len()];
        
        // 处理点号语法（dotted key），如 serde.w = true
        if key.contains('.') {
            let parts: Vec<&str> = key.splitn(2, '.').collect();
            if parts.len() == 2 {
                let dep_name = parts[0];
                let sub_key = parts[1];
                let converted_sub_key = convert_key_nu_to_cargo(sub_key);
                return format!("{}{}.{} = {}", indent, dep_name, converted_sub_key, value);
            }
        }
        
        let converted_key = convert_key_nu_to_cargo(key);
        
        // 转换内联表格中的键名
        let converted_value = if value.starts_with('{') && value.ends_with('}') {
            convert_inline_table_nu_to_cargo(value)
        } else {
            value.to_string()
        };
        
        return format!("{}{} = {}", indent, converted_key, converted_value);
    }
    
    line.to_string()
}

/// 转换内联表格：Cargo -> Nu
fn convert_inline_table_cargo_to_nu(value: &str) -> String {
    let inner = value.trim_start_matches('{').trim_end_matches('}').trim();
    if inner.is_empty() {
        return "{}".to_string();
    }

    let parts = parse_inline_table_parts(inner);
    let converted: Vec<String> = parts.iter()
        .map(|part| convert_inline_kv_cargo_to_nu(part))
        .collect();

    format!("{{ {} }}", converted.join(", "))
}

/// 转换内联表格：Nu -> Cargo
fn convert_inline_table_nu_to_cargo(value: &str) -> String {
    let inner = value.trim_start_matches('{').trim_end_matches('}').trim();
    if inner.is_empty() {
        return "{}".to_string();
    }

    let parts = parse_inline_table_parts(inner);
    let converted: Vec<String> = parts.iter()
        .map(|part| convert_inline_kv_nu_to_cargo(part))
        .collect();

    format!("{{ {} }}", converted.join(", "))
}

/// 解析内联表格的键值对部分
fn parse_inline_table_parts(inner: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for ch in inner.chars() {
        if escape_next {
            current.push(ch);
            escape_next = false;
            continue;
        }

        match ch {
            '\\' => {
                current.push(ch);
                escape_next = true;
            }
            '"' => {
                current.push(ch);
                in_string = !in_string;
            }
            '[' if !in_string => {
                current.push(ch);
                depth += 1;
            }
            ']' if !in_string => {
                current.push(ch);
                depth -= 1;
            }
            ',' if !in_string && depth == 0 => {
                parts.push(current.trim().to_string());
                current = String::new();
            }
            _ => {
                current.push(ch);
            }
        }
    }
    
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }

    parts
}

/// 转换内联键值对：Cargo -> Nu
fn convert_inline_kv_cargo_to_nu(kv: &str) -> String {
    if let Some(eq_pos) = kv.find('=') {
        let key = kv[..eq_pos].trim();
        let value = kv[eq_pos + 1..].trim();
        let converted_key = convert_key_cargo_to_nu(key);
        format!("{} = {}", converted_key, value)
    } else {
        kv.to_string()
    }
}

/// 转换内联键值对：Nu -> Cargo
fn convert_inline_kv_nu_to_cargo(kv: &str) -> String {
    if let Some(eq_pos) = kv.find('=') {
        let key = kv[..eq_pos].trim();
        let value = kv[eq_pos + 1..].trim();
        let converted_key = convert_key_nu_to_cargo(key);
        format!("{} = {}", converted_key, value)
    } else {
        kv.to_string()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Cargo2Nu Tests ====================

    #[test]
    fn test_cargo2nu_basic_sections() {
        let converter = Cargo2NuConverter::new();
        
        let input = r#"[package]
name = "mylib"
version = "0.1.0"
edition = "2021"
"#;
        let output = converter.convert(input);
        assert!(output.contains("[P]"));
        assert!(output.contains("id = \"mylib\""));
        assert!(output.contains("v = \"0.1.0\""));
        assert!(output.contains("ed = \"2021\""));
    }

    #[test]
    fn test_cargo2nu_workspace() {
        let converter = Cargo2NuConverter::new();
        
        let input = r#"[workspace]
members = ["lib1", "lib2"]
exclude = ["temp"]
resolver = "2"
"#;
        let output = converter.convert(input);
        assert!(output.contains("[W]"));
        assert!(output.contains("m = [\"lib1\", \"lib2\"]"));
        assert!(output.contains("ex = [\"temp\"]"));
        assert!(output.contains("r = \"2\""));
    }

    #[test]
    fn test_cargo2nu_workspace_dependencies() {
        let converter = Cargo2NuConverter::new();
        
        let input = r#"[workspace.dependencies]
serde = "1.0"
"#;
        let output = converter.convert(input);
        assert!(output.contains("[W.D]"));
    }

    #[test]
    fn test_cargo2nu_workspace_package() {
        let converter = Cargo2NuConverter::new();
        
        let input = r#"[workspace.package]
version = "1.0.0"
edition = "2021"
authors = ["Test"]
"#;
        let output = converter.convert(input);
        assert!(output.contains("[W.P]"));
        assert!(output.contains("v = \"1.0.0\""));
        assert!(output.contains("ed = \"2021\""));
        assert!(output.contains("au = [\"Test\"]"));
    }

    #[test]
    fn test_cargo2nu_dependencies() {
        let converter = Cargo2NuConverter::new();
        
        let input = r#"[dependencies]
serde = "1.0"

[dev-dependencies]
pretty_assertions = "1.0"

[build-dependencies]
cc = "1.0"
"#;
        let output = converter.convert(input);
        assert!(output.contains("[D]"));
        assert!(output.contains("[DD]"));
        assert!(output.contains("[BD]"));
    }

    #[test]
    fn test_cargo2nu_targets() {
        let converter = Cargo2NuConverter::new();
        
        let input = r#"[lib]
proc-macro = true
crate-type = ["cdylib"]

[[bin]]
name = "mybin"
path = "src/bin/main.rs"

[[example]]
name = "demo"
required-features = ["full"]

[features]
default = ["std"]
"#;
        let output = converter.convert(input);
        assert!(output.contains("[L]"));
        assert!(output.contains("pm = true"));
        assert!(output.contains("ct = [\"cdylib\"]"));
        assert!(output.contains("[[B]]"));
        assert!(output.contains("id = \"mybin\""));
        assert!(output.contains("[[EX]]"));
        assert!(output.contains("rf = [\"full\"]"));
        assert!(output.contains("[FE]"));
    }

    #[test]
    fn test_cargo2nu_inline_table() {
        let converter = Cargo2NuConverter::new();
        
        let input = r#"[dependencies]
serde = { version = "1.0", features = ["derive"], default-features = false }
tokio = { workspace = true, features = ["full"] }
"#;
        let output = converter.convert(input);
        assert!(output.contains("v = \"1.0\""));
        assert!(output.contains("df = false"));
        assert!(output.contains("w = true"));
    }

    #[test]
    fn test_cargo2nu_preserve_profile() {
        let converter = Cargo2NuConverter::new();
        
        let input = r#"[profile.release]
opt-level = 3
lto = true
"#;
        let output = converter.convert(input);
        assert!(output.contains("[profile.release]"));
        assert!(output.contains("opt-level = 3"));
        assert!(output.contains("lto = true"));
    }

    #[test]
    fn test_cargo2nu_preserve_patch() {
        let converter = Cargo2NuConverter::new();
        
        let input = r#"[patch.crates-io]
serde = { path = "../serde" }
"#;
        let output = converter.convert(input);
        assert!(output.contains("[patch.crates-io]"));
        assert!(output.contains("serde = { path = \"../serde\" }"));
    }

    #[test]
    fn test_cargo2nu_preserve_target() {
        let converter = Cargo2NuConverter::new();
        
        let input = r#"[target.'cfg(windows)'.dependencies]
winapi = "0.3"
"#;
        let output = converter.convert(input);
        assert!(output.contains("[target.'cfg(windows)'.dependencies]"));
    }

    // ==================== Nu2Cargo Tests ====================

    #[test]
    fn test_nu2cargo_basic_sections() {
        let converter = Nu2CargoConverter::new();
        
        let input = r#"[P]
id = "mylib"
v = "0.1.0"
ed = "2021"
"#;
        let output = converter.convert(input);
        assert!(output.contains("[package]"));
        assert!(output.contains("name = \"mylib\""));
        assert!(output.contains("version = \"0.1.0\""));
        assert!(output.contains("edition = \"2021\""));
    }

    #[test]
    fn test_nu2cargo_workspace() {
        let converter = Nu2CargoConverter::new();
        
        let input = r#"[W]
m = ["lib1", "lib2"]
ex = ["temp"]
r = "2"
"#;
        let output = converter.convert(input);
        assert!(output.contains("[workspace]"));
        assert!(output.contains("members = [\"lib1\", \"lib2\"]"));
        assert!(output.contains("exclude = [\"temp\"]"));
        assert!(output.contains("resolver = \"2\""));
    }

    #[test]
    fn test_nu2cargo_workspace_dependencies() {
        let converter = Nu2CargoConverter::new();
        
        let input = r#"[W.D]
serde = "1.0"
"#;
        let output = converter.convert(input);
        assert!(output.contains("[workspace.dependencies]"));
    }

    #[test]
    fn test_nu2cargo_inline_table() {
        let converter = Nu2CargoConverter::new();
        
        let input = r#"[D]
serde = { v = "1.0", features = ["derive"], df = false }
tokio = { w = true, features = ["full"] }
"#;
        let output = converter.convert(input);
        assert!(output.contains("version = \"1.0\""));
        assert!(output.contains("default-features = false"));
        assert!(output.contains("workspace = true"));
    }

    // ==================== Roundtrip Tests ====================

    #[test]
    fn test_roundtrip_simple() {
        let cargo2nu = Cargo2NuConverter::new();
        let nu2cargo = Nu2CargoConverter::new();
        
        let original = r#"[package]
name = "mylib"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1.0"
"#;
        let nu = cargo2nu.convert(original);
        let restored = nu2cargo.convert(&nu);
        
        // 验证关键内容存在
        assert!(restored.contains("[package]"));
        assert!(restored.contains("name = \"mylib\""));
        assert!(restored.contains("version = \"0.1.0\""));
        assert!(restored.contains("[dependencies]"));
    }

    #[test]
    fn test_roundtrip_workspace() {
        let cargo2nu = Cargo2NuConverter::new();
        let nu2cargo = Nu2CargoConverter::new();
        
        let original = r#"[workspace]
members = ["lib1", "lib2"]
resolver = "2"

[workspace.dependencies]
serde = "1.0"
"#;
        let nu = cargo2nu.convert(original);
        let restored = nu2cargo.convert(&nu);
        
        assert!(restored.contains("[workspace]"));
        assert!(restored.contains("members = [\"lib1\", \"lib2\"]"));
        assert!(restored.contains("resolver = \"2\""));
        assert!(restored.contains("[workspace.dependencies]"));
    }

    #[test]
    fn test_roundtrip_inline_table() {
        let cargo2nu = Cargo2NuConverter::new();
        let nu2cargo = Nu2CargoConverter::new();
        
        let original = r#"[dependencies]
serde = { version = "1.0", features = ["derive"] }
"#;
        let nu = cargo2nu.convert(original);
        let restored = nu2cargo.convert(&nu);
        
        assert!(restored.contains("[dependencies]"));
        assert!(restored.contains("version = \"1.0\""));
        assert!(restored.contains("features = [\"derive\"]"));
    }

    #[test]
    fn test_roundtrip_preserved_sections() {
        let cargo2nu = Cargo2NuConverter::new();
        let nu2cargo = Nu2CargoConverter::new();
        
        let original = r#"[profile.release]
opt-level = 3
lto = true

[patch.crates-io]
serde = { path = "../serde" }
"#;
        let nu = cargo2nu.convert(original);
        let restored = nu2cargo.convert(&nu);
        
        // 保留节应该完全不变
        assert!(restored.contains("[profile.release]"));
        assert!(restored.contains("opt-level = 3"));
        assert!(restored.contains("[patch.crates-io]"));
    }

    #[test]
    fn test_cargo2nu_dotted_key() {
        let converter = Cargo2NuConverter::new();
        
        let input = r#"[dependencies]
serde.workspace = true
tokio.workspace = true
"#;
        let output = converter.convert(input);
        assert!(output.contains("serde.w = true"));
        assert!(output.contains("tokio.w = true"));
    }

    #[test]
    fn test_nu2cargo_dotted_key() {
        let converter = Nu2CargoConverter::new();
        
        let input = r#"[D]
serde.w = true
tokio.w = true
"#;
        let output = converter.convert(input);
        assert!(output.contains("serde.workspace = true"));
        assert!(output.contains("tokio.workspace = true"));
    }

    #[test]
    fn test_roundtrip_dotted_key() {
        let cargo2nu = Cargo2NuConverter::new();
        let nu2cargo = Nu2CargoConverter::new();
        
        let original = r#"[dependencies]
serde.workspace = true
"#;
        let nu = cargo2nu.convert(original);
        assert!(nu.contains("serde.w = true"));
        
        let restored = nu2cargo.convert(&nu);
        assert!(restored.contains("serde.workspace = true"));
    }
}
