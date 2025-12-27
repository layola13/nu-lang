// Workspace Core Types
// Defines the fundamental data structures for workspace handling

use std::collections::HashMap;
use std::path::PathBuf;

/// Workspace 类型枚举
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceType {
    /// 仅包含 [workspace]，无 [package] - 纯虚拟 workspace
    Virtual,
    /// 同时包含 [workspace] 和 [package] - 混合 workspace
    Mixed,
    /// 单个项目，无 workspace
    Single,
}

impl WorkspaceType {
    /// 从 TOML 内容判断 workspace 类型
    pub fn from_cargo_toml(content: &str) -> Self {
        let has_workspace = content.contains("[workspace]");
        let has_package = content.contains("[package]");

        match (has_workspace, has_package) {
            (true, false) => WorkspaceType::Virtual,
            (true, true) => WorkspaceType::Mixed,
            (false, _) => WorkspaceType::Single,
        }
    }

    /// 从 Nu.toml 内容判断 workspace 类型
    pub fn from_nu_toml(content: &str) -> Self {
        let has_workspace = content.contains("[W]");
        let has_package = content.contains("[P]");

        match (has_workspace, has_package) {
            (true, false) => WorkspaceType::Virtual,
            (true, true) => WorkspaceType::Mixed,
            (false, _) => WorkspaceType::Single,
        }
    }

    /// 是否为 workspace 项目
    pub fn is_workspace(&self) -> bool {
        matches!(self, WorkspaceType::Virtual | WorkspaceType::Mixed)
    }
}

/// 依赖规格
#[derive(Debug, Clone, Default)]
pub struct DependencySpec {
    /// 版本号
    pub version: Option<String>,
    /// 路径依赖
    pub path: Option<String>,
    /// Git 仓库
    pub git: Option<String>,
    /// Git 分支
    pub branch: Option<String>,
    /// Git 标签
    pub tag: Option<String>,
    /// Git 修订
    pub rev: Option<String>,
    /// Features 列表
    pub features: Vec<String>,
    /// 是否可选
    pub optional: bool,
    /// 默认 features
    pub default_features: Option<bool>,
    /// 是否从 workspace 继承
    pub workspace: bool,
    /// 包名（如果与依赖名不同）
    pub package: Option<String>,
    /// 其他未解析的字段
    pub extra: HashMap<String, toml::Value>,
}

/// Workspace 包配置（可继承的字段）
#[derive(Debug, Clone, Default)]
pub struct WorkspacePackage {
    pub version: Option<String>,
    pub edition: Option<String>,
    pub authors: Option<Vec<String>>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub license_file: Option<String>,
    pub repository: Option<String>,
    pub documentation: Option<String>,
    pub homepage: Option<String>,
    pub readme: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub rust_version: Option<String>,
    pub publish: Option<bool>,
    pub exclude: Option<Vec<String>>,
    pub include: Option<Vec<String>>,
}

/// Workspace Lints 配置
#[derive(Debug, Clone, Default)]
pub struct WorkspaceLints {
    pub rust: HashMap<String, toml::Value>,
    pub clippy: HashMap<String, toml::Value>,
}

/// Workspace 配置的统一表示
#[derive(Debug, Clone, Default)]
pub struct WorkspaceConfig {
    /// Workspace 类型
    pub workspace_type: WorkspaceType,
    /// 成员列表（已展开 glob）
    pub members: Vec<String>,
    /// 排除列表
    pub exclude: Vec<String>,
    /// Resolver 版本
    pub resolver: Option<String>,
    /// Workspace 级别依赖
    pub dependencies: HashMap<String, DependencySpec>,
    /// Workspace 级别 package 配置
    pub package: Option<WorkspacePackage>,
    /// Workspace 元数据
    pub metadata: HashMap<String, toml::Value>,
    /// Workspace lints 配置
    pub lints: Option<WorkspaceLints>,
    /// Patch 配置 (registry -> package -> spec)
    pub patches: HashMap<String, HashMap<String, DependencySpec>>,
}

impl Default for WorkspaceType {
    fn default() -> Self {
        WorkspaceType::Single
    }
}

/// 特殊目录配置
#[derive(Debug, Clone, Default)]
pub struct SpecialDirs {
    /// tests 目录
    pub tests: Option<PathBuf>,
    /// examples 目录
    pub examples: Option<PathBuf>,
    /// benches 目录
    pub benches: Option<PathBuf>,
    /// build.rs 或 build.nu 文件
    pub build_script: Option<PathBuf>,
}

/// 成员包配置
#[derive(Debug, Clone, Default)]
pub struct MemberConfig {
    /// 包名
    pub name: Option<String>,
    /// 版本
    pub version: Option<String>,
    /// Edition
    pub edition: Option<String>,
    /// 是否为 proc-macro
    pub proc_macro: bool,
    /// 依赖列表
    pub dependencies: HashMap<String, DependencySpec>,
    /// 开发依赖
    pub dev_dependencies: HashMap<String, DependencySpec>,
    /// 构建依赖
    pub build_dependencies: HashMap<String, DependencySpec>,
    /// Features
    pub features: HashMap<String, Vec<String>>,
    /// 二进制目标
    pub bins: Vec<BinaryTarget>,
    /// 示例目标
    pub examples: Vec<BinaryTarget>,
    /// 测试目标
    pub tests: Vec<BinaryTarget>,
    /// 基准测试目标
    pub benches: Vec<BinaryTarget>,
    /// Lib 配置
    pub lib: Option<LibTarget>,
    /// 原始 TOML 内容（用于保留未解析的字段）
    pub raw_toml: Option<toml::Value>,
}

/// 二进制/示例/测试目标
#[derive(Debug, Clone, Default)]
pub struct BinaryTarget {
    pub name: String,
    pub path: Option<String>,
    pub required_features: Vec<String>,
}

/// Lib 目标配置
#[derive(Debug, Clone, Default)]
pub struct LibTarget {
    pub name: Option<String>,
    pub path: Option<String>,
    pub proc_macro: bool,
    pub crate_type: Vec<String>,
}

/// 源文件信息
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// 相对路径
    pub path: PathBuf,
    /// 文件类型
    pub file_type: SourceFileType,
}

/// 源文件类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceFileType {
    /// Rust 源文件 (.rs)
    Rust,
    /// Nu 源文件 (.nu)
    Nu,
    /// 构建脚本
    BuildScript,
}

/// Workspace 成员
#[derive(Debug, Clone, Default)]
pub struct WorkspaceMember {
    /// 成员相对路径
    pub path: PathBuf,
    /// 成员配置
    pub config: MemberConfig,
    /// 源文件列表
    pub source_files: Vec<SourceFile>,
    /// 特殊目录
    pub special_dirs: SpecialDirs,
}

/// 转换选项
#[derive(Debug, Clone, Default)]
pub struct ConvertOptions {
    /// 强制覆盖
    pub force: bool,
    /// 增量转换
    pub incremental: bool,
    /// 详细输出
    pub verbose: bool,
    /// 仅预览
    pub dry_run: bool,
    /// 排除模式
    pub exclude: Vec<String>,
    /// 仅包含成员
    pub only: Vec<String>,
}

/// 转换报告
#[derive(Debug, Clone, Default)]
pub struct ConvertReport {
    /// Workspace 类型
    pub workspace_type: WorkspaceType,
    /// 成员总数
    pub members_total: usize,
    /// 已转换成员数
    pub members_converted: usize,
    /// 文件总数
    pub files_total: usize,
    /// 已转换文件数
    pub files_converted: usize,
    /// 跳过文件数
    pub files_skipped: usize,
    /// 失败文件数
    pub files_failed: usize,
    /// 警告列表
    pub warnings: Vec<String>,
    /// 错误列表
    pub errors: Vec<String>,
}

impl ConvertReport {
    /// 创建新的报告
    pub fn new(workspace_type: WorkspaceType) -> Self {
        Self {
            workspace_type,
            ..Default::default()
        }
    }

    /// 添加警告
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// 添加错误
    pub fn add_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
    }

    /// 是否成功（无错误）
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }

    /// 格式化输出报告
    pub fn format(&self) -> String {
        let mut output = String::new();
        
        output.push_str(&format!("\n转换报告:\n"));
        output.push_str(&format!("  Workspace 类型: {:?}\n", self.workspace_type));
        output.push_str(&format!("  成员: {}/{}\n", self.members_converted, self.members_total));
        output.push_str(&format!("  文件: {} 转换, {} 跳过, {} 失败\n", 
            self.files_converted, self.files_skipped, self.files_failed));
        
        if !self.warnings.is_empty() {
            output.push_str(&format!("\n警告 ({}):\n", self.warnings.len()));
            for warning in &self.warnings {
                output.push_str(&format!("  ⚠ {}\n", warning));
            }
        }
        
        if !self.errors.is_empty() {
            output.push_str(&format!("\n错误 ({}):\n", self.errors.len()));
            for error in &self.errors {
                output.push_str(&format!("  ✗ {}\n", error));
            }
        }
        
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_type_from_cargo_toml() {
        // Virtual workspace
        let virtual_ws = r#"
[workspace]
members = ["lib1", "lib2"]
"#;
        assert_eq!(WorkspaceType::from_cargo_toml(virtual_ws), WorkspaceType::Virtual);

        // Mixed workspace
        let mixed_ws = r#"
[workspace]
members = ["lib1"]

[package]
name = "root"
"#;
        assert_eq!(WorkspaceType::from_cargo_toml(mixed_ws), WorkspaceType::Mixed);

        // Single project
        let single = r#"
[package]
name = "mylib"
version = "0.1.0"
"#;
        assert_eq!(WorkspaceType::from_cargo_toml(single), WorkspaceType::Single);
    }

    #[test]
    fn test_workspace_type_from_nu_toml() {
        // Virtual workspace
        let virtual_ws = r#"
[W]
m = ["lib1", "lib2"]
"#;
        assert_eq!(WorkspaceType::from_nu_toml(virtual_ws), WorkspaceType::Virtual);

        // Mixed workspace
        let mixed_ws = r#"
[W]
m = ["lib1"]

[P]
id = "root"
"#;
        assert_eq!(WorkspaceType::from_nu_toml(mixed_ws), WorkspaceType::Mixed);

        // Single project
        let single = r#"
[P]
id = "mylib"
v = "0.1.0"
"#;
        assert_eq!(WorkspaceType::from_nu_toml(single), WorkspaceType::Single);
    }

    #[test]
    fn test_workspace_type_is_workspace() {
        assert!(WorkspaceType::Virtual.is_workspace());
        assert!(WorkspaceType::Mixed.is_workspace());
        assert!(!WorkspaceType::Single.is_workspace());
    }

    #[test]
    fn test_convert_report() {
        let mut report = ConvertReport::new(WorkspaceType::Virtual);
        report.members_total = 5;
        report.members_converted = 4;
        report.files_total = 100;
        report.files_converted = 95;
        report.files_skipped = 3;
        report.files_failed = 2;
        report.add_warning("Member 'test' not found");
        report.add_error("Failed to convert file.rs");

        assert!(!report.is_success());
        let formatted = report.format();
        assert!(formatted.contains("成员: 4/5"));
        assert!(formatted.contains("警告"));
        assert!(formatted.contains("错误"));
    }
}
