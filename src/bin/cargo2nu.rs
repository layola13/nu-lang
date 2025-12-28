// Cargo Project to Nu Project Converter (with Full Workspace support)
// v2.0 - Uses workspace module for complete TOML conversion

use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::Path;

use nu_compiler::workspace::{
    Cargo2NuConverter, CargoWorkspaceAnalyzer, WorkspaceType, ConvertReport,
    ConfigFileHandler, IncrementalConverter, ConversionDecision,
};

/// Cargo 项目到 Nu 项目转换器
#[derive(Parser, Debug)]
#[command(name = "cargo2nu")]
#[command(author = "Nu Language Team")]
#[command(version = "2.0")]
#[command(about = "转换 Cargo 项目到 Nu 项目（支持 Workspace）")]
struct Args {
    /// 输入 Cargo 项目目录
    input: String,

    /// 输出 Nu 项目目录
    output: String,

    /// 详细输出
    #[arg(short, long)]
    verbose: bool,

    /// 仅预览，不实际写入文件
    #[arg(long)]
    dry_run: bool,

    /// 增量转换（仅转换更新的文件）
    #[arg(short, long)]
    incremental: bool,

    /// 强制覆盖（忽略时间戳）
    #[arg(short, long)]
    force: bool,

    /// 排除指定成员（可多次使用）
    #[arg(long, value_name = "MEMBER")]
    exclude: Vec<String>,

    /// 仅转换指定成员（可多次使用）
    #[arg(long, value_name = "MEMBER")]
    only: Vec<String>,
}

/// 转换 Cargo.toml 到 Nu.toml
fn convert_cargo_toml_to_nu_toml(cargo_content: &str) -> String {
    let converter = Cargo2NuConverter::new();
    converter.convert(cargo_content)
}

/// 检查是否为 workspace
fn is_workspace(cargo_toml_path: &Path) -> Result<bool> {
    let content = fs::read_to_string(cargo_toml_path)?;
    Ok(WorkspaceType::from_cargo_toml(&content).is_workspace())
}

/// 获取 workspace 成员（使用分析器）
fn get_workspace_members(cargo_toml_path: &Path) -> Result<Vec<String>> {
    let root_dir = cargo_toml_path.parent().unwrap_or(Path::new("."));
    let mut analyzer = CargoWorkspaceAnalyzer::from_dir(root_dir)?;
    
    let members = analyzer.expand_members()?;
    Ok(members.iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect())
}

/// 过滤成员列表
fn filter_members(members: Vec<String>, exclude: &[String], only: &[String]) -> Vec<String> {
    members.into_iter()
        .filter(|m| {
            // 如果指定了 only，只保留匹配的
            if !only.is_empty() {
                return only.iter().any(|o| m.contains(o));
            }
            // 排除匹配 exclude 的
            !exclude.iter().any(|e| m.contains(e))
        })
        .collect()
}

/// 转换整个项目
fn convert_project(args: &Args) -> Result<ConvertReport> {
    let input_dir = Path::new(&args.input);
    let output_dir = Path::new(&args.output);
    
    if !args.dry_run {
        fs::create_dir_all(output_dir)?;
    }

    let cargo_toml = input_dir.join("Cargo.toml");
    let mut report = ConvertReport::new(WorkspaceType::Single);

    let incremental = IncrementalConverter::new()
        .force(args.force)
        .incremental(args.incremental);

    if cargo_toml.exists() && is_workspace(&cargo_toml)? {
        if args.verbose {
            println!("检测到 Workspace 结构");
        }
        report.workspace_type = WorkspaceType::from_cargo_toml(&fs::read_to_string(&cargo_toml)?);

        // 转换根 Cargo.toml -> Nu.toml
        let target_toml = output_dir.join("Nu.toml");
        let decision = incremental.should_convert(&cargo_toml, &target_toml);
        
        if decision != ConversionDecision::Skip {
            let cargo_content = fs::read_to_string(&cargo_toml)?;
            let nu_content = convert_cargo_toml_to_nu_toml(&cargo_content);
            
            if args.dry_run {
                println!("[dry-run] 将创建: Nu.toml");
            } else {
                fs::write(&target_toml, nu_content)?;
                println!("✓ Nu.toml (workspace 根)");
            }
            report.files_converted += 1;
        } else if args.verbose {
            println!("⊘ Nu.toml (跳过，未更新)");
            report.files_skipped += 1;
        }

        // 如果是 Mixed workspace（根目录也是 package），转换根目录的 src/
        if report.workspace_type == WorkspaceType::Mixed {
            let root_src = input_dir.join("src");
            if root_src.exists() {
                if args.verbose {
                    println!("\n转换根 package 的 src/ 目录");
                }
                if !args.dry_run {
                    fs::create_dir_all(output_dir.join("src"))?;
                }
                let (c, s, f) = convert_rust_files_recursive(&root_src, &output_dir.join("src"), &root_src, args, &incremental)?;
                report.files_converted += c;
                report.files_skipped += s;
                report.files_failed += f;
            }
            
            // 转换根目录的 examples/
            let root_examples = input_dir.join("examples");
            if root_examples.exists() {
                if args.verbose {
                    println!("\n转换根 package 的 examples/ 目录");
                }
                if !args.dry_run {
                    fs::create_dir_all(output_dir.join("examples"))?;
                }
                let (c, s, f) = convert_rust_files_recursive(&root_examples, &output_dir.join("examples"), &root_examples, args, &incremental)?;
                report.files_converted += c;
                report.files_skipped += s;
                report.files_failed += f;
            }
            
            // 转换根目录的 tests/
            let root_tests = input_dir.join("tests");
            if root_tests.exists() {
                if args.verbose {
                    println!("\n转换根 package 的 tests/ 目录");
                }
                if !args.dry_run {
                    fs::create_dir_all(output_dir.join("tests"))?;
                }
                let (c, s, f) = convert_rust_files_recursive(&root_tests, &output_dir.join("tests"), &root_tests, args, &incremental)?;
                report.files_converted += c;
                report.files_skipped += s;
                report.files_failed += f;
            }
            
            // 转换根目录的 build.rs
            let build_rs = input_dir.join("build.rs");
            let build_nu = output_dir.join("build.nu");
            if build_rs.exists() {
                let decision = incremental.should_convert(&build_rs, &build_nu);
                if decision != ConversionDecision::Skip {
                    match fs::read_to_string(&build_rs) {
                        Ok(rust_content) => {
                            let converter = nu_compiler::rust2nu::Rust2NuConverter::new();
                            match converter.convert(&rust_content) {
                                Ok(nu_content) => {
                                    if args.dry_run {
                                        println!("[dry-run] 将创建: build.nu");
                                    } else {
                                        fs::write(&build_nu, nu_content)?;
                                        println!("  ✓ build.nu");
                                    }
                                    report.files_converted += 1;
                                }
                                Err(e) => {
                                    eprintln!("  ✗ build.rs 转换失败: {}", e);
                                    report.files_failed += 1;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("  ✗ 无法读取 build.rs: {}", e);
                            report.files_failed += 1;
                        }
                    }
                }
            }
        }

        // 获取并过滤成员
        let members = get_workspace_members(&cargo_toml)?;
        let filtered_members = filter_members(members, &args.exclude, &args.only);
        report.members_total = filtered_members.len();
        
        if args.verbose {
            println!("找到 {} 个 workspace 成员", filtered_members.len());
        }

        for member in filtered_members {
            let member_input = input_dir.join(&member);
            let member_output = output_dir.join(&member);

            if member_input.exists() {
                if args.verbose {
                    println!("\n转换成员: {}", member);
                }
                match convert_single_project(&member_input, &member_output, args, &incremental) {
                    Ok((converted, skipped, failed)) => {
                        report.members_converted += 1;
                        report.files_converted += converted;
                        report.files_skipped += skipped;
                        report.files_failed += failed;
                    }
                    Err(e) => {
                        report.add_warning(format!("成员 {} 转换失败: {}", member, e));
                    }
                }
            } else {
                report.add_warning(format!("成员目录不存在: {}", member_input.display()));
            }
        }

        // 处理配置文件
        if !args.dry_run {
            let config_files = ConfigFileHandler::process_all_cargo_to_nu(input_dir, output_dir);
            for f in config_files {
                if args.verbose {
                    println!("✓ {}", f);
                }
            }
        }
    } else {
        // 单个项目转换
        let (converted, skipped, failed) = convert_single_project(input_dir, output_dir, args, &incremental)?;
        report.files_converted = converted;
        report.files_skipped = skipped;
        report.files_failed = failed;

        // 处理配置文件
        if !args.dry_run {
            ConfigFileHandler::process_all_cargo_to_nu(input_dir, output_dir);
        }
    }

    Ok(report)
}

/// 转换单个项目，返回 (converted, skipped, failed)
fn convert_single_project(
    input_dir: &Path, 
    output_dir: &Path, 
    args: &Args,
    incremental: &IncrementalConverter,
) -> Result<(usize, usize, usize)> {
    let mut converted = 0;
    let mut skipped = 0;
    let mut failed = 0;

    if !args.dry_run {
        fs::create_dir_all(output_dir)?;
        fs::create_dir_all(output_dir.join("src"))?;
    }

    // 转换 Cargo.toml -> Nu.toml
    let cargo_toml = input_dir.join("Cargo.toml");
    let nu_toml = output_dir.join("Nu.toml");
    
    if cargo_toml.exists() {
        let decision = incremental.should_convert(&cargo_toml, &nu_toml);
        if decision != ConversionDecision::Skip {
            let cargo_content = fs::read_to_string(&cargo_toml)?;
            let nu_content = convert_cargo_toml_to_nu_toml(&cargo_content);
            
            if args.dry_run {
                println!("[dry-run]   将创建: Nu.toml");
            } else {
                fs::write(&nu_toml, nu_content)?;
                println!("  ✓ Nu.toml");
            }
            converted += 1;
        } else {
            skipped += 1;
        }
    }

    // 转换 build.rs -> build.nu
    let build_rs = input_dir.join("build.rs");
    let build_nu = output_dir.join("build.nu");
    
    if build_rs.exists() {
        let decision = incremental.should_convert(&build_rs, &build_nu);
        if decision != ConversionDecision::Skip {
            match fs::read_to_string(&build_rs) {
                Ok(rust_content) => {
                    let converter = nu_compiler::rust2nu::Rust2NuConverter::new();
                    match converter.convert(&rust_content) {
                        Ok(nu_content) => {
                            if args.dry_run {
                                println!("[dry-run]   将创建: build.nu");
                            } else {
                                fs::write(&build_nu, nu_content)?;
                                println!("  ✓ build.nu");
                            }
                            converted += 1;
                        }
                        Err(e) => {
                            eprintln!("  ✗ build.rs 转换失败: {}", e);
                            failed += 1;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("  ✗ 无法读取 build.rs: {}", e);
                    failed += 1;
                }
            }
        } else {
            skipped += 1;
        }
    }

    // 转换 src/*.rs -> src/*.nu
    let src_dir = input_dir.join("src");
    if src_dir.exists() {
        let (c, s, f) = convert_rust_files_recursive(&src_dir, &output_dir.join("src"), &src_dir, args, incremental)?;
        converted += c;
        skipped += s;
        failed += f;
    }

    // 转换 tests 目录
    let tests_dir = input_dir.join("tests");
    if tests_dir.exists() {
        if !args.dry_run {
            fs::create_dir_all(output_dir.join("tests"))?;
        }
        let (c, s, f) = convert_rust_files_recursive(&tests_dir, &output_dir.join("tests"), &tests_dir, args, incremental)?;
        converted += c;
        skipped += s;
        failed += f;
    }

    // 转换 examples 目录
    let examples_dir = input_dir.join("examples");
    if examples_dir.exists() {
        if !args.dry_run {
            fs::create_dir_all(output_dir.join("examples"))?;
        }
        let (c, s, f) = convert_rust_files_recursive(&examples_dir, &output_dir.join("examples"), &examples_dir, args, incremental)?;
        converted += c;
        skipped += s;
        failed += f;
    }

    // 转换 benches 目录
    let benches_dir = input_dir.join("benches");
    if benches_dir.exists() {
        if !args.dry_run {
            fs::create_dir_all(output_dir.join("benches"))?;
        }
        let (c, s, f) = convert_rust_files_recursive(&benches_dir, &output_dir.join("benches"), &benches_dir, args, incremental)?;
        converted += c;
        skipped += s;
        failed += f;
    }

    Ok((converted, skipped, failed))
}

/// 递归转换目录中的所有 Rust 文件
fn convert_rust_files_recursive(
    src_dir: &Path,
    output_dir: &Path,
    base_src_dir: &Path,
    args: &Args,
    incremental: &IncrementalConverter,
) -> Result<(usize, usize, usize)> {
    let mut converted = 0;
    let mut skipped = 0;
    let mut failed = 0;

    if !args.dry_run {
        fs::create_dir_all(output_dir)?;
    }

    for entry in fs::read_dir(src_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let dir_name = path.file_name().unwrap();
            let sub_output_dir = output_dir.join(dir_name);
            let (c, s, f) = convert_rust_files_recursive(&path, &sub_output_dir, base_src_dir, args, incremental)?;
            converted += c;
            skipped += s;
            failed += f;
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let file_name = path.file_stem().unwrap().to_str().unwrap();
            let output_path = output_dir.join(format!("{}.nu", file_name));

            let decision = incremental.should_convert(&path, &output_path);
            if decision == ConversionDecision::Skip {
                skipped += 1;
                continue;
            }

            match fs::read_to_string(&path) {
                Ok(rust_content) => {
                    let converter = nu_compiler::rust2nu::Rust2NuConverter::new();
                    match converter.convert(&rust_content) {
                        Ok(nu_content) => {
                            let relative_path = path.strip_prefix(base_src_dir).unwrap_or(&path);
                            let nu_relative = relative_path.with_extension("nu");
                            
                            if args.dry_run {
                                println!("[dry-run]   将创建: {}", nu_relative.display());
                            } else {
                                fs::write(&output_path, nu_content)?;
                                println!("  ✓ {}", nu_relative.display());
                            }
                            converted += 1;
                        }
                        Err(e) => {
                            let relative_path = path.strip_prefix(base_src_dir).unwrap_or(&path);
                            eprintln!("  ✗ {} 转换失败: {}", relative_path.display(), e);
                            failed += 1;
                        }
                    }
                }
                Err(e) => {
                    let relative_path = path.strip_prefix(base_src_dir).unwrap_or(&path);
                    eprintln!("  ✗ 无法读取 {}: {}", relative_path.display(), e);
                    failed += 1;
                }
            }
        }
    }

    Ok((converted, skipped, failed))
}

const ASCII_LOGO: &str = r#"
   _   __          __
  / | / /_  __    / /___ _____  ____ _
 /  |/ / / / /___/ / __ `/ __ \/ __ `/
/ /|  / /_/ /___/ / /_/ / / / / /_/ /
/_/ |_/\__,_/   /_/\__,_/_/ /_/\__, /
                              /____/
Nu-lang: Rust, Condensed. v2.0
"#;

fn main() -> Result<()> {
    let args = Args::parse();

    println!("{}", ASCII_LOGO);

    let input_dir = Path::new(&args.input);

    if !input_dir.exists() {
        eprintln!("错误: 输入目录不存在: {}", input_dir.display());
        std::process::exit(1);
    }

    println!("转换 Cargo 项目到 Nu 项目:");
    println!("  输入: {}", args.input);
    println!("  输出: {}", args.output);
    if args.dry_run {
        println!("  模式: dry-run（仅预览）");
    }
    if args.incremental {
        println!("  模式: 增量转换");
    }
    if args.force {
        println!("  模式: 强制覆盖");
    }
    println!();

    let report = convert_project(&args).context("项目转换失败")?;

    println!("{}", report.format());

    if report.is_success() {
        println!("✅ 转换完成!");
    } else {
        println!("⚠️ 转换完成，但有错误");
        std::process::exit(1);
    }

    Ok(())
}
