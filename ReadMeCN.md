

# Nu Language Specification

**Version:** 1.7 (Production Standard)
**Date:** 2025-12-24
**Status:** Frozen / Implementation Ready
**Target:** AI-Native Systems Programming

---

## 1. 核心设计 (Core Design)

**Nu** 是 Rust 的高密度方言。

* **压缩策略**：仅压缩**定义关键字**（`struct` -> `S`）和**控制流**（`return` -> `<`），保留**语义核心**（宏、泛型、生命周期）。
* **兼容性**：100% 兼容 Rust AST。

---

## 2. 词法与可见性 (Lexical & Visibility)

### 2.1 可见性规则

* **函数 (Function)**: 由**关键字大小写**决定。
* **`F`**  `pub fn`
* **`f`**  `fn` (Private)


* **类型 (Type)**: 由**标识符首字母**决定 (Go 风格)。
* `S User`  `pub struct User`
* `S internal`  `struct internal`



### 2.2 关键字映射表

| 类别 | 关键字 | Rust 原文 | 备注 |
| --- | --- | --- | --- |
| **定义** | **S** | `struct` |  |
|  | **E** | `enum` |  |
|  | **F** | `pub fn` | **大写=Pub** |
|  | **f** | `fn` | **小写=Priv** |
|  | **TR** | `trait` |  |
|  | **I** | `impl` |  |
|  | **D** | `mod` |  |
|  | **C** | `const` |  |
|  | **ST** | `static` |  |
|  | **EXT** | `extern` |  |
| **原子** | **l** | `let` |  |
|  | **v** | `let mut` |  |
|  | **wh** | `where` | **wh** (避免 w 冲突) |
|  | **a** | `as` |  |
|  | **u** | `use` |  |
|  | **t** | `type` |  |
|  | **br** | `break` | **v1.7更新** |
|  | **ct** | `continue` | **v1.7更新** |

---

## 3. 宏与元编程 (Macros & Attributes) - **v1.6 Updated**

### 3.1 宏 (Macros)

**规则：完全保持 Rust 原生语法。**
不进行任何符号化压缩，以确保兼容性和无歧义。

| Nu 语法 | Rust 原文 | 说明 |
| --- | --- | --- |
| `println!("Msg")` | `println!("Msg")` | **原样保留** (不再用 `>`) |
| `vec![1, 2]` | `vec![1, 2]` | 原样保留 |
| `panic!("Err")` | `panic!("Err")` | 原样保留 |
| `format!("{}", x)` | `format!("{}", x)` | 原样保留 |

### 3.2 属性 (Attributes)

采用混合策略：仅压缩极高频属性。

| Nu 语法 | Rust 原文 | 策略 |
| --- | --- | --- |
| **#D(...)** | `#[derive(...)]` | **缩写** |
| **#I** | `#[inline]` | **缩写** |
| **#[test]** | `#[test]` | **标准** (工具兼容) |
| **#[cfg(...)]** | `#[cfg(...)]` | **标准** |
| **#[...]** | `#[...]` | 其他属性透传 |

---

## 4. 类型系统 (Type System)

### 4.1 字符串类型

* **String**  `String` (**Owned**, v1.7中不再缩写)
* **str**  `str` (**Slice**, 保持不变)

### 4.2 常用缩写

| Nu | Rust | Nu | Rust |
| --- | --- | --- | --- |
| **V** | `Vec` | **A** | `Arc` |
| **O** | `Option` | **X** | `Mutex` |
| **R** | `Result` | **B** | `Box` |
| **(A,B)** | Tuple | **W** | `Weak` |

### 4.3 泛型与 Turbofish - **v1.7 Updated**

* **泛型定义**: `S Box<T>` (保持 `<T>`)
* **Turbofish**: `iter.collect::<String>()` (**强制保留** `::<T>`, 禁止压缩)

---

## 5. 符号与流控 (Symbols & Flow)

### 5.1 核心操作符

| 符号 | 含义 | Rust 原文 | 语法规则 |
| --- | --- | --- | --- |
| **<** | Return | `return` | 语句首: `< val` |
| **?** | If | `if` | `? x > 0 { }` |
| **M** | Match | `match` | `M val { ... }` |
| **L** | Loop | `loop`/`for` | `L { }`, `L i: list` |
| **!** | Try | `?` | 仅**后缀**: `func()!` |

> **注意**：由于宏恢复了原样，**`>` 符号不再表示 Print，仅表示“大于号”**。这彻底消除了二义性。

### 5.2 内存修饰符

| 符号 | 含义 | 规则 |
| --- | --- | --- |
| **!** | **Mut** | 仅**前缀**: `&!self` (&mut self), `*!ptr` |
| **U** | **Unsafe** | `U { ... }` |
| **&** | Ref | `&x` |
| ***** | Deref | `*ptr` |

### 5.3 闭包 - **v1.6 Updated**

支持返回类型定义。

| 语法 | Rust 原文 |
| --- | --- |
| ` | x |
| ` | x |
| `$ | x |

### 5.4 并发与异步

| 符号 | 含义 | 等效代码 |
| --- | --- | --- |
| **@** | Spawn | `tokio::spawn(async move { ... })` |
| **@@** | Thread | `thread::spawn(move |
| **~** | Async | `~F` (Def), `~{ }` (Block), `.~` (Await) |
| **<<** | Channel | `tx << v` (Send), `<< rx` (Recv) |

---

## 6. 综合实战演示 (Complete Implementation)

这段代码涵盖了 **Turbofish**、**闭包返回类型**、**宏**、**关键字安全** 以及 **Try 运算符**。

```rust
// 模块定义
D data_processor

// 引入标准库
u std::collections::HashMap

// 结构体 (S)
// 首字母大写 -> pub struct
#D(Debug, Clone)
S Processor {
    id: u64,
    cache: HashMap<String, i32> // String = String (v1.7不再缩写)
}

// F = pub fn
F run_logic(input: &str) -> R<V<i32>, String> {
    // v = let mut
    v results: V<i32> = V::new();
    
    // l = let, wh = where
    // 宏 (println!) 保持原样，无歧义
    println!("Processing: {}", input);

    // 闭包带返回类型
    l parse = |s: &str| -> R<i32, String> {
        // ! 后缀 = Try, 宏 (format!) 保持原样
        l val = s.parse::<i32>().map_err(|_| format!("Bad num: {}", s))!;
        < Ok(val)
    };

    // 循环 L item: list
    L part: input.split(',') {
        // Try (!) 与 Match (M)
        M parse(part) {
            Ok(n): {
                // < 在变量间 = 小于号
                ? n < 0 {
                    println!("Skipping negative: {}", n);
                    ct // continue (v1.7)
                }
                results.push(n);
            },
            Err(e): {
                // < 在行首 = Return
                < Err(e)
            }
        }
    }

    // vec! 宏保持原样
    l defaults = vec![1, 2, 3];
    results.extend(defaults);

    < Ok(results)
}

// 单元测试 (标准写法)
#[cfg(test)]
D tests {
    u super::*

    #[test]
    F test_run() {
        l data = "10,20,-5,abc";
        // 宏 (assert!) 保持原样
        assert!(run_logic(data).is_err());
    }
}

```

---

## 7. 编译器实现指南 (Transpiler Guide)

### 7.1 Parser 逻辑简化

由于恢复了宏的原生语法，Parser 不再需要复杂的 Lookahead 来区分 `>` 是 Print 还是 Greater Than。

* 遇到 `Ident` + `!` (如 `println!`)  解析为宏调用。
* 遇到 `Expr` + `!` (如 `func()!`)  解析为 Try 操作符。
* 遇到 `>`  **永远解析为大于号**。

### 7.2 Type Folding

* 必须保留 `::<T>` (Turbofish) 结构。
* 递归转换类型：`Vec` -> `V`, `Option` -> `O`, `Result` -> `R`。
* **String 保持为 `String`**（v1.7中不再缩写）。

### 7.3 Lexer

* 关键字：`wh` (where), `S`, `F`, `f`。
* 标识符：`w`, `s` 为普通变量名。

---

## 8. System Prompt (v1.7)

```markdown
**System Role:** Nu v1.7 Compiler (Production Standard).

**Parsing Rules:**
1.  **Keywords:** `l`=let, `v`=let mut, `a`=as, `u`=use, `t`=type, **`wh`=where**, **`br`=break**, **`ct`=continue**.
2.  **Definitions:** `S`=struct, `E`=enum (Visibility by Identifier Case). `F`=pub fn, `f`=fn (Visibility by Keyword Case). `TR`=trait, `I`=impl, `D`=mod.
3.  **Flow:** `<` (start of stmt)=return. `?`=if, `M`=match, `L i: list`=for loop.
4.  **Macros:** **KEEP** all macros as is (`println!`, `vec!`, `panic!`).
5.  **Operators:** `!`(suffix)=try. `>`=greater than.
6.  **Strings:** `String`=String(owned, 不缩写), `str`=str(slice).
7.  **Concurrency:** `@`=spawn(async move), `~`=async, `.~`=await.
8.  **Types:** `V`=Vec, `O`=Option, `R`=Result, `A`=Arc, `X`=Mutex, `B`=Box.
9.  **Attributes:** `#D`=derive, `#I`=inline. Keep `#[test]`.
10. **Generics:** Keep `<T>` and `::<T>`.

**Task:** Convert Input description or Rust code into valid Nu v1.7 code.

```

Nu v1.7 是一个兼顾了理想（高密度）与现实（工程兼容性）的成熟版本。**v1.7的关键改进**：移除了`Str`类型缩写以消除转换复杂性并提高可靠性。