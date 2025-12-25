// Cargo Project to Nu Project Converter (with Workspace support)
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

fn convert_cargo_toml_to_nu_toml(cargo_content: &str) -> Result<String> {
    let mut nu_content = String::new();
    let mut in_workspace = false;
    let mut in_workspace_members = false;

    for line in cargo_content.lines() {
        let line = line.trim();

        // 检测workspace节
        if line.starts_with("[workspace]") {
            nu_content.push_str("[W]\n");
            in_workspace = true;
            continue;
        }

        // 检测workspace.members
        if in_workspace && line.starts_with("members ") {
            nu_content.push_str(&line.replace("members ", "m "));
            nu_content.push('\n');
            in_workspace_members = true;
            continue;
        }

        // 检测workspace结束
        if in_workspace && line.starts_with("[") && !line.starts_with("[workspace") {
            in_workspace = false;
            in_workspace_members = false;
        }

        // 常规转换
        if line.starts_with("[package]") {
            nu_content.push_str("[P]\n");
        } else if line.starts_with("[dependencies]") {
            nu_content.push_str("[D]\n");
        } else if line.starts_with("[dev-dependencies]") {
            nu_content.push_str("[DD]\n");
        } else if line.starts_with("[build-dependencies]") {
            nu_content.push_str("[BD]\n");
        } else if line.starts_with("name ") {
            nu_content.push_str(&line.replace("name ", "id "));
            nu_content.push('\n');
        } else if line.starts_with("version ") {
            nu_content.push_str(&line.replace("version ", "v "));
            nu_content.push('\n');
        } else if line.starts_with("edition ") {
            nu_content.push_str(&line.replace("edition ", "ed "));
            nu_content.push('\n');
        } else if in_workspace_members && !line.is_empty() {
            // workspace members内容保持不变
            nu_content.push_str(line);
            nu_content.push('\n');
        } else {
            // 其他行保持原样
            nu_content.push_str(line);
            nu_content.push('\n');
        }
    }

    Ok(nu_content)
}

fn is_workspace(cargo_toml_path: &Path) -> Result<bool> {
    let content = fs::read_to_string(cargo_toml_path)?;
    Ok(content.contains("[workspace]"))
}

fn get_workspace_members(cargo_toml_path: &Path) -> Result<Vec<String>> {
    let content = fs::read_to_string(cargo_toml_path)?;
    let mut members = Vec::new();
    let mut in_workspace = false;
    let mut in_members = false;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with("[workspace]") {
            in_workspace = true;
            continue;
        }

        if in_workspace && line.starts_with("[") && !line.starts_with("[workspace") {
            break;
        }

        if in_workspace && line.starts_with("members") {
            in_members = true;
            // 处理单行格式: members = ["member1", "member2"]
            if line.contains('[') && line.contains(']') {
                let start = line.find('[').unwrap();
                let end = line.find(']').unwrap();
                let members_str = &line[start + 1..end];
                for member in members_str.split(',') {
                    let member = member.trim().trim_matches('"').trim_matches('\'');
                    if !member.is_empty() {
                        members.push(member.to_string());
                    }
                }
                in_members = false;
            }
            continue;
        }

        if in_members {
            // 处理多行格式
            if line.contains(']') {
                in_members = false;
                continue;
            }
            let member = line.trim_matches(|c| c == '"' || c == '\'' || c == ',' || c == ' ');
            if !member.is_empty() && !member.starts_with('#') {
                members.push(member.to_string());
            }
        }
    }

    Ok(members)
}

fn convert_project(input_dir: &Path, output_dir: &Path) -> Result<()> {
    // 创建输出目录
    fs::create_dir_all(output_dir)?;

    let cargo_toml = input_dir.join("Cargo.toml");

    // 检查是否为workspace
    if cargo_toml.exists() && is_workspace(&cargo_toml)? {
        println!("检测到Workspace结构");

        // 转换根Cargo.toml -> Nu.toml
        let cargo_content = fs::read_to_string(&cargo_toml)?;
        let nu_content = convert_cargo_toml_to_nu_toml(&cargo_content)?;
        fs::write(output_dir.join("Nu.toml"), nu_content)?;
        println!("✓ Nu.toml (workspace根)");

        // 获取workspace成员
        let members = get_workspace_members(&cargo_toml)?;
        println!("找到 {} 个workspace成员", members.len());

        // 递归转换每个成员
        for member in members {
            let member_input = input_dir.join(&member);
            let member_output = output_dir.join(&member);

            if member_input.exists() {
                println!("\n转换成员: {}", member);
                convert_single_project(&member_input, &member_output)?;
            } else {
                println!("⚠ 警告: 成员目录不存在: {}", member_input.display());
            }
        }
    } else {
        // 单个项目转换
        convert_single_project(input_dir, output_dir)?;
    }

    Ok(())
}

fn convert_single_project(input_dir: &Path, output_dir: &Path) -> Result<()> {
    // 创建输出目录
    fs::create_dir_all(output_dir)?;
    fs::create_dir_all(output_dir.join("src"))?;

    // 转换Cargo.toml -> Nu.toml
    let cargo_toml = input_dir.join("Cargo.toml");
    if cargo_toml.exists() {
        let cargo_content = fs::read_to_string(&cargo_toml)?;
        let nu_content = convert_cargo_toml_to_nu_toml(&cargo_content)?;
        fs::write(output_dir.join("Nu.toml"), nu_content)?;
        println!("  ✓ Nu.toml");
    }

    // 转换src/*.rs -> src/*.nu (递归处理子目录)
    let src_dir = input_dir.join("src");
    if src_dir.exists() {
        convert_rust_files_recursive(&src_dir, &output_dir.join("src"), &src_dir)?;
    }

    Ok(())
}

/// 递归转换目录中的所有Rust文件
fn convert_rust_files_recursive(
    src_dir: &Path,
    output_dir: &Path,
    base_src_dir: &Path,
) -> Result<()> {
    // 创建输出目录
    fs::create_dir_all(output_dir)?;

    for entry in fs::read_dir(src_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // 递归处理子目录
            let dir_name = path.file_name().unwrap();
            let sub_output_dir = output_dir.join(dir_name);
            convert_rust_files_recursive(&path, &sub_output_dir, base_src_dir)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            // 转换.rs文件
            let file_name = path.file_stem().unwrap().to_str().unwrap();
            let output_path = output_dir.join(format!("{}.nu", file_name));

            // 使用rust2nu转换
            let rust_content = fs::read_to_string(&path)?;
            let converter = nu_compiler::rust2nu::Rust2NuConverter::new();
            let nu_content = converter.convert(&rust_content)?;

            fs::write(&output_path, nu_content)?;

            // 计算相对路径用于显示
            let relative_path = path.strip_prefix(base_src_dir).unwrap_or(&path);
            let nu_relative = relative_path.with_extension("nu");
            println!("  ✓ src/{}", nu_relative.display());
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("用法: cargo2nu <input_cargo_project> <output_nu_project>");
        eprintln!("示例: cargo2nu examples_project examples_nu_project");
        eprintln!("支持: 单项目和Workspace项目");
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
