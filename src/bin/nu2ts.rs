// nu2ts - Nu to TypeScript Converter CLI
// 将Nu代码转换为TypeScript代码

// Development-phase warnings (same as lib.rs)
#![allow(dead_code)]
#![allow(clippy::ptr_arg)]

use anyhow::{Context, Result};
use clap::Parser;
use nu_compiler::nu2ts::{Nu2TsConverter, RuntimeMode, Target, TsConfig};
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "nu2ts")]
#[command(about = "Convert Nu code to TypeScript", long_about = None)]
struct Cli {
    /// Input Nu file or directory
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    /// Output TypeScript file or directory (optional, defaults to INPUT with .ts extension)
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

    /// Runtime mode: inline (default) or import
    #[arg(long, value_name = "MODE", default_value = "import")]
    runtime: String,

    /// Target platform: node (default), browser, or deno
    #[arg(long, value_name = "TARGET", default_value = "node")]
    target: String,

    /// Generate package.json for the project
    #[arg(long)]
    gen_package: bool,

    /// Generate tsconfig.json for the project
    #[arg(long)]
    gen_tsconfig: bool,

    /// Project mode: convert entire Nu project to TypeScript project
    #[arg(short = 'P', long)]
    project: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // 解析配置
    let runtime_mode = match cli.runtime.as_str() {
        "inline" => RuntimeMode::Inline,
        "import" => RuntimeMode::Import,
        _ => anyhow::bail!("Invalid runtime mode. Use 'inline' or 'import'"),
    };

    let target = match cli.target.as_str() {
        "node" => Target::Node,
        "browser" => Target::Browser,
        "deno" => Target::Deno,
        _ => anyhow::bail!("Invalid target. Use 'node', 'browser', or 'deno'"),
    };

    let config = TsConfig {
        runtime_mode,
        target,
        strict: true,
        no_format: false,
        source_map: false,
    };

    let mut converter = Nu2TsConverter::new(config.clone());

    if cli.project {
        // 项目模式：转换整个Nu项目
        convert_project(&mut converter, &cli.input, cli.output.as_ref(), &cli)?;
    } else if cli.input.is_file() {
        // 单文件转换
        convert_file(
            &mut converter,
            &cli.input,
            cli.output.as_ref(),
            cli.force,
            cli.verbose,
        )?;
    } else if cli.input.is_dir() {
        // 目录转换
        if cli.recursive {
            convert_directory_recursive(
                &mut converter,
                &cli.input,
                cli.output.as_ref(),
                cli.force,
                cli.verbose,
            )?;
        } else {
            convert_directory(
                &mut converter,
                &cli.input,
                cli.output.as_ref(),
                cli.force,
                cli.verbose,
            )?;
        }

        // 生成配置文件（如果需要）
        if cli.gen_package || cli.gen_tsconfig {
            let output_dir = cli.output.as_ref().unwrap_or(&cli.input);
            generate_config_files(output_dir, cli.gen_package, cli.gen_tsconfig, &config)?;
        }
    } else {
        anyhow::bail!("Input path does not exist: {}", cli.input.display());
    }

    Ok(())
}

fn convert_file(
    converter: &mut Nu2TsConverter,
    input: &PathBuf,
    output: Option<&PathBuf>,
    force: bool,
    verbose: bool,
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
        None => input.with_extension("ts"),
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

    // 转换为TypeScript代码
    let ts_code = converter
        .convert(&nu_code)
        .with_context(|| format!("Failed to convert file: {}", input.display()))?;

    // 写入输出文件
    fs::write(&output_path, ts_code)
        .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

    println!("✓ {}", output_path.display());

    // 自动生成 runtime 文件（Import 模式）
    if converter.config().runtime_mode == RuntimeMode::Import {
        let runtime_dir = output_path.parent().unwrap_or(std::path::Path::new("."));
        let runtime_path = runtime_dir.join("nu_runtime.ts");
        if !runtime_path.exists() {
            use nu_compiler::nu2ts::runtime;
            fs::write(&runtime_path, runtime::generate_runtime_file_content())?;
            if verbose {
                println!("✓ Generated {}", runtime_path.display());
            }
        }
    }

    Ok(())
}

fn convert_directory(
    converter: &mut Nu2TsConverter,
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

    // 遍历目录中的.nu文件
    for entry in fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("nu") {
            let output_path = output_base
                .join(path.file_name().unwrap())
                .with_extension("ts");
            convert_file(converter, &path, Some(&output_path), force, verbose)?;
        }
    }

    // 自动生成 runtime 文件（Import 模式）
    if converter.config().runtime_mode == RuntimeMode::Import {
        let runtime_path = output_base.join("nu_runtime.ts");
        if !runtime_path.exists() {
            use nu_compiler::nu2ts::runtime;
            fs::write(&runtime_path, runtime::generate_runtime_file_content())?;
            if verbose {
                println!("✓ Generated nu_runtime.ts");
            }
        }
    }

    Ok(())
}

fn convert_directory_recursive(
    converter: &mut Nu2TsConverter,
    input_dir: &PathBuf,
    output_dir: Option<&PathBuf>,
    force: bool,
    verbose: bool,
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
        let output_path = output_base.join(relative_path).with_extension("ts");

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
        )?;
    }

    // 自动生成 runtime 文件（Import 模式）
    if converter.config().runtime_mode == RuntimeMode::Import {
        let runtime_path = output_base.join("nu_runtime.ts");
        if !runtime_path.exists() {
            use nu_compiler::nu2ts::runtime;
            fs::write(&runtime_path, runtime::generate_runtime_file_content())?;
            if verbose {
                println!("✓ Generated nu_runtime.ts");
            }
        }
    }

    Ok(())
}

fn convert_project(
    converter: &mut Nu2TsConverter,
    input_dir: &PathBuf,
    output_dir: Option<&PathBuf>,
    cli: &Cli,
) -> Result<()> {
    println!("🚀 Converting Nu project to TypeScript...");

    // 检查是否是Nu项目
    let nu_toml = input_dir.join("Nu.toml");
    if !nu_toml.exists() {
        anyhow::bail!(
            "Not a Nu project: Nu.toml not found in {}",
            input_dir.display()
        );
    }

    // 确定输出目录
    let output_base = output_dir
        .cloned()
        .unwrap_or_else(|| input_dir.with_extension("_ts"));

    // 创建输出目录结构
    fs::create_dir_all(&output_base)?;
    let src_dir = output_base.join("src");
    fs::create_dir_all(&src_dir)?;

    println!("📁 Output directory: {}", output_base.display());

    // 转换src目录下的所有.nu文件
    let input_src = input_dir.join("src");
    if input_src.exists() {
        convert_directory_recursive(
            converter,
            &input_src,
            Some(&src_dir),
            cli.force,
            cli.verbose,
        )?;
    }

    // 生成package.json
    generate_package_json(&output_base, input_dir)?;
    println!("✓ Generated package.json");

    // 生成tsconfig.json
    generate_tsconfig_json(&output_base)?;
    println!("✓ Generated tsconfig.json");

    // 如果是import模式，生成runtime文件
    if matches!(converter.config().runtime_mode, RuntimeMode::Import) {
        use nu_compiler::nu2ts::runtime;
        fs::write(
            src_dir.join("nu_runtime.ts"),
            runtime::generate_runtime_file_content(),
        )?;
        println!("✓ Generated nu_runtime.ts");
    }

    println!("✅ Project conversion completed!");
    println!("📦 To run the project:");
    println!("   cd {}", output_base.display());
    println!("   npm install");
    println!("   npm start");

    Ok(())
}

fn generate_config_files(
    output_dir: &PathBuf,
    gen_package: bool,
    gen_tsconfig: bool,
    _config: &TsConfig,
) -> Result<()> {
    if gen_package {
        generate_package_json(output_dir, output_dir)?;
        println!("✓ Generated package.json");
    }

    if gen_tsconfig {
        generate_tsconfig_json(output_dir)?;
        println!("✓ Generated tsconfig.json");
    }

    Ok(())
}

fn generate_package_json(output_dir: &PathBuf, input_dir: &PathBuf) -> Result<()> {
    // 尝试从Nu.toml读取项目信息
    let nu_toml = input_dir.join("Nu.toml");
    let (name, version) = if nu_toml.exists() {
        let content = fs::read_to_string(&nu_toml)?;
        let name = content
            .lines()
            .find(|l| l.starts_with("name"))
            .and_then(|l| l.split('=').nth(1))
            .map(|s| s.trim().trim_matches('"'))
            .unwrap_or("nu-project");
        let version = content
            .lines()
            .find(|l| l.starts_with("version"))
            .and_then(|l| l.split('=').nth(1))
            .map(|s| s.trim().trim_matches('"'))
            .unwrap_or("0.1.0");
        (name.to_string(), version.to_string())
    } else {
        ("nu-project".to_string(), "0.1.0".to_string())
    };

    let package_json = format!(
        r#"{{
  "name": "{}",
  "version": "{}",
  "description": "Converted from Nu project",
  "main": "dist/main.js",
  "scripts": {{
    "build": "tsc",
    "start": "node dist/main.js",
    "dev": "ts-node src/main.ts",
    "watch": "tsc --watch"
  }},
  "keywords": ["nu", "typescript"],
  "author": "",
  "license": "MIT",
  "devDependencies": {{
    "@types/node": "^20.0.0",
    "typescript": "^5.0.0",
    "ts-node": "^10.9.0"
  }}
}}
"#,
        name, version
    );

    fs::write(output_dir.join("package.json"), package_json)?;
    Ok(())
}

fn generate_tsconfig_json(output_dir: &PathBuf) -> Result<()> {
    let tsconfig = r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "lib": ["ES2020"],
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}
"#;

    fs::write(output_dir.join("tsconfig.json"), tsconfig)?;
    Ok(())
}

fn generate_runtime_file(src_dir: &PathBuf) -> Result<()> {
    let runtime_code = nu_compiler::nu2ts::runtime::generate_micro_runtime();
    fs::write(src_dir.join("nu_runtime.ts"), runtime_code)?;
    Ok(())
}
