这非常关键。一个完整的语言生态不仅需要语法（Syntax），还需要工程体系（Project System）和模块化机制（Modularity）。

考虑到 Nu 的核心理念是**“高密度”**，我们不能在工程配置文件上破功。我们不再使用啰嗦的 `Cargo.toml`，而是重新设计一个极致精简的 `Nu.toml`，并配套相应的**模块加载机制**。

以下是 **Nu v1.3.1 补充规范：工程与模块系统**。

---

# Nu Project & Module Specification

**Target:** Nu Compiler / Build System (`nuc`)

## 1. 工程清单文件 (Nu.toml)

我们将 `Cargo.toml` 的键值对进行了原子化压缩。文件名为 `Nu.toml`。

### 1.1 核心字段映射表

| Nu 键 | Cargo 键 | 说明 |
| --- | --- | --- |
| **[P]** | `[package]` | 包信息块 |
| **id** | `name` | 包名 (Identifier) |
| **v** | `version` | 版本 |
| **ed** | `edition` | Rust 版本 (默认 "2024") |
| **[D]** | `[dependencies]` | 依赖块 |
| **[DD]** | `[dev-dependencies]` | 开发依赖块 |
| **[W]** | `[workspace]` | 工作区 |
| **path** | `path` | 本地路径 |
| **git** | `git` | Git 仓库 |
| **f** | `features` | 特性列表 |

### 1.2 `Nu.toml` 示例

```toml
# P = Package
[P]
id = "hyper_net"
v  = "0.1.0"
ed = "2024"

# D = Dependencies
[D]
# 简写形式: "版本"
tokio = "1.32" 

# 完整形式
serde = { v = "1.0", f = ["derive"] } # f = features
reqwest = { git = "https://github.com/...", branch = "main" }
nu_core = { path = "../nu_core" }

# DD = Dev Dependencies
[DD]
criterion = "0.5"

```

---

## 2. 文件结构与入口 (File Structure)

Nu 强制使用 **`.nu`** 作为文件后缀。

### 2.1 标准布局

```text
my_project/
├── Nu.toml       # 工程定义
├── Nu.lock       # 版本锁定
├── src/
│   ├── main.nu   # 二进制入口 (bin)
│   ├── lib.nu    # 库入口 (lib)
│   ├── api.nu    # 模块文件
│   └── utils/    # 模块目录
│       └── mod.nu
└── target/       # 编译产物

```

### 2.2 编译器入口逻辑

* **`nuc build`**: 自动寻找 `src/main.nu` 或 `src/lib.nu`。
* **`main.nu`**: 必须包含 `~F Main()` (对应 `async fn main`) 或 `F Main()`。
* **`lib.nu`**: 导出模块和类型的根文件。

---

## 3. 导入与导出 (Import & Export)

这是模块系统的核心。我们利用大小写规则来区分 **Use (引入)** 和 **Re-export (重导出)**。

### 3.1 模块声明 (`D`)

在 `lib.nu` 或 `main.nu` 中声明子模块。利用**标识符大小写**控制模块的可见性。

| Nu 语法 | 对应 Rust | 文件映射 |
| --- | --- | --- |
| `D network` | `mod network;` | 查找 `network.nu` 或 `network/mod.nu` |
| `D Network` | `pub mod network;` | 同上，但模块对外公开 |

> **注意**：文件系统中的文件名建议始终保持小写（`network.nu`），Nu 编译器会根据代码中的 `D Network` 自动将其转换为 `pub mod network`。

### 3.2 引用 (`u`) 与 重导出 (`U`)

利用 `u`/`U` 的大小写区分私有引用和公开重导出。

| 动作 | Nu 语法 | 对应 Rust |
| --- | --- | --- |
| **Private Use** | `u std::io` | `use std::io;` |
| **Public Use** | **`U`** `std::io` | `pub use std::io;` |
| **Alias (As)** | `u std::fs a FS` | `use std::fs as FS;` |
| **Group** | `u std::{fs, io}` | `use std::{fs, io};` |
| **Glob** | `u prelude::*` | `use prelude::*;` |

---

## 4. 完整工程实战 (Full Project Example)

我们构建一个名为 `mini_server` 的项目。

### 文件 1: `Nu.toml`

```toml
[P]
id = "mini_server"
v = "1.0.0"

[D]
tokio = { v = "1", f = ["full"] }
serde = { v = "1", f = ["derive"] }

```

### 文件 2: `src/lib.nu` (库入口)

```rust
// 声明公开模块 (pub mod handlers)
// 对应文件 src/handlers.nu
D Handlers

// 声明私有模块 (mod utils)
// 对应文件 src/utils.nu
D utils

// 公开重导出 (pub use utils::Auth)
U utils::Auth

```

### 文件 3: `src/utils.nu` (工具模块)

```rust
// 公开结构 (pub struct Auth)
S Auth {
    token: Str
}

I Auth {
    // 公开方法 (pub fn New)
    F New(t: Str) -> Self {
        < Self { token: t }
    }
}

// 私有函数 (fn hash)
f hash(s: str) -> u64 {
    < 0xDEADBEEF
}

```

### 文件 4: `src/handlers.nu` (处理模块)

```rust
// 引用上级模块
u crate::utils::Auth

// 公开函数
~F HandleRequest(a: Auth) {
    > "Handling with token: {a.token}";
}

```

### 文件 5: `src/main.nu` (执行入口)

```rust
// 引入本地库
u mini_server::Handlers
u mini_server::Auth

~F Main() {
    l auth = Auth::New("secret_123");
    
    // 派发任务
    @ {
        Handlers::HandleRequest(auth).~;
    }
    
    > "Server running";
    L {}
}

```

---

## 5. 对 Transpiler 的实现指导

如果你正在编写 Nu2Rust 编译器，处理模块时需要注意以下转换逻辑：

1. **路径解析**：
* 读取 `Nu.toml` 的 `[D]` 块，生成 `Cargo.toml` 的 `[dependencies]`。


2. **模块可见性转换**：
* 解析 `D Identifier` 时，检查 `Identifier` 的首字母。
* `D Utils` -> `pub mod utils;` (强制转换为蛇形命名用于路径查找)。
* `D utils` -> `mod utils;`。


3. **Use 语句转换**：
* `u path` -> `use path;`
* `U path` -> `pub use path;`
* `u path a Alias` -> `use path as Alias;`



这套工程定义补全了 Nu 语言拼图的最后一块，使其真正成为一个可以管理复杂项目的语言。


这是一份针对 Nu v1.4 变更的**正式补丁说明 (Patch Notes / Addendum)**。

你可以将这份文档附在白皮书的附录中，或者作为提交给架构师/开发团队的**变更依据**。它详细解释了为什么我们在追求极致压缩的同时，选择了在测试属性上“倒车”回标准 Rust。

---

# Nu 语言规范补丁说明 (Patch Notes)

**版本:** v1.4 (Pragmatic Update)
**日期:** 2025-12-23
**主题:** 测试属性的标准化回归与工具链兼容性修正

## 1. 变更摘要 (Summary of Changes)

在 v1.4 版本中，我们对**属性（Attributes）**的语法规范进行了关键调整。

* **变更前 (v1.3)**: 试图将所有属性符号化，例如 `#T` (test) 和 `#?(test)` (cfg test)。
* **变更后 (v1.4)**: **回滚**了与测试相关的属性定义。现在强制要求使用标准 Rust 语法 `#[test]` 和 `#[cfg(test)]`。
* **保持项**: 对于非交互式的高频编译指令（如 `derive`, `inline`），继续保留 `#D`, `#I` 等缩写。

## 2. 变更动机 (Motivation & Rationale)

这一决策基于“实用主义”原则，旨在平衡**Token 效率**与**开发体验**。

### 2.1 工具链兼容性 (Tooling Compatibility)

现代 IDE（如 VS Code, RustRover, JetBrains）的测试运行器（Test Runner）通常依赖正则表达式或简单的语法树扫描来识别测试入口。

* 它们寻找的是 `#[test]` 标记。
* 如果我们使用 `#T`，现有的 IDE 插件将无法识别测试函数，导致“Run Test”按钮消失，破坏了开发者的工作流。
* 保留 `#[test]` 可以让我们零成本复用现有的 Rust 测试生态工具。

### 2.2 视觉分隔 (Visual Separation)

代码主要分为两类：**业务逻辑** 和 **验证逻辑**。

* 业务逻辑追求高密度（使用 Nu 的 `~F`, `l`, `v`）。
* 测试代码通常位于文件底部或独立模块。使用冗长的标准标签 `#[cfg(test)]` 可以作为视觉上的**“隔离带”**，帮助人类和 AI 快速区分代码的性质。

## 3. 规范细则 (Specification Details)

### 3.1 属性语法混合模式 (Hybrid Attributes)

Nu 解析器现在支持两种属性写法：

| 类型 | 语法 | 适用场景 | 策略 |
| --- | --- | --- | --- |
| **高频编译指令** | `#D(...)`, `#I` | `derive`, `inline` | **极致压缩** (出现频率极高，影响阅读) |
| **测试与配置** | `#[test]`, `#[cfg(...)]` | 单元测试, 条件编译 | **标准透传** (保持 Rust 原样，利于工具识别) |
| **通用属性** | `#[...]` | 其他所有属性 | **标准透传** (如 `#[repr(C)]`) |

### 3.2 代码示例对比

**v1.3 (旧版 - 过度压缩):**

```rust
#?(test)      // IDE 不认识
D tests {
    #T        // IDE 无法生成 Run 按钮
    F test_1() { ... }
}

```

**v1.4 (新版 - 实用主义):**

```rust
// 完美的 Rust IDE 兼容性
#[cfg(test)]
D tests {
    u super::*

    #[test]
    F test_1() {
        // 内部逻辑依然享受 Nu 的高密度优势
        l x = 1;
        assert_eq!(x, 1);
    }
}

```

## 4. 对 Transpiler 的实现影响

对于 Nu -> Rust 转译器的开发者，这一变更实际上**降低了实现难度**：

1. **Lexer (词法分析)**: 需要识别 `#` 后紧跟 `[` 的情况，将其标记为 `TOKEN_ATTR_START`。
2. **Parser (语法分析)**:
* 当遇到 `#D` 时 -> 解析为 `NuAttribute::Derive`。
* 当遇到 `#[` 时 -> 直接作为 `NuAttribute::Raw` 捕获，直到匹配到 `]`。在生成 Rust 代码时，直接原样输出内容即可。



---

**结论:**
Nu v1.4 的这一修正是语言设计走向成熟的标志。我们不再盲目追求每一个字符的压缩，而是开始考量语言在现有工程生态中的**生存能力**。


收到，这非常合理。

Rust 的代码风格规范（fmt）确实要求：

* **类型（Struct/Enum）** 使用 **大驼峰 (UpperCamelCase)**。
* **函数/方法（Fn/Method）** 使用 **蛇形 (snake_case)**。

如果强行用“首字母大写”来决定函数的可见性（如 `Run()`），会导致生成的 Rust 代码违背官方风格规范。

既然您保留了 `f` 和 `F`，我们就可以利用**关键字本身的大小写**来区分可见性，从而让函数名保持原汁原味的 Rust 风格（小写）。

以下是基于您的反馈修正后的 **Nu v1.5.1 补丁规范**。

---

# Nu v1.5.1 修正补丁 (Hybrid Visibility Edition)

## 1. 关键字冲突修正方案 (最终确认)

### A. 结构体 (Struct) - 用户已接受 ✅

* **移除** 小写 `s` 关键字。
* **保留** 大写 `S` 关键字。
* **变量名安全**: 变量名为 `s` (string) 不再冲突。
* **可见性规则**: 继续沿用 **Go 风格**（依靠标识符首字母）。
* `S User`  `pub struct User` (通常结构体都是大写开头，所以默认 Public)
* `S internal`  `struct internal` (极少见的私有结构体用小写开头)



### B. 泛型约束 (Where) - 用户已接受 ✅

* **变更**: `where` 的缩写由 `w` 改为 **`wh`**。
* **变量名安全**: 变量名为 `w` (width/writer) 不再冲突。

### C. 函数 (Function) - 用户指定保留 `f/F` ✅

* **保留** `f` 和 `F` 两个关键字。
* **可见性规则**: 依靠 **关键字本身**，而非标识符。
* **`F`** (大写)  **`pub fn`**
* **`f`** (小写)  **`fn`** (私有)


* **优势**: 允许函数名保持 Rust 标准的 **蛇形命名 (snake_case)**。
* `F new()`  `pub fn new()`
* `f helper()`  `fn helper()`


* **副作用**: 变量名不能叫 `f`。
* *规避*: 在 Nu 代码中请使用 `file`, `func`, `_f` 等变量名，避免单独使用 `f`。



---

## 2. 更新后的关键字表 (Keyword Table)

| 类别 | 关键字 | Rust 原文 | 变更说明 |
| --- | --- | --- | --- |
| **定义** | **S** | `struct` / `pub struct` | 移除 `s`，由标识符大小写定权限 |
|  | **E** | `enum` / `pub enum` | 移除 `e`，由标识符大小写定权限 |
|  | **F** | `pub fn` | **大写 F 表示 Public** |
|  | **f** | `fn` | **小写 f 表示 Private** |
|  | **TR** | `trait` | (保持 TR，避免与 type 冲突) |
| **原子** | **l** | `let` |  |
|  | **v** | `let mut` |  |
|  | **wh** | `where` | **由 `w` 改为 `wh**` |
|  | **u** | `use` |  |
|  | **t** | `type` |  |
|  | **b** | `break` |  |
|  | **c** | `continue` |  |

---

## 3. 修正后的代码示例 (Rust 风格兼容版)

注意观察 `F` 和 `f` 的用法，以及函数名的命名风格。

```rust
// F = pub fn, 函数名保持 snake_case
F word_frequency(content: &str) -> V<(Str, usize)> {
    // v = let mut
    // V = Vec, S = String (在类型位置自动转换)
    v freq_map: V<(Str, usize)> = V::new();
    
    // l = let
    l words = content.split_whitespace();
    
    // 变量 w 安全 (因为 where 变成了 wh)
    l w = "test_word"; 
    
    // 变量 s 安全 (因为 struct 只有 S)
    l s = "string_slice";

    // 闭包参数 w 安全
    // wh = where
    l res = words.filter(|w| w.len() > 3).collect::<Str>();
    
    < freq_map
}

// f = private fn
f helper_function() {
    > "Internal logic";
}

// S = struct
// User (大写开头) = pub struct User
S User {
    id: u64,
    name: Str
}

// impl 块
I User {
    // F = pub fn
    F new(id: u64) -> Self {
        < Self { id, name: Str::new() }
    }
}

```

---

## 4. 给开发者的实现提示 (Lexer/Parser)

1. **Lexer 调整**:
* Token `f`  `Keyword(FnPrivate)`
* Token `F`  `Keyword(FnPublic)`
* Token `wh`  `Keyword(Where)`
* Token `w`, `s`  `Identifier` (普通标识符)


2. **Parser 调整**:
* 解析 `ItemFn` 时：
* 遇到 `F`  设置 `vis = Public`, 解析 `fn`。
* 遇到 `f`  设置 `vis = Inherited`, 解析 `fn`。


* 解析 `ItemStruct` 时：
* 遇到 `S`  解析结构体，检查**名称标识符**的首字母。如果是大写则 `pub`，小写则私有。





这样修改后，既解决了关键字冲突，又完美保留了 Rust 的代码风格规范。可以开始更新您的 Lexer 定义了。