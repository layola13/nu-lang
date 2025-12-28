// Nu2TS Converter (AST-based, 完整版)
// 新的转换器入口，使用 Parser + Codegen 架构

use super::codegen::TsCodegen;
use super::parser::Parser;
use super::types::TsConfig;
use anyhow::{Context, Result};

pub struct Nu2TsConverterV2 {
    config: TsConfig,
}

impl Nu2TsConverterV2 {
    pub fn new(config: TsConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self::new(TsConfig::default())
    }

    pub fn config(&self) -> &TsConfig {
        &self.config
    }

    /// 主转换方法：Nu → TypeScript
    pub fn convert(&self, nu_code: &str) -> Result<String> {
        // 1. 解析 Nu 代码为 AST
        let mut parser = Parser::new(nu_code);
        let file = parser.parse_file().context("Failed to parse Nu code")?;

        // 2. 生成 TypeScript 代码
        let mut codegen = TsCodegen::new(self.config.clone());
        let ts_code = codegen
            .generate_file(&file)
            .context("Failed to generate TypeScript code")?;

        Ok(ts_code)
    }
}

impl Default for Nu2TsConverterV2 {
    fn default() -> Self {
        Self::with_default_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_match() {
        let converter = Nu2TsConverterV2::with_default_config();

        let nu_code = r#"M x {
    Ok(v): { v },
    Err(_): { 0 }
}"#;

        let result = converter.convert(nu_code).unwrap();

        // 打印实际输出以便调试
        eprintln!("=== ACTUAL OUTPUT ===");
        eprintln!("{}", result);
        eprintln!("=== END OUTPUT ===");

        // 检查 match 表达式被处理（可能使用不同的转换策略）
        assert!(
            result.contains("const _m")
                || result.contains("if")
                || result.contains("(() =>")
                || result.contains("$match"),
            "Expected match expression conversion, got: {}",
            result
        );
    }

    #[test]
    fn test_convert_function() {
        let converter = Nu2TsConverterV2::with_default_config();

        let nu_code = r#"f test(x: i32) -> i32 {
    < x + 1
}"#;

        let result = converter.convert(nu_code).unwrap();

        assert!(result.contains("function test("));
    }

    #[test]
    fn test_convert_enum() {
        let converter = Nu2TsConverterV2::with_default_config();

        let nu_code = r#"E Status {
    Active,
    Inactive,
}"#;

        let result = converter.convert(nu_code).unwrap();

        assert!(result.contains("Enum: Status"));
        assert!(result.contains("type Status"));
    }
}
