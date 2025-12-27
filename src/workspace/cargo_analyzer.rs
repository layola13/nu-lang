// Cargo Workspace Analyzer
// Analyzes Cargo workspace structure, expands globs, validates members

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::fs;
use glob::glob;

use super::error::WorkspaceError;
use super::types::*;
use super::cargo_parser::CargoParser;

/// Cargo Workspace 分析器
pub struct CargoWorkspaceAnalyzer {
    /// 项目根目录
    root_dir: PathBuf,
    /// Workspace 配置
    config: WorkspaceConfig,
    /// 展开后的成员路径
    expanded_members: Vec<PathBuf>,
    /// 成员依赖图（用于循环检测）
    dependency_graph: HashMap<String, Vec<String>>,
}

impl CargoWorkspaceAnalyzer {
    /// 从目录创建分析器
    pub fn from_dir(root_dir: &Path) -> Result<Self, WorkspaceError> {
        let cargo_toml = root_dir.join("Cargo.toml");
        if !cargo_toml.exists() {
            return Err(WorkspaceError::NotCargoProject {
                path: root_dir.to_path_buf(),
            });
        }

        let config = CargoParser::parse_file(&cargo_toml)?;
        
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

            // 检查是否包含 glob 字符
            if pattern.contains('*') || pattern.contains('?') || pattern.contains('[') {
                // 使用 glob 展开
                match glob(&pattern_str) {
                    Ok(paths) => {
                        for entry in paths {
                            match entry {
                                Ok(path) => {
                                    if path.is_dir() && path.join("Cargo.toml").exists() {
                                        let relative = path.strip_prefix(&self.root_dir)
                                            .unwrap_or(&path)
                                            .to_path_buf();
                                        let relative_str = relative.to_string_lossy().to_string();
                                        
                                        // 检查是否被排除
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
                // 直接路径
                let member_path = PathBuf::from(pattern);
                let full_path = self.root_dir.join(&member_path);
                
                if full_path.is_dir() && full_path.join("Cargo.toml").exists() {
                    if !exclude_set.contains(pattern) {
                        members.push(member_path);
                    }
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
            
            // 跳过 glob 模式
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
            } else if !member_path.join("Cargo.toml").exists() {
                errors.push((
                    member.clone(),
                    WorkspaceError::NotCargoProject {
                        path: member_path,
                    },
                ));
            }
        }

        errors
    }

    /// 构建依赖图
    pub fn build_dependency_graph(&mut self) -> Result<(), WorkspaceError> {
        self.expand_members()?;
        self.dependency_graph.clear();

        for member_path in &self.expanded_members.clone() {
            let cargo_toml = self.root_dir.join(member_path).join("Cargo.toml");
            if !cargo_toml.exists() {
                continue;
            }

            let content = fs::read_to_string(&cargo_toml).map_err(|e| {
                WorkspaceError::FileReadError {
                    path: cargo_toml.clone(),
                    source: e,
                }
            })?;

            let member_name = member_path.to_string_lossy().to_string();
            let deps = self.extract_path_dependencies(&content);
            self.dependency_graph.insert(member_name, deps);
        }

        Ok(())
    }

    /// 从 TOML 内容提取路径依赖
    fn extract_path_dependencies(&self, content: &str) -> Vec<String> {
        let mut deps = Vec::new();
        
        // 简单解析 path = "..." 模式
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.contains("path") && trimmed.contains("=") {
                // 提取路径值
                if let Some(start) = trimmed.find('"') {
                    if let Some(end) = trimmed[start + 1..].find('"') {
                        let path = &trimmed[start + 1..start + 1 + end];
                        // 规范化路径
                        let normalized = PathBuf::from(path)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.to_string());
                        deps.push(normalized);
                    }
                }
            }
        }

        deps
    }

    /// 检测循环依赖
    pub fn detect_cycles(&mut self) -> Result<Option<Vec<String>>, WorkspaceError> {
        self.build_dependency_graph()?;

        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for node in self.dependency_graph.keys() {
            if !visited.contains(node) {
                if let Some(cycle) = self.dfs_detect_cycle(node, &mut visited, &mut rec_stack, &mut path) {
                    return Ok(Some(cycle));
                }
            }
        }

        Ok(None)
    }

    /// DFS 检测循环
    fn dfs_detect_cycle(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(neighbors) = self.dependency_graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if let Some(cycle) = self.dfs_detect_cycle(neighbor, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(neighbor) {
                    // 找到循环
                    let mut cycle = path.clone();
                    cycle.push(neighbor.clone());
                    return Some(cycle);
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
        None
    }

    /// 获取所有成员的完整路径
    pub fn get_member_paths(&mut self) -> Result<Vec<PathBuf>, WorkspaceError> {
        self.expand_members()?;
        Ok(self.expanded_members.iter()
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
        // 测试 Virtual workspace
        let virtual_content = r#"
[workspace]
members = ["lib1", "lib2"]
"#;
        assert_eq!(
            WorkspaceType::from_cargo_toml(virtual_content),
            WorkspaceType::Virtual
        );

        // 测试 Mixed workspace
        let mixed_content = r#"
[package]
name = "root"

[workspace]
members = ["lib1"]
"#;
        assert_eq!(
            WorkspaceType::from_cargo_toml(mixed_content),
            WorkspaceType::Mixed
        );

        // 测试 Single project
        let single_content = r#"
[package]
name = "mylib"
"#;
        assert_eq!(
            WorkspaceType::from_cargo_toml(single_content),
            WorkspaceType::Single
        );
    }

    #[test]
    fn test_extract_path_dependencies() {
        let analyzer = CargoWorkspaceAnalyzer {
            root_dir: PathBuf::from("."),
            config: WorkspaceConfig::default(),
            expanded_members: Vec::new(),
            dependency_graph: HashMap::new(),
        };

        let content = r#"
[dependencies]
local-lib = { path = "../local-lib" }
other = { path = "./other", version = "0.1" }
"#;
        let deps = analyzer.extract_path_dependencies(content);
        assert!(deps.contains(&"local-lib".to_string()));
        assert!(deps.contains(&"other".to_string()));
    }

    #[test]
    fn test_cycle_detection_no_cycle() {
        let mut analyzer = CargoWorkspaceAnalyzer {
            root_dir: PathBuf::from("."),
            config: WorkspaceConfig::default(),
            expanded_members: Vec::new(),
            dependency_graph: HashMap::new(),
        };

        // A -> B -> C (无循环)
        analyzer.dependency_graph.insert("A".to_string(), vec!["B".to_string()]);
        analyzer.dependency_graph.insert("B".to_string(), vec!["C".to_string()]);
        analyzer.dependency_graph.insert("C".to_string(), vec![]);

        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        let cycle = analyzer.dfs_detect_cycle("A", &mut visited, &mut rec_stack, &mut path);
        assert!(cycle.is_none());
    }

    #[test]
    fn test_cycle_detection_with_cycle() {
        let mut analyzer = CargoWorkspaceAnalyzer {
            root_dir: PathBuf::from("."),
            config: WorkspaceConfig::default(),
            expanded_members: Vec::new(),
            dependency_graph: HashMap::new(),
        };

        // A -> B -> C -> A (有循环)
        analyzer.dependency_graph.insert("A".to_string(), vec!["B".to_string()]);
        analyzer.dependency_graph.insert("B".to_string(), vec!["C".to_string()]);
        analyzer.dependency_graph.insert("C".to_string(), vec!["A".to_string()]);

        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        let cycle = analyzer.dfs_detect_cycle("A", &mut visited, &mut rec_stack, &mut path);
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert!(cycle.contains(&"A".to_string()));
        assert!(cycle.contains(&"B".to_string()));
        assert!(cycle.contains(&"C".to_string()));
    }
}
