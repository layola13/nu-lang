// Nu Project to Cargo Project Converter (with Workspace support)
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

fn convert_nu_toml_to_cargo_toml(nu_content: &str) -> Result<String> {
    let mut cargo_content = String::new();
    let mut in_workspace = false;
    let mut in_workspace_members = false;

    for line in nu_content.lines() {
        let line = line.trim();

        // 检测workspace节
        if line.starts_with("[W]") {
            cargo_content.push_str("[workspace]\n");
            in_workspace = true;
            continue;
        }

        // 检测workspace.members
        if in_workspace && line.starts_with("m ") {
            cargo_content.push_str(&line.replace("m ", "members "));
            cargo_content.push('\n');
            in_workspace_members = true;
            continue;
        }

        // 检测workspace结束
        if in_workspace && line.starts_with("[") && !line.starts_with("[W") {
            in_workspace = false;
            in_workspace_members = false;
        }

        // 常规转换
        if line.starts_with("[P]") {
            cargo_content.push_str("[package]\n");
        } else if line == "[D]" {
            cargo_content.push_str("[dependencies]\n");
        } else if line.starts_with("[DD]") {
            cargo_content.push_str("[dev-dependencies]\n");
        } else if line.starts_with("[BD]") {
            cargo_content.push_str("[build-dependencies]\n");
        } else if line.starts_with("id ") {
            cargo_content.push_str(&line.replace("id ", "name "));
            cargo_content.push('\n');
        } else if line.starts_with("v ") {
            cargo_content.push_str(&line.replace("v ", "version "));
            cargo_content.push('\n');
        } else if line.starts_with("ed ") {
            cargo_content.push_str(&line.replace("ed ", "edition "));
            cargo_content.push('\n');
        } else if in_workspace_members && !line.is_empty() {
            // workspace members内容保持不变
            cargo_content.push_str(line);
            cargo_content.push('\n');
        } else {
            // 其他行保持原样
            cargo_content.push_str(line);
            cargo_content.push('\n');
        }
    }

    Ok(cargo_content)
}

fn is_workspace(nu_toml_path: &Path) -> Result<bool> {
    let content = fs::read_to_string(nu_toml_path)?;
    Ok(content.contains("[W]"))
}

fn get_workspace_members(nu_toml_path: &Path) -> Result<Vec<String>> {
    let content = fs::read_to_string(nu_toml_path)?;
    let mut members = Vec::new();
    let mut in_workspace = false;
    let mut in_members = false;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with("[W]") {
            in_workspace = true;
            continue;
        }

        if in_workspace && line.starts_with("[") && !line.starts_with("[W") {
            break;
        }

        if in_workspace && line.starts_with("m ") {
            in_members = true;
            // 处理单行格式: m = ["member1", "member2"]
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

    let nu_toml = input_dir.join("Nu.toml");

    // 检查是否为workspace
    if nu_toml.exists() && is_workspace(&nu_toml)? {
        println!("检测到Workspace结构");

        // 转换根Nu.toml -> Cargo.toml
        let nu_content = fs::read_to_string(&nu_toml)?;
        let cargo_content = convert_nu_toml_to_cargo_toml(&nu_content)?;
        fs::write(output_dir.join("Cargo.toml"), cargo_content)?;
        println!("✓ Cargo.toml (workspace根)");

        // 获取workspace成员
        let members = get_workspace_members(&nu_toml)?;
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

    // 转换Nu.toml -> Cargo.toml
    let nu_toml = input_dir.join("Nu.toml");
    if nu_toml.exists() {
        let nu_content = fs::read_to_string(&nu_toml)?;
        let cargo_content = convert_nu_toml_to_cargo_toml(&nu_content)?;
        fs::write(output_dir.join("Cargo.toml"), cargo_content)?;
        println!("  ✓ Cargo.toml");
    }

    // 转换src/*.nu -> src/*.rs (递归处理子目录)
    let src_dir = input_dir.join("src");
    if src_dir.exists() {
        convert_nu_files_recursive(&src_dir, &output_dir.join("src"), &src_dir)?;
    }

    Ok(())
}

/// 递归转换目录中的所有Nu文件
fn convert_nu_files_recursive(
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
            convert_nu_files_recursive(&path, &sub_output_dir, base_src_dir)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("nu") {
            // 转换.nu文件
            let file_name = path.file_stem().unwrap().to_str().unwrap();
            let output_path = output_dir.join(format!("{}.rs", file_name));

            // 使用nu2rust转换
            let nu_content = fs::read_to_string(&path)?;
            let converter = nu_compiler::nu2rust::Nu2RustConverter::new();
            let rust_content = converter.convert(&nu_content)?;

            fs::write(&output_path, rust_content)?;

            // 计算相对路径用于显示
            let relative_path = path.strip_prefix(base_src_dir).unwrap_or(&path);
            let rs_relative = relative_path.with_extension("rs");
            println!("  ✓ src/{}", rs_relative.display());
        }
    }

    Ok(())
}

const ASCII_LOGO: &str = r#"
   _   __          __
  / | / /_  __    / /___ _____  ____ _
 /  |/ / / / /___/ / __ `/ __ \/ __ `/
/ /|  / /_/ /___/ / /_/ / / / / /_/ /
/_/ |_/\__,_/   /_/\__,_/_/ /_/\__, /
                              /____/
Nu-lang: Rust, Condensed. v1.6.5
"#;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        println!("{}", ASCII_LOGO);
        eprintln!("用法: nu2cargo <input_nu_project> <output_cargo_project>");
        eprintln!("示例: nu2cargo examples_nu_project examples_cargo_restored");
        eprintln!("支持: 单项目和Workspace项目");
        std::process::exit(1);
    }
    
    // 显示ASCII Logo
    println!("{}", ASCII_LOGO);

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
