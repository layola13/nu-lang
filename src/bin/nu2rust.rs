// nu2rust - Nu to Rust Converter CLI
// 将Nu代码转换回标准Rust代码

use anyhow::{Context, Result};
use clap::Parser;
use nu_compiler::nu2rust::{LazySourceMap, Nu2RustConverter};
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "nu2rust")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Convert Nu code back to standard Rust", long_about = None)]
struct Cli {
    /// Input Nu file or directory
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    /// Output Rust file or directory (optional, defaults to INPUT with .rs extension)
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

    /// Generate source map file (.rs.map)
    #[arg(short = 's', long)]
    sourcemap: bool,
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
    let cli = Cli::parse();
    
    // 显示ASCII Logo
    println!("{}", ASCII_LOGO);

    let converter = Nu2RustConverter::new();

    if cli.input.is_file() {
        // 单文件转换
        convert_file(
            &converter,
            &cli.input,
            cli.output.as_ref(),
            cli.force,
            cli.verbose,
            cli.sourcemap,
        )?;
    } else if cli.input.is_dir() {
        // 目录转换
        if cli.recursive {
            convert_directory_recursive(
                &converter,
                &cli.input,
                cli.output.as_ref(),
                cli.force,
                cli.verbose,
                cli.sourcemap,
            )?;
        } else {
            convert_directory(
                &converter,
                &cli.input,
                cli.output.as_ref(),
                cli.force,
                cli.verbose,
                cli.sourcemap,
            )?;
        }
    } else {
        anyhow::bail!("Input path does not exist: {}", cli.input.display());
    }

    Ok(())
}

fn convert_file(
    converter: &Nu2RustConverter,
    input: &PathBuf,
    output: Option<&PathBuf>,
    force: bool,
    verbose: bool,
    generate_sourcemap: bool,
) -> Result<()> {
    // 检查输入文件扩展名
    if input.extension().and_then(|s| s.to_str()) != Some("nu") {
        if verbose {
            println!("Skipping non-Nu file: {}", input.display());
        }
        return Ok(());
    }

    // 确定输出文件路径
    let output_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("rs"),
    };

    // 检查输出文件是否存在
    if output_path.exists() && !force {
        anyhow::bail!(
            "Output file already exists: {} (use -f to overwrite)",
            output_path.display()
        );
    }

    if verbose {
        println!(
            "Converting: {} -> {}",
            input.display(),
            output_path.display()
        );
    }

    // 读取Nu代码
    let nu_code = fs::read_to_string(input)
        .with_context(|| format!("Failed to read input file: {}", input.display()))?;

    // 转换为Rust代码
    let rust_code = if generate_sourcemap {
        // 创建 SourceMap
        let mut sourcemap = LazySourceMap::new(
            input.file_name().unwrap().to_string_lossy().to_string(),
            output_path.file_name().unwrap().to_string_lossy().to_string(),
        );
        
        // 使用 sourcemap 进行转换
        let code = converter
            .convert_with_sourcemap(&nu_code, Some(&mut sourcemap))
            .with_context(|| format!("Failed to convert file: {}", input.display()))?;
        
        // 保存 sourcemap 文件
        let map_path = output_path.with_extension("rs.map");
        sourcemap.save_to_file(&map_path)
            .with_context(|| format!("Failed to write sourcemap file: {}", map_path.display()))?;
        
        if verbose {
            println!("Generated sourcemap: {} ({} mappings)", map_path.display(), sourcemap.mapping_count());
        }
        
        code
    } else {
        converter
            .convert(&nu_code)
            .with_context(|| format!("Failed to convert file: {}", input.display()))?
    };

    // 写入输出文件
    fs::write(&output_path, rust_code)
        .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

    println!("✓ {}", output_path.display());

    Ok(())
}

fn convert_directory(
    converter: &Nu2RustConverter,
    input_dir: &PathBuf,
    output_dir: Option<&PathBuf>,
    force: bool,
    verbose: bool,
    generate_sourcemap: bool,
) -> Result<()> {
    let output_base = output_dir.cloned().unwrap_or_else(|| input_dir.clone());

    // 创建输出目录
    if !output_base.exists() {
        fs::create_dir_all(&output_base)?;
    }

    // 遍历目录中的.nu文件
    for entry in fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("nu") {
            let output_path = output_base
                .join(path.file_name().unwrap())
                .with_extension("rs");
            convert_file(converter, &path, Some(&output_path), force, verbose, generate_sourcemap)?;
        }
    }

    Ok(())
}

fn convert_directory_recursive(
    converter: &Nu2RustConverter,
    input_dir: &PathBuf,
    output_dir: Option<&PathBuf>,
    force: bool,
    verbose: bool,
    generate_sourcemap: bool,
) -> Result<()> {
    let output_base = output_dir.cloned().unwrap_or_else(|| input_dir.clone());

    // 遍历所有.nu文件
    for entry in WalkDir::new(input_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("nu"))
    {
        let input_path = entry.path();

        // 计算相对路径
        let relative_path = input_path.strip_prefix(input_dir)?;
        let output_path = output_base.join(relative_path).with_extension("rs");

        // 创建输出目录
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        convert_file(
            converter,
            &input_path.to_path_buf(),
            Some(&output_path),
            force,
            verbose,
            generate_sourcemap,
        )?;
    }

    Ok(())
}
