// Nu.toml to CMakeLists.txt Converter
// 将 Nu 项目配置转换为 CMake 构建配置

use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;

#[derive(Parser, Debug)]
#[command(name = "nu2cmake")]
#[command(about = "Convert Nu.toml to CMakeLists.txt", long_about = None)]
struct Args {
    /// Input Nu project directory (containing Nu.toml)
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    /// Output directory for CMakeLists.txt
    #[arg(short, long, value_name = "OUTPUT")]
    output: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.verbose {
        println!("Nu.toml to CMakeLists.txt Converter");
        println!("Input: {:?}", args.input);
    }

    let nu_toml_path = args.input.join("Nu.toml");
    if !nu_toml_path.exists() {
        anyhow::bail!("Nu.toml not found in {:?}", args.input);
    }

    // 读取 Nu.toml
    let toml_content = fs::read_to_string(&nu_toml_path)
        .with_context(|| format!("Failed to read Nu.toml: {:?}", nu_toml_path))?;

    let toml_value: Value = toml::from_str(&toml_content)
        .with_context(|| "Failed to parse Nu.toml")?;

    // 确定输出目录
    let output_dir = args.output.as_ref().unwrap_or(&args.input);

    // 检查是否是 workspace
    if toml_value.get("W").is_some() || toml_value.get("workspace").is_some() {
        // Workspace 项目
        convert_workspace(&toml_value, &args.input, output_dir, args.verbose)?;
    } else {
        // 单项目
        convert_single_project(&toml_value, output_dir, args.verbose)?;
    }

    if args.verbose {
        println!("✓ Conversion completed successfully!");
    }

    Ok(())
}

fn convert_single_project(toml: &Value, output_dir: &Path, verbose: bool) -> Result<()> {
    let cmake_content = generate_cmake_for_project(toml, None)?;

    let output_path = output_dir.join("CMakeLists.txt");
    fs::write(&output_path, cmake_content)
        .with_context(|| format!("Failed to write CMakeLists.txt: {:?}", output_path))?;

    if verbose {
        println!("✓ Generated: {:?}", output_path);
    }

    Ok(())
}

fn convert_workspace(toml: &Value, input_dir: &Path, output_dir: &Path, verbose: bool) -> Result<()> {
    // 生成根 CMakeLists.txt
    let root_cmake = generate_workspace_root_cmake(toml)?;
    let root_cmake_path = output_dir.join("CMakeLists.txt");
    
    fs::create_dir_all(output_dir)?;
    fs::write(&root_cmake_path, root_cmake)
        .with_context(|| format!("Failed to write root CMakeLists.txt: {:?}", root_cmake_path))?;

    if verbose {
        println!("✓ Generated: {:?}", root_cmake_path);
    }

    // 获取成员列表
    let members = get_workspace_members(toml)?;

    // 为每个成员生成 CMakeLists.txt
    for member in members {
        let member_toml_path = input_dir.join(&member).join("Nu.toml");
        if member_toml_path.exists() {
            let member_toml_content = fs::read_to_string(&member_toml_path)?;
            let member_toml: Value = toml::from_str(&member_toml_content)?;

            let member_cmake = generate_cmake_for_project(&member_toml, Some(toml))?;
            let member_output_dir = output_dir.join(&member);
            fs::create_dir_all(&member_output_dir)?;

            let member_cmake_path = member_output_dir.join("CMakeLists.txt");
            fs::write(&member_cmake_path, member_cmake)?;

            if verbose {
                println!("✓ Generated: {:?}", member_cmake_path);
            }
        }
    }

    Ok(())
}

fn generate_workspace_root_cmake(toml: &Value) -> Result<String> {
    let mut cmake = String::new();

    cmake.push_str("cmake_minimum_required(VERSION 3.15)\n");
    cmake.push_str("project(NuWorkspace LANGUAGES CXX)\n\n");
    cmake.push_str("set(CMAKE_CXX_STANDARD 17)\n");
    cmake.push_str("set(CMAKE_CXX_STANDARD_REQUIRED ON)\n\n");

    // Workspace 共享依赖
    if let Some(workspace_deps) = toml.get("W").and_then(|w| w.get("D"))
        .or_else(|| toml.get("workspace").and_then(|w| w.get("dependencies")))
    {
        cmake.push_str("# Workspace shared dependencies\n");
        if let Value::Table(deps) = workspace_deps {
            for (name, _version) in deps {
                let pkg_name = dependency_to_cmake_package(name);
                cmake.push_str(&format!("find_package({} REQUIRED)\n", pkg_name));
            }
        }
        cmake.push_str("\n");
    }

    // 添加子项目
    let members = get_workspace_members(toml)?;
    cmake.push_str("# Add subdirectories\n");
    for member in members {
        cmake.push_str(&format!("add_subdirectory({})\n", member));
    }

    Ok(cmake)
}

fn generate_cmake_for_project(toml: &Value, workspace_toml: Option<&Value>) -> Result<String> {
    let mut cmake = String::new();

    // 项目信息
    let package = toml.get("P").or_else(|| toml.get("package"));
    let project_name = package
        .and_then(|p| p.get("id").or_else(|| p.get("n")).or_else(|| p.get("name")))
        .and_then(|v| v.as_str())
        .unwrap_or("nu_project");

    let version = package
        .and_then(|p| p.get("v").or_else(|| p.get("version")))
        .and_then(|v| v.as_str())
        .unwrap_or("0.1.0");

    // 如果不是 workspace 成员，生成完整的项目头
    if workspace_toml.is_none() {
        cmake.push_str("cmake_minimum_required(VERSION 3.15)\n");
        cmake.push_str(&format!("project({} VERSION {} LANGUAGES CXX)\n\n", project_name, version));
        cmake.push_str("set(CMAKE_CXX_STANDARD 17)\n");
        cmake.push_str("set(CMAKE_CXX_STANDARD_REQUIRED ON)\n\n");
    } else {
        cmake.push_str(&format!("project({} VERSION {})\n\n", project_name, version));
    }

    // 依赖
    let dependencies = toml.get("D").or_else(|| toml.get("dependencies"));
    if let Some(Value::Table(deps)) = dependencies {
        cmake.push_str("# Dependencies\n");
        for (name, spec) in deps {
            // 跳过 workspace 依赖（已在根级处理）
            if let Value::Table(spec_table) = spec {
                if spec_table.get("workspace").and_then(|v| v.as_bool()).unwrap_or(false) {
                    continue;
                }
                if spec_table.get("path").is_some() {
                    // 路径依赖，不需要 find_package
                    continue;
                }
            }

            let pkg_name = dependency_to_cmake_package(name);
            cmake.push_str(&format!("find_package({} REQUIRED)\n", pkg_name));
        }
        cmake.push_str("\n");
    }

    // 源文件
    cmake.push_str("# Source files\n");
    cmake.push_str("file(GLOB_RECURSE SOURCES \"src/*.cpp\")\n");
    cmake.push_str("file(GLOB_RECURSE HEADERS \"src/*.hpp\")\n\n");

    // 可执行文件或库
    cmake.push_str("# Target\n");
    let is_library = package
        .and_then(|p| p.get("crate-type"))
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().any(|v| v.as_str() == Some("lib")))
        .unwrap_or(false);

    if is_library {
        cmake.push_str(&format!("add_library({} ${{SOURCES}} ${{HEADERS}})\n\n", project_name));
    } else {
        cmake.push_str(&format!("add_executable({} ${{SOURCES}} ${{HEADERS}})\n\n", project_name));
    }

    // 链接库
    if let Some(Value::Table(deps)) = dependencies {
        if !deps.is_empty() {
            cmake.push_str("# Link libraries\n");
            cmake.push_str(&format!("target_link_libraries({}\n", project_name));
            cmake.push_str("    PRIVATE\n");
            
            for (name, spec) in deps {
                if let Value::Table(spec_table) = spec {
                    if spec_table.get("path").is_some() {
                        // 路径依赖 - 直接使用依赖名
                        cmake.push_str(&format!("    {}\n", name));
                        continue;
                    }
                }
                
                let link_name = dependency_to_cmake_link(name);
                cmake.push_str(&format!("    {}\n", link_name));
            }
            
            cmake.push_str(")\n\n");
        }
    }

    // 包含目录
    cmake.push_str("# Include directories\n");
    cmake.push_str(&format!("target_include_directories({}\n", project_name));
    cmake.push_str("    PUBLIC\n");
    cmake.push_str("    ${CMAKE_CURRENT_SOURCE_DIR}/src\n");
    cmake.push_str(")\n");

    Ok(cmake)
}

fn get_workspace_members(toml: &Value) -> Result<Vec<String>> {
    let members = toml
        .get("W")
        .and_then(|w| w.get("m"))
        .or_else(|| toml.get("workspace").and_then(|w| w.get("members")))
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Workspace members not found"))?;

    Ok(members
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect())
}

fn dependency_to_cmake_package(name: &str) -> String {
    match name {
        "fmt" => "fmt".to_string(),
        "spdlog" => "spdlog".to_string(),
        "boost" => "Boost".to_string(),
        "eigen" => "Eigen3".to_string(),
        "opencv" => "OpenCV".to_string(),
        "qt" | "qt6" => "Qt6".to_string(),
        _ => name.to_string(),
    }
}

fn dependency_to_cmake_link(name: &str) -> String {
    match name {
        "fmt" => "fmt::fmt".to_string(),
        "spdlog" => "spdlog::spdlog".to_string(),
        "eigen" => "Eigen3::Eigen".to_string(),
        "opencv" => "opencv::opencv".to_string(),
        _ => format!("{}::{}", name, name),
    }
}