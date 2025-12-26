# Sourcemap 实现方案 - Nu Lang Density Lens 的核心基础设施

## 🎯 为什么 Sourcemap 是必需的？

### 决策性论点

**没有 Sourcemap = 分屏文本查看器（玩具）**  
**有 Sourcemap = 智能透镜（生产力工具）**

这个决策直接决定了插件能否从"Demo"进化为"不可或缺的开发工具"。

---

## 💔 没有 Sourcemap 的灾难场景

### 场景 1：同步滚动的崩溃

**问题**：
```rust
// Rust: 20 行
#[derive(Debug, Clone, Serialize)]
pub struct User {
    /// 用户 ID
    pub id: u64,
    /// 用户名
    pub name: String,
    /// 邮箱
    pub email: Option<String>,
    // ... 更多字段和注释
}
```

```nu
// Nu: 3 行
#D(Debug, Clone, Serialize)
S User { id: u64, name: Str, email: O<Str> }
```

**用户行为**：在 Rust 编辑器滚动到第 10 行（`pub name: String`）

**没有 Sourcemap**：
- 简单比例计算：`nuLine = 10 * (3/20) = 1.5` → 滚动到第 2 行
- 但第 2 行是 `S User {`，用户看不到 `name` 字段
- **视觉联系断裂，用户晕头转向**

**有 Sourcemap**：
- 知道 Rust L10 的 `name` 字段对应 Nu L2 的 `name: Str`
- 精确滚动到 Nu 的第 2 行，并高亮 `name` 部分
- **逻辑同步，用户清晰理解对应关系**

### 场景 2：错误映射 - 安全视图的灵魂

**用户工作流**：
1. 用户写 Nu 代码
2. 插件转换为 Rust
3. 运行 `cargo check`
4. Rust 编译器报错：`error[E0308]: mismatched types --> src/main.rs:42:15`

**没有 Sourcemap**：
```
❌ 插件只能显示：
"Rust compilation error at line 42, column 15"

用户心路历程：
"Rust 的 42 行？我在写 Nu 啊...
让我数数...Nu 的 18 行对应 Rust 的...嗯...
算了，还是直接写 Rust 吧，这太痛苦了。"

结果：用户放弃使用 Nu
```

**有 Sourcemap**：
```
✅ 插件查表：src/main.rs:42:15 → main.nu:18:10

插件在 Nu 编辑器第 18 行显示：
  ~~~~~~~~~~~~ 红色波浪线
  error[E0308]: mismatched types
  expected `i32`, found `&str`

用户心路历程：
"哇，直接在我的 Nu 代码里就能看到 Rust 的类型错误！
这太爽了，就像在写带类型检查的 Python！"

结果：用户爱上 Nu
```

### 场景 3：局部翻译与重构

**需求**：用户选中一段代码 → 右键 → "翻译选区"

**没有 Sourcemap**：
```rust
// 用户选中从字符 150 到 300
// 插件傻傻地按字符串截取
let selected = source[150..300];
convert(selected); // ❌ 可能截断了结构体，导致语法错误
```

**有 Sourcemap**：
```rust
// 插件知道字符 150-300 对应完整的 AST 节点：
// - FunctionDefinition: fn calculate()
// 精确提取该节点并转换
convert(ast_node); // ✅ 保证语法完整
```

---

## 🏗️ Sourcemap 数据结构设计

### 标准格式（JSON）

```json
{
  "version": "1.0",
  "file": "main.rs",
  "nu_file": "main.nu",
  "mappings": [
    {
      "id": "node_001",
      "node_type": "StructDefinition",
      "name": "User",
      "rust_span": {
        "start_byte": 120,
        "end_byte": 450,
        "start_line": 5,
        "start_col": 0,
        "end_line": 15,
        "end_col": 1
      },
      "nu_span": {
        "start_byte": 50,
        "end_byte": 120,
        "start_line": 3,
        "start_col": 0,
        "end_line": 5,
        "end_col": 1
      },
      "children": ["node_002", "node_003"]
    },
    {
      "id": "node_002",
      "node_type": "StructField",
      "name": "id",
      "parent": "node_001",
      "rust_span": {
        "start_line": 6,
        "start_col": 4,
        "end_line": 6,
        "end_col": 18
      },
      "nu_span": {
        "start_line": 3,
        "start_col": 10,
        "end_line": 3,
        "end_col": 18
      }
    }
  ],
  "line_map": {
    "rust_to_nu": {
      "5": 3,
      "6": 3,
      "7": 3,
      "10": 4,
      "15": 5
    },
    "nu_to_rust": {
      "3": 5,
      "4": 10,
      "5": 15
    }
  }
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `version` | string | Sourcemap 格式版本 |
| `file` | string | 源 Rust 文件路径 |
| `nu_file` | string | 目标 Nu 文件路径 |
| `mappings` | array | AST 节点映射列表（树形结构） |
| `line_map` | object | 简化的行号快速查找表（用于 Phase 1） |

**节点映射（Mapping）**：

| 字段 | 说明 |
|------|------|
| `id` | 节点唯一标识符 |
| `node_type` | AST 节点类型（如 `StructDefinition`, `FunctionDefinition`） |
| `name` | 节点名称（如结构体名、函数名） |
| `rust_span` | 在 Rust 代码中的位置（字节偏移 + 行列号） |
| `nu_span` | 在 Nu 代码中的位置 |
| `children` | 子节点 ID 列表（支持树形查找） |
| `parent` | 父节点 ID（用于反向查找） |

---

## 🛠️ 实现方案：分阶段推进

### Phase 1: "The Lazy Map" - 基于行号的快速映射

**目标**：解决 80% 的同步滚动问题，快速上线

#### 1.1 Rust → Nu 方向

**在 `rust2nu` 中实现**：

```rust
// src/rust2nu/sourcemap.rs

use std::collections::HashMap;
use syn::Span;

pub struct LazySourceMap {
    /// Rust 行号 -> Nu 行号
    pub rust_to_nu: HashMap<usize, usize>,
    /// Nu 行号 -> Rust 行号
    pub nu_to_rust: HashMap<usize, usize>,
}

impl LazySourceMap {
    pub fn new() -> Self {
        Self {
            rust_to_nu: HashMap::new(),
            nu_to_rust: HashMap::new(),
        }
    }

    /// 记录一个定义的起始行映射
    pub fn add_mapping(&mut self, rust_line: usize, nu_line: usize) {
        self.rust_to_nu.insert(rust_line, nu_line);
        self.nu_to_rust.insert(nu_line, rust_line);
    }

    /// 查找最近的映射行
    pub fn find_nearest_nu_line(&self, rust_line: usize) -> Option<usize> {
        // 查找 <= rust_line 的最大键
        self.rust_to_nu
            .iter()
            .filter(|(&k, _)| k <= rust_line)
            .max_by_key(|(&k, _)| k)
            .map(|(_, &v)| v)
    }

    pub fn to_json(&self) -> String {
        serde_json::json!({
            "version": "1.0-lazy",
            "line_map": {
                "rust_to_nu": self.rust_to_nu,
                "nu_to_rust": self.nu_to_rust
            }
        }).to_string()
    }
}
```

**在代码生成时收集映射**：

```rust
// src/rust2nu/codegen.rs

impl CodeGenerator {
    fn generate_struct(&mut self, item: &ItemStruct, map: &mut LazySourceMap) -> String {
        let rust_line = item.span().start().line;
        let nu_line = self.current_line; // 当前生成的 Nu 代码行号

        // 记录映射
        map.add_mapping(rust_line, nu_line);

        // 生成 Nu 代码
        let mut output = String::new();
        output.push_str(&format!("S {} {{\n", item.ident));
        // ...
        output
    }
}
```

**CLI 输出 Sourcemap**：

```rust
// src/bin/rust2nu.rs

fn main() {
    let args = Args::parse();
    let mut map = LazySourceMap::new();

    let nu_code = convert_rust_to_nu(&rust_code, &mut map);

    if args.output_sourcemap {
        let map_path = format!("{}.map", args.output);
        fs::write(map_path, map.to_json())?;
    }

    println!("{}", nu_code);
}
```

**使用示例**：

```bash
$ rust2nu main.rs --output main.nu --sourcemap
# 生成：
# - main.nu (转换后的代码)
# - main.nu.map (Sourcemap 文件)
```

#### 1.2 VSCode 插件集成

```typescript
// src/services/sourcemapLoader.ts

export interface LazyMap {
  version: string;
  line_map: {
    rust_to_nu: Record<number, number>;
    nu_to_rust: Record<number, number>;
  };
}

export class SourcemapLoader {
  private maps: Map<string, LazyMap> = new Map();

  async loadMap(filePath: string): Promise<LazyMap | null> {
    const mapPath = `${filePath}.map`;
    try {
      const content = await fs.promises.readFile(mapPath, 'utf-8');
      const map = JSON.parse(content) as LazyMap;
      this.maps.set(filePath, map);
      return map;
    } catch (error) {
      return null;
    }
  }

  findNuLine(filePath: string, rustLine: number): number | null {
    const map = this.maps.get(filePath);
    if (!map) return null;

    const mapping = map.line_map.rust_to_nu;
    const keys = Object.keys(mapping)
      .map(Number)
      .filter(k => k <= rustLine);
    
    if (keys.length === 0) return null;
    
    const nearestKey = Math.max(...keys);
    return mapping[nearestKey];
  }
}
```

**同步滚动实现**：

```typescript
// src/features/syncScroll.ts

export class SyncScrollController {
  constructor(
    private leftEditor: vscode.TextEditor,
    private rightEditor: vscode.TextEditor,
    private mapLoader: SourcemapLoader
  ) {}

  syncLeftToRight() {
    const rustLine = this.leftEditor.selection.active.line + 1;
    const nuLine = this.mapLoader.findNuLine(
      this.leftEditor.document.fileName,
      rustLine
    );

    if (nuLine) {
      const position = new vscode.Position(nuLine - 1, 0);
      this.rightEditor.revealRange(
        new vscode.Range(position, position),
        vscode.TextEditorRevealType.InCenter
      );
    }
  }
}
```

**优点**：
- ✅ 