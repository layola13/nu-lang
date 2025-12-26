# VSCode Extension Release Guide

## 自动构建和发布流程

### GitHub Action 工作流

项目包含一个自动化工作流 `.github/workflows/vscode-extension.yml`，用于构建和发布 Nu Language VSCode 扩展。

### 触发条件

工作流在以下情况下自动触发：

1. **推送到主分支**
   ```bash
   git push origin main
   ```
   - 仅当 `vscode-nu-lang/` 目录有变更时触发
   - 自动构建并上传 VSIX 文件作为 Artifact

2. **Pull Request**
   ```bash
   # 创建 PR 到 main/master 分支
   ```
   - 验证扩展可以成功编译
   - 在 3 个平台上测试（Linux, macOS, Windows）

3. **创建 Release**
   ```bash
   # 在 GitHub 上创建新 Release
   ```
   - 自动构建 VSIX 文件
   - 将 VSIX 文件附加到 Release 资产

4. **手动触发**
   - 进入 GitHub Actions 页面
   - 选择 "VSCode Extension Build"
   - 点击 "Run workflow"

### 构建流程

```
┌─────────────────────────────────────────────┐
│  1. Checkout code                            │
└──────────────┬──────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────┐
│  2. Setup Node.js 18                         │
│     - 使用 npm cache 加速                     │
└──────────────┬──────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────┐
│  3. Install dependencies (npm ci)            │
│     - 清理安装，确保一致性                    │
└──────────────┬──────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────┐
│  4. Compile TypeScript (npm run compile)     │
│     - 编译 src/ → out/                       │
└──────────────┬──────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────┐
│  5. Install vsce                             │
│     - VSCode Extension Manager               │
└──────────────┬──────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────┐
│  6. Get version from package.json            │
│     - 读取当前版本号                         │
└──────────────┬──────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────┐
│  7. Package extension (vsce package)         │
│     - 生成 nu-lang-{version}.vsix            │
└──────────────┬──────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────┐
│  8. Upload VSIX as Artifact                  │
│     - 可在 Actions 页面下载                   │
└─────────────────────────────────────────────┘
```

### 下载构建产物

#### 方法 1：从 GitHub Actions Artifacts

1. 进入项目的 [Actions 页面](https://github.com/YOUR_USERNAME/nu/actions)
2. 选择最新的 "VSCode Extension Build" 工作流运行
3. 在 "Artifacts" 部分找到 `nu-lang-{version}.vsix`
4. 点击下载

#### 方法 2：从 Release 页面

如果是通过 Release 触发的构建：

1. 进入 [Releases 页面](https://github.com/YOUR_USERNAME/nu/releases)
2. 找到对应的 Release
3. 在 "Assets" 部分下载 `nu-lang-{version}.vsix`

### 发布新版本

#### 步骤 1：更新版本号

编辑 `vscode-nu-lang/package.json`：

```json
{
  "version": "0.0.6",  // 更新版本号
  "name": "nu-lang",
  ...
}
```

#### 步骤 2：提交更改

```bash
cd vscode-nu-lang
git add package.json
git commit -m "chore: bump version to 0.0.6"
git push origin main
```

#### 步骤 3：创建 Git Tag（可选）

如果需要创建正式 Release：

```bash
git tag vscode-v0.0.6
git push origin vscode-v0.0.6
```

#### 步骤 4：在 GitHub 创建 Release

1. 进入 GitHub 仓库
2. 点击 "Releases" → "Draft a new release"
3. 选择刚创建的 tag：`vscode-v0.0.6`
4. 填写 Release 标题和说明
5. 点击 "Publish release"
6. GitHub Action 会自动将 VSIX 文件附加到 Release

### 测试构建

工作流还包含跨平台测试任务：

```yaml
test:
  runs-on: [ubuntu-latest, macos-latest, windows-latest]
```

在以下平台上验证编译：
- ✅ Ubuntu Linux
- ✅ macOS
- ✅ Windows

### 手动触发构建

如果需要手动构建（不推送代码）：

1. 进入 [Actions 页面](https://github.com/YOUR_USERNAME/nu/actions)
2. 选择 "VSCode Extension Build"
3. 点击 "Run workflow" 按钮
4. 选择分支
5. 点击绿色 "Run workflow" 按钮

### 本地测试

在推送前本地测试构建：

```bash
cd vscode-nu-lang

# 安装依赖
npm install

# 编译 TypeScript
npm run compile

# 打包扩展
npm run package

# 测试安装
code --install-extension nu-lang-*.vsix
```

### 故障排除

#### 构建失败

**问题**: TypeScript 编译错误

```bash
# 本地测试编译
cd vscode-nu-lang
npm run compile

# 查看详细错误
npx tsc --noEmit
```

**问题**: vsce package 失败

```bash
# 检查 package.json 配置
# 确保所有必需字段都存在：
# - name, version, publisher, engines, main
```

**问题**: 缺少 LICENSE 文件警告

```bash
# vsce 会警告但仍然打包
# 可以添加 LICENSE 文件消除警告
cp LICENSE vscode-nu-lang/
```

#### Artifact 下载失败

**问题**: Artifact 已过期

- GitHub Actions Artifacts 默认保留 90 天
- 过期后需要重新触发构建

**解决方案**: 手动触发工作流重新构建

#### Release Asset 上传失败

**问题**: GITHUB_TOKEN 权限不足

- 确保仓库设置中启用了 Actions 权限
- 设置 → Actions → General → Workflow permissions → Read and write permissions

### CI/CD 配置

#### 缓存优化

工作流使用 npm 缓存加速构建：

```yaml
- uses: actions/setup-node@v3
  with:
    cache: 'npm'
    cache-dependency-path: vscode-nu-lang/package-lock.json
```

#### 依赖安装

使用 `npm ci` 而不是 `npm install` 确保一致性：

```bash
npm ci  # 清理安装，基于 package-lock.json
```

### 版本管理策略

推荐使用语义化版本：

- **MAJOR.MINOR.PATCH** (例如：0.0.5)
  - **MAJOR**: 不兼容的 API 更改
  - **MINOR**: 向后兼容的功能添加
  - **PATCH**: 向后兼容的 bug 修复

**示例**:
- `0.0.5` → `0.0.6`: Bug 修复
- `0.0.6` → `0.1.0`: 新功能
- `0.1.0` → `1.0.0`: 重大更新

### 发布检查清单

发布前确认：

- [ ] 更新 `package.json` 中的版本号
- [ ] 更新 `CHANGELOG.md`（如果有）
- [ ] 本地测试扩展功能
- [ ] 运行 `npm run compile` 确认无错误
- [ ] 运行 `npm run package` 确认可以打包
- [ ] 提交并推送代码
- [ ] 等待 CI 构建成功
- [ ] 下载并测试 Artifact
- [ ] （可选）创建 Git tag 和 Release

### 相关链接

- [VSCode Extension Publishing](https://code.visualstudio.com/api/working-with-extensions/publishing-extension)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [vsce CLI](https://github.com/microsoft/vscode-vsce)

### 联系和支持

如有问题，请：
1. 查看 [TROUBLESHOOTING.md](../vscode-nu-lang/TROUBLESHOOTING.md)
2. 提交 Issue 到 GitHub
3. 查看 Actions 日志获取详细错误信息