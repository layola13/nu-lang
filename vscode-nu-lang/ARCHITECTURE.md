
# VSCode Nu Lang Density Lens - 架构设计文档

## 📋 需求总结

根据战略定位文档 (HANDWRITING_FIRST_STRATEGY.md)，核心理念是：

**让开发者手写 Nu 代码，通过 SourceMap 实现无感知的 Rust 错误回溯**

### 关键角色定位
- Nu (.nu) = 源代码 (开发者手写)
- Rust (.rs) = 构建产物 (编译器生成)
- SourceMap (.map) = 生死攸关 (错误回溯桥梁)

### 功能优先级 (基于文档评估)

#### P0 - 必须实现 (生存线)
1. ✅ nu2rust 的正确性 - 生成 100% 可编译的 Rust 代码
2. ✅ nu2rust 的 SourceMap 生成 - 每次转换生成精确源映射
3. ✅ VSCode 错误映射 - 将 Cargo 错误映射到 Nu 编辑器

#### P1 - 应该实现 (竞争力)
4. 实时编译 (保存触发)
5. 智能感知 (IntelliSense)

#### P2 - 可以实现 (锦上添花)
6. rust2nu 迁移工具 (一次性导入)

---

## 🏗️ 系统架构

### 整体数据流

```
┌─────────────────────────────────────────────────────────────┐
│                    VSCode 编辑器                              │
│  ┌──────────────┐                      ┌──────────────┐     │
│  │  main.nu     │  保存触发            │  main.rs     │     │
│  │  (用户编写)   │ ───────────>        │  (生成)       │     │
│  └──────┬───────┘                      └──────┬───────┘     │
│         │                                      │             │
│         │ ① 触发编译                           │ ③ cargo check│
│         v                                      v             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │         Nu2Rust Service (TypeScript)                │   │
│  │  ┌──────────────────────────────────────────────┐  │   │
│  │  │ 调用 nu2rust CLI (Rust binary)               │  │   │
│  │  │  - 输入: main.nu                             │  │   │
│  │  │  - 输出: main.rs + main.rs.map               │  │   │
│  │  └──────────────────────────────────────────────┘  │   │
│  └────────────────────┬────────────────────────────────┘   │
│                       │ ② 生成 .rs + .map                   │
│                       v                                     │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              SourceMap (.map 文件)                   │   │
│  │  {                                                   │   │
│  │    "mappings": [                                     │   │
│  │      {"rust": {"line": 47}, "nu": {"line": 3}}     │   │
│  │    ]                                                 │   │
│  │  }                                                   │   │
│  └────────────────────┬────────────────────────────────┘   │
│                       │ ④ 错误映射                          │
│                       v                                     │
│  ┌─────────────────────────────────────────────────────┐   │
│  │         Error Mapper (TypeScript)                    │   │
│  │  - 读取 cargo check 错误: main.rs:47:15             │   │
│  │  - 查询 .map: rust L47 → nu L3                      │   │
│  │  - 在 Nu 编辑器显示红色波浪线                         │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 模块划分

```
vscode-nu-lang/
├── src/
│   ├── extension.ts                    # 插件入口
│   │   - 注册命令
│   │   - 初始化服务
│   │   - 管理生命周期
│   │
│   ├── services/
│   │   ├── conversionService.ts        # 核心转换服务
│   │   │   - rust2nu(code): Promise<Result>
│   │   │   - nu2rust(code): Promise<Result>
│   │   │   - 管理子进程调用 CLI
│   │   │
│   │   ├── sourcemapService.ts         # SourceMap 管理
│   │   │   - loadMap(filePath): Promise<SourceMap>
│   │   │   - findNuPosition(rustPos): NuPosition
│   │   │   - findRustPosition(nuPos): RustPosition
│   │   │
│   │   ├── cargoService.ts             # Cargo 集成
│   │   │   - runCheck(rsPath): Promise<Diagnostic[]>
│   │   │   - parseErrors(output): Diagnostic[]
│   │   │
│   │   └── binaryManager.ts            # CLI 二进制管理
│   │       - detectBinary(name): string
│   │       - validateBinary(path): boolean
│   │
│   ├── features/
│   │   ├── autoCompile.ts              # 自动编译
│   │   │   - 监听 .nu 文件保存
│   │   │   - 触发 nu2rust 编译
│   │   │   - 生成 .rs + .map
│   │   │
│   │   ├── errorMapper.ts              # 错误映射
│   │   │   - mapRustErrors(diagnostics, map)
│   │   │   - 创建 VSCode Diagnostic
│   │   │   - 显示红色波浪线
│   │   │
│   │   ├── densityLens.ts              # Density Lens 视图
│   │   │   - openCompressedView()
│   │   │   - openSafetyView()
│   │   │   - 分栏显示
│   │   │
│   │   └── syncScroll.ts               # 同步滚动 (Phase 3)
│   │       - 基于 SourceMap 的精确滚动
│   │
│   ├── ui/
│   │   ├── statusBar.ts                # 状态栏显示
│   │   │   - 显示编译状态
│   │   │   - 显示统计数据
│   │   │
│   │   └── diagnosticProvider.ts       # 诊断信息提供者
│   │       - 管理错误列表
│   │
│   └── utils/
│       ├── logger.ts                   # 日志工具
│       └── config.ts                   # 配置管理
│
├── package.json                        # 插件清单
├── tsconfig.json                       # TypeScript 配置
└── README.md                           # 文档
```

---

## 🔑 核心实现：SourceMap

### SourceMap 数据格式

基于 SOURCEMAP_IMPLEMENTATION.md 的 Phase 1 设计：

```typescript
// LazySourceMap (简化版，用于 MVP)
interface LazySourceMap {
  version: string;          // "1.0-lazy"
  file: string;            // "main.rs"
  nu_file: string;         // "main.nu"
  line_map: {
    rust_to_nu: Record<number, number>;  // { 47: 3, 50: 5 }
    nu_to_rust: Record<number, number>;  // { 3: 47, 5: 50 }
  };
}
```

### Rust 端实现 (rust2nu/nu2rust)

需要在 `src/rust2nu/mod.rs` 和 `src/nu2rust/mod.rs` 中添加：

```rust
// src/rust2nu/sourcemap.rs
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LazySourceMap {
    pub version: String,
    pub file: String,
    pub nu_file: String,
    pub line_map: LineMap,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LineMap {
    pub rust_to_nu: HashMap<usize, usize>,
    pub nu_to_rust: HashMap<usize, usize>,
}

impl LazySourceMap {
    pub fn new(rust_file: &str, nu_file: &str) -> Self {
        Self {
            version: "1.0-lazy".to_string(),
            file: rust_file.to_string(),
            nu_file: nu_file.to_string(),
            line_map: LineMap {
                rust_to_nu: HashMap::new(),
                nu_to_rust: HashMap::new(),
            },
        }
    }

    pub fn add_mapping(&mut self, rust_line: usize, nu_line: usize) {
        self.line_map.rust_to_nu.insert(rust_line, nu_line);
        self.line_map.nu_to_rust.insert(nu_line, rust_line);
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
```

### TypeScript 端实现

```typescript
// src/services/sourcemapService.ts
export interface LazySourceMap {
  version: string;
  file: string;
  nu_file: string;
  line_map: {
    rust_to_nu: Record<number, number>;
    nu_to_rust: Record<number, number>;
  };
}

export class SourceMapService {
  private maps = new Map<string, LazySourceMap>();

  async loadMap(filePath: string): Promise<LazySourceMap | null> {
    const mapPath = `${filePath}.map`;
    try {
      const content = await vscode.workspace.fs.readFile(
        vscode.Uri.file(mapPath)
      );
      const map = JSON.parse(content.toString()) as LazySourceMap;
      this.maps.set(filePath, map);
      return map;
    } catch (error) {
      console.error(`Failed to load sourcemap: ${mapPath}`, error);
      return null;
    }
  }

  findNuLine(rustFile: string, rustLine: number): number | null {
    const map = this.maps.get(rustFile);
    if (!map) return null;

    // 查找 <= rustLine 的最大键 (最近映射)
    const keys = Object.keys(map.line_map.rust_to_nu)
      .map(Number)
      .filter(k => k <= rustLine);
    
    if (keys.length === 0) return null;
    
    const nearestKey = Math.max(...keys);
    return map.line_map.rust_to_nu[nearestKey];
  }

  findRustLine(nuFile: string, nuLine: number): number | null {
    const map = this.maps.get(nuFile);
    if (!map) return null;

    const keys = Object.keys(map.line_map.nu_to_rust)
      .map(Number)
      .filter(k => k <= nuLine);
    
    if (keys.length === 0) return null;
    
    const nearestKey = Math.max(...keys);
    return map.line_map.nu_to_rust[nearestKey];
  }
}
```

---

## 🎯 Phase 1 实施计划 (MVP)

### 目标
- 实现"手写 Nu，看到 Rust 错误"的核心体验
- 时间：2-3 周

### 任务清单

#### 1. Rust CLI 增强 (nu2rust)
- [ ] 在 `src/nu2rust/mod.rs` 中添加 `LazySourceMap` 支持
- [ ] 修改代码生成器，记录行号映射
- [ ] 添加 `--sourcemap` CLI 参数
- [ ] 输出 `.rs.map` 文件

#### 2. VSCode 插件基础设施
- [ ] 初始化 TypeScript 项目 (`npm init`, `tsconfig.json`)
- [ ] 安装依赖 (`@types/vscode`, `@types/node`)
- [ ] 创建 `src/extension.ts` 入口
- [ ] 配置编译脚本

#### 3. 核心服务实现
- [ ] `conversionService.ts` - 调用 nu2rust CLI
- [ ] `sourcemapService.ts` - 加载和查询 .map 文件
- [ ] `cargoService.ts` - 运行 cargo check
- [ ] `binaryManager.ts` - 检测 nu2rust 路径

#### 4. 自动编译功能
- [ ] `autoCompile.ts` - 监听 .nu 文件保存
- [ ] 触发 nu2rust 编译
- [ ] 生成 .rs 和 .map 文件
- [ ] 触发 cargo check

#### 5. 错误映射功能
- [ ] `errorMapper.ts` - 解析 cargo 错误
- [ ] 查询 SourceMap 获取 Nu 位置
- [ ] 创建 VSCode Diagnostic
- [ ] 在 Nu 编辑器显示红色波浪线

#### 6. 用户界面
- [ ] 状态栏显示编译状态
- [ ] 配置项 (nu2rust 路径)
- [ ] 命令注册

#### 7. 测试与验证
- [ ] 创建测试 Nu 文件
- [ ] 验证编译流程
- [ ] 验证错误映射准确性
- [ ] 跨平台测试

---

## 🔄 Phase 2 扩展 (Density Lens)

### 目标
- 添加可视化对比视图
- 实时统计数据
- 时间：3-4 周

### 额外功能
- rust2nu 压缩视图 (学习工具)
- Token 估算 (使用 gpt-tokenizer)
- 分栏并排显示
- 实时刷新 (debounce)

---

## 🚀 Phase 3 