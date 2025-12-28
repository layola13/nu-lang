// Nu Workspace Analyzer
// Analyzes Nu workspace structure, expands globs, validates members

use glob::glob;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::error::WorkspaceError;
use super::nu_parser::NuParser;
use super::types::*;

/// Nu Workspace 分析器
pub struct NuWorkspaceAnalyzer {
    /// 项目根目录
    root_dir: PathBuf,
    /// Workspace 配置
    config: WorkspaceConfig,
    /// 展开后的成员路径
    expanded_members: Vec<PathBuf>,
    /// 成员依赖图
    dependency_graph: HashMap<String, Vec<String>>,
}

impl NuWorkspaceAnalyzer {
    /// 从目录创建分析器
    pub fn from_dir(root_dir: &Path) -> Result<Self, WorkspaceError> {
        let nu_toml = root_dir.join("Nu.toml");
        if !nu_toml.exists() {
            return Err(WorkspaceError::NotNuProject {
                path: root_dir.to_path_buf(),
            });
        }

        let config = NuParser::parse_file(&nu_toml)?;

        Ok(Self {
            root_dir: root_dir.to_path_buf(),
            config,
            expanded_members: Vec::new(),
            dependency_graph: HashMap::new(),
        })
    }

    /// 获取 workspace 类型
    pub fn workspace_type(&self) -> &WorkspaceType {
        &self.config.workspace_type
    }

    /// 获取配置
    pub fn config(&self) -> &WorkspaceConfig {
        &self.config
    }

    /// 展开 glob 模式并返回所有成员路径
    pub fn expand_members(&mut self) -> Result<&Vec<PathBuf>, WorkspaceError> {
        if !self.expanded_members.is_empty() {
            return Ok(&self.expanded_members);
        }

        let mut members = Vec::new();
        let exclude_set: HashSet<_> = self.config.exclude.iter().collect();

        for pattern in &self.config.members {
            let full_pattern = self.root_dir.join(pattern);
            let pattern_str = full_pattern.to_string_lossy();

            if pattern.contains('*') || pattern.contains('?') || pattern.contains('[') {
                match glob(&pattern_str) {
                    Ok(paths) => {
                        for entry in paths {
                            match entry {
                                Ok(path) => {
                                    if path.is_dir() && path.join("Nu.toml").exists() {
                                        let relative = path
                                            .strip_prefix(&self.root_dir)
                                            .unwrap_or(&path)
                                            .to_path_buf();
                                        let relative_str = relative.to_string_lossy().to_string();

                                        if !exclude_set.contains(&relative_str) {
                                            members.push(relative);
                                        }
                                    }
                                }
                                Err(_) => continue,
                            }
                        }
                    }
                    Err(_) => {
                        return Err(WorkspaceError::InvalidGlobPattern {
                            pattern: pattern.clone(),
                        });
                    }
                }
            } else {
                let member_path = PathBuf::from(pattern);
                let full_path = self.root_dir.join(&member_path);

                if full_path.is_dir()
                    && full_path.join("Nu.toml").exists()
                    && !exclude_set.contains(pattern)
                {
                    members.push(member_path);
                }
            }
        }

        self.expanded_members = members;
        Ok(&self.expanded_members)
    }

    /// 验证所有成员路径
    pub fn validate_members(&self) -> Vec<(String, WorkspaceError)> {
        let mut errors = Vec::new();

        for member in &self.config.members {
            let member_path = self.root_dir.join(member);

            if member.contains('*') || member.contains('?') || member.contains('[') {
                continue;
            }

            if !member_path.exists() {
                errors.push((
                    member.clone(),
                    WorkspaceError::MemberNotFound {
                        member: member.clone(),
                    },
                ));
            } else if !member_path.join("Nu.toml").exists() {
                errors.push((
                    member.clone(),
                    WorkspaceError::NotNuProject { path: member_path },
                ));
            }
        }

        errors
    }

    /// 获取所有成员的完整路径
    pub fn get_member_paths(&mut self) -> Result<Vec<PathBuf>, WorkspaceError> {
        self.expand_members()?;
        Ok(self
            .expanded_members
            .iter()
            .map(|p| self.root_dir.join(p))
            .collect())
    }

    /// 获取 workspace 依赖
    pub fn get_workspace_dependencies(&self) -> &HashMap<String, DependencySpec> {
        &self.config.dependencies
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_type_detection() {
        let virtual_content = r#"
[W]
m = ["lib1", "lib2"]
"#;
        assert_eq!(
            WorkspaceType::from_nu_toml(virtual_content),
            WorkspaceType::Virtual
        );

        let mixed_content = r#"
[P]
id = "root"

[W]
m = ["lib1"]
"#;
        assert_eq!(
            WorkspaceType::from_nu_toml(mixed_content),
            WorkspaceType::Mixed
        );

        let single_content = r#"
[P]
id = "mylib"
"#;
        assert_eq!(
            WorkspaceType::from_nu_toml(single_content),
            WorkspaceType::Single
        );
    }
}
