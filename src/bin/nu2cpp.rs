// Nu to C++ Converter CLI
// 将 Nu 代码转换为 C++ 代码

use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};

use nu_compiler::nu2cpp::{CppCodegen, Nu2CppConverter, NuToCppAstConverter, SourceMap};

#[derive(Parser, Debug)]
#[command(name = "nu2cpp")]
#[command(about = "Convert Nu code to C++", long_about = None)]
struct Args {
    /// Input Nu file or directory
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    /// Output C++ file or directory
    #[arg(value_name = "OUTPUT")]
    output: Option<PathBuf>,

    /// Process directories recursively
    #[arg(short, long)]
    recursive: bool,

    /// Force overwrite existing files
    #[arg(short, long)]
    force: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Generate source map (.cpp.map)
    #[arg(short = 'm', long)]
    sourcemap: bool,

    /// Use new AST-based converter (experimental, incomplete)
    #[arg(long = "use-ast")]
    use_ast: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.verbose {
        println!("Nu to C++ Converter");
        println!("Input: {:?}", args.input);
        println!("Output: {:?}", args.output);
    }

    let converter = Nu2CppConverter::new();

    if args.input.is_file() {
        // 转换单个文件
        convert_file(&converter, &args)?;
    } else if args.input.is_dir() {
        // 转换目录
        convert_directory(&converter, &args)?;
    } else {
        anyhow::bail!("Input path does not exist: {:?}", args.input);
    }

    if args.verbose {
        println!("✓ Conversion completed successfully!");
    }

    Ok(())
}

fn convert_file(converter: &Nu2CppConverter, args: &Args) -> Result<()> {
    let input_path = &args.input;

    // 读取输入文件
    let nu_code = fs::read_to_string(input_path)
        .with_context(|| format!("Failed to read input file: {:?}", input_path))?;

    // 确定输出路径
    let output_path = if let Some(ref output) = args.output {
        output.clone()
    } else {
        // 默认：同目录，.nu -> .cpp
        let mut path = input_path.clone();
        path.set_extension("cpp");
        path
    };

    // 检查文件是否存在
    if output_path.exists() && !args.force {
        anyhow::bail!(
            "Output file already exists: {:?}\nUse --force to overwrite",
            output_path
        );
    }

    // 转换代码 - 默认使用字符串转换器（稳定）
    let cpp_code = if args.use_ast {
        // 使用新的AST转换器（实验性）
        if args.verbose {
            println!("Using AST-based converter (experimental, incomplete)");
        }
        let mut ast_converter = NuToCppAstConverter::new();
        let unit = ast_converter.convert(&nu_code)?;
        let mut codegen = CppCodegen::new();
        codegen.generate(&unit)
    } else {
        // 默认：使用字符串转换器（稳定）
        if args.verbose {
            println!("Using string-based converter (stable)");
        }
        let mut sourcemap = if args.sourcemap {
            let source_name = input_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown.nu")
                .to_string();
            let target_name = output_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown.cpp")
                .to_string();
            Some(SourceMap::new(source_name, target_name))
        } else {
            None
        };

        let code = if let Some(ref mut sm) = sourcemap {
            converter.convert_with_sourcemap(&nu_code, Some(sm))?
        } else {
            converter.convert(&nu_code)?
        };

        // 写入源码映射 (only for string converter)
        if let Some(sm) = sourcemap {
            let mut map_path = output_path.clone();
            map_path.set_extension("cpp.map");

            sm.save_to_file(&map_path)
                .with_context(|| format!("Failed to write source map: {:?}", map_path))?;

            if args.verbose {
                println!(
                    "✓ Source map: {:?} ({} mappings)",
                    map_path,
                    sm.mapping_count()
                );
            }
        }

        code
    };

    // 写入输出文件
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory: {:?}", parent))?;
    }

    fs::write(&output_path, &cpp_code)
        .with_context(|| format!("Failed to write output file: {:?}", output_path))?;

    if args.verbose {
        println!("✓ Converted: {:?} -> {:?}", input_path, output_path);
    }

    Ok(())
}

fn convert_directory(converter: &Nu2CppConverter, args: &Args) -> Result<()> {
    let input_dir = &args.input;
    let output_dir = args
        .output
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Output directory is required for directory conversion"))?;

    // 创建输出目录
    fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create output directory: {:?}", output_dir))?;

    // 复制 Nu.toml 到输出目录（如果存在）
    let nu_toml_src = input_dir.join("Nu.toml");
    if nu_toml_src.exists() {
        let nu_toml_dst = output_dir.join("Nu.toml");
        fs::copy(&nu_toml_src, &nu_toml_dst)
            .with_context(|| format!("Failed to copy Nu.toml: {:?}", nu_toml_src))?;
        if args.verbose {
            println!("✓ Copied: {:?} -> {:?}", nu_toml_src, nu_toml_dst);
        }
    }

    // 创建带递归和force标志的Args（目录转换默认递归和force）
    let recursive_args = Args {
        input: args.input.clone(),
        output: args.output.clone(),
        recursive: true, // 目录转换默认递归
        force: true,     // 目录转换默认force，避免文件覆盖问题
        verbose: args.verbose,
        sourcemap: args.sourcemap,
        use_ast: args.use_ast,
    };

    // 遍历输入目录
    convert_directory_recursive(converter, input_dir, output_dir, &recursive_args)?;

    Ok(())
}

fn convert_directory_recursive(
    converter: &Nu2CppConverter,
    input_dir: &Path,
    output_dir: &Path,
    args: &Args,
) -> Result<()> {
    for entry in fs::read_dir(input_dir)
        .with_context(|| format!("Failed to read directory: {:?}", input_dir))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            // 只转换 .nu 文件
            if path.extension().and_then(|s| s.to_str()) == Some("nu") {
                let relative_path = path.strip_prefix(input_dir)?;
                let output_path = output_dir.join(relative_path).with_extension("cpp");

                // 创建临时 Args 用于单文件转换
                let file_args = Args {
                    input: path.clone(),
                    output: Some(output_path),
                    recursive: args.recursive,
                    force: args.force,
                    verbose: args.verbose,
                    sourcemap: args.sourcemap,
                    use_ast: args.use_ast,
                };

                convert_file(converter, &file_args)?;
            }
        } else if path.is_dir() && args.recursive {
            // 递归处理子目录
            let dir_name = path.file_name().unwrap();
            let output_subdir = output_dir.join(dir_name);
            fs::create_dir_all(&output_subdir)?;
            convert_directory_recursive(converter, &path, &output_subdir, args)?;
        }
    }

    Ok(())
}
