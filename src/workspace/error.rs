// Workspace Error Types
// Defines error types for workspace operations

use std::path::PathBuf;
use thiserror::Error;

/// Workspace 操作错误
#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("无法读取文件: {path}")]
    FileReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("无法写入文件: {path}")]
    FileWriteError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("无效的 TOML 格式: {message}")]
    TomlParseError {
        message: String,
        line: Option<usize>,
    },

    #[error("Workspace 成员不存在: {member}")]
    MemberNotFound { member: String },

    #[error("检测到循环依赖: {}", cycle.join(" -> "))]
    CyclicDependency { cycle: Vec<String> },

    #[error("Glob 模式无效: {pattern}")]
    InvalidGlobPattern { pattern: String },

    #[error("路径依赖无效: {path}")]
    InvalidPathDependency { path: String },

    #[error("转换失败: {file}")]
    ConversionError {
        file: PathBuf,
        #[source]
        source: anyhow::Error,
    },

    #[error("目录不存在: {path}")]
    DirectoryNotFound { path: PathBuf },

    #[error("不是有效的 Cargo 项目: {path}")]
    NotCargoProject { path: PathBuf },

    #[error("不是有效的 Nu 项目: {path}")]
    NotNuProject { path: PathBuf },

    #[error("配置错误: {message}")]
    ConfigError { message: String },
}

/// 验证结果
#[derive(Debug)]
pub enum ValidationResult {
    /// 验证通过
    Valid,
    /// 警告（可继续）
    Warning(String),
    /// 错误（应停止）
    Error(String), // 改为存储错误消息而不是WorkspaceError
}

impl ValidationResult {
    /// 是否为有效结果
    pub fn is_valid(&self) -> bool {
        matches!(self, ValidationResult::Valid)
    }

    /// 是否为警告
    pub fn is_warning(&self) -> bool {
        matches!(self, ValidationResult::Warning(_))
    }

    /// 是否为错误
    pub fn is_error(&self) -> bool {
        matches!(self, ValidationResult::Error(_))
    }
}

impl From<std::io::Error> for WorkspaceError {
    fn from(err: std::io::Error) -> Self {
        WorkspaceError::FileReadError {
            path: PathBuf::new(),
            source: err,
        }
    }
}

impl From<toml::de::Error> for WorkspaceError {
    fn from(err: toml::de::Error) -> Self {
        WorkspaceError::TomlParseError {
            message: err.to_string(),
            line: None, // toml::de::Error 不提供行号信息
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result() {
        let valid = ValidationResult::Valid;
        assert!(valid.is_valid());
        assert!(!valid.is_warning());
        assert!(!valid.is_error());

        let warning = ValidationResult::Warning("test warning".to_string());
        assert!(!warning.is_valid());
        assert!(warning.is_warning());
        assert!(!warning.is_error());

        let error = ValidationResult::Error("Member not found: test".to_string());
        assert!(!error.is_valid());
        assert!(!error.is_warning());
        assert!(error.is_error());
    }

    #[test]
    fn test_error_display() {
        let err = WorkspaceError::MemberNotFound {
            member: "lib1".to_string(),
        };
        assert_eq!(err.to_string(), "Workspace 成员不存在: lib1");

        let err = WorkspaceError::CyclicDependency {
            cycle: vec!["a".to_string(), "b".to_string(), "a".to_string()],
        };
        assert_eq!(err.to_string(), "检测到循环依赖: a -> b -> a");
    }
}
