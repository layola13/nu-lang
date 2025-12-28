// TOML Mapping Configuration
// Defines bidirectional mappings between Cargo.toml and Nu.toml formats

use std::collections::HashMap;
use std::sync::LazyLock;

/// 节名映射：Cargo.toml -> Nu.toml
pub static SECTION_CARGO_TO_NU: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| {
        let mut m = HashMap::new();
        // 基本节
        m.insert("[package]", "[P]");
        m.insert("[workspace]", "[W]");
        m.insert("[dependencies]", "[D]");
        m.insert("[dev-dependencies]", "[DD]");
        m.insert("[build-dependencies]", "[BD]");

        // Workspace 子节
        m.insert("[workspace.dependencies]", "[W.D]");
        m.insert("[workspace.package]", "[W.P]");
        // workspace.lints 保留部分压缩
        m.insert("[workspace.lints]", "[W.lints]");
        m.insert("[workspace.lints.rust]", "[W.lints.rust]");
        m.insert("[workspace.lints.clippy]", "[W.lints.clippy]");
        // workspace.metadata 保留部分压缩（动态处理）

        // 目标节
        m.insert("[lib]", "[L]");
        m.insert("[features]", "[FE]");
        m.insert("[[bin]]", "[[B]]");
        m.insert("[[example]]", "[[EX]]");
        m.insert("[[test]]", "[[T]]");
        m.insert("[[bench]]", "[[BE]]");

        // 保留不变的节（不在映射表中，由代码逻辑处理）
        // [profile.*], [patch.*], [replace], [target.*], [badges]

        m
    });

/// 节名映射：Nu.toml -> Cargo.toml
pub static SECTION_NU_TO_CARGO: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| {
        let mut m = HashMap::new();
        // 反向映射
        for (cargo, nu) in SECTION_CARGO_TO_NU.iter() {
            m.insert(*nu, *cargo);
        }
        m
    });

/// 键名映射：Cargo.toml -> Nu.toml
pub static KEY_CARGO_TO_NU: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    // Package 字段
    m.insert("name", "id");
    m.insert("version", "v");
    m.insert("edition", "ed");
    m.insert("authors", "au");
    m.insert("description", "desc");
    m.insert("license", "lic");
    m.insert("license-file", "lic-file");
    m.insert("repository", "repo");
    m.insert("documentation", "doc");
    m.insert("homepage", "home");
    m.insert("keywords", "kw");
    m.insert("categories", "cat");
    m.insert("rust-version", "rv");

    // Workspace 字段
    m.insert("members", "m");
    m.insert("exclude", "ex");
    m.insert("resolver", "r");

    // 依赖字段
    m.insert("workspace", "w");
    m.insert("default-features", "df");
    m.insert("optional", "opt");
    m.insert("package", "pkg");

    // Lib 字段
    m.insert("proc-macro", "pm");
    m.insert("crate-type", "ct");

    // Bin/Example/Test 字段
    m.insert("required-features", "rf");

    // 保留不变的键（不在映射表中）
    // path, git, branch, tag, rev, features, readme, publish, include

    m
});

/// 键名映射：Nu.toml -> Cargo.toml
pub static KEY_NU_TO_CARGO: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    // 反向映射
    for (cargo, nu) in KEY_CARGO_TO_NU.iter() {
        m.insert(*nu, *cargo);
    }
    m
});

/// 需要保留不变的节前缀
pub static PRESERVED_SECTION_PREFIXES: &[&str] = &[
    "[profile.",
    "[patch.",
    "[replace]",
    "[target.",
    "[badges]",
    "[package.metadata.",
];

/// 需要保留不变的键
pub static PRESERVED_KEYS: &[&str] = &[
    "path", "git", "branch", "tag", "rev", "features", "readme", "publish", "include",
];

/// 转换节名：Cargo -> Nu
pub fn convert_section_cargo_to_nu(section: &str) -> String {
    // 检查是否为保留节
    for prefix in PRESERVED_SECTION_PREFIXES {
        if section.starts_with(prefix) {
            // workspace.metadata 特殊处理
            if section.starts_with("[workspace.metadata.") {
                return section.replace("[workspace.", "[W.");
            }
            return section.to_string();
        }
    }

    // 查找映射
    if let Some(nu_section) = SECTION_CARGO_TO_NU.get(section) {
        return nu_section.to_string();
    }

    // 处理动态节名（如 [workspace.metadata.xxx]）
    if section.starts_with("[workspace.") {
        return section.replace("[workspace.", "[W.");
    }

    // 未知节保持不变
    section.to_string()
}

/// 转换节名：Nu -> Cargo
pub fn convert_section_nu_to_cargo(section: &str) -> String {
    // 检查是否为保留节
    for prefix in PRESERVED_SECTION_PREFIXES {
        if section.starts_with(prefix) {
            return section.to_string();
        }
    }

    // 查找映射
    if let Some(cargo_section) = SECTION_NU_TO_CARGO.get(section) {
        return cargo_section.to_string();
    }

    // 处理动态节名（如 [W.metadata.xxx]）
    if section.starts_with("[W.") {
        return section.replace("[W.", "[workspace.");
    }

    // 未知节保持不变
    section.to_string()
}

/// 转换键名：Cargo -> Nu
pub fn convert_key_cargo_to_nu(key: &str) -> String {
    // 检查是否为保留键
    if PRESERVED_KEYS.contains(&key) {
        return key.to_string();
    }

    // 查找映射
    if let Some(nu_key) = KEY_CARGO_TO_NU.get(key) {
        return nu_key.to_string();
    }

    // 未知键保持不变
    key.to_string()
}

/// 转换键名：Nu -> Cargo
pub fn convert_key_nu_to_cargo(key: &str) -> String {
    // 检查是否为保留键
    if PRESERVED_KEYS.contains(&key) {
        return key.to_string();
    }

    // 查找映射
    if let Some(cargo_key) = KEY_NU_TO_CARGO.get(key) {
        return cargo_key.to_string();
    }

    // 未知键保持不变
    key.to_string()
}

/// 检查节是否应该保留不变
pub fn is_preserved_section(section: &str) -> bool {
    for prefix in PRESERVED_SECTION_PREFIXES {
        if section.starts_with(prefix) {
            return true;
        }
    }
    false
}

/// 检查键是否应该保留不变
pub fn is_preserved_key(key: &str) -> bool {
    PRESERVED_KEYS.contains(&key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_cargo_to_nu() {
        assert_eq!(convert_section_cargo_to_nu("[package]"), "[P]");
        assert_eq!(convert_section_cargo_to_nu("[workspace]"), "[W]");
        assert_eq!(convert_section_cargo_to_nu("[dependencies]"), "[D]");
        assert_eq!(convert_section_cargo_to_nu("[dev-dependencies]"), "[DD]");
        assert_eq!(convert_section_cargo_to_nu("[build-dependencies]"), "[BD]");
        assert_eq!(
            convert_section_cargo_to_nu("[workspace.dependencies]"),
            "[W.D]"
        );
        assert_eq!(convert_section_cargo_to_nu("[workspace.package]"), "[W.P]");
        assert_eq!(convert_section_cargo_to_nu("[lib]"), "[L]");
        assert_eq!(convert_section_cargo_to_nu("[features]"), "[FE]");
        assert_eq!(convert_section_cargo_to_nu("[[bin]]"), "[[B]]");
        assert_eq!(convert_section_cargo_to_nu("[[example]]"), "[[EX]]");
        assert_eq!(convert_section_cargo_to_nu("[[test]]"), "[[T]]");
        assert_eq!(convert_section_cargo_to_nu("[[bench]]"), "[[BE]]");
    }

    #[test]
    fn test_section_nu_to_cargo() {
        assert_eq!(convert_section_nu_to_cargo("[P]"), "[package]");
        assert_eq!(convert_section_nu_to_cargo("[W]"), "[workspace]");
        assert_eq!(convert_section_nu_to_cargo("[D]"), "[dependencies]");
        assert_eq!(convert_section_nu_to_cargo("[DD]"), "[dev-dependencies]");
        assert_eq!(convert_section_nu_to_cargo("[BD]"), "[build-dependencies]");
        assert_eq!(
            convert_section_nu_to_cargo("[W.D]"),
            "[workspace.dependencies]"
        );
        assert_eq!(convert_section_nu_to_cargo("[W.P]"), "[workspace.package]");
        assert_eq!(convert_section_nu_to_cargo("[L]"), "[lib]");
        assert_eq!(convert_section_nu_to_cargo("[FE]"), "[features]");
        assert_eq!(convert_section_nu_to_cargo("[[B]]"), "[[bin]]");
        assert_eq!(convert_section_nu_to_cargo("[[EX]]"), "[[example]]");
        assert_eq!(convert_section_nu_to_cargo("[[T]]"), "[[test]]");
        assert_eq!(convert_section_nu_to_cargo("[[BE]]"), "[[bench]]");
    }

    #[test]
    fn test_section_roundtrip() {
        let sections = vec![
            "[package]",
            "[workspace]",
            "[dependencies]",
            "[dev-dependencies]",
            "[build-dependencies]",
            "[workspace.dependencies]",
            "[workspace.package]",
            "[lib]",
            "[features]",
            "[[bin]]",
            "[[example]]",
            "[[test]]",
            "[[bench]]",
        ];

        for section in sections {
            let nu = convert_section_cargo_to_nu(section);
            let back = convert_section_nu_to_cargo(&nu);
            assert_eq!(section, back, "Roundtrip failed for {}", section);
        }
    }

    #[test]
    fn test_key_cargo_to_nu() {
        assert_eq!(convert_key_cargo_to_nu("name"), "id");
        assert_eq!(convert_key_cargo_to_nu("version"), "v");
        assert_eq!(convert_key_cargo_to_nu("edition"), "ed");
        assert_eq!(convert_key_cargo_to_nu("members"), "m");
        assert_eq!(convert_key_cargo_to_nu("exclude"), "ex");
        assert_eq!(convert_key_cargo_to_nu("resolver"), "r");
        assert_eq!(convert_key_cargo_to_nu("workspace"), "w");
        assert_eq!(convert_key_cargo_to_nu("default-features"), "df");
        assert_eq!(convert_key_cargo_to_nu("optional"), "opt");
    }

    #[test]
    fn test_key_nu_to_cargo() {
        assert_eq!(convert_key_nu_to_cargo("id"), "name");
        assert_eq!(convert_key_nu_to_cargo("v"), "version");
        assert_eq!(convert_key_nu_to_cargo("ed"), "edition");
        assert_eq!(convert_key_nu_to_cargo("m"), "members");
        assert_eq!(convert_key_nu_to_cargo("ex"), "exclude");
        assert_eq!(convert_key_nu_to_cargo("r"), "resolver");
        assert_eq!(convert_key_nu_to_cargo("w"), "workspace");
        assert_eq!(convert_key_nu_to_cargo("df"), "default-features");
        assert_eq!(convert_key_nu_to_cargo("opt"), "optional");
    }

    #[test]
    fn test_key_roundtrip() {
        let keys = vec![
            "name",
            "version",
            "edition",
            "members",
            "exclude",
            "resolver",
            "workspace",
            "default-features",
            "optional",
            "authors",
            "description",
            "license",
            "repository",
            "documentation",
            "homepage",
            "keywords",
            "categories",
            "proc-macro",
            "crate-type",
            "required-features",
        ];

        for key in keys {
            let nu = convert_key_cargo_to_nu(key);
            let back = convert_key_nu_to_cargo(&nu);
            assert_eq!(key, back, "Roundtrip failed for {}", key);
        }
    }

    #[test]
    fn test_preserved_sections() {
        // 保留节应该不变
        assert_eq!(
            convert_section_cargo_to_nu("[profile.release]"),
            "[profile.release]"
        );
        assert_eq!(
            convert_section_cargo_to_nu("[patch.crates-io]"),
            "[patch.crates-io]"
        );
        assert_eq!(
            convert_section_cargo_to_nu("[target.'cfg(windows)'.dependencies]"),
            "[target.'cfg(windows)'.dependencies]"
        );
        assert_eq!(convert_section_cargo_to_nu("[badges]"), "[badges]");

        assert!(is_preserved_section("[profile.release]"));
        assert!(is_preserved_section("[patch.crates-io]"));
        assert!(is_preserved_section("[target.'cfg(windows)'.dependencies]"));
        assert!(!is_preserved_section("[package]"));
    }

    #[test]
    fn test_preserved_keys() {
        // 保留键应该不变
        assert_eq!(convert_key_cargo_to_nu("path"), "path");
        assert_eq!(convert_key_cargo_to_nu("git"), "git");
        assert_eq!(convert_key_cargo_to_nu("branch"), "branch");
        assert_eq!(convert_key_cargo_to_nu("features"), "features");

        assert!(is_preserved_key("path"));
        assert!(is_preserved_key("git"));
        assert!(is_preserved_key("features"));
        assert!(!is_preserved_key("name"));
    }

    #[test]
    fn test_workspace_metadata_section() {
        // workspace.metadata 应该部分压缩
        assert_eq!(
            convert_section_cargo_to_nu("[workspace.metadata.spellcheck]"),
            "[W.metadata.spellcheck]"
        );
        assert_eq!(
            convert_section_nu_to_cargo("[W.metadata.spellcheck]"),
            "[workspace.metadata.spellcheck]"
        );
    }
}
