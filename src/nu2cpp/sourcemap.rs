// C++ Source Map Support
// 用于Nu到C++转换的源码映射

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMap {
    pub source_file: String,
    pub target_file: String,
    pub mappings: Vec<LineMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineMapping {
    pub cpp_line: usize,
    pub nu_line: usize,
}

impl SourceMap {
    pub fn new(source_file: String, target_file: String) -> Self {
        Self {
            source_file,
            target_file,
            mappings: Vec::new(),
        }
    }

    pub fn add_mapping(&mut self, cpp_line: usize, nu_line: usize) {
        self.mappings.push(LineMapping { cpp_line, nu_line });
    }

    pub fn mapping_count(&self) -> usize {
        self.mappings.len()
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let json = fs::read_to_string(path)?;
        let map = serde_json::from_str(&json)?;
        Ok(map)
    }

    pub fn find_nu_line(&self, cpp_line: usize) -> Option<usize> {
        self.mappings
            .iter()
            .find(|m| m.cpp_line == cpp_line)
            .map(|m| m.nu_line)
    }

    pub fn find_cpp_line(&self, nu_line: usize) -> Option<usize> {
        self.mappings
            .iter()
            .find(|m| m.nu_line == nu_line)
            .map(|m| m.cpp_line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sourcemap_basic() {
        let mut map = SourceMap::new("test.nu".to_string(), "test.cpp".to_string());
        
        map.add_mapping(1, 1);
        map.add_mapping(2, 2);
        map.add_mapping(5, 3);
        
        assert_eq!(map.mapping_count(), 3);
        assert_eq!(map.find_nu_line(5), Some(3));
        assert_eq!(map.find_cpp_line(3), Some(5));
    }
}