# Implementation Plan: Cargo Workspace 完整支持

## Overview

本实现计划将 Cargo Workspace 完整支持功能分解为可执行的编码任务。实现采用自底向上的方式，先构建核心数据结构和转换逻辑，再构建上层的项目管理和 CLI 功能。

## Tasks

- [x] 1. 核心数据结构和类型定义
  - [x] 1.1 创建 workspace 模块和核心类型
    - 创建 `src/workspace/mod.rs` 模块入口
    - 定义 `WorkspaceType` 枚举（Virtual、Mixed、Single）
    - 定义 `WorkspaceConfig` 结构体
    - 定义 `WorkspaceMember` 结构体
    - 定义 `SpecialDirs` 结构体
    - _Requirements: 1.1, 1.2, 1.3, 1.4_

  - [x] 1.2 创建 TOML 映射配置
    - 创建 `src/workspace/mapping.rs`
    - 实现节名映射表（HashMap）
    - 实现键名映射表（HashMap）
    - 实现双向查找函数
    - _Requirements: 3.1, 4.1, 7.1-7.7_

  - [x] 1.3 编写核心类型的单元测试
    - 测试 WorkspaceType 识别逻辑
    - 测试映射表的双向查找
    - _Requirements: 1.1-1.4_

- [x] 2. TOML 解析和转换引擎
  - [x] 2.1 实现 Cargo.toml 解析器
    - 创建 `src/workspace/cargo_parser.rs`
    - 实现 `parse_cargo_toml()` 函数
    - 支持 workspace 节解析
    - 支持 members/exclude 解析（单行和多行格式）
    - 支持 workspace.dependencies 解析
    - 支持 workspace.package 解析
    - 支持 workspace.lints 解析
    - _Requirements: 2.1, 2.2, 3.1, 4.2, 4.3, 4.4_

  - [x] 2.2 实现 Nu.toml 解析器
    - 创建 `src/workspace/nu_parser.rs`
    - 实现 `parse_nu_toml()` 函数
    - 支持 [W] 节解析
    - 支持 m/ex 解析
    - 支持 [W.D]、[W.P]、[W.lints] 解析
    - _Requirements: 1.4, 2.5, 3.4_

  - [x] 2.3 实现 Cargo2Nu TOML 转换器
    - 创建 `src/workspace/cargo2nu_toml.rs`
    - 实现 `Cargo2NuTomlConverter` 结构体
    - 实现节名转换逻辑
    - 实现键名转换逻辑
    - 实现依赖继承标记转换（workspace → w）
    - 保留不转换的节（profile、patch、target）
    - _Requirements: 3.1, 3.2, 3.3, 4.1, 4.4, 5.2, 7.1-7.7_

  - [x] 2.4 实现 Nu2Cargo TOML 转换器
    - 创建 `src/workspace/nu2cargo_toml.rs`
    - 实现 `Nu2CargoTomlConverter` 结构体
    - 实现反向节名转换
    - 实现反向键名转换
    - 实现依赖继承标记还原（w → workspace）
    - _Requirements: 3.4, 3.5, 4.5_

  - [x] 2.5 编写属性测试：TOML 节名双向转换一致性
    - **Property 3: TOML 节名双向转换一致性**
    - 使用 proptest 生成随机节名
    - 验证 cargo2nu → nu2cargo 往返一致
    - **Validates: Requirements 3.1, 3.4, 4.4, 7.1-7.7**

  - [x] 2.6 编写属性测试：键名双向转换一致性
    - **Property 4: 键名双向转换一致性**
    - 使用 proptest 生成随机键名
    - 验证转换往返一致
    - **Validates: Requirements 2.3, 3.2, 3.3, 3.5, 4.1**

- [x] 3. Checkpoint - 确保 TOML 转换测试通过
  - 运行所有测试，确保 TOML 转换逻辑正确
  - 如有问题请询问用户

- [x] 4. Workspace 分析器
  - [x] 4.1 实现 Cargo Workspace 分析器
    - 创建 `src/workspace/cargo_analyzer.rs`
    - 实现 `CargoWorkspaceAnalyzer` 结构体
    - 实现 workspace 类型检测
    - 实现成员路径提取
    - 实现 glob 模式展开
    - 实现成员路径验证
    - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.4_

  - [x] 4.2 实现 Nu Workspace 分析器
    - 创建 `src/workspace/nu_analyzer.rs`
    - 实现 `NuWorkspaceAnalyzer` 结构体
    - 实现 Nu workspace 类型检测
    - 实现成员路径提取
    - _Requirements: 1.4, 2.5_

  - [x] 4.3 实现循环依赖检测
    - 在分析器中添加 `detect_cycles()` 方法
    - 使用拓扑排序检测循环
    - 返回循环路径信息
    - _Requirements: 8.2_

  - [x] 4.4 编写属性测试：Workspace 类型识别正确性
    - **Property 1: Workspace 类型识别正确性**
    - 生成各种类型的 TOML 配置
    - 验证类型识别正确
    - **Validates: Requirements 1.1, 1.2, 1.3, 1.4**

  - [x] 4.5 编写属性测试：成员列表解析完整性
    - **Property 2: 成员列表解析完整性**
    - 生成随机成员列表（单行/多行格式）
    - 验证解析结果完整
    - **Validates: Requirements 2.1, 2.2, 2.5**

- [x] 5. 项目转换器
  - [x] 5.1 实现 Cargo2Nu 项目转换器
    - 重构 `src/bin/cargo2nu.rs`
    - 使用新的 workspace 模块
    - 实现递归成员转换
    - 实现特殊目录处理（tests、examples、benches）
    - 实现 build.rs → build.nu 转换
    - _Requirements: 6.1, 6.2, 6.3, 6.4_

  - [x] 5.2 实现 Nu2Cargo 项目转换器
    - 重构 `src/bin/nu2cargo.rs`
    - 使用新的 workspace 模块
    - 实现递归成员还原
    - 实现特殊目录还原
    - 实现 build.nu → build.rs 转换
    - _Requirements: 6.5_

  - [x] 5.3 实现路径依赖处理
    - 在转换器中添加路径依赖验证
    - 保留相对路径格式
    - 处理 patch.crates-io 节
    - _Requirements: 5.1, 5.2, 5.3, 5.4_

  - [x] 5.4 编写属性测试：路径依赖保留
    - **Property 6: 路径依赖保留**
    - 生成包含路径依赖的配置
    - 验证路径值不变
    - **Validates: Requirements 5.1, 5.3, 5.4**

  - [x] 5.5 编写属性测试：保留节不变性
    - **Property 7: 保留节不变性**
    - 生成包含 profile/patch/target 节的配置
    - 验证内容完全不变
    - **Validates: Requirements 5.2, 7.7, 4.2**

- [x] 6. Checkpoint - 确保项目转换测试通过
  - 运行所有测试，确保项目转换逻辑正确
  - 如有问题请询问用户

- [x] 7. 增量转换和配置文件处理
  - [x] 7.1 实现增量转换逻辑
    - 创建 `src/workspace/incremental.rs`
    - 实现文件时间戳比较
    - 实现增量转换决策逻辑
    - 支持 --force 强制转换
    - _Requirements: 9.1, 9.2, 9.3_

  - [x] 7.2 实现配置文件处理
    - 实现 .cargo/config.toml → .nu/config.toml 复制
    - 实现 rust-toolchain.toml 复制
    - 实现 Cargo.lock → Nu.lock 复制
    - 实现 .gitignore 扩展名更新
    - _Requirements: 10.1, 10.2, 10.3, 10.4_

  - [x] 7.3 编写属性测试：增量转换正确性
    - **Property 10: 增量转换正确性**
    - 模拟文件时间戳变化
    - 验证增量转换决策正确
    - **Validates: Requirements 9.1, 9.2**

  - [x] 7.4 编写属性测试：Gitignore 扩展名更新
    - **Property 13: Gitignore 扩展名更新**
    - 生成包含 .rs 规则的 gitignore
    - 验证转换后为 .nu 规则
    - **Validates: Requirements 10.3**

- [x] 8. 错误处理和报告
  - [x] 8.1 实现错误类型系统
    - 创建 `src/workspace/error.rs`
    - 定义 `WorkspaceError` 枚举
    - 实现错误恢复策略
    - _Requirements: 8.1, 8.2, 8.3_

  - [x] 8.2 实现转换报告生成
    - 创建 `ConvertReport` 结构体
    - 实现统计信息收集
    - 实现报告格式化输出
    - _Requirements: 8.4_

  - [x] 8.3 编写属性测试：错误处理健壮性
    - **Property 9: 错误处理健壮性**
    - 生成包含无效成员的配置
    - 验证警告输出和继续处理
    - **Validates: Requirements 8.1, 8.3**

- [x] 9. CLI 接口增强
  - [x] 9.1 增强 cargo2nu CLI
    - 添加 --verbose 选项
    - 添加 --dry-run 选项
    - 添加 --incremental 选项
    - 添加 --force 选项
    - 添加 --exclude 选项
    - 添加 --only 选项
    - 更新 --help 输出
    - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5_

  - [x] 9.2 增强 nu2cargo CLI
    - 添加相同的 CLI 选项
    - 保持与 cargo2nu 一致的接口
    - _Requirements: 12.1-12.5_

  - [x] 9.3 编写属性测试：Dry-run 无副作用
    - **Property 11: Dry-run 无副作用**
    - 执行 dry-run 转换
    - 验证无文件写入
    - **Validates: Requirements 12.3**

  - [x] 9.4 编写属性测试：排除/包含过滤正确性
    - **Property 12: 排除/包含过滤正确性**
    - 使用 exclude/only 选项
    - 验证结果符合过滤条件
    - **Validates: Requirements 12.4, 12.5**

- [x] 10. Checkpoint - 确保 CLI 测试通过
  - 运行所有测试，确保 CLI 功能正确
  - 如有问题请询问用户

- [x] 11. 往返转换测试
  - [x] 11.1 编写属性测试：往返转换语义等价性
    - **Property 8: 往返转换语义等价性**
    - 生成随机有效的 Cargo.toml
    - 执行 Cargo → Nu → Cargo 往返
    - 验证语义等价
    - **Validates: Requirements 11.1, 11.2, 11.3**

  - [x] 11.2 编写属性测试：依赖继承标记保留
    - **Property 5: 依赖继承标记保留**
    - 生成包含 workspace = true 的依赖
    - 验证继承关系和附加属性保留
    - **Validates: Requirements 3.2, 3.3, 3.5**

- [x] 12. 集成测试
  - [x] 12.1 创建集成测试框架
    - 创建 `tests/workspace_integration.rs`
    - 设置测试项目目录
    - 实现测试辅助函数
    - _Requirements: 11.1, 11.2_

  - [x] 12.2 添加 serde 项目转换测试
    - 转换 test_workspace_libs/serde
    - 验证结构完整性
    - 验证往返转换
    - _Requirements: 11.1_

  - [x] 12.3 添加 tokio 项目转换测试
    - 转换 test_workspace_libs/tokio
    - 验证结构完整性
    - 验证往返转换
    - _Requirements: 11.1_

  - [x] 12.4 添加简单 workspace 编译测试
    - 转换 test_workspace_simple
    - 执行 cargo build 验证
    - _Requirements: 11.1_

- [x] 13. Final Checkpoint - 确保所有测试通过
  - 运行完整测试套件
  - 验证所有属性测试通过
  - 验证集成测试通过
  - 如有问题请询问用户

- [x] 14. 文档和清理
  - [x] 14.1 更新 README.md
    - 添加 workspace 支持说明
    - 更新 CLI 用法示例
    - 添加 Nu.toml 格式文档
    - _Requirements: 12.1_

  - [x] 14.2 更新 WORKSPACE_SUPPORT.md
    - 更新实现状态
    - 添加新特性说明
    - 添加使用示例
    - _Requirements: 12.1_

  - [x] 14.3 代码清理
    - 移除废弃代码
    - 统一代码风格
    - 添加必要注释
    - _Requirements: N/A_

## Notes

- 所有任务都是必需的，确保完整的测试覆盖
- 每个 Checkpoint 用于验证阶段性成果
- 属性测试使用 `proptest` 库，每个测试至少运行 100 次迭代
- 集成测试使用真实的开源项目（tokio、serde）验证转换正确性
- 目标是完全支持 tokio 项目的双向转换
