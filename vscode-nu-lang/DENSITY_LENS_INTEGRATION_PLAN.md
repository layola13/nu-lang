# Nu Lang Density Lens - 集成评估与实施方案

## 📊 需求评估总结

### 当前状态 (v0.0.1)
现有 `vscode-nu-lang` 插件提供：
- ✅ 完整的 Nu v1.5.1 语法高亮（基于 TextMate Grammar）
- ✅ 括号匹配和自动补全
- ✅ 注释支持
- ✅ 基础语言配置

### 缺失功能（Density Lens 需求）
- ❌ 双向代码转换视图（Rust ↔ Nu）
- ❌ 实时转换引擎集成
- ❌ 统计数据可视化（压缩率、Token 估算）
- ❌ 同步滚动
- ❌ 错误映射
- ❌ 命令与交互系统

---

## 🎯 核心功能差距分析

| 功能模块 | 当前状态 | 需求级别 | 实施复杂度 | 优先级 |
|---------|---------|---------|-----------|--------|
| 语法高亮 | ✅ 已完成 | 基础 | - | - |
| Rust→Nu 压缩视图 | ❌ 缺失 | MVP | ⭐⭐⭐ | P0 |
| Nu→Rust 安全视图 | ❌ 缺失 | MVP | ⭐⭐⭐⭐ | P0 |
| 统计数据 HUD | ❌ 缺失 | MVP | ⭐⭐ | P0 |
| 分栏并排视图 | ❌ 缺失 | Phase 2 | ⭐⭐⭐ | P1 |
| 实时同步刷新 | ❌ 缺失 | Phase 2 | ⭐⭐⭐⭐ | P1 |
| AST 同步滚动 | ❌ 缺失 | Phase 3 | ⭐⭐⭐⭐⭐ | P2 |
| 错误映射 | ❌ 缺失 | Phase 3 | ⭐⭐⭐⭐ | P2 |
| LSP Server | ❌ 缺失 | Phase 3 | ⭐⭐⭐⭐⭐ | P3 |

---

## 🏗️ 技术架构设计

### 插件结构扩展

```
vscode-nu-lang/
├── package.json                    [MODIFY] 新增命令和配置
├── README.md                       [MODIFY] 添加 Density Lens 文档
├── src/                           [NEW] TypeScript 源代码目录
│   ├── extension.ts               [NEW] 插件入口
│   ├── commands/
│   │   ├── openCompressedView.ts  [NEW] Rust→Nu 命令
│   │   ├── openSafetyView.ts      [NEW] Nu→Rust 命令
│   │   └── translateSelection.ts  [NEW] 选区翻译
│   ├── services/
│   │   ├── conversionService.ts   [NEW] 核心转换服务（调用 CLI）
│   │   ├── binaryManager.ts       [NEW] 管理 rust2nu/nu2rust 二进制
│   │   └── tokenEstimator.ts      [NEW] Token 计算（使用 gpt-tokenizer）
│   ├── providers/
│   │   ├── lensContentProvider.ts [NEW] 虚拟文档提供者
│   │   └── virtualDocProvider.ts  [NEW] .rs.nu 虚拟文件
│   ├── features/
│   │   ├── syncScroll.ts          [NEW] 同步滚动（Phase 3）
│   │   ├── errorMapper.ts         [NEW] 错误映射（Phase 3）
│   │   └── autoRefresh.ts         [NEW] 自动刷新（debounce）
│   ├── ui/
│   │   ├── statusBar.ts           [NEW] 状态栏统计
│   │   ├── statsOverlay.ts        [NEW] 悬浮统计卡片
│   │   └── splitViewManager.ts    [NEW] 分栏视图管理
│   └── utils/
│       ├── astMapper.ts           [NEW] AST 节点映射（Phase 3）
│       └── logger.ts              [NEW] 日志工具
├── syntaxes/                      [EXISTING] TextMate 语法
└── language-configuration.json    [EXISTING] 语言配置
```

---

## 📝 package.json 修改方案

### 新增依赖

```json
{
  "devDependencies": {
    "@types/node": "^18.0.0",
    "@types/vscode": "^1.75.0",
    "typescript": "^5.0.0",
    "vsce": "^2.15.0"
  },
  "dependencies": {
    "gpt-tokenizer": "^2.1.1"
  },
  "scripts": {
    "compile": "tsc -p ./",
    "watch": "tsc -watch -p ./",
    "package": "vsce package"
  },
  "main": "./out/extension.js"
}
```

### 新增配置项

```json
{
  "contributes": {
    "configuration": {
      "title": "Nu Lens",
      "properties": {
        "nuLens.rust2nuPath": {
          "type": "string",
          "default": "",
          "description": "Path to rust2nu binary (leave empty for auto-detect)"
        },
        "nuLens.nu2rustPath": {
          "type": "string",
          "default": "",
          "description": "Path to nu2rust binary (leave empty for auto-detect)"
        },
        "nuLens.autoRefresh": {
          "type": "boolean",
          "default": true,
          "description": "Automatically refresh Lens view when code changes"
        },
        "nuLens.refreshDelay": {
          "type": "number",
          "default": 500,
          "description": "Debounce delay for auto-refresh (milliseconds)"
        },
        "nuLens.showTokenEstimation": {
          "type": "boolean",
          "default": true,
          "description": "Show GPT token estimation in statistics"
        },
        "nuLens.enableSyncScroll": {
          "type": "boolean",
          "default": true,
          "description": "Enable synchronized scrolling between views"
        }
      }
    }
  }
}
```

### 新增命令

```json
{
  "contributes": {
    "commands": [
      {
        "command": "nuLens.openCompressedView",
        "title": "Nu Lens: Open Compressed View (Rust → Nu)",
        "when": "editorLangId == rust"
      },
      {
        "command": "nuLens.openSafetyView",
        "title": "Nu Lens: Open Safety View (Nu → Rust)",
        "when": "resourceExtname == .nu"
      },
      {
        "command": "nuLens.translateSelection",
        "title": "Nu Lens: Translate Selection",
        "when": "editorHasSelection"
      },
      {
        "command": "nuLens.toggleAutoRefresh",
        "title": "Nu Lens: Toggle Auto-Refresh"
      }
    ],
    "menus": {
      "editor/context": [
        {
          "command": "nuLens.openCompressedView",
          "when": "editorLangId == rust",
          "group": "nuLens@1"
        },
        {
          "command": "nuLens.openSafetyView",
          "when": "resourceExtname == .nu",
          "group": "nuLens@2"
        },
        {
          "command": "nuLens.translateSelection",
          "when": "editorHasSelection",
          "group": "nuLens@3"
        }
      ],
      "commandPalette": [
        {
          "command": "nuLens.openCompressedView",
          "when": "editorLangId == rust"
        },
        {
          "command": "nuLens.openSafetyView",
          "when": "resourceExtname == .nu"
        }
      ]
    }
  }
}
```

---

## 🔧 核心实现方案

### 1. 转换服务 (conversionService.ts)

```typescript
import * as cp from 'child_process';
import * as vscode from 'vscode';

export interface ConversionResult {
  success: boolean;
  output: string;
  error?: string;
  stats?: {
    linesOriginal: number;
    linesConverted: number;
    charsOriginal: number;
    charsConverted: number;
    tokensOriginal?: number;
    tokensConverted?: number;
  };
}

export class ConversionService {
  private rust2nuPath: string;
  private nu2rustPath: string;

  constructor(context: vscode.ExtensionContext) {
    const config = vscode.workspace.getConfiguration('nuLens');
    this.rust2nuPath = config.get('rust2nuPath') || 'rust2nu';
    this.nu2rustPath = config.get('nu2rustPath') || 'nu2rust';
  }

  async rust2nu(rustCode: string): Promise<ConversionResult> {
    return this.executeConversion(this.rust2nuPath, rustCode);
  }

  async nu2rust(nuCode: string): Promise<ConversionResult> {
    return this.executeConversion(this.nu2rustPath, nuCode);
  }

  private async executeConversion(
    binaryPath: string,
    input: string
  ): Promise<ConversionResult> {
    return new Promise((resolve) => {
      const process = cp.spawn(binaryPath, ['-']);
      let output = '';
      let error = '';

      process.stdin.write(input);
      process.stdin.end();

      process.stdout.on('data', (data) => {
        output += data.toString();
      });

      process.stderr.on('data', (data) => {
        error += data.toString();
      });

      process.on('close', (code) => {
        if (code === 0) {
          resolve({
            success: true,
            output,
            stats: this.calculateStats(input, output),
          });
        } else {
          resolve({
            success: false,
            output: '',
            error: error || 'Conversion failed',
          });
        }
      });
    });
  }

  private calculateStats(original: string, converted: string) {
    return {
      linesOriginal: original.split('\n').length,
      linesConverted: converted.split('\n').length,
      charsOriginal: original.length,
      charsConverted: converted.length,
    };
  }
}
```

### 2. 压缩视图命令 (openCompressedView.ts)

```typescript
import * as vscode from 'vscode';
import { ConversionService } from '../services/conversionService';

export async function openCompressedView(
  context: vscode.ExtensionContext,
  conversionService: ConversionService
) {
  const editor = vscode.window.activeTextEditor;
  if (!editor) {
    vscode.window.showErrorMessage('No active editor');
    return;
  }

  const rustCode = editor.document.getText();
  const result = await conversionService.rust2nu(rustCode);

  if (!result.success) {
    vscode.window.showErrorMessage(`Conversion failed: ${result.error}`);
    return;
  }

  // 创建虚拟文档
  const nuUri = vscode.Uri.parse(
    `nu-lens:${editor.document.uri.path}.nu`
  );

  const nuDoc = await vscode.workspace.openTextDocument(
    nuUri.with({ scheme: 'untitled' })
  );

  await vscode.window.showTextDocument(nuDoc, {
    viewColumn: vscode.ViewColumn.Beside,
    preview: false,
  });

  const edit = new vscode.WorkspaceEdit();
  edit.insert(nuDoc.uri, new vscode.Position(0, 0), result.output);
  await vscode.workspace.applyEdit(edit);

  // 显示统计数据
  showStats(result.stats);
}

function showStats(stats: any) {
  if (!stats) return;

  const compression = (
    ((stats.charsOriginal - stats.charsConverted) / stats.charsOriginal) *
    100
  ).toFixed(1);

  vscode.window.setStatusBarMessage(
    `⚡ ${100 - parseFloat(compression)}% Code | Compressed by ${compression}%`,
    5000
  );
}
```

### 3. Token 估算器 (tokenEstimator.ts)

```typescript
import { encode } from 'gpt-tokenizer';

export class TokenEstimator {
  estimateTokens(code: string): number {
    try {
      return encode(code).length;
    } catch (error) {
      // Fallback: rough estimation (1 token ≈ 4 chars)
      return Math.ceil(code.length / 4);
    }
  }

  calculateTokenEfficiency(
    originalTokens: number,
    compressedTokens: number
  ): number {
    if (compressedTokens === 0) return 0;
    return parseFloat((originalTokens / compressedTokens).toFixed(2));
  }
}
```

---

## 📊 README.md 集成方案

在现有 README.md 的**功能特性**部分后添加新章节：

```markdown
## 🔬 Density Lens - 代码密度透镜 (v0.1.0+)

Nu Lang Density Lens 提供 Rust 与 Nu 代码的双向实时映射视图，帮助你：

- **可视化压缩**：精确展示 Nu 代码压缩率
- **验证安全性**：检查 Nu → Rust 转换结果
- **对比学习**：通过并排视图理解 Nu 语法

### 🎯 核心功能

#### 1️⃣ 压缩视图 (Rust → Nu)
**用途**：查看 Rust 代码转换为 Nu 后的精简版本

**使用方法**：
1. 打开任意 `.rs` 文件
2. 右键菜单 → **"Nu Lens: Open Compressed View"**
3. 或命令面板 (`Ctrl+Shift+P`) → `Nu: Open Compressed View`

**效果**：
- 右侧并排显示转换后的 `.nu` 代码
- 状态栏实时显示：`⚡ 42% Code | 1.8x Tokens`
- 悬浮卡片展示详细统计

#### 2️⃣ 安全视图 (Nu → Rust)
**用途**：验证 Nu 代码转换为标准 Rust 的正确性

**使用方法**：
1. 