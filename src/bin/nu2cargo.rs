// Nu Project to Cargo Project Converter
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

fn convert_nu_toml_to_cargo_toml(nu_content: &str) -> Result<String> {
    let mut cargo_content = String::new();

    // 简单的行级转换
    for line in nu_content.lines() {
        let line = line.trim();

        if line.starts_with("[P]") {
            cargo_content.push_str("[package]\n");
        } else if line == "[D]" {
            cargo_content.push_str("[dependencies]\n");
        } else if line.starts_with("[DD]") {
            cargo_content.push_str("[dev-dependencies]\n");
        } else if line.starts_with("id ") {
            cargo_content.push_str(&line.replace("id ", "name "));
            cargo_content.push('\n');
        } else if line.starts_with("v ") {
            cargo_content.push_str(&line.replace("v ", "version "));
            cargo_content.push('\n');
        } else if line.starts_with("ed ") {
            cargo_content.push_str(&line.replace("ed ", "edition "));
            cargo_content.push('\n');
        } else if !line.is_empty() && !line.starts_with("#") {
            cargo_content.push_str(line);
            cargo_content.push('\n');
        } else if line.starts_with("#") {
            cargo_content.push_str(line);
            cargo_content.push('\n');
        } else if line.is_empty() {
            cargo_content.push('\n');
        }
    }

    Ok(cargo_content)
}

fn convert_project(input_dir: &Path, output_dir: &Path) -> Result<()> {
    // 创建输出目录
    fs::create_dir_all(output_dir)?;
    fs::create_dir_all(output_dir.join("src"))?;

    // 转换Nu.toml -> Cargo.toml
    let nu_toml = input_dir.join("Nu.toml");
    if nu_toml.exists() {
        let nu_content = fs::read_to_string(&nu_toml)?;
        let cargo_content = convert_nu_toml_to_cargo_toml(&nu_content)?;
        fs::write(output_dir.join("Cargo.toml"), cargo_content)?;
        println!("✓ Cargo.toml");
    }

    // 转换src/*.nu -> src/*.rs
    let src_dir = input_dir.join("src");
    if src_dir.exists() {
        for entry in fs::read_dir(&src_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("nu") {
                let file_name = path.file_stem().unwrap().to_str().unwrap();
                let output_path = output_dir.join("src").join(format!("{}.rs", file_name));

                // 使用nu2rust转换
                let nu_content = fs::read_to_string(&path)?;
                let converter = nu_compiler::nu2rust::Nu2RustConverter::new();
                let rust_content = converter.convert(&nu_content)?;

                fs::write(&output_path, rust_content)?;
                println!("✓ src/{}.rs", file_name);
            }
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("用法: nu2cargo <input_nu_project> <output_cargo_project>");
        eprintln!("示例: nu2cargo examples_nu_project examples_cargo_restored");
        std::process::exit(1);
    }

    let input_dir = Path::new(&args[1]);
    let output_dir = Path::new(&args[2]);

    if !input_dir.exists() {
        eprintln!("错误: 输入目录不存在: {}", input_dir.display());
        std::process::exit(1);
    }

    println!("转换Nu项目到Cargo项目:");
    println!("  输入: {}", input_dir.display());
    println!("  输出: {}", output_dir.display());
    println!();

    convert_project(input_dir, output_dir).context("项目转换失败")?;

    println!();
    println!("✅ 转换完成!");

    Ok(())
}
