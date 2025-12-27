// Cargo.toml Parser
// Parses Cargo.toml files into structured WorkspaceConfig

use std::collections::HashMap;
use std::path::Path;
use toml::Value;

use super::error::WorkspaceError;
use super::types::*;

/// Cargo.toml 解析器
pub struct CargoParser;

impl CargoParser {
    /// 解析 Cargo.toml 文件
    pub fn parse_file(path: &Path) -> Result<WorkspaceConfig, WorkspaceError> {
        let content = std::fs::read_to_string(path).map_err(|e| WorkspaceError::FileReadError {
            path: path.to_path_buf(),
            source: e,
        })?;
        Self::parse(&content)
    }

    /// 解析 Cargo.toml 内容
    pub fn parse(content: &str) -> Result<WorkspaceConfig, WorkspaceError> {
        let toml_value: Value = content.parse().map_err(|e: toml::de::Error| {
            WorkspaceError::TomlParseError {
                message: e.to_string(),
                line: None,
            }
        })?;

        let mut config = WorkspaceConfig::default();
        config.workspace_type = WorkspaceType::from_cargo_toml(content);

        if let Some(workspace) = toml_value.get("workspace") {
            Self::parse_workspace_section(workspace, &mut config)?;
        }

        Ok(config)
    }

    /// 解析 [workspace] 节
    fn parse_workspace_section(workspace: &Value, config: &mut WorkspaceConfig) -> Result<(), WorkspaceError> {
        if let Some(table) = workspace.as_table() {
            // 解析 members
            if let Some(members) = table.get("members") {
                config.members = Self::parse_string_array(members)?;
            }

            // 解析 exclude
            if let Some(exclude) = table.get("exclude") {
                config.exclude = Self::parse_string_array(exclude)?;
            }

            // 解析 resolver
            if let Some(resolver) = table.get("resolver") {
                config.resolver = resolver.as_str().map(|s| s.to_string());
            }

            // 解析 workspace.dependencies
            if let Some(deps) = table.get("dependencies") {
                config.dependencies = Self::parse_dependencies(deps)?;
            }

            // 解析 workspace.package
            if let Some(pkg) = table.get("package") {
                config.package = Some(Self::parse_workspace_package(pkg)?);
            }

            // 解析 workspace.lints
            if let Some(lints) = table.get("lints") {
                config.lints = Some(Self::parse_workspace_lints(lints)?);
            }

            // 解析 workspace.metadata
            if let Some(metadata) = table.get("metadata") {
                if let Some(meta_table) = metadata.as_table() {
                    config.metadata = meta_table.iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                }
            }
        }

        Ok(())
    }

    /// 解析字符串数组（支持单行和多行格式）
    fn parse_string_array(value: &Value) -> Result<Vec<String>, WorkspaceError> {
        match value {
            Value::Array(arr) => {
                arr.iter()
                    .map(|v| {
                        v.as_str()
                            .map(|s| s.to_string())
                            .ok_or_else(|| WorkspaceError::TomlParseError {
                                message: "Expected string in array".to_string(),
                                line: None,
                            })
                    })
                    .collect()
            }
            Value::String(s) => Ok(vec![s.clone()]),
            _ => Err(WorkspaceError::TomlParseError {
                message: "Expected array or string".to_string(),
                line: None,
            }),
        }
    }

    /// 解析依赖表
    fn parse_dependencies(deps: &Value) -> Result<HashMap<String, DependencySpec>, WorkspaceError> {
        let mut result = HashMap::new();

        if let Some(table) = deps.as_table() {
            for (name, value) in table {
                let spec = Self::parse_dependency_spec(value)?;
                result.insert(name.clone(), spec);
            }
        }

        Ok(result)
    }

    /// 解析单个依赖规格
    fn parse_dependency_spec(value: &Value) -> Result<DependencySpec, WorkspaceError> {
        let mut spec = DependencySpec::default();

        match value {
            Value::String(version) => {
                spec.version = Some(version.clone());
            }
            Value::Table(table) => {
                if let Some(v) = table.get("version") {
                    spec.version = v.as_str().map(|s| s.to_string());
                }
                if let Some(v) = table.get("path") {
                    spec.path = v.as_str().map(|s| s.to_string());
                }
                if let Some(v) = table.get("git") {
                    spec.git = v.as_str().map(|s| s.to_string());
                }
                if let Some(v) = table.get("branch") {
                    spec.branch = v.as_str().map(|s| s.to_string());
                }
                if let Some(v) = table.get("tag") {
                    spec.tag = v.as_str().map(|s| s.to_string());
                }
                if let Some(v) = table.get("rev") {
                    spec.rev = v.as_str().map(|s| s.to_string());
                }
                if let Some(v) = table.get("features") {
                    spec.features = Self::parse_string_array(v)?;
                }
                if let Some(v) = table.get("optional") {
                    spec.optional = v.as_bool().unwrap_or(false);
                }
                if let Some(v) = table.get("default-features") {
                    spec.default_features = v.as_bool();
                }
                if let Some(v) = table.get("workspace") {
                    spec.workspace = v.as_bool().unwrap_or(false);
                }
                if let Some(v) = table.get("package") {
                    spec.package = v.as_str().map(|s| s.to_string());
                }

                // 保存其他未解析的字段
                for (key, val) in table {
                    if !matches!(key.as_str(), 
                        "version" | "path" | "git" | "branch" | "tag" | "rev" | 
                        "features" | "optional" | "default-features" | "workspace" | "package"
                    ) {
                        spec.extra.insert(key.clone(), val.clone());
                    }
                }
            }
            _ => {
                return Err(WorkspaceError::TomlParseError {
                    message: "Invalid dependency format".to_string(),
                    line: None,
                });
            }
        }

        Ok(spec)
    }

    /// 解析 workspace.package
    fn parse_workspace_package(pkg: &Value) -> Result<WorkspacePackage, WorkspaceError> {
        let mut result = WorkspacePackage::default();

        if let Some(table) = pkg.as_table() {
            result.version = table.get("version").and_then(|v| v.as_str()).map(|s| s.to_string());
            result.edition = table.get("edition").and_then(|v| v.as_str()).map(|s| s.to_string());
            result.authors = table.get("authors").and_then(|v| Self::parse_string_array(v).ok());
            result.description = table.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
            result.license = table.get("license").and_then(|v| v.as_str()).map(|s| s.to_string());
            result.license_file = table.get("license-file").and_then(|v| v.as_str()).map(|s| s.to_string());
            result.repository = table.get("repository").and_then(|v| v.as_str()).map(|s| s.to_string());
            result.documentation = table.get("documentation").and_then(|v| v.as_str()).map(|s| s.to_string());
            result.homepage = table.get("homepage").and_then(|v| v.as_str()).map(|s| s.to_string());
            result.readme = table.get("readme").and_then(|v| v.as_str()).map(|s| s.to_string());
            result.keywords = table.get("keywords").and_then(|v| Self::parse_string_array(v).ok());
            result.categories = table.get("categories").and_then(|v| Self::parse_string_array(v).ok());
            result.rust_version = table.get("rust-version").and_then(|v| v.as_str()).map(|s| s.to_string());
            result.publish = table.get("publish").and_then(|v| v.as_bool());
            result.exclude = table.get("exclude").and_then(|v| Self::parse_string_array(v).ok());
            result.include = table.get("include").and_then(|v| Self::parse_string_array(v).ok());
        }

        Ok(result)
    }

    /// 解析 workspace.lints
    fn parse_workspace_lints(lints: &Value) -> Result<WorkspaceLints, WorkspaceError> {
        let mut result = WorkspaceLints::default();

        if let Some(table) = lints.as_table() {
            if let Some(rust) = table.get("rust") {
                if let Some(rust_table) = rust.as_table() {
                    result.rust = rust_table.iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                }
            }
            if let Some(clippy) = table.get("clippy") {
                if let Some(clippy_table) = clippy.as_table() {
                    result.clippy = clippy_table.iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                }
            }
        }

        Ok(result)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_virtual_workspace() {
        let content = r#"
[workspace]
members = ["lib1", "lib2", "crates/*"]
exclude = ["target", "temp"]
resolver = "2"
"#;
        let config = CargoParser::parse(content).unwrap();
        assert_eq!(config.workspace_type, WorkspaceType::Virtual);
        assert_eq!(config.members, vec!["lib1", "lib2", "crates/*"]);
        assert_eq!(config.exclude, vec!["target", "temp"]);
        assert_eq!(config.resolver, Some("2".to_string()));
    }

    #[test]
    fn test_parse_mixed_workspace() {
        let content = r#"
[package]
name = "root"
version = "0.1.0"

[workspace]
members = ["lib1"]
"#;
        let config = CargoParser::parse(content).unwrap();
        assert_eq!(config.workspace_type, WorkspaceType::Mixed);
        assert_eq!(config.members, vec!["lib1"]);
    }

    #[test]
    fn test_parse_workspace_dependencies() {
        let content = r#"
[workspace]
members = ["lib1"]

[workspace.dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }
local-lib = { path = "../local" }
"#;
        let config = CargoParser::parse(content).unwrap();
        
        let serde = config.dependencies.get("serde").unwrap();
        assert_eq!(serde.version, Some("1.0".to_string()));
        
        let tokio = config.dependencies.get("tokio").unwrap();
        assert_eq!(tokio.version, Some("1.0".to_string()));
        assert_eq!(tokio.features, vec!["full"]);
        
        let local = config.dependencies.get("local-lib").unwrap();
        assert_eq!(local.path, Some("../local".to_string()));
    }

    #[test]
    fn test_parse_workspace_package() {
        let content = r#"
[workspace]
members = ["lib1"]

[workspace.package]
version = "1.0.0"
edition = "2021"
authors = ["Author <author@example.com>"]
license = "MIT"
repository = "https://github.com/example/repo"
"#;
        let config = CargoParser::parse(content).unwrap();
        let pkg = config.package.unwrap();
        
        assert_eq!(pkg.version, Some("1.0.0".to_string()));
        assert_eq!(pkg.edition, Some("2021".to_string()));
        assert_eq!(pkg.authors, Some(vec!["Author <author@example.com>".to_string()]));
        assert_eq!(pkg.license, Some("MIT".to_string()));
        assert_eq!(pkg.repository, Some("https://github.com/example/repo".to_string()));
    }

    #[test]
    fn test_parse_workspace_lints() {
        let content = r#"
[workspace]
members = ["lib1"]

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
all = "warn"
"#;
        let config = CargoParser::parse(content).unwrap();
        let lints = config.lints.unwrap();
        
        assert!(lints.rust.contains_key("unsafe_code"));
        assert!(lints.clippy.contains_key("all"));
    }

    #[test]
    fn test_parse_dependency_with_workspace_inheritance() {
        let content = r#"
[workspace]
members = ["lib1"]

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
"#;
        let config = CargoParser::parse(content).unwrap();
        let serde = config.dependencies.get("serde").unwrap();
        
        assert_eq!(serde.version, Some("1.0".to_string()));
        assert_eq!(serde.features, vec!["derive"]);
    }
}
