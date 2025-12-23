// rust2nu - Rust to Nu Converter CLI
// 将Rust代码压缩为Nu高密度语法

use clap::Parser;
use nu_compiler::Rust2NuConverter;
use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Result};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "rust2nu")]
#[command(about = "Convert Rust code to Nu high-density syntax", long_about = None)]
struct Cli {
    /// Input Rust file or directory
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    /// Output Nu file or directory (optional, defaults to INPUT with .nu extension)
    #[arg(short, long, value_name = "OUTPUT")]
    output: Option<PathBuf>,

    /// Process directories recursively
    #[arg(short, long)]
    recursive: bool,

    /// Overwrite existing files
    #[arg(short = 'f', long)]
    force: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let converter = Rust2NuConverter::new();

    if cli.input.is_file() {
        // 单文件转换
        convert_file(&converter, &cli.input, cli.output.as_ref(), cli.force, cli.verbose)?;
    } else if cli.input.is_dir() {
        // 目录转换
        if cli.recursive {
            convert_directory_recursive(&converter, &cli.input, cli.output.as_ref(), cli.force, cli.verbose)?;
        } else {
            convert_directory(&converter, &cli.input, cli.output.as_ref(), cli.force, cli.verbose)?;
        }
    } else {
        anyhow::bail!("Input path does not exist: {}", cli.input.display());
    }

    Ok(())
}

fn convert_file(
    converter: &Rust2NuConverter,
    input: &PathBuf,
    output: Option<&PathBuf>,
    force: bool,
    verbose: bool,
) -> Result<()> {
    // 检查输入文件扩展名
    if input.extension().and_then(|s| s.to_str()) != Some("rs") {
        if verbose {
            println!("Skipping non-Rust file: {}", input.display());
        }
        return Ok(());
    }

    // 确定输出文件路径
    let output_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("nu"),
    };

    // 检查输出文件是否存在
    if output_path.exists() && !force {
        anyhow::bail!(
            "Output file already exists: {} (use -f to overwrite)",
            output_path.display()
        );
    }

    if verbose {
        println!("Converting: {} -> {}", input.display(), output_path.display());
    }

    // 读取Rust代码
    let rust_code = fs::read_to_string(input)
        .with_context(|| format!("Failed to read input file: {}", input.display()))?;

    // 转换为Nu代码
    let nu_code = converter.convert(&rust_code)
        .with_context(|| format!("Failed to convert file: {}", input.display()))?;

    // 写入输出文件
    fs::write(&output_path, nu_code)
        .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

    println!("✓ {}", output_path.display());

    Ok(())
}

fn convert_directory(
    converter: &Rust2NuConverter,
    input_dir: &PathBuf,
    output_dir: Option<&PathBuf>,
    force: bool,
    verbose: bool,
) -> Result<()> {
    let output_base = output_dir.cloned().unwrap_or_else(|| input_dir.clone());

    // 创建输出目录
    if !output_base.exists() {
        fs::create_dir_all(&output_base)?;
    }

    // 遍历目录中的.rs文件
    for entry in fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let output_path = output_base.join(path.file_name().unwrap()).with_extension("nu");
            convert_file(converter, &path, Some(&output_path), force, verbose)?;
        }
    }

    Ok(())
}

fn convert_directory_recursive(
    converter: &Rust2NuConverter,
    input_dir: &PathBuf,
    output_dir: Option<&PathBuf>,
    force: bool,
    verbose: bool,
) -> Result<()> {
    let output_base = output_dir.cloned().unwrap_or_else(|| input_dir.clone());

    // 遍历所有.rs文件
    for entry in WalkDir::new(input_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("rs"))
    {
        let input_path = entry.path();
        
        // 计算相对路径
        let relative_path = input_path.strip_prefix(input_dir)?;
        let output_path = output_base.join(relative_path).with_extension("nu");

        // 创建输出目录
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        convert_file(converter, &input_path.to_path_buf(), Some(&output_path), force, verbose)?;
    }

    Ok(())
}