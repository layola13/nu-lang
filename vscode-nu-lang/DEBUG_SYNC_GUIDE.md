# Nu Language Debug Sync Guide

## 功能概述

Nu Language 插件 v0.0.4 引入了**调试同步功能**，在调试 Nu 代码时自动打开双窗口视图：
- **左侧**：Rust 代码（调试器实际调试的代码）
- **右侧**：Nu 源代码（您编写的原始代码）

## 工作原理

### 为什么需要双窗口？

Nu 代码会被转换为 Rust 代码，然后编译成二进制文件。调试器（LLDB/GDB）只能调试 Rust 代码，因为：
1. 调试信息（DWARF）指向的是 Rust 源文件
2. 调试器需要解析源代码语法来设置断点
3. Nu 语法与 Rust 不同，调试器无法直接理解

### 双窗口解决方案

```
┌─────────────────────┬─────────────────────┐
│  hello.rs (Rust)    │  hello.nu (Nu)      │
│  [调试器在这里工作]   │  [您的源代码]        │
│                     │                     │
│  pub fn main() {    │  f main() {         │
│    println!(...);   │    println!(...);   │
│  }                  │  }                  │
│                     │                     │
│  ← 断点、单步执行     │  ← 同步高亮显示      │
└─────────────────────┴─────────────────────┘
```

## 使用方法

### 1. 准备工作

确保已安装调试器扩展（任选其一）：
- [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb)（推荐）
- [C/C++](https://marketplace.visualstudio.com/items?itemName=ms-vscode.cpptools)

### 2. 启动调试

1. 打开任意 `.nu` 文件（例如 `hello.nu`）
2. 按 **F5** 键启动调试
3. 插件会自动：
   - 将 `.nu` 转换为 `.rs`
   - 编译带调试信息的二进制文件
   - 打开双窗口视图（Rust 左，Nu 右）
   - 启动调试器

### 3. 调试操作

调试器在 **Rust 窗口（左侧）** 工作：

| 操作 | 快捷键 | 说明 |
|------|--------|------|
| 继续执行 | F5 | 运行到下一个断点 |
| 单步跳过 | F10 | 执行当前行，不进入函数 |
| 单步进入 | F11 | 进入函数内部 |
| 单步跳出 | Shift+F11 | 跳出当前函数 |
| 停止调试 | Shift+F5 | 结束调试会话 |

### 4. 设置断点

**在 Rust 文件（左侧）设置断点**：
- 点击行号左侧，出现红点即为断点
- 程序会在断点处停止执行

**查看对应的 Nu 代码**：
- 右侧 Nu 窗口会自动高亮对应的行
- 使用 SourceMap 实现精确映射

## 当前限制

### 1. 调试 Rust 代码，而非 Nu 代码

调试器实际调试的是生成的 Rust 代码，这意味着：
- ✅ 可以查看所有变量值
- ✅ 可以单步执行
- ✅ 可以查看调用栈
- ❌ 断点必须在 Rust 文件设置
- ❌ 变量名可能略有不同（如 `let` vs `l`）

### 2. SourceMap 映射

当前使用的是简化版 SourceMap（基于行号映射）：
- ✅ 可以映射函数、结构体等主要定义
- ⚠️ 行内代码映射可能不精确
- ⚠️ 宏展开的代码难以映射

### 3. 符号文件问题

如果调试器打开 `@___lldb_unnamed_symbol` 文件：
- **原因**：程序停在了 Rust 运行时启动代码（`_start`）
- **解决**：按 F10/F11 单步执行几次，进入 `main` 函数
- **或者**：在 `hello.rs` 的 `main` 函数第一行设置断点，然后按 F5 继续

## 高级配置

### 自定义调试配置

创建 `.vscode/launch.json`：

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Nu File",
      "program": "${workspaceFolder}/target/debug/your_binary",
      "args": ["arg1", "arg2"],
      "cwd": "${workspaceFolder}",
      "stopOnEntry": false,
      "sourceLanguages": ["rust"]
    }
  ]
}
```

### 禁用双窗口

如果只想调试 Rust 代码，可以在设置中禁用双窗口功能（未来版本将支持）。

## 故障排除

### 问题：调试器打开符号文件而非源码

**症状**：打开 `@___lldb_unnamed_symbol249` 等文件

**原因**：程序停在了二进制入口点 `_start`，而不是 `main` 函数

**解决方案 1**：单步执行
1. 按 F10（单步跳过）或 F11（单步进入）
2. 重复几次，直到进入 `main` 函数
3. 调试器会自动打开 `hello.rs`

**解决方案 2**：设置断点
1. 打开 `hello.rs` 文件
2. 在 `fn main()` 的第一行设置断点（点击行号左侧）
3. 按 F5 继续执行
4. 程序会停在断点处

### 问题：Nu 窗口没有高亮

**可能原因**：
1. SourceMap 文件不存在（`.rs.map`）
2. 调试会话还未启动

**解决方案**：
1. 确保编译时生成了 `.rs.map` 文件
2. 检查 `hello.rs.map` 是否存在
3. 重新启动调试会话（Shift+F5 然后 F5）

### 问题：调试器启动失败

**可能原因**：
1. 未安装调试器扩展
2. 编译失败
3. 二进制路径不正确

**解决方案**：
1. 安装 CodeLLDB 或 C/C++ 扩展
2. 检查输出面板的错误信息
3. 手动运行 `cargo build` 验证编译是否成功

## 未来改进

### 计划中的功能

1. **在 Nu 文件中直接设置断点**
   - 自动映射到对应的 Rust 代码行
   - 使用 SourceMap 实现精确映射

2. **变量名映射**
   - 在调试器中显示 Nu 变量名
   - 例如：`l x` 显示为 `let x`

3. **自定义 Debug Adapter**
   - 完全透明的 Nu 调试体验
   - 用户无需知道 Rust 的存在

4. **Source Map v2**
   - AST 级别的精确映射
   - 支持表达式级别的映射

## 参考资料

- [SOURCEMAP_IMPLEMENTATION.md](./SOURCEMAP_IMPLEMENTATION.md) - SourceMap 实现细节
- [DEBUG_GUIDE.md](./DEBUG_GUIDE.md) - 调试功能完整指南
- [HANDWRITING_FIRST_STRATEGY.md](./HANDWRITING_FIRST_STRATEGY.md) - 设计理念

## 反馈与贡献

遇到问题或有改进建议？请：
1. 查看 [TROUBLESHOOTING.md](./TROUBLESHOOTING.md)
2. 提交 Issue 到 GitHub
3. 加入社区讨论