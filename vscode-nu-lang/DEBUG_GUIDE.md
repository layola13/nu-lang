# VSCode Nu Lang 插件调试指南

## 问题：插件未自动触发编译

### 诊断步骤

#### 1. 检查插件是否加载

打开 VSCode Developer Tools:
- 菜单: `Help` → `Toggle Developer Tools`
- 或按 `Ctrl+Shift+I` (Windows/Linux) / `Cmd+Option+I` (Mac)

在 Console 中查找：
```
Extension 'nu-lang.nu-lang' activated
```

如果没有看到，说明插件未激活。

#### 2. 检查插件激活条件

插件的 `package.json` 中定义了激活事件：
```json
"activationEvents": [
  "onLanguage:nu"
]
```

这意味着只有当打开 `.nu` 文件时插件才会激活。

**验证步骤**:
1. 关闭所有文件
2. 打开一个 `.nu` 文件（如 `temp_examples_nu/hello.nu`）
3. 检查 Developer Tools Console 是否有激活消息

#### 3. 检查文件关联

确保 `.nu` 文件被识别为 Nu 语言：

1. 打开 `.nu` 文件
2. 查看右下角状态栏的语言标识
3. 应该显示 "Nu" 或 "nu"

如果显示 "Plain Text"，需要手动设置：
1. 点击右下角的语言标识
2. 在弹出菜单中选择 "Configure File Association for '.nu'..."
3. 选择 "Nu"

#### 4. 手动触发命令

即使插件未自动运行，也可以手动测试：

1. 打开命令面板: `Ctrl+Shift+P`
2. 输入 "Nu: Compile"
3. 如果看到命令，说明插件已加载
4. 执行命令测试功能

#### 5. 查看插件输出

打开输出面板:
1. `View` → `Output` 或 `Ctrl+Shift+U`
2. 在下拉菜单中选择 "Nu Language"
3. 查看是否有日志输出

#### 6. 检查extension.ts

可能的问题：`activationEvents` 配置不正确

当前配置应该是：
```json
"activationEvents": [
  "onLanguage:nu"
]
```

但更安全的方式是：
```json
"activationEvents": [
  "onLanguage:nu",
  "workspaceContains:**/*.nu"
]
```

或者使用立即激活（开发/测试阶段）：
```json
"activationEvents": [
  "*"
]
```

## 快速修复方案

### 方案 1: 修改 package.json 使插件立即激活

编辑 `vscode-nu-lang/package.json`:

```json
{
  "activationEvents": [
    "*"
  ]
}
```

然后重新编译和打包：
```bash
cd vscode-nu-lang
npm run compile
npx vsce package --allow-missing-repository
code --install-extension nu-lang-0.0.1.vsix --force
```

重新加载 VSCode 窗口: `Ctrl+Shift+P` → "Reload Window"

### 方案 2: 添加调试日志

修改 `src/extension.ts` 的 `activate` 函数，添加日志：

```typescript
export function activate(context: vscode.ExtensionContext) {
    console.log('Nu Language Extension activating...');
    vscode.window.showInformationMessage('Nu Language Extension activated!');
    
    // 原有代码...
}
```

这样可以在 Developer Tools 和通知中看到激活消息。

### 方案 3: 检查二进制路径

插件可能因为找不到 `nu2rust` 而静默失败。

添加错误提示：

```typescript
// 在 binaryManager.ts 中
getNu2rustPath(): string | null {
    const path = this.detectNu2rust();
    if (!path) {
        vscode.window.showErrorMessage(
            'nu2rust binary not found! Please configure nu-lang.nu2rustPath in settings.'
        );
    }
    return path;
}
```

## 完整测试流程

```bash
# 1. 确认 CLI 工具可用
which nu2rust
nu2rust --help

# 2. 手动测试转换
cd /home/sonygod/projects/nu
nu2rust temp_examples_nu/hello.nu --sourcemap -f -v

# 3. 检查插件安装
code --list-extensions | grep nu-lang

# 4. 查看插件文件
ls -l ~/.vscode-server/extensions/nu-lang.nu-lang-*/

# 5. 检查编译输出
ls -l vscode-nu-lang/out/

# 6. 重新安装插件
cd vscode-nu-lang
npm run compile
npx vsce package --allow-missing-repository
code --install-extension nu-lang-0.0.1.vsix --force
```

## 常见问题

### Q: 插件显示已安装但不工作
A: 尝试：
1. 重新加载窗口 (`Ctrl+Shift+P` → "Reload Window")
2. 重启 VSCode
3. 检查 Developer Tools Console 的错误

### Q: 找不到 nu2rust
A: 
1. 运行 `which nu2rust` 确认安装
2. 在设置中手动指定完整路径
3. 检查文件权限 (`chmod +x`)

### Q: 保存文件没有反应
A:
1. 检查是否打开了 `.nu` 文件
2. 检查文件是否正确识别为 Nu 语言
3. 手动运行命令 "Nu: Compile Current File"
4. 查看输出面板的错误信息

### Q: 状态栏没有显示
A:
1. 可能插件未激活
2. 检查 `extension.ts` 中状态栏代码是否执行
3. 添加调试日志确认

## 推荐的调试配置

在 `vscode-nu-lang/.vscode/launch.json` 中添加：

```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "name": "Extension",
            "type": "extensionHost",
            "request": "launch",
            "args": [
                "--extensionDevelopmentPath=${workspaceFolder}"
            ],
            "outFiles": [
                "${workspaceFolder}/out/**/*.js"
            ],
            "preLaunchTask": "${defaultBuildTask}"
        }
    ]
}
```

然后按 `F5` 启动调试，会打开一个新的 VSCode 窗口用于测试插件。