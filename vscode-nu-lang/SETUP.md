# VSCode Nu Language Extension - Setup Guide

## 项目结构

```
vscode-nu-lang/
├── src/                          # TypeScript 源代码
│   ├── extension.ts             # 插件入口
│   ├── services/                # 核心服务
│   │   ├── binaryManager.ts     # 二进制路径管理
│   │   ├── conversionService.ts # Nu -> Rust 转换服务
│   │   ├── sourcemapService.ts  # SourceMap 处理
│   │   └── cargoService.ts      # Cargo check 集成
│   ├── features/                # 功能模块
│   │   ├── autoCompile.ts       # 自动编译功能
│   │   └── errorMapper.ts       # 错误映射功能
│   └── ui/                      # UI 组件
│       └── statusBar.ts         # 状态栏管理
├── out/                         # 编译输出 (JavaScript)
├── syntaxes/                    # 语法高亮定义
├── package.json                 # 插件配置和依赖
├── tsconfig.json                # TypeScript 配置
└── language-configuration.json  # 语言配置
```

## 已实现的功能

### 1. 核心服务模块

- ✅ **BinaryManager**: 自动检测和管理 `nu2rust` 和 `cargo` 二进制路径
- ✅ **ConversionService**: 调用 `nu2rust` CLI 转换 `.nu` 文件为 `.rs` 文件
- ✅ **SourcemapService**: 加载和查询 `.map` 文件，支持 Rust ↔ Nu 行号映射
- ✅ **CargoService**: 运行 `cargo check` 并解析 JSON 格式的错误输出

### 2. 功能模块

- ✅ **AutoCompile**: 
  - 监听 `.nu` 文件保存事件
  - 自动调用 `nu2rust` 编译
  - 可配置是否自动运行 `cargo check`
  - 支持手动编译命令

- ✅ **ErrorMapper**:
  - 解析 `cargo check` 的错误输出
  - 使用 SourceMap 将 Rust 错误映射回 Nu 源码
  - 在编辑器中显示红色波浪线和错误信息
  - 支持错误、警告和提示的不同级别

### 3. UI 组件

- ✅ **StatusBar**: 
  - 显示编译状态（编译中、成功、失败）
  - 显示自动编译开关状态
  - 支持点击切换自动编译

### 4. VSCode 命令

- ✅ `Nu: Compile Current File` - 手动编译当前 `.nu` 文件
- ✅ `Nu: Check Rust Output` - 对生成的 Rust 代码运行 cargo check
- ✅ `Nu: Toggle Auto Compile` - 切换自动编译功能

### 5. 配置项

- ✅ `nu-lang.nu2rustPath` - nu2rust 二进制路径（空则自动检测）
- ✅ `nu-lang.cargoPath` - cargo 二进制路径（默认: cargo）
- ✅ `nu-lang.autoCompile` - 是否自动编译（默认: true）
- ✅ `nu-lang.autoCheck` - 编译后是否自动运行 cargo check（默认: true）

## 编译和安装

### 1. 安装依赖

```bash
cd vscode-nu-lang
npm install
```

### 2. 编译 TypeScript

```bash
npm run compile
```

### 3. 在 VSCode 中调试

1. 打开 `vscode-nu-lang` 文件夹
2. 按 F5 启动调试
3. 在新的 VSCode 窗口中打开包含 `.nu` 文件的项目
4. 编辑并保存 `.nu` 文件，观察自动编译功能

### 4. 打包安装

```bash
# 安装 vsce（如果还没安装）
npm install -g @vscode/vsce

# 打包插件
vsce package

# 安装生成的 .vsix 文件
code --install-extension nu-lang-0.0.1.vsix
```

## 使用说明

### 基本工作流程

1. **编写 Nu 代码**: 创建 `.nu` 文件并编写代码
2. **自动编译**: 保存文件时自动编译为 `.rs` 和 `.rs.map`
3. **错误检查**: 自动运行 `cargo check` 检查 Rust 代码
4. **错误显示**: 错误通过 SourceMap 映射回 Nu 源码，显示红色波浪线

### 配置 nu2rust 路径

如果 `nu2rust` 不在系统 PATH 中，需要配置路径：

```json
{
  "nu-lang.nu2rustPath": "/path/to/nu2rust"
}
```

插件会自动搜索以下位置：
1. 配置的路径
2. 当前项目的 `target/release/nu2rust`
3. 当前项目的 `target/debug/nu2rust`
4. 系统 PATH 中的 `nu2rust`

### 状态栏指示器

- `$(check) Nu: Auto` - 自动编译已启用
- `$(circle-slash) Nu: Manual` - 自动编译已禁用
- `$(sync~spin) Nu: Compiling...` - 正在编译
- `$(check) Nu: Compiled successfully` - 编译成功
- `$(error) Nu: Compilation failed` - 编译失败
- `$(warning) Nu: 3 errors` - 有错误或警告

## 技术实现要点

### 1. 文件监听

使用 `vscode.workspace.onDidSaveTextDocument` 监听保存事件，确保只处理 `.nu` 文件。

### 2. 子进程调用

使用 Node.js 的 `child_process.exec` 执行 `nu2rust` 和 `cargo` 命令，支持大输出缓冲区。

### 3. SourceMap 解析

读取 `.rs.map` JSON 文件，建立 Rust 行号到 Nu 行号的映射关系。

### 4. 错误映射算法

1. 解析 `cargo check --message-format=json` 输出
2. 提取错误的 Rust 文件位置（行号、列号）
3. 使用 SourceMap 查找对应的 Nu 位置
4. 创建 VSCode Diagnostic 对象
5. 通过 DiagnosticCollection 显示在编辑器中

### 5. 诊断信息管理

使用 `vscode.languages.createDiagnosticCollection` 管理错误显示，支持：
- 错误级别（Error、Warning、Information）
- 相关信息（Related Information）
- 错误代码（Error Code）

## 测试验证

### 1. 测试自动编译

1. 创建 `test.nu` 文件
2. 保存文件
3. 检查是否生成 `test.rs` 和 `test.rs.map`
4. 查看状态栏显示

### 2. 测试错误映射

1. 创建包含错误的 `.nu` 文件
2. 保存触发编译
3. 观察错误是否正确显示在源码位置
4. 悬停错误查看详细信息

### 3. 测试命令

1. 打开命令面板 (Ctrl+Shift+P / Cmd+Shift+P)
2. 搜索 "Nu:" 查看所有命令
3. 测试每个命令的功能

## 下一步开发

### 待实现功能

1. **语法检查**: 在编译前进行基本的 Nu 语法检查
2. **代码补全**: 提供 Nu 语法的智能补全
3. **悬停提示**: 显示变量和函数的类型信息
4. **跳转定义**: 支持跳转到定义和引用
5. **重构支持**: 重命名、提取函数等重构操作
6. **调试支持**: 集成调试器，支持断点和单步执行

### 性能优化

1. **增量编译**: 只编译修改的部分
2. **缓存优化**: 缓存 SourceMap 和编译结果
3. **并发编译**: 支持多文件并行编译
4. **智能触发**: 只在有实际修改时触发编译

## 故障排除

### 问题: 插件无法加载

**解决方案**: 
- 检查 `out/extension.js` 是否存在
- 运行 `npm run compile` 重新编译
- 查看 VSCode 开发者工具的控制台错误

### 问题: 找不到 nu2rust

**解决方案**:
- 确保 `nu2rust` 已编译（运行 `cargo build --release`）
- 配置 `nu-lang.nu2rustPath` 指向正确路径
- 检查二进制是否有执行权限

### 问题: 错误位置不正确

**解决方案**:
- 检查 `.rs.map` 文件是否生成
- 验证 SourceMap 格式是否正确
- 查看 nu2rust 是否正确生成 SourceMap

### 问题: cargo check 失败

**解决方案**:
- 确保 Rust 和 Cargo 已安装
- 检查生成的 `.rs` 文件是否有效
- 验证 Cargo.toml 配置是否正确

## 贡献指南

欢迎贡献代码！请遵循以下步骤：

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## 许可证

本项目采用 MIT 许可证。