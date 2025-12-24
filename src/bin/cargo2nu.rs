// Cargo Project to Nu Project Converter
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

fn convert_cargo_toml_to_nu_toml(cargo_content: &str) -> Result<String> {
    let mut nu_content = String::new();

    // 简单的行级转换
    for line in cargo_content.lines() {
        let line = line.trim();

        if line.starts_with("[package]") {
            nu_content.push_str("[P]\n");
        } else if line.starts_with("[dependencies]") {
            nu_content.push_str("[D]\n");
        } else if line.starts_with("[dev-dependencies]") {
            nu_content.push_str("[DD]\n");
        } else if line.starts_with("name ") {
            nu_content.push_str(&line.replace("name ", "id "));
            nu_content.push('\n');
        } else if line.starts_with("version ") {
            nu_content.push_str(&line.replace("version ", "v "));
            nu_content.push('\n');
        } else if line.starts_with("edition ") {
            nu_content.push_str(&line.replace("edition ", "ed "));
            nu_content.push('\n');
        } else {
            // 其他行（注释、空行、普通内容）保持原样
            nu_content.push_str(line);
            nu_content.push('\n');
        }
    }

    Ok(nu_content)
}

fn convert_project(input_dir: &Path, output_dir: &Path) -> Result<()> {
    // 创建输出目录
    fs::create_dir_all(output_dir)?;
    fs::create_dir_all(output_dir.join("src"))?;

    // 转换Cargo.toml -> Nu.toml
    let cargo_toml = input_dir.join("Cargo.toml");
    if cargo_toml.exists() {
        let cargo_content = fs::read_to_string(&cargo_toml)?;
        let nu_content = convert_cargo_toml_to_nu_toml(&cargo_content)?;
        fs::write(output_dir.join("Nu.toml"), nu_content)?;
        println!("✓ Nu.toml");
    }

    // 转换src/*.rs -> src/*.nu
    let src_dir = input_dir.join("src");
    if src_dir.exists() {
        for entry in fs::read_dir(&src_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                let file_name = path.file_stem().unwrap().to_str().unwrap();
                let output_path = output_dir.join("src").join(format!("{}.nu", file_name));

                // 使用rust2nu转换
                let rust_content = fs::read_to_string(&path)?;
                let converter = nu_compiler::rust2nu::Rust2NuConverter::new();
                let nu_content = converter.convert(&rust_content)?;

                fs::write(&output_path, nu_content)?;
                println!("✓ src/{}.nu", file_name);
            }
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("用法: cargo2nu <input_cargo_project> <output_nu_project>");
        eprintln!("示例: cargo2nu examples_project examples_nu_project");
        std::process::exit(1);
    }

    let input_dir = Path::new(&args[1]);
    let output_dir = Path::new(&args[2]);

    if !input_dir.exists() {
        eprintln!("错误: 输入目录不存在: {}", input_dir.display());
        std::process::exit(1);
    }

    println!("转换Cargo项目到Nu项目:");
    println!("  输入: {}", input_dir.display());
    println!("  输出: {}", output_dir.display());
    println!();

    convert_project(input_dir, output_dir).context("项目转换失败")?;

    println!();
    println!("✅ 转换完成!");

    Ok(())
}
