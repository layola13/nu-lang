# VSCode Nu Lang 插件配置指南

## 🎯 快速开始

### 步骤 1: 确认插件已安装

打开 VSCode，检查插件是否已激活：
1. 打开任意 `.nu` 文件（如果没有，创建一个测试文件）
2. 查看状态栏右下角是否有 "Nu: Auto-compile OFF/ON" 按钮
3. 如果看到，说明插件已成功安装

### 步骤 2: 配置 nu2rust 和 rust2nu 路径

插件需要知道这两个 CLI 工具的位置。有两种配置方式：

#### 方式 A: 自动检测（推荐）

如果您已经编译了这些工具，将它们添加到系统 PATH：

```bash
# 进入项目目录
cd /home/sonygod/projects/nu

# 编译 release 版本
cargo build --release --bin nu2rust
cargo build --release --bin rust2nu

# 将二进制文件复制到系统路径
sudo cp target/release/nu2rust /usr/local/bin/
sudo cp target/release/rust2nu /usr/local/bin/

# 验证安装
which nu2rust
which rust2nu
```

插件会自动在以下路径搜索：
- `/usr/local/bin/`
- `/usr/bin/`
- `~/.cargo/bin/`
- 项目的 `target/release/` 目录

#### 方式 B: 手动配置（精确控制）

1. 打开 VSCode 设置（`Ctrl+,` 或 `Cmd+,`）
2. 搜索 "nu-lang"
3. 配置以下项：

**通过 UI 配置：**
- **Nu-lang: Nu2rust Path**: 设置为 `/home/sonygod/projects/nu/target/release/nu2rust`
- **Nu-lang: Cargo Path**: 保持默认 `cargo`（如果 cargo 在 PATH 中）

**或通过 settings.json 配置：**

按 `Ctrl+Shift+P`，输入 "Preferences: Open Settings (JSON)"，添加：

```json
{
  "nu-lang.nu2rustPath": "/home/sonygod/projects/nu/target/release/nu2rust",
  "nu-lang.cargoPath": "cargo",
  "nu-lang.autoCompile": true,
  "nu-lang.autoCheck": true
}
```

### 步骤 3: 验证配置

创建测试文件 `test.nu`：

```nu
// test.nu - Nu 语言测试文件
F add(a: i32, b: i32) -> i32 {
    < a + b
}

f main() {
    l result = add(5, 3)
    println!("Result: {}", result)
}
```

保存文件（`Ctrl+S`），观察：

1. **状态栏显示**: "Nu: Compiling..." → "Nu: Compiled ✓"
2. **生成文件**: 
   - `test.rs`（转换后的 Rust 代码）
   - `test.rs.map`（SourceMap 文件）
3. **如果有错误**: 会在 Nu 文件中显示红色波浪线

## 🔧 配置项详解

### 必需配置

| 配置项 | 说明 | 默认值 | 示例 |
|--------|------|--------|------|
| `nu-lang.nu2rustPath` | nu2rust 二进制路径 | `""` (自动检测) | `/usr/local/bin/nu2rust` |
| `nu-lang.cargoPath` | cargo 二进制路径 | `"cargo"` | `/usr/bin/cargo` |

### 可选配置

| 配置项 | 说明 | 默认值 |
|--------|------|--------|
| `nu-lang.autoCompile` | 保存时自动编译 | `true` |
| `nu-lang.autoCheck` | 编译后自动运行 cargo check | `true` |

## 🎮 使用命令

打开命令面板（`Ctrl+Shift+P`），输入 "Nu"：

### 可用命令

1. **Nu: Compile Current File**
   - 手动编译当前 .nu 文件
   - 快捷键：无（可自定义）

2. **Nu: Check Rust Output**
   - 运行 cargo check 检查生成的 Rust 代码
   - 会映射错误到 Nu 源码位置

3. **Nu: Toggle Auto Compile**
   - 开启/关闭自动编译功能
   - 或点击状态栏按钮

## 🐛 故障排除

### 问题 1: 状态栏没有 "Nu: Auto-compile" 按钮

**原因**: 插件未激活

**解决**:
1. 打开任意 `.nu` 文件激活插件
2. 检查 VSCode 输出面板（Output → Nu Language）查看错误
3. 重启 VSCode

### 问题 2: 保存文件没有生成 .rs 文件

**可能原因**:
- nu2rust 路径配置错误
- nu2rust 没有执行权限
- 自动编译被关闭

**解决步骤**:

1. **检查 nu2rust 是否可执行**:
   ```bash
   # 测试命令
   /home/sonygod/projects/nu/target/release/nu2rust --version
   
   # 如果没有权限
   chmod +x /home/sonygod/projects/nu/target/release/nu2rust
   ```

2. **检查 VSCode 输出**:
   - 打开 "Output" 面板
   - 选择 "Nu Language" 频道
   - 查看错误信息

3. **手动测试编译**:
   ```bash
   cd /home/sonygod/projects/nu
   ./target/release/nu2rust test.nu --sourcemap -v
   ```

### 问题 3: 错误没有映射到 Nu 文件

**原因**: SourceMap 文件缺失或格式错误

**解决**:
1. 检查 `.rs.map` 文件是否存在
2. 查看 .map 文件内容是否为有效 JSON
3. 重新编译：删除 .rs 和 .rs.map，保存 .nu 文件

### 问题 4: cargo check 失败

**原因**: cargo 未安装或不在 PATH 中

**解决**:
```bash
# 检查 cargo
which cargo

# 如果未安装
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 重新加载环境
source $HOME/.cargo/env
```

## 📊 工作流程示例

### 场景 1: 编写新的 Nu 代码

```bash
# 1. 创建 Nu 文件
touch my_app.nu

# 2. 在 VSCode 中打开
code my_app.nu

# 3. 编写代码
```

```nu
// my_app.nu
F calculate(x: i32) -> i32 {
    l doubled = x * 2
    < doubled + 10
}

f main() {
    l result = calculate(5)
    println!("Result: {}", result)
}
```

```bash
# 4. 保存（Ctrl+S）
# → 自动生成 my_app.rs 和 my_app.rs.map
# → 自动运行 cargo check
# → 如果有错误，显示在 Nu 编辑器中

# 5. 编译运行 Rust 版本
rustc my_app.rs -o my_app
./my_app
```

### 场景 2: 修复编译错误

假设您写了这样的代码：

```nu
F add(a: i32, b: i32) -> Str {  // 错误：返回类型不匹配
    < a + b
}
```

**插件行为**:
1. 保存文件
2. 生成 .rs 文件
3. cargo check 报错："expected `String`, found `i32`"
4. 插件读取 .rs.map
5. 映射错误到 Nu 代码第 2 行
6. 显示红色波浪线和错误信息

**修复**:
```nu
F add(a: i32, b: i32) -> i32 {  // 修正：返回 i32
    < a + b
}
```

保存后，红色波浪线消失。

## 🔍 查看生成的文件

### SourceMap 示例

`test.rs.map`:
```json
{
  "nu_file": "test.nu",
  "rust_file": "test.rs",
  "line_map": [
    [1, 1],   // Rust 第 1 行 → Nu 第 1 行
    [2, 2],
    [3, 3],
    [6, 6],
    [7, 7]
  ]
}
```

### 使用 SourceMap

当 cargo 报错："error at test.rs:7:5"

插件查询 line_map：
- 找到 `[7, 7]`
- 映射到 Nu 文件第 7 行
- 在该行显示错误

## 💡 高级技巧

### 技巧 1: 快速切换自动编译

点击状态栏的 "Nu: Auto-compile ON/OFF" 按钮即可切换。

### 技巧 2: 查看详细日志

1. 打开 Output 面板（`Ctrl+Shift+`）
2. 选择 "Nu Language"
3. 查看编译和检查过程的详细信息

### 技巧 3: 自定义键绑定

在 `keybindings.json` 中添加：

```json
[
  {
    "key": "ctrl+alt+c",
    "command": "nu-lang.compile",
    "when": "editorLangId == nu"
  },
  {
    "key": "ctrl+alt+k",
    "command": "nu-lang.check",
    "when": "editorLangId == nu"
  }
]
```

## 📚 相关文档

- [SETUP.md](./SETUP.md) - 详细的安装和开发指南
- [TEST_REPORT.md](./TEST_REPORT.md) - 功能测试报告
- [ARCHITECTURE.md](./ARCHITECTURE.md) - 技术架构文档
- [SOURCEMAP_IMPLEMENTATION.md](./SOURCEMAP_IMPLEMENTATION.md) - SourceMap 实现细节

## 🎯 下一步

配置完成后，您可以：

1. ✅ 手写 Nu 代码，享受高密度编程
2. ✅ 自动转换为 Rust，无需手动操作
3. ✅ 实时看到 Rust 编译器错误，精确定位到 Nu 代码
4. ✅ 使用 Rust 生态工具链（cargo check, clippy, rustfmt）

**开始使用吧！** 🚀