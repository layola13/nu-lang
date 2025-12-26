// SourceMap Generator for Nu to Rust Conversion
// Phase 1: LazySourceMap - Line-to-line mapping only

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// LazySourceMap 结构体 - Phase 1 实现
/// 只存储行号映射，不包含列信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LazySourceMap {
    /// 源文件名（Nu 文件）
    pub nu_file: String,
    
    /// 目标文件名（Rust 文件）
    pub rust_file: String,
    
    /// 行号映射表：(rust_line, nu_line)
    /// 按 rust_line 排序，用于二分查找
    pub line_map: Vec<(usize, usize)>,
}

impl LazySourceMap {
    /// 创建新的 SourceMap
    pub fn new(nu_file: String, rust_file: String) -> Self {
        Self {
            nu_file,
            rust_file,
            line_map: Vec::new(),
        }
    }
    
    /// 添加行号映射
    /// 
    /// # Arguments
    /// * `rust_line` - 生成的 Rust 代码行号（1-based）
    /// * `nu_line` - 对应的 Nu 源代码行号（1-based）
    pub fn add_mapping(&mut self, rust_line: usize, nu_line: usize) {
        // 避免重复添加相同的映射
        if let Some(&(last_rust, last_nu)) = self.line_map.last() {
            if last_rust == rust_line && last_nu == nu_line {
                return;
            }
        }
        
        self.line_map.push((rust_line, nu_line));
    }
    
    /// 查找最接近的 Nu 行号
    /// 使用二分查找算法，时间复杂度 O(log n)
    /// 
    /// # Arguments
    /// * `rust_line` - Rust 代码行号
    /// 
    /// # Returns
    /// 对应的 Nu 源代码行号（如果存在）
    pub fn find_nearest_nu_line(&self, rust_line: usize) -> Option<usize> {
        if self.line_map.is_empty() {
            return None;
        }
        
        // 使用二分查找
        match self.line_map.binary_search_by(|probe| probe.0.cmp(&rust_line)) {
            // 精确匹配
            Ok(idx) => Some(self.line_map[idx].1),
            // 未精确匹配，找到插入位置
            Err(0) => {
                // rust_line 小于所有已知映射，返回第一个映射
                Some(self.line_map[0].1)
            }
            Err(idx) => {
                // 返回前一个映射（最接近且不超过的）
                Some(self.line_map[idx - 1].1)
            }
        }
    }
    
    /// 序列化为 JSON 字符串
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
    
    /// 从 JSON 字符串反序列化
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
    
    /// 保存到文件
    /// 
    /// # Arguments
    /// * `path` - 输出文件路径（通常是 .rs.map）
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = self.to_json()?;
        fs::write(path, json)?;
        Ok(())
    }
    
    /// 从文件加载
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let json = fs::read_to_string(path)?;
        Self::from_json(&json)
    }
    
    /// 获取映射数量
    pub fn mapping_count(&self) -> usize {
        self.line_map.len()
    }
    
    /// 清空所有映射
    pub fn clear(&mut self) {
        self.line_map.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_sourcemap() {
        let sm = LazySourceMap::new("test.nu".to_string(), "test.rs".to_string());
        assert_eq!(sm.nu_file, "test.nu");
        assert_eq!(sm.rust_file, "test.rs");
        assert_eq!(sm.line_map.len(), 0);
    }
    
    #[test]
    fn test_add_mapping() {
        let mut sm = LazySourceMap::new("test.nu".to_string(), "test.rs".to_string());
        sm.add_mapping(1, 1);
        sm.add_mapping(5, 3);
        sm.add_mapping(10, 7);
        
        assert_eq!(sm.line_map.len(), 3);
        assert_eq!(sm.line_map[0], (1, 1));
        assert_eq!(sm.line_map[1], (5, 3));
        assert_eq!(sm.line_map[2], (10, 7));
    }
    
    #[test]
    fn test_find_nearest_nu_line() {
        let mut sm = LazySourceMap::new("test.nu".to_string(), "test.rs".to_string());
        sm.add_mapping(1, 1);
        sm.add_mapping(5, 3);
        sm.add_mapping(10, 7);
        
        // 精确匹配
        assert_eq!(sm.find_nearest_nu_line(1), Some(1));
        assert_eq!(sm.find_nearest_nu_line(5), Some(3));
        assert_eq!(sm.find_nearest_nu_line(10), Some(7));
        
        // 查找最接近的
        assert_eq!(sm.find_nearest_nu_line(3), Some(1)); // 在1和5之间，返回1
        assert_eq!(sm.find_nearest_nu_line(7), Some(3)); // 在5和10之间，返回5对应的3
        assert_eq!(sm.find_nearest_nu_line(15), Some(7)); // 超过最大值，返回最后一个
    }
    
    #[test]
    fn test_json_serialization() {
        let mut sm = LazySourceMap::new("test.nu".to_string(), "test.rs".to_string());
        sm.add_mapping(1, 1);
        sm.add_mapping(5, 3);
        
        let json = sm.to_json().unwrap();
        assert!(json.contains("\"nu_file\""));
        assert!(json.contains("\"rust_file\""));
        assert!(json.contains("\"line_map\""));
        
        let sm2 = LazySourceMap::from_json(&json).unwrap();
        assert_eq!(sm2.nu_file, sm.nu_file);
        assert_eq!(sm2.rust_file, sm.rust_file);
        assert_eq!(sm2.line_map, sm.line_map);
    }
    
    #[test]
    fn test_avoid_duplicate_mappings() {
        let mut sm = LazySourceMap::new("test.nu".to_string(), "test.rs".to_string());
        sm.add_mapping(1, 1);
        sm.add_mapping(1, 1); // 重复
        sm.add_mapping(1, 1); // 重复
        
        assert_eq!(sm.line_map.len(), 1);
    }
}