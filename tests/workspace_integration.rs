// Integration Tests for Workspace Conversion
// Tests real-world project conversion scenarios

use std::fs;
use std::path::Path;
use std::process::Command;

/// 辅助函数：运行 cargo2nu 转换
fn run_cargo2nu(input: &str, output: &str, args: &[&str]) -> bool {
    let mut cmd = Command::new("./target/release/cargo2nu");
    cmd.arg(input).arg(output);
    for arg in args {
        cmd.arg(arg);
    }
    
    let result = cmd.output();
    match result {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// 辅助函数：运行 nu2cargo 转换
fn run_nu2cargo(input: &str, output: &str, args: &[&str]) -> bool {
    let mut cmd = Command::new("./target/release/nu2cargo");
    cmd.arg(input).arg(output);
    for arg in args {
        cmd.arg(arg);
    }
    
    let result = cmd.output();
    match result {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// 辅助函数：验证目录结构
fn verify_structure(dir: &Path, expected_files: &[&str]) -> bool {
    for file in expected_files {
        let path = dir.join(file);
        if !path.exists() {
            eprintln!("Missing file: {}", path.display());
            return false;
        }
    }
    true
}

/// 辅助函数：清理测试输出目录
fn cleanup_dir(dir: &str) {
    let _ = fs::remove_dir_all(dir);
}

// ==================== test_workspace_simple 测试 ====================

#[test]
fn test_simple_workspace_cargo2nu() {
    let input = "test_workspace_simple";
    let output = "/tmp/test_ws_simple_nu";
    
    cleanup_dir(output);
    
    // 检查输入目录存在
    if !Path::new(input).exists() {
        eprintln!("Skipping test: {} not found", input);
        return;
    }
    
    // 运行转换
    assert!(run_cargo2nu(input, output, &["-v"]), "cargo2nu failed");
    
    // 验证输出结构
    let expected = vec![
        "Nu.toml",
        "lib1/Nu.toml",
        "lib1/src/lib.nu",
        "lib2/Nu.toml",
        "lib2/src/lib.nu",
    ];
    
    assert!(verify_structure(Path::new(output), &expected), "Structure verification failed");
    
    // 验证 Nu.toml 内容
    let nu_toml = fs::read_to_string(format!("{}/Nu.toml", output)).unwrap();
    assert!(nu_toml.contains("[W]"), "Missing [W] section");
    assert!(nu_toml.contains("m = "), "Missing members");
}

#[test]
fn test_simple_workspace_roundtrip() {
    let input = "test_workspace_simple";
    let nu_output = "/tmp/test_ws_simple_nu_rt";
    let cargo_output = "/tmp/test_ws_simple_cargo_rt";
    
    cleanup_dir(nu_output);
    cleanup_dir(cargo_output);
    
    if !Path::new(input).exists() {
        eprintln!("Skipping test: {} not found", input);
        return;
    }
    
    // Cargo -> Nu
    assert!(run_cargo2nu(input, nu_output, &[]), "cargo2nu failed");
    
    // Nu -> Cargo
    assert!(run_nu2cargo(nu_output, cargo_output, &[]), "nu2cargo failed");
    
    // 验证往返后的结构
    let expected = vec![
        "Cargo.toml",
        "lib1/Cargo.toml",
        "lib1/src/lib.rs",
        "lib2/Cargo.toml",
        "lib2/src/lib.rs",
    ];
    
    assert!(verify_structure(Path::new(cargo_output), &expected), "Roundtrip structure verification failed");
    
    // 验证 Cargo.toml 内容
    let cargo_toml = fs::read_to_string(format!("{}/Cargo.toml", cargo_output)).unwrap();
    assert!(cargo_toml.contains("[workspace]"), "Missing [workspace] section");
    assert!(cargo_toml.contains("members = "), "Missing members");
}

#[test]
fn test_simple_workspace_dry_run() {
    let input = "test_workspace_simple";
    let output = "/tmp/test_ws_simple_dryrun";
    
    cleanup_dir(output);
    
    if !Path::new(input).exists() {
        eprintln!("Skipping test: {} not found", input);
        return;
    }
    
    // 运行 dry-run
    assert!(run_cargo2nu(input, output, &["--dry-run"]), "cargo2nu dry-run failed");
    
    // 验证没有创建任何文件
    assert!(!Path::new(output).exists() || fs::read_dir(output).unwrap().count() == 0,
            "Dry-run should not create files");
}

#[test]
fn test_simple_workspace_incremental() {
    let input = "test_workspace_simple";
    let output = "/tmp/test_ws_simple_incr";
    
    cleanup_dir(output);
    
    if !Path::new(input).exists() {
        eprintln!("Skipping test: {} not found", input);
        return;
    }
    
    // 第一次转换
    assert!(run_cargo2nu(input, output, &[]), "First conversion failed");
    
    // 记录文件修改时间
    let nu_toml_path = format!("{}/Nu.toml", output);
    let mtime1 = fs::metadata(&nu_toml_path).unwrap().modified().unwrap();
    
    // 等待一小段时间
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // 增量转换（应该跳过）
    assert!(run_cargo2nu(input, output, &["-i"]), "Incremental conversion failed");
    
    // 验证文件没有被修改
    let mtime2 = fs::metadata(&nu_toml_path).unwrap().modified().unwrap();
    assert_eq!(mtime1, mtime2, "File should not be modified in incremental mode");
}

#[test]
fn test_simple_workspace_exclude() {
    let input = "test_workspace_simple";
    let output = "/tmp/test_ws_simple_exclude";
    
    cleanup_dir(output);
    
    if !Path::new(input).exists() {
        eprintln!("Skipping test: {} not found", input);
        return;
    }
    
    // 排除 lib2
    assert!(run_cargo2nu(input, output, &["--exclude", "lib2"]), "Exclude conversion failed");
    
    // 验证 lib1 存在，lib2 不存在
    assert!(Path::new(&format!("{}/lib1", output)).exists(), "lib1 should exist");
    assert!(!Path::new(&format!("{}/lib2", output)).exists(), "lib2 should not exist");
}

#[test]
fn test_simple_workspace_only() {
    let input = "test_workspace_simple";
    let output = "/tmp/test_ws_simple_only";
    
    cleanup_dir(output);
    
    if !Path::new(input).exists() {
        eprintln!("Skipping test: {} not found", input);
        return;
    }
    
    // 只转换 lib1
    assert!(run_cargo2nu(input, output, &["--only", "lib1"]), "Only conversion failed");
    
    // 验证 lib1 存在，lib2 不存在
    assert!(Path::new(&format!("{}/lib1", output)).exists(), "lib1 should exist");
    assert!(!Path::new(&format!("{}/lib2", output)).exists(), "lib2 should not exist");
}

// ==================== TOML 内容验证测试 ====================

#[test]
fn test_toml_section_conversion() {
    let input = "test_workspace_simple";
    let output = "/tmp/test_ws_toml_sections";
    
    cleanup_dir(output);
    
    if !Path::new(input).exists() {
        eprintln!("Skipping test: {} not found", input);
        return;
    }
    
    assert!(run_cargo2nu(input, output, &[]), "Conversion failed");
    
    // 读取并验证 Nu.toml
    let nu_toml = fs::read_to_string(format!("{}/Nu.toml", output)).unwrap();
    
    // 验证节名转换
    assert!(nu_toml.contains("[W]"), "Should have [W] section");
    assert!(!nu_toml.contains("[workspace]"), "Should not have [workspace] section");
    
    // 验证键名转换
    assert!(nu_toml.contains("m = "), "Should have 'm' key for members");
    assert!(!nu_toml.contains("members = "), "Should not have 'members' key");
}

#[test]
fn test_toml_roundtrip_equivalence() {
    let input = "test_workspace_simple";
    let nu_output = "/tmp/test_ws_toml_equiv_nu";
    let cargo_output = "/tmp/test_ws_toml_equiv_cargo";
    
    cleanup_dir(nu_output);
    cleanup_dir(cargo_output);
    
    if !Path::new(input).exists() {
        eprintln!("Skipping test: {} not found", input);
        return;
    }
    
    // 读取原始 Cargo.toml
    let original = fs::read_to_string(format!("{}/Cargo.toml", input)).unwrap();
    
    // Cargo -> Nu -> Cargo
    assert!(run_cargo2nu(input, nu_output, &[]), "cargo2nu failed");
    assert!(run_nu2cargo(nu_output, cargo_output, &[]), "nu2cargo failed");
    
    // 读取往返后的 Cargo.toml
    let restored = fs::read_to_string(format!("{}/Cargo.toml", cargo_output)).unwrap();
    
    // 验证关键内容被保留
    if original.contains("[workspace]") {
        assert!(restored.contains("[workspace]"), "Should preserve [workspace]");
    }
    if original.contains("members") {
        assert!(restored.contains("members"), "Should preserve members");
    }
    if original.contains("resolver") {
        assert!(restored.contains("resolver"), "Should preserve resolver");
    }
}
