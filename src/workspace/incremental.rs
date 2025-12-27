// Incremental Conversion Logic
// Handles file timestamp comparison and incremental conversion decisions

use std::fs;
use std::path::Path;
use std::time::SystemTime;

/// 增量转换决策
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionDecision {
    /// 需要转换（源文件更新或目标不存在）
    Convert,
    /// 跳过（目标文件更新）
    Skip,
    /// 强制转换
    Force,
}

/// 增量转换器
pub struct IncrementalConverter {
    /// 是否强制转换
    force: bool,
    /// 是否启用增量模式
    incremental: bool,
}

impl Default for IncrementalConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl IncrementalConverter {
    /// 创建新的增量转换器
    pub fn new() -> Self {
        Self {
            force: false,
            incremental: true,
        }
    }

    /// 设置强制模式
    pub fn force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    /// 设置增量模式
    pub fn incremental(mut self, incremental: bool) -> Self {
        self.incremental = incremental;
        self
    }

    /// 决定是否需要转换文件
    pub fn should_convert(&self, source: &Path, target: &Path) -> ConversionDecision {
        // 强制模式
        if self.force {
            return ConversionDecision::Force;
        }

        // 非增量模式，总是转换
        if !self.incremental {
            return ConversionDecision::Convert;
        }

        // 目标不存在，需要转换
        if !target.exists() {
            return ConversionDecision::Convert;
        }

        // 比较时间戳
        match (get_modified_time(source), get_modified_time(target)) {
            (Some(source_time), Some(target_time)) => {
                if source_time > target_time {
                    ConversionDecision::Convert
                } else {
                    ConversionDecision::Skip
                }
            }
            // 无法获取时间戳，默认转换
            _ => ConversionDecision::Convert,
        }
    }

    /// 批量检查需要转换的文件
    pub fn filter_files_to_convert<'a>(
        &self,
        files: &'a [(impl AsRef<Path>, impl AsRef<Path>)],
    ) -> Vec<(&'a Path, &'a Path, ConversionDecision)> {
        files
            .iter()
            .map(|(src, tgt)| {
                let decision = self.should_convert(src.as_ref(), tgt.as_ref());
                (src.as_ref(), tgt.as_ref(), decision)
            })
            .collect()
    }
}

/// 获取文件修改时间
fn get_modified_time(path: &Path) -> Option<SystemTime> {
    fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
}

/// 配置文件处理器
pub struct ConfigFileHandler;

impl ConfigFileHandler {
    /// 复制 .cargo/config.toml -> .nu/config.toml
    pub fn copy_cargo_config(src_dir: &Path, dst_dir: &Path) -> std::io::Result<bool> {
        let src_config = src_dir.join(".cargo").join("config.toml");
        let dst_config = dst_dir.join(".nu").join("config.toml");

        if src_config.exists() {
            fs::create_dir_all(dst_dir.join(".nu"))?;
            fs::copy(&src_config, &dst_config)?;
            return Ok(true);
        }

        // 也检查 config 文件（无扩展名）
        let src_config_alt = src_dir.join(".cargo").join("config");
        if src_config_alt.exists() {
            fs::create_dir_all(dst_dir.join(".nu"))?;
            fs::copy(&src_config_alt, &dst_config)?;
            return Ok(true);
        }

        Ok(false)
    }

    /// 复制 .nu/config.toml -> .cargo/config.toml
    pub fn copy_nu_config(src_dir: &Path, dst_dir: &Path) -> std::io::Result<bool> {
        let src_config = src_dir.join(".nu").join("config.toml");
        let dst_config = dst_dir.join(".cargo").join("config.toml");

        if src_config.exists() {
            fs::create_dir_all(dst_dir.join(".cargo"))?;
            fs::copy(&src_config, &dst_config)?;
            return Ok(true);
        }

        Ok(false)
    }

    /// 复制 rust-toolchain.toml
    pub fn copy_toolchain(src_dir: &Path, dst_dir: &Path) -> std::io::Result<bool> {
        let src_toolchain = src_dir.join("rust-toolchain.toml");
        let dst_toolchain = dst_dir.join("rust-toolchain.toml");

        if src_toolchain.exists() {
            fs::copy(&src_toolchain, &dst_toolchain)?;
            return Ok(true);
        }

        // 也检查无扩展名版本
        let src_toolchain_alt = src_dir.join("rust-toolchain");
        let dst_toolchain_alt = dst_dir.join("rust-toolchain");
        if src_toolchain_alt.exists() {
            fs::copy(&src_toolchain_alt, &dst_toolchain_alt)?;
            return Ok(true);
        }

        Ok(false)
    }

    /// 复制 Cargo.lock -> Nu.lock
    pub fn copy_lock_cargo_to_nu(src_dir: &Path, dst_dir: &Path) -> std::io::Result<bool> {
        let src_lock = src_dir.join("Cargo.lock");
        let dst_lock = dst_dir.join("Nu.lock");

        if src_lock.exists() {
            fs::copy(&src_lock, &dst_lock)?;
            return Ok(true);
        }

        Ok(false)
    }

    /// 复制 Nu.lock -> Cargo.lock
    pub fn copy_lock_nu_to_cargo(src_dir: &Path, dst_dir: &Path) -> std::io::Result<bool> {
        let src_lock = src_dir.join("Nu.lock");
        let dst_lock = dst_dir.join("Cargo.lock");

        if src_lock.exists() {
            fs::copy(&src_lock, &dst_lock)?;
            return Ok(true);
        }

        Ok(false)
    }

    /// 转换 .gitignore 中的扩展名（.rs -> .nu）
    pub fn convert_gitignore_cargo_to_nu(src_dir: &Path, dst_dir: &Path) -> std::io::Result<bool> {
        let src_gitignore = src_dir.join(".gitignore");
        let dst_gitignore = dst_dir.join(".gitignore");

        if src_gitignore.exists() {
            let content = fs::read_to_string(&src_gitignore)?;
            let converted = convert_gitignore_extensions(&content, ".rs", ".nu");
            fs::write(&dst_gitignore, converted)?;
            return Ok(true);
        }

        Ok(false)
    }

    /// 转换 .gitignore 中的扩展名（.nu -> .rs）
    pub fn convert_gitignore_nu_to_cargo(src_dir: &Path, dst_dir: &Path) -> std::io::Result<bool> {
        let src_gitignore = src_dir.join(".gitignore");
        let dst_gitignore = dst_dir.join(".gitignore");

        if src_gitignore.exists() {
            let content = fs::read_to_string(&src_gitignore)?;
            let converted = convert_gitignore_extensions(&content, ".nu", ".rs");
            fs::write(&dst_gitignore, converted)?;
            return Ok(true);
        }

        Ok(false)
    }

    /// 处理所有配置文件（Cargo -> Nu）
    pub fn process_all_cargo_to_nu(src_dir: &Path, dst_dir: &Path) -> Vec<String> {
        let mut processed = Vec::new();

        if Self::copy_cargo_config(src_dir, dst_dir).unwrap_or(false) {
            processed.push(".nu/config.toml".to_string());
        }
        if Self::copy_toolchain(src_dir, dst_dir).unwrap_or(false) {
            processed.push("rust-toolchain.toml".to_string());
        }
        if Self::copy_lock_cargo_to_nu(src_dir, dst_dir).unwrap_or(false) {
            processed.push("Nu.lock".to_string());
        }
        if Self::convert_gitignore_cargo_to_nu(src_dir, dst_dir).unwrap_or(false) {
            processed.push(".gitignore".to_string());
        }

        processed
    }

    /// 处理所有配置文件（Nu -> Cargo）
    pub fn process_all_nu_to_cargo(src_dir: &Path, dst_dir: &Path) -> Vec<String> {
        let mut processed = Vec::new();

        if Self::copy_nu_config(src_dir, dst_dir).unwrap_or(false) {
            processed.push(".cargo/config.toml".to_string());
        }
        if Self::copy_toolchain(src_dir, dst_dir).unwrap_or(false) {
            processed.push("rust-toolchain.toml".to_string());
        }
        if Self::copy_lock_nu_to_cargo(src_dir, dst_dir).unwrap_or(false) {
            processed.push("Cargo.lock".to_string());
        }
        if Self::convert_gitignore_nu_to_cargo(src_dir, dst_dir).unwrap_or(false) {
            processed.push(".gitignore".to_string());
        }

        processed
    }
}

/// 转换 gitignore 中的扩展名
fn convert_gitignore_extensions(content: &str, from_ext: &str, to_ext: &str) -> String {
    content
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            // 跳过注释和空行
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return line.to_string();
            }
            // 替换扩展名
            if trimmed.ends_with(from_ext) {
                let prefix = &trimmed[..trimmed.len() - from_ext.len()];
                format!("{}{}", prefix, to_ext)
            } else if trimmed.contains(&format!("*{}", from_ext)) {
                line.replace(&format!("*{}", from_ext), &format!("*{}", to_ext))
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_conversion_decision_force() {
        let converter = IncrementalConverter::new().force(true);
        let decision = converter.should_convert(Path::new("any"), Path::new("any"));
        assert_eq!(decision, ConversionDecision::Force);
    }

    #[test]
    fn test_conversion_decision_target_not_exists() {
        let converter = IncrementalConverter::new();
        let decision = converter.should_convert(
            Path::new("Cargo.toml"),
            Path::new("nonexistent_target.toml"),
        );
        assert_eq!(decision, ConversionDecision::Convert);
    }

    #[test]
    fn test_gitignore_conversion() {
        let input = r#"# Build artifacts
target/
*.rs.bk
*.rs
src/*.rs
"#;
        let expected = r#"# Build artifacts
target/
*.nu.bk
*.nu
src/*.nu"#;
        let result = convert_gitignore_extensions(input, ".rs", ".nu");
        assert_eq!(result.trim(), expected.trim());
    }

    #[test]
    fn test_gitignore_roundtrip() {
        let original = r#"# Ignore
*.rs
target/"#;
        let to_nu = convert_gitignore_extensions(original, ".rs", ".nu");
        let back = convert_gitignore_extensions(&to_nu, ".nu", ".rs");
        assert_eq!(original.trim(), back.trim());
    }

    #[test]
    fn test_incremental_with_timestamps() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("source.rs");
        let tgt = dir.path().join("target.nu");

        // 创建源文件
        File::create(&src).unwrap().write_all(b"source").unwrap();
        
        // 创建目标文件（稍后）
        std::thread::sleep(std::time::Duration::from_millis(10));
        File::create(&tgt).unwrap().write_all(b"target").unwrap();

        let converter = IncrementalConverter::new();
        
        // 目标更新，应该跳过
        let decision = converter.should_convert(&src, &tgt);
        assert_eq!(decision, ConversionDecision::Skip);

        // 更新源文件
        std::thread::sleep(std::time::Duration::from_millis(10));
        File::create(&src).unwrap().write_all(b"updated").unwrap();

        // 源更新，应该转换
        let decision = converter.should_convert(&src, &tgt);
        assert_eq!(decision, ConversionDecision::Convert);
    }
}
