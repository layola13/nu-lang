# Nu Language VSCode 插件安装指南

## 快速安装方法

### 方法 1: 本地开发测试（推荐用于开发）

1. 在 VSCode 中打开 `vscode-nu-lang` 文件夹
2. 按 `F5` 键启动扩展开发主机
3. 在新打开的 VSCode 窗口中，打开或创建 `.nu` 文件
4. 语法高亮会自动生效

### 方法 2: 安装到 VSCode 扩展目录

**Linux/macOS:**
```bash
cp -r vscode-nu-lang ~/.vscode/extensions/nu-lang-0.0.1
```

**Windows (PowerShell):**
```powershell
Copy-Item -Recurse vscode-nu-lang "$env:USERPROFILE\.vscode\extensions\nu-lang-0.0.1"
```

安装后重启 VSCode 即可生效。

### 方法 3: 打包成 VSIX 并安装

1. 安装打包工具（如果尚未安装）：
```bash
npm install -g vsce
```

2. 在 `vscode-nu-lang` 目录下打包：
```bash
cd vscode-nu-lang
vsce package
```

3. 会生成 `nu-lang-0.0.1.vsix` 文件

4. 在 VSCode 中安装：
   - 按 `Ctrl+Shift+P` (Windows/Linux) 或 `Cmd+Shift+P` (macOS)
   - 输入 "Extensions: Install from VSIX"
   - 选择生成的 `.vsix` 文件

## 验证安装

1. 在 VSCode 中创建或打开 `test.nu` 文件
2. 输入以下代码：

```nu
F main() {
    l x = 42
    > "Hello, Nu!"
    < 0
}
```

3. 如果看到语法高亮（关键字 `F`, `l`, `>`, `<` 等有不同颜色），说明安装成功！

## 测试语法高亮

项目中包含了 `test-example.nu` 文件，其中包含了所有 Nu v1.5.1 的语法元素。
打开该文件可以查看完整的语法高亮效果。

## 调试语法高亮

如果某些语法没有正确高亮：

1. 在代码上右键 -> "命令面板" -> 输入 "Developer: Inspect Editor Tokens and Scopes"
2. 点击代码，查看 token 的 scope 信息
3. 检查 `syntaxes/nu.tmLanguage.json` 中对应的正则表达式

## 卸载

**从扩展目录卸载：**
```bash
# Linux/macOS
rm -rf ~/.vscode/extensions/nu-lang-0.0.1

# Windows (PowerShell)
Remove-Item -Recurse "$env:USERPROFILE\.vscode\extensions\nu-lang-0.0.1"
```

**从 VSIX 卸载：**
在 VSCode 扩展面板中找到 "Nu Language"，点击卸载。

## 故障排除

### 问题：语法高亮不生效

**解决方案：**
1. 确认文件扩展名是 `.nu`
2. 重启 VSCode
3. 检查 VSCode 右下角语言模式是否显示为 "Nu"
4. 如果显示其他语言，点击手动选择 "Nu"

### 问题：部分关键字没有高亮

**解决方案：**
1. 确认使用的是支持的 Nu v1.5.1 语法
2. 使用 "Inspect Editor Tokens and Scopes" 工具检查 token 识别
3. 检查主题是否支持对应的 scope（如 `keyword.control.nu`）

### 问题：打包失败

**解决方案：**
1. 确认已安装 `vsce`：`npm install -g vsce`
2. 检查 `package.json` 格式是否正确
3. 确保所有必需文件都存在

## 更新插件

当有新版本时：
1. 下载新版本文件
2. 按照安装方法重新安装
3. 重启 VSCode