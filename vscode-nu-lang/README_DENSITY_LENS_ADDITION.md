# README.md 集成内容（插入到现有 README.md 的第 11 行后）

---

## 🔬 Density Lens - 代码密度透镜

> **全新功能预览**：双向 Rust ↔ Nu 代码转换与可视化（即将推出）

Nu Lang Density Lens 是一个革命性的开发工具，提供：

- **📊 实时压缩率展示**：直观看到 Nu 代码比 Rust 短多少
- **🔄 双向转换视图**：Rust → Nu（压缩）和 Nu → Rust（安全验证）
- **⚡ Token 效率分析**：为 AI 辅助编程优化代码密度
- **👁️ 并排对比学习**：最佳的 Nu 语法学习方式

### 核心特性

#### 1️⃣ 压缩视图 (Rust → Nu)

**场景**：想看看我的 Rust 代码用 Nu 写有多简洁？

**操作**：
```
1. 打开任意 .rs 文件
2. 右键 → "Nu Lens: Open Compressed View"
3. 右侧自动显示转换后的 .nu 代码
```

**实时统计**：
```
状态栏显示：⚡ 42% Code | 1.8x Tokens
悬浮卡片：
  📊 Density Stats
  -----------------
  Lines:    85 → 38 (-55%)
  Chars:    2.4k → 1.1k (-54%)
  Tokens*:  ~450 → ~220 (-51%)
  -----------------
  *Estimated via GPT-3.5 Tokenizer
```

#### 2️⃣ 安全视图 (Nu → Rust)

**场景**：我写了 Nu 代码，但需要确认转回 Rust 是否正确

**操作**：
```
1. 打开 .nu 文件
2. 右键 → "Nu Lens: Open Safety View"
3. 右侧实时显示标准 Rust 代码
```

**实时同步**：
- 左侧修改 Nu 代码
- 右侧 Rust 视图自动更新
- 保存时可选择覆盖 Rust 文件

#### 3️⃣ 选区翻译

**场景**：只想看某一个函数的压缩效果

**操作**：
```
1. 选中一段代码
2. 右键 → "Nu Lens: Translate Selection"
3. 悬停提示或注释中显示转换结果
```

### 配置选项

在 VS Code 设置中搜索 `Nu Lens` 或编辑 `settings.json`：

```json
{
  // 二进制路径（留空自动检测）
  "nuLens.rust2nuPath": "",
  "nuLens.nu2rustPath": "",
  
  // 自动刷新
  "nuLens.autoRefresh": true,
  "nuLens.refreshDelay": 500,
  
  // 功能开关
  "nuLens.showTokenEstimation": true,
  "nuLens.enableSyncScroll": true
}
```

### 系统要求

**必需**：
- 安装 `rust2nu` 和 `nu2rust` CLI 工具

```bash
# 从项目根目录安装
cd /path/to/nu-lang-project
cargo install --path . --bin rust2nu
cargo install --path . --bin nu2rust
```

**可选**：
- 配置自定义二进制路径（如果不在 PATH 中）

### 使用技巧

#### 💡 学习 Nu 语法
1. 打开熟悉的 Rust 项目
2. 对每个模块使用"压缩视图"
3. 对比观察 Nu 的语法简化规律
4. 尝试直接编写 Nu 代码

#### 💡 验证转换正确性
1. 编写 Nu 代码
2. 使用"安全视图"检查生成的 Rust
3. 运行测试确保行为一致
4. 提交时可选择保留 Rust 或 Nu

#### 💡 优化 Token 使用
1. 为 AI 辅助工具（Copilot/ChatGPT）准备代码
2. 使用压缩视图查看 Token 节省率
3. 在提示词中使用 Nu 代码以节省上下文

### 开发路线图

| 阶段 | 功能 | 状态 |
|------|------|------|
| **Phase 1: MVP** | 基础转换 + 简单统计 | 🚧 开发中 |
| **Phase 2: Lens** | 并排视图 + 自动刷新 | 📋 计划中 |
| **Phase 3: Polish** | AST 同步滚动 + LSP | 💡 设计中 |

### 命令列表

| 命令 | 快捷键 | 说明 |
|------|--------|------|
| `Nu Lens: Open Compressed View` | - | Rust → Nu 压缩视图 |
| `Nu Lens: Open Safety View` | - | Nu → Rust 安全视图 |
| `Nu Lens: Translate Selection` | - | 翻译选中代码 |
| `Nu Lens: Toggle Auto-Refresh` | - | 切换自动刷新 |

### 故障排除

**问题：命令不可用**
- 确认安装了 `rust2nu` 和 `nu2rust` 工具
- 检查二进制是否在 PATH 中或配置了路径
- 重启 VS Code

**问题：转换失败**
- 检查源代码语法是否正确
- 查看输出面板的错误信息
- 确认使用的是最新版本的 CLI 工具

**问题：统计数据不显示**
- 确认 `nuLens.showTokenEstimation` 为 `true`
- 检查是否有安装 `gpt-tokenizer` 依赖
- 尝试重新加载窗口

### 推广口号

- **"Don't guess, verify."** —— 别猜，验证它
- **"Write for AI, Read for Human."** —— 为 AI 写，为人读
- **"See your code in High Definition."** —— 高清看代码

---

## 技术细节（保留现有的技术细节章节）

...（原有内容）...