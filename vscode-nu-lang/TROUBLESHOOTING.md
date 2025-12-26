# VSCode Nu Lang 插件故障排除指南

## 🔍 问题：状态栏没有显示 Nu 编译状态

### 可能原因和解决方案

#### 1. 插件未激活

**症状**: 
- 状态栏没有 "Nu: Auto-compile OFF/ON" 按钮
- 保存 .nu 文件没有任何反应

**检查方法**:
```bash
# 检查插件是否安装
code --list-extensions | grep nu-lang

# 应该看到输出: nu-lang.nu-lang
```

**解决方案**:
1. **重新加载 VSCode 窗口**: 
   - 按 `Ctrl+Shift+P` (或 `Cmd+Shift+P`)
   - 输入 "Reload Window"
   - 按回车

2. **检查 VSCode 输出面板**:
   - 打开输出面板: `Ctrl+Shift+U` (或 `View` → `Output`)
   - 选择下拉菜单中的 "Nu Language"
   - 查看是否有错误信息

3. **检查开发者工具**:
   - 打开开发者工具: `Help` → `Toggle Developer Tools`
   - 查看 Console 标签页是否有红色错误

#### 2. nu2rust 二进制未配置

**症状**:
- 插件已激活（能看到状态栏按钮）
- 但保存文件后没有生成 .rs 文件
- 输出面板显示 "nu2rust not found" 或类似错误

**检查方法**:
```bash
# 检查 nu2rust 是否可执行
which nu2rust

# 或者检查项目编译目录
ls -l /home/sonygod/projects/nu/target/release/nu2rust

# 测试 nu2rust 是否能运行
/home/sonygod/projects/nu/target/release/nu2rust --version
```

**解决方案**:

**方案 A: 添加到 PATH（推荐）**
```bash
# 编译 nu2rust
cd /home/sonygod/projects/nu
cargo build --release --bin nu2rust

# 添加到 PATH
sudo cp target/release/nu2rust /usr/local/bin/
sudo chmod +x /usr/local/bin/nu2rust

# 验证
which nu2rust
nu2rust --version
```

**方案 B: 在 VSCode 中配置路径**

打开 VSCode 设置（`Ctrl+,`），搜索 "nu-lang"，设置：

```json
{
  "nu-lang.nu2rustPath": "/home/sonygod/projects/nu/target/release/nu2rust",
  "nu-lang.cargoPath": "cargo"
}
```

或者直接编辑 `settings.json`:
```bash
# 打开 settings.json
code ~/.config/Code/User/settings.json
```

添加：
```json
{
  "nu-lang.nu2rustPath": "/home/sonygod/projects/nu/target/release/nu2rust",
  "nu-lang.cargoPath": "cargo",
  "nu-lang.autoCompile": true,
  "nu-lang.autoCheck": true
}
```

#### 3. 文件权限问题

**检查方法**:
```bash
# 检查 nu2rust 是否有执行权限
ls -l /home/sonygod/projects/nu/target/release/nu2rust

# 应该看到: -rwxr-xr-x (注意 x 表示可执行)
```

**解决方案**:
```bash
chmod +x /home/sonygod/projects/nu/target/release/nu2rust
```

#### 4. 插件版本不匹配

**症状**:
- 插件代码已更新，但行为没变
- 看到的是旧版本功能

**解决方案**:
```bash
# 重新编译插件
cd /home/sonygod/projects/nu/vscode-nu-lang
npm run compile

# 重新打包
npx vsce package --allow-missing-repository

# 强制重新安装
code --install-extension nu-lang-0.0.1.vsix --force

# 重新加载 VSCode 窗口
# Ctrl+Shift+P → "Reload Window"
```

## 🧪 测试流程

### 步骤 1: 验证 nu2rust 可用

```bash
cd /home/sonygod/projects/nu

# 创建测试文件
cat > test_plugin.nu << 'EOF'
f hello() -> String {
    "Hello, Nu!".to_string()
}

f main() {
    println!("{}", hello());
}
EOF

# 手动测试转换
./target/release/nu2rust test_plugin.nu --sourcemap -v

# 应该生成:
# - test_plugin.rs
# - test_plugin.rs.map

# 检查生成的文件
ls -l test_plugin.rs*
cat test_plugin.rs.map
```

### 步骤 2: 验证 VSCode 插件

1. **打开 test_plugin.nu 文件**
   ```bash
   code test_plugin.nu
   ```

2. **检查状态栏**
   - 应该看到右下角有 "Nu: Auto-compile OFF" 或 "Nu: Auto-compile ON"
   - 点击可以切换状态

3. **打开输出面板**
   - `Ctrl+Shift+U`
   - 选择 "Nu Language"

4. **编辑并保存文件**
   ```nu
   // 添加一行注释
   f hello() -> String {
       "Hello, Nu!".to_string()
   }
   ```
   
5. **按 Ctrl+S 保存**

6. **观察输出面板**
   应该看到类似：
   ```
   [Nu Language] Compiling: test_plugin.nu
   [Nu Language] nu2rust command: /usr/local/bin/nu2rust test_plugin.nu --sourcemap
   [Nu Language] Compilation successful: test_plugin.rs
   [Nu Language] SourceMap generated: test_plugin.rs.map
   [Nu Language] Running cargo check...
   ```

7. **检查生成的文件**
   ```bash
   ls -l test_plugin.rs*
   ```

### 步骤 3: 验证错误映射

1. **创建有错误的 Nu 文件**
   ```nu
   F add(a: i32, b: i32) -> String {  // 错误：返回类型不匹配
       < a + b
   }
   
   f main() {
       l result = add(5, 3)
       println!("Result: {}", result)
   }
   ```

2. **保存文件**

3. **检查编辑器**
   - 应该看到红色波浪线在 `< a + b` 行
   - 鼠标悬停应显示错误信息

## 📋 完整诊断检查清单

运行以下命令收集诊断信息：

```bash
#!/bin/bash
echo "=== VSCode Nu Lang 插件诊断 ==="
echo ""

echo "1. 检查插件安装:"
code --list-extensions | grep nu-lang
echo ""

echo "2. 检查 nu2rust 二进制:"
which nu2rust || echo "nu2rust 不在 PATH 中"
ls -l /home/sonygod/projects/nu/target/release/nu2rust 2>/dev/null || echo "nu2rust 未编译"
echo ""

echo "3. 测试 nu2rust 执行:"
/home/sonygod/projects/nu/target/release/nu2rust --version 2>&1 || echo "nu2rust 无法执行"
echo ""

echo "4. 检查 cargo:"
which cargo || echo "cargo 不在 PATH 中"
cargo --version 2>&1 || echo "cargo 无法执行"
echo ""

echo "5. 检查 VSCode 设置:"
if [ -f ~/.config/Code/User/settings.json ]; then
    echo "settings.json 存在"
    grep -A 5 "nu-lang" ~/.config/Code/User/settings.json || echo "没有 nu-lang 配置"
else
    echo "settings.json 不存在"
fi
echo ""

echo "6. 检查插件文件:"
ls -l /home/sonygod/projects/nu/vscode-nu-lang/out/extension.js 2>/dev/null || echo "插件未编译"
echo ""

echo "=== 诊断完成 ==="
```

保存为 `diagnose.sh` 并运行：
```bash
chmod +x diagnose.sh
./diagnose.sh
```

## 🆘 仍然无法解决？

### 收集详细信息

1. **VSCode 版本**:
   ```bash
   code --version
   ```

2. **Node.js 版本**:
   ```bash
   node --version
   npm --version
   ```

3. **系统信息**:
   ```bash
   uname -a
   lsb_release -a
   ```

4. **VSCode 日志**:
   - 打开 `Help` → `Toggle Developer Tools`
   - 复制 Console 中的所有错误信息

5. **插件输出**:
   - 打开 `Output` 面板
   - 选择 "Nu Language"
   - 复制所有输出

### 联系支持

将以上信息发送到：
- GitHub Issues: [项目仓库]
- 或在项目目录创建 issue.txt

## 🔧 常见解决方案速查

| 问题 | 快速解决 |
|------|---------|
| 状态栏没有按钮 | `Ctrl+Shift+P` → "Reload Window" |
| nu2rust not found | 配置 `nu-lang.nu2rustPath` 或添加到 PATH |
| 没有生成 .rs 文件 | 检查输出面板错误，测试 nu2rust 手动执行 |
| 错误没有显示 | 检查 .rs.map 文件是否生成 |
| 插件更新不生效 | 重新编译、打包、强制安装、重新加载窗口 |

## ✅ 成功标志

当一切正常时，您应该看到：

1. ✅ 状态栏右下角有 "Nu: Auto-compile ON" 按钮
2. ✅ 保存 .nu 文件后，输出面板显示编译信息
3. ✅ 生成 .rs 和 .rs.map 文件
4. ✅ cargo check 错误在 Nu 编辑器中显示红色波浪线
5. ✅ 状态栏状态变化: "Compiling..." → "Compiled ✓" 或显示错误