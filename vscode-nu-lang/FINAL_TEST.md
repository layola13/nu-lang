# VSCode Nu Lang 插件 - 最终测试指南

## 当前状态

✅ **插件版本**: 0.0.2 (71.37KB)
✅ **nu2rust CLI**: 已安装 `/usr/local/bin/nu2rust`
✅ **CLI 功能**: 已验证可生成 .rs 和 .rs.map 文件
✅ **插件已安装**: `nu-lang.nu-lang`

## 测试步骤

### 1. 重新加载 VSCode

**这一步非常重要！**

```
Ctrl+Shift+P → 输入 "Reload Window" → 回车
```

或者完全重启 VSCode。

### 2. 打开测试文件

```bash
code /home/sonygod/projects/nu/temp_examples_nu/hello.nu
```

### 3. 检查插件激活

**检查点 A: 状态栏**
- 右下角应该显示 "Nu: Auto-compile ON" 或 "Nu: Auto-compile OFF"
- 如果看到这个按钮，说明插件已激活 ✅

**检查点 B: 语言标识**
- 右下角应该显示 "Nu" 而不是 "Plain Text"
- 如果显示 "Plain Text"，点击它并选择 "Nu"

**检查点 C: 命令面板**
```
Ctrl+Shift+P → 输入 "Nu:"
应该看到：
- Nu: Compile Current File
- Nu: Check Rust Output
- Nu: Toggle Auto Compile
```

### 4. 测试自动编译

**步骤**:
1. 在 hello.nu 中添加一个注释：
   ```nu
   // Test comment
   ```

2. 按 `Ctrl+S` 保存

3. 打开输出面板查看日志：
   ```
   Ctrl+Shift+U
   选择下拉菜单中的 "Nu Language"
   ```

4. 检查文件系统：
   ```bash
   ls -l temp_examples_nu/hello.rs*
   ```
   
   应该看到：
   - `hello.rs` (最新时间戳)
   - `hello.rs.map` (最新时间戳)

### 5. 测试手动编译

如果自动编译不工作，尝试手动命令：

```
Ctrl+Shift+P → "Nu: Compile Current File"
```

查看输出面板的错误信息。

### 6. 检查配置

打开设置：`Ctrl+,` → 搜索 "nu-lang"

应该看到：
- ✅ Nu-lang: Nu2rust Path (可选，留空自动检测)
- ✅ Nu-lang: Cargo Path (默认: cargo)
- ✅ Nu-lang: Auto Compile (默认: true)
- ✅ Nu-lang: Auto Check (默认: true)

### 7. 调试方法

如果插件仍然不工作，打开 Developer Tools:

```
Help → Toggle Developer Tools
或 Ctrl+Shift+I
```

在 Console 标签页查看错误信息。

## 预期结果

### ✅ 成功标志

1. **状态栏按钮**: 
   - 看到 "Nu: Auto-compile ON"
   - 点击可切换开关

2. **文件生成**:
   ```bash
   $ ls -lh temp_examples_nu/hello.rs*
   -rw-r--r-- 1 sonygod sonygod 558 Dec 26 10:XX hello.rs
   -rw-r--r-- 1 sonygod sonygod 860 Dec 26 10:XX hello.rs.map
   ```

3. **输出日志**:
   ```
   [Nu Language] File saved: temp_examples_nu/hello.nu
   [Nu Language] Compiling: temp_examples_nu/hello.nu
   [Nu Language] Command: nu2rust temp_examples_nu/hello.nu -o temp_examples_nu/hello.rs --sourcemap -f
   [Nu Language] Compilation successful
   [Nu Language] Running cargo check...
   ```

4. **错误映射**:
   - 如果 Nu 代码有错误
   - 应该在 Nu 编辑器中看到红色波浪线
   - 鼠标悬停显示 Rust 编译器错误

### ❌ 失败标志

1. **No状态栏按钮**: 插件未激活
2. **保存后无反应**: 自动编译未触发
3. **输出面板无日志**: 服务未运行
4. **文件未更新**: CLI 调用失败

## 故障排除

### 问题 1: 插件未激活

**解决方案**:
```bash
# 1. 确认插件安装
code --list-extensions | grep nu-lang

# 2. 检查插件目录
ls -l ~/.vscode-server/extensions/nu-lang.nu-lang-*/

# 3. 重新安装
cd /home/sonygod/projects/nu/vscode-nu-lang
code --install-extension nu-lang-0.0.2.vsix --force

# 4. 完全重启 VSCode
```

### 问题 2: nu2rust not found

**解决方案**:
```bash
# 确认 nu2rust 可用
which nu2rust
nu2rust --help

# 如果找不到，重新复制
sudo cp /home/sonygod/projects/nu/target/release/nu2rust /usr/local/bin/
sudo chmod +x /usr/local/bin/nu2rust
```

### 问题 3: 文件权限错误

**解决方案**:
```bash
# 检查目录权限
ls -ld temp_examples_nu/

# 如果需要，修复权限
chmod 755 temp_examples_nu/
chmod 644 temp_examples_nu/*.nu
```

### 问题 4: 插件代码问题

**解决方案**:
```bash
# 检查编译输出
ls -l vscode-nu-lang/out/

# 重新编译
cd vscode-nu-lang
npm run compile

# 查看是否有错误
cat vscode-nu-lang/out/extension.js | head -20
```

## 完整重置流程

如果一切都失败了，从头开始：

```bash
cd /home/sonygod/projects/nu

# 1. 重新编译 nu2rust
cargo clean
cargo build --release --bin nu2rust
sudo cp target/release/nu2rust /usr/local/bin/

# 2. 清理并重新编译插件
cd vscode-nu-lang
rm -rf out node_modules *.vsix
npm install
npm run compile

# 3. 打包新版本
npx vsce package --allow-missing-repository --out nu-lang-latest.vsix

# 4. 卸载旧版本
code --uninstall-extension nu-lang.nu-lang

# 5. 安装新版本
code --install-extension nu-lang-latest.vsix

# 6. 完全重启 VSCode（不是重新加载窗口）
```

## 成功案例示例

```bash
$ cd /home/sonygod/projects/nu

# 打开文件
$ code temp_examples_nu/hello.nu

# 在 VSCode 中:
# 1. 看到状态栏 "Nu: Auto-compile ON"
# 2. 修改文件，添加一行注释
# 3. Ctrl+S 保存
# 4. Ctrl+Shift+U 打开输出，选择 "Nu Language"
# 5. 看到编译日志

# 在终端验证:
$ ls -lh temp_examples_nu/hello.rs*
-rw-r--r-- 1 sonygod sonygod 558 Dec 26 10:26 hello.rs
-rw-r--r-- 1 sonygod sonygod 860 Dec 26 10:26 hello.rs.map

$ cat temp_examples_nu/hello.rs | head -5
// Test comment
pub fn hello_world() -> String {
    "Hello, World!".to_string()
}
```

## 总结

- ✅ CLI 工具（nu2rust）工作正常
- ✅ 插件代码已编译（out/ 目录）
- ✅ 插件已打包（nu-lang-0.0.2.vsix）
- ✅ 插件已安装

**下一步**：请按照上述测试步骤验证插件功能。如果仍有问题，请提供输出面板的日志和 Developer Tools 的错误信息。