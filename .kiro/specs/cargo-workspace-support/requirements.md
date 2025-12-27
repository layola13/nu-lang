# Requirements Document

## Introduction

本文档定义了 Nu 语言编译器中 `cargo2nu` 和 `nu2cargo` 工具对 Cargo Workspace 完整支持的需求规范。

当前实现已支持基本的 workspace 结构转换，但缺少对许多高级 Cargo Workspace 特性的支持。本需求旨在实现完整的双向转换能力，确保复杂的 Rust 生态系统项目（如 serde、tokio、clap 等）能够无损地在 Cargo 和 Nu 项目格式之间转换。

## Glossary

- **Cargo_Workspace**: Cargo 的多包项目管理机制，允许在一个根目录下管理多个相关的 crate
- **Nu_Workspace**: Nu 语言对应的多包项目管理机制，使用 Nu.toml 配置
- **Workspace_Root**: 包含 `[workspace]` 或 `[W]` 配置的根目录
- **Workspace_Member**: workspace 中的单个 crate/包
- **Virtual_Workspace**: 仅包含 workspace 配置而不包含 `[package]` 的根 Cargo.toml
- **Mixed_Workspace**: 同时包含 `[workspace]` 和 `[package]` 的根 Cargo.toml
- **Cargo2Nu_Converter**: 将 Cargo 项目转换为 Nu 项目的工具
- **Nu2Cargo_Converter**: 将 Nu 项目转换回 Cargo 项目的工具
- **Path_Dependency**: 使用本地路径引用的依赖项
- **Workspace_Inheritance**: 成员包从 workspace 继承配置的机制

## Requirements

### Requirement 1: Workspace 结构检测与识别

**User Story:** 作为开发者，我希望工具能自动识别各种 workspace 结构，以便正确处理不同类型的项目。

#### Acceptance Criteria

1. WHEN Cargo2Nu_Converter 遇到包含 `[workspace]` 节的 Cargo.toml THEN THE Cargo2Nu_Converter SHALL 识别该项目为 workspace 项目
2. WHEN Cargo2Nu_Converter 遇到同时包含 `[workspace]` 和 `[package]` 的 Cargo.toml THEN THE Cargo2Nu_Converter SHALL 识别该项目为 Mixed_Workspace
3. WHEN Cargo2Nu_Converter 遇到仅包含 `[workspace]` 而无 `[package]` 的 Cargo.toml THEN THE Cargo2Nu_Converter SHALL 识别该项目为 Virtual_Workspace
4. WHEN Nu2Cargo_Converter 遇到包含 `[W]` 节的 Nu.toml THEN THE Nu2Cargo_Converter SHALL 识别该项目为 Nu_Workspace 项目

### Requirement 2: Workspace 成员解析

**User Story:** 作为开发者，我希望工具能正确解析所有 workspace 成员配置格式，以便完整转换项目。

#### Acceptance Criteria

1. WHEN Cargo2Nu_Converter 解析 `members = ["a", "b"]` 单行格式 THEN THE Cargo2Nu_Converter SHALL 正确提取所有成员路径
2. WHEN Cargo2Nu_Converter 解析多行 members 数组格式 THEN THE Cargo2Nu_Converter SHALL 正确提取所有成员路径
3. WHEN Cargo2Nu_Converter 遇到 `exclude = [...]` 配置 THEN THE Cargo2Nu_Converter SHALL 保留排除列表并转换为 `ex = [...]`
4. WHEN Cargo2Nu_Converter 遇到 glob 模式的成员路径（如 `"crates/*"`）THEN THE Cargo2Nu_Converter SHALL 展开 glob 并识别所有匹配的成员目录
5. WHEN Nu2Cargo_Converter 解析 `m = [...]` 格式 THEN THE Nu2Cargo_Converter SHALL 正确还原为 `members = [...]`

### Requirement 3: Workspace 依赖继承

**User Story:** 作为开发者，我希望 workspace 依赖继承机制能被完整保留，以便成员包能正确引用共享依赖。

#### Acceptance Criteria

1. WHEN Cargo2Nu_Converter 遇到 `[workspace.dependencies]` 节 THEN THE Cargo2Nu_Converter SHALL 转换为 `[W.D]` 节
2. WHEN Cargo2Nu_Converter 遇到成员包中的 `dep.workspace = true` THEN THE Cargo2Nu_Converter SHALL 转换为 `dep.w = true`
3. WHEN Cargo2Nu_Converter 遇到 `dep = { workspace = true, features = [...] }` THEN THE Cargo2Nu_Converter SHALL 转换为 `dep = { w = true, features = [...] }`
4. WHEN Nu2Cargo_Converter 遇到 `[W.D]` 节 THEN THE Nu2Cargo_Converter SHALL 还原为 `[workspace.dependencies]`
5. WHEN Nu2Cargo_Converter 遇到 `dep.w = true` THEN THE Nu2Cargo_Converter SHALL 还原为 `dep.workspace = true`

### Requirement 4: Workspace 元数据配置

**User Story:** 作为开发者，我希望 workspace 级别的元数据配置能被保留，以便项目配置完整性。

#### Acceptance Criteria

1. WHEN Cargo2Nu_Converter 遇到 `resolver = "2"` THEN THE Cargo2Nu_Converter SHALL 转换为 `r = "2"`
2. WHEN Cargo2Nu_Converter 遇到 `[workspace.metadata.*]` 节 THEN THE Cargo2Nu_Converter SHALL 保留为 `[W.metadata.*]`
3. WHEN Cargo2Nu_Converter 遇到 `[workspace.lints.*]` 节 THEN THE Cargo2Nu_Converter SHALL 转换为 `[W.lints.*]`
4. WHEN Cargo2Nu_Converter 遇到 `[workspace.package]` 节 THEN THE Cargo2Nu_Converter SHALL 转换为 `[W.P]`
5. WHEN Nu2Cargo_Converter 遇到上述 Nu 格式 THEN THE Nu2Cargo_Converter SHALL 正确还原为对应的 Cargo 格式

### Requirement 5: 路径依赖处理

**User Story:** 作为开发者，我希望 workspace 内的路径依赖能被正确处理，以便成员间的依赖关系保持正确。

#### Acceptance Criteria

1. WHEN Cargo2Nu_Converter 遇到 `dep = { path = "../member" }` THEN THE Cargo2Nu_Converter SHALL 保留路径依赖格式
2. WHEN Cargo2Nu_Converter 遇到 `[patch.crates-io]` 节 THEN THE Cargo2Nu_Converter SHALL 转换为 `[patch.crates-io]` 保持不变
3. WHEN Cargo2Nu_Converter 遇到相对路径依赖 THEN THE Cargo2Nu_Converter SHALL 验证路径有效性并保留相对路径
4. WHEN Nu2Cargo_Converter 转换路径依赖 THEN THE Nu2Cargo_Converter SHALL 确保路径在新项目结构中仍然有效

### Requirement 6: 特殊目录处理

**User Story:** 作为开发者，我希望 workspace 中的特殊目录（tests、examples、benches）能被正确处理。

#### Acceptance Criteria

1. WHEN Cargo2Nu_Converter 遇到 `tests/` 目录 THEN THE Cargo2Nu_Converter SHALL 递归转换所有 `.rs` 文件为 `.nu` 文件
2. WHEN Cargo2Nu_Converter 遇到 `examples/` 目录 THEN THE Cargo2Nu_Converter SHALL 递归转换所有 `.rs` 文件为 `.nu` 文件
3. WHEN Cargo2Nu_Converter 遇到 `benches/` 目录 THEN THE Cargo2Nu_Converter SHALL 递归转换所有 `.rs` 文件为 `.nu` 文件
4. WHEN Cargo2Nu_Converter 遇到 `build.rs` 文件 THEN THE Cargo2Nu_Converter SHALL 转换为 `build.nu` 文件
5. WHEN Nu2Cargo_Converter 遇到上述 Nu 格式目录和文件 THEN THE Nu2Cargo_Converter SHALL 正确还原为 Rust 格式

### Requirement 7: Cargo.toml 高级配置

**User Story:** 作为开发者，我希望 Cargo.toml 中的高级配置能被完整保留，以便项目功能完整。

#### Acceptance Criteria

1. WHEN Cargo2Nu_Converter 遇到 `[[bin]]` 节 THEN THE Cargo2Nu_Converter SHALL 转换为 `[[B]]` 节
2. WHEN Cargo2Nu_Converter 遇到 `[[example]]` 节 THEN THE Cargo2Nu_Converter SHALL 转换为 `[[EX]]` 节
3. WHEN Cargo2Nu_Converter 遇到 `[[test]]` 节 THEN THE Cargo2Nu_Converter SHALL 转换为 `[[T]]` 节
4. WHEN Cargo2Nu_Converter 遇到 `[[bench]]` 节 THEN THE Cargo2Nu_Converter SHALL 转换为 `[[BE]]` 节
5. WHEN Cargo2Nu_Converter 遇到 `[lib]` 节 THEN THE Cargo2Nu_Converter SHALL 转换为 `[L]` 节
6. WHEN Cargo2Nu_Converter 遇到 `[features]` 节 THEN THE Cargo2Nu_Converter SHALL 转换为 `[FE]` 节
7. WHEN Cargo2Nu_Converter 遇到 `[profile.*]` 节 THEN THE Cargo2Nu_Converter SHALL 保留为 `[profile.*]`

### Requirement 8: 错误处理与验证

**User Story:** 作为开发者，我希望工具能提供清晰的错误信息和验证，以便快速定位问题。

#### Acceptance Criteria

1. IF Cargo2Nu_Converter 遇到无效的 workspace 成员路径 THEN THE Cargo2Nu_Converter SHALL 输出警告信息并继续处理其他成员
2. IF Cargo2Nu_Converter 遇到循环依赖 THEN THE Cargo2Nu_Converter SHALL 输出错误信息并终止转换
3. IF Nu2Cargo_Converter 遇到无效的 Nu.toml 格式 THEN THE Nu2Cargo_Converter SHALL 输出详细错误信息指明问题位置
4. WHEN 转换完成 THEN THE Cargo2Nu_Converter SHALL 输出转换统计信息（成员数、文件数、成功/失败数）

### Requirement 9: 增量转换支持

**User Story:** 作为开发者，我希望能够增量转换 workspace，以便只更新变化的部分。

#### Acceptance Criteria

1. WHEN Cargo2Nu_Converter 使用 `--incremental` 选项 THEN THE Cargo2Nu_Converter SHALL 仅转换自上次转换后修改的文件
2. WHEN Cargo2Nu_Converter 检测到源文件比目标文件新 THEN THE Cargo2Nu_Converter SHALL 重新转换该文件
3. WHEN Cargo2Nu_Converter 使用 `--force` 选项 THEN THE Cargo2Nu_Converter SHALL 强制重新转换所有文件

### Requirement 10: 配置文件保留

**User Story:** 作为开发者，我希望项目中的非代码配置文件能被保留，以便项目完整性。

#### Acceptance Criteria

1. WHEN Cargo2Nu_Converter 遇到 `.cargo/config.toml` THEN THE Cargo2Nu_Converter SHALL 复制到输出目录的 `.nu/config.toml`
2. WHEN Cargo2Nu_Converter 遇到 `rust-toolchain.toml` THEN THE Cargo2Nu_Converter SHALL 复制到输出目录
3. WHEN Cargo2Nu_Converter 遇到 `.gitignore` THEN THE Cargo2Nu_Converter SHALL 复制并更新文件扩展名规则
4. WHEN Cargo2Nu_Converter 遇到 `Cargo.lock` THEN THE Cargo2Nu_Converter SHALL 复制为 `Nu.lock`

### Requirement 11: 双向转换一致性

**User Story:** 作为开发者，我希望 Cargo → Nu → Cargo 的往返转换能保持语义一致性。

#### Acceptance Criteria

1. FOR ALL 有效的 Cargo workspace 项目，Cargo2Nu_Converter 转换后再用 Nu2Cargo_Converter 转换回来 SHALL 产生语义等价的项目结构
2. FOR ALL 有效的 Nu workspace 项目，Nu2Cargo_Converter 转换后再用 Cargo2Nu_Converter 转换回来 SHALL 产生语义等价的项目结构
3. WHEN 执行往返转换 THEN THE 转换器 SHALL 保留所有依赖版本、features 和配置选项

### Requirement 12: CLI 接口增强

**User Story:** 作为开发者，我希望有更丰富的命令行选项，以便灵活控制转换过程。

#### Acceptance Criteria

1. WHEN 用户执行 `cargo2nu --help` THEN THE Cargo2Nu_Converter SHALL 显示所有可用选项和用法示例
2. WHEN 用户指定 `--verbose` 选项 THEN THE Cargo2Nu_Converter SHALL 输出详细的转换过程信息
3. WHEN 用户指定 `--dry-run` 选项 THEN THE Cargo2Nu_Converter SHALL 仅显示将要执行的操作而不实际执行
4. WHEN 用户指定 `--exclude <pattern>` 选项 THEN THE Cargo2Nu_Converter SHALL 跳过匹配的成员或文件
5. WHEN 用户指定 `--only <members>` 选项 THEN THE Cargo2Nu_Converter SHALL 仅转换指定的成员
