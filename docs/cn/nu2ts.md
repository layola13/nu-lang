这是 **Nu v1.6 to TypeScript (nu2ts) 完整映射规范**。

这份文档是编译器后端开发的“圣经”，定义了从 Nu AST 节点生成 TypeScript 代码的所有规则。

---

# Nu v1.6 -> TypeScript 映射规范手册

**核心原则：**

1. **类型降维 (Type Erasure)**：所有内存管理语义（Borrow checker, Lifetimes, Smart Pointers）在编译时剔除。
2. **语义对齐 (Semantic Alignment)**：将 Nu 的高密度符号转换为 TS 的标准关键字。
3. **零运行时 (Zero-Runtime)**：除了极小的 `Result` 类型定义外，不引入任何运行时库。

---

## 1. 模块与文件结构 (Modules & Files)

Nu 的模块系统映射为 ES Modules。

| Nu v1.6 AST 节点 | TypeScript 输出 | 编译器处理逻辑 |
| --- | --- | --- |
| **D** `my_mod` | `import * as my_mod from "./my_mod";` | `D` (Mod) 视作文件引入或命名空间 |
| **u** `std::...` | *(忽略或映射)* | 标准库引用通常映射为原生 JS 对象，无需 import |
| **u** `crate::utils` | `import { ... } from "./utils";` | 内部模块引用转换为相对路径 import |
| **u** `super::*` | `import * from "../index";` | 路径层级修正 |

---

## 2. 定义与可见性 (Definitions & Visibility)

利用 Nu v1.6 的大小写规则控制 TS 的 `export`。

| 类别 | Nu v1.6 源码 | TypeScript 输出 | 规则说明 |
| --- | --- | --- | --- |
| **函数** | **F** `run()` | `export function run()` | **F** (大写) = `export` |
|  | **f** `help()` | `function help()` | **f** (小写) = 模块内私有 |
| **结构体** | **S** `User` | `export class User` | 标识符首字母**大写** = `export` |
|  | **S** `meta` | `class meta` | 标识符首字母**小写** = 私有 |
| **枚举** | **E** `State` | `export type State = ...` | 映射为 Discriminated Union 类型 |
| **常量** | **C** `MAX` | `export const MAX` | 默认导出常量 |
| **Trait** | **TR** `Fly` | `export interface Fly` | Trait 映射为 Interface |
| **Impl** | **I** `User` | *(合并入 Class)* | TS 不支持分离 impl，需在 Class 内部生成方法 |

---

## 3. 基础类型映射 (Primitive Types)

Nu 的严格类型系统降级为 TS 的宽泛类型。

| Nu 类型 | TypeScript 类型 | 备注 |
| --- | --- | --- |
| `i8`, `i16`...`i64` | `number` | TS 不区分整型精度 |
| `u8`, `u16`...`u64` | `number` | 同上 |
| `f32`, `f64` | `number` | 同上 |
| `bool` | `boolean` | - |
| `char` | `string` | 单字符也是 string |
| `Str` (`String`) | `string` | **Owned String** 映射为 string |
| `str` (`&str`) | `string` | **Slice** 映射为 string |
| `()` (Unit) | `void` | 仅用于返回值 |
| `any` / `_` | `any` | 类型推断占位符 |

---

## 4. 复杂类型与容器 (Complex Types)

| Nu 类型 | TypeScript 类型 | 结构/备注 |
| --- | --- | --- |
| **V**`<T>` (`Vec`) | `Array<T>` | 或 `T[]` |
| **O**`<T>` (`Option`) | `T | null` | **重要**：不使用 `undefined`，统一用 `null` 表示 None |
| **R**`<T, E>` (`Result`) | `Result<T, E>` | 需在头部生成 Type Alias (见第9节) |
| **(A, B)** (Tuple) | `[A, B]` | 固定长度数组 |
| **[T; N]** (Array) | `Array<T>` | 忽略长度约束 `N` |
| `HashMap<K, V>` | `Map<K, V>` | - |

---

## 5. 内存管理与指针 (Memory Model - Erasure)

这是编译器的核心：**剥离所有内存外壳**。

| Nu v1.6 类型 | TypeScript 类型 | 编译器动作 |
| --- | --- | --- |
| **B**`<T>` (`Box`) | `T` | 移除 `Box` 包装 |
| **A**`<T>` (`Arc`) | `T` | 移除引用计数包装 |
| **X**`<T>` (`Mutex`) | `T` | 移除锁包装 (JS 单线程无锁) |
| **W**`<T>` (`Weak`) | `T` | 移除弱引用包装 |
| `&T` (Ref) | `T` | 移除引用符号 `&` |
| `&!T` (Mut Ref) | `T` | 移除可变引用符号 |
| `*T` (Ptr) | `T` | 移除解引用符号 |
| `'a` (Lifetime) | *(None)* | **完全删除** 所有生命周期标注 |

---

## 6. 流程控制与符号 (Control Flow Symbols)

Nu v1.6 的高密度符号还原为 TS 关键字。

| Nu v1.6 符号 | TypeScript 语句 | 上下文约束 |
| --- | --- | --- |
| **<** `val` | `return val;` | 当 `<` 位于语句**行首**时 |
| **<** | `return;` | 无返回值返回 |
| **?** `x > 0 {}` | `if (x > 0) {}` | `?` 映射为 `if` |
| **br** | `break;` | **v1.6 规范** |
| **ct** | `continue;` | **v1.6 规范** |
| **L** `{}` | `while (true) {}` | 无限循环 |
| **L** `x: list` | `for (const x of list)` | 迭代器循环 |
| **M** `val {}` | `switch (val)` | 简单模式匹配 |
| **wh** | *(Generic Constraint)* | 仅在泛型定义中使用，TS编译时忽略 |

---

## 7. 运算符与表达式 (Operators & Expressions)

| Nu 表达式 | TypeScript 表达式 | 转换逻辑 |
| --- | --- | --- |
| `val`**!** | `val!` | **后缀!**：若为 Option，转为非空断言 |
| `func()`**!** | *(Result Unwrap)* | 若为 Result，需生成 helper 调用或 throw |
| `>` | `>` | **大于号** (无歧义) |
| `as` / **a** | `as` | 类型转换 |
| `..` (Range) | *(Loop Logic)* | `0..10` 在 `L` 中转为 `for(let i=0; i<10; i++)` |
| ` |  | ` (Closure) |
| ` | x | ` |
| `match` 分支 | `case` | 需要将模式解构转换为 JS 变量解构 |

---

## 8. 链式调用处理 (Chain Handling - The "Onion Peeling")

针对 `.iter().map().collect()` 的特殊处理。

| Nu 方法链 | TypeScript 映射 | 处理策略 |
| --- | --- | --- |
| `.iter()` | *(删除)* | **幽灵节点**：直接忽略，提升 receiver |
| `.into_iter()` | *(删除)* | **幽灵节点**：同上 |
| `.collect::<T>()` | *(删除)* | **终止节点**：直接忽略，TS Map 即返回数组 |
| `.len()` | `.length` | **属性化**：去除括号，改为属性访问 |
| `.unwrap()` | `!` | 转换为非空断言 (Option场景) |
| `.unwrap_or(v)` | `?? v` | 转换为 Nullish Coalescing 操作符 |
| `.clone()` | `structuredClone()` | 深拷贝 (可选，或直接忽略引用) |
| `.to_string()` | `.toString()` | - |
| `.push()` | `.push()` | - |
| `.contains(x)` | `.includes(x)` | 方法更名 |

---

## 9. 宏与元编程映射 (Macros)

Nu v1.6 恢复了宏的原生语法。

| Nu 宏调用 | TypeScript 输出 | 转换细节 |
| --- | --- | --- |
| `println!("Msg")` | `console.log("Msg")` | - |
| `println!("{}", x)` | `console.log(x)` | 简化格式化字符串，直接传参 |
| `format!("a{}", x)` | ``a${x}`` | 转换为 JS **模板字符串** |
| `vec![1, 2]` | `[1, 2]` | 转换为数组字面量 |
| `panic!("Err")` | `throw new Error("Err")` | - |
| `assert!(cond)` | `if(!cond) throw...` | 或保留 `console.assert` |
| `todo!()` | `throw new Error("TODO")` | - |

---

## 10. 必需的 Micro-Prelude (Standard Polyfill)

为了支持 `Result` 类型，编译器必须在生成的每个 TS 文件（或统一的 header）中注入以下类型定义：

```typescript
// --- Nu2TS Runtime Polyfill ---
export type Result<T, E> = { tag: 'ok', val: T } | { tag: 'err', err: E };
export const Ok = <T>(val: T): Result<T, any> => ({ tag: 'ok', val });
export const Err = <E>(err: E): Result<any, E> => ({ tag: 'err', err });
// ------------------------------

```

---

## 11. 实战转换示例 (Example)

**Nu v1.6 Source:**

```rust
F process(list: V<i32>) -> R<i32, Str> {
    L x: list {
        ? x > 10 {
            println!("Big: {}", x);
            br;
        }
    }
    < Ok(0)
}

```

**TypeScript Target:**

```typescript
export function process(list: Array<number>): Result<number, string> {
    for (const x of list) {
        if (x > 10) {
            console.log("Big: ", x); // 格式化串被简化
            break; // br -> break
        }
    }
    return Ok(0); // < -> return
}

```