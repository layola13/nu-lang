这是一个经过**重大修正**的 Nu v1.6 转 TypeScript 详细映射清单。

采纳了您的评估意见，**我们放弃"绝对零运行时"的执念，转为"微运行时（Micro-Runtime）"策略**。这意味着编译器将在输出中注入约 30-50 行核心代码（Polyfill），以确保 `Result`、`?` 操作符和 `format!` 的行为正确性。

同时，已严格执行**恢复 `String` 写法**（不再使用 `Str`）的指令。

---

# Nu v1.6 to TypeScript 完整映射详单 (Revised)

**版本**: v1.6.2 (Post-Critique)
**核心策略**: Type Erasure + Micro-Runtime Injection
**关键变更**: `String` 回归，引入错误处理 Helper，增强模式匹配。

---

## 1. 必需的微运行时 (Micro-Runtime Polyfill)

为了解决评估中指出的"错误处理体系崩塌"问题，生成的 TS 代码**必须**包含（或引用）以下定义。

| Helper 函数/类型 | TypeScript 实现逻辑 | 解决问题 |
| --- | --- | --- |
| **Result Type** | `type Result<T, E> = { tag: 'ok', val: T } \| { tag: 'err', err: E };` | 弥补 TS 缺失的 Result 类型 |
| **Constructors** | `const Ok = <T>(v: T) => ({ tag: 'ok', val: v });`<br>`const Err = <E>(e: E) => ({ tag: 'err', err: e });` | 构造 Result |
| **$unwrap** | `function $unwrap<T, E>(r: Result<T, E>): T { if(r.tag==='err') throw r.err; return r.val; }` | **修复**: 防止 `Result` 被错误地仅当做 Option 处理 |
| **$fmt** | *(简易版)* `(s: string, ...a: any[]) => s.replace(/{}/g, () => String(a.shift()));` | **修复**: 提供 `format!` 的运行时支持 |

### 完整 Micro-Runtime 代码

```typescript
// ==================== Nu2TS Micro-Runtime ====================
// Auto-generated polyfill for Nu v1.6 -> TypeScript compilation
// Size: ~40 lines, <1KB minified

// Result Type
export type Result<T, E> = 
  | { tag: 'ok'; val: T }
  | { tag: 'err'; err: E };

// Result Constructors
export const Ok = <T>(val: T): Result<T, any> => ({ tag: 'ok', val });
export const Err = <E>(err: E): Result<any, E> => ({ tag: 'err', err });

// Unwrap Helper (throws on error)
export function $unwrap<T, E>(r: Result<T, E>): T {
  if (r.tag === 'err') {
    throw new Error(`Unwrap failed: ${r.err}`);
  }
  return r.val;
}

// Format String Helper (simple implementation)
export function $fmt(template: string, ...args: any[]): string {
  let i = 0;
  return template.replace(/{}/g, () => {
    return i < args.length ? String(args[i++]) : '{}';
  });
}

// Option Helpers (optional, can be inlined)
export function isSome<T>(opt: T | null): opt is T {
  return opt !== null;
}

export function isNone<T>(opt: T | null): opt is null {
  return opt === null;
}

// Match Helper for complex patterns (optional)
export function $match<T, R>(
  value: T,
  patterns: Array<[(v: T) => boolean, (v: T) => R]>
): R {
  for (const [predicate, handler] of patterns) {
    if (predicate(value)) {
      return handler(value);
    }
  }
  throw new Error('Non-exhaustive match');
}
// ============================================================
```

---

## 2. 类型系统映射 (Type System Mapping)

**注意**: 已恢复使用 `String`。

### 2.1 基础类型

| Nu v1.6 类型 | TypeScript 目标 | 备注 |
| --- | --- | --- |
| **String** | `string` | **已修正**: 不再是 `Str`，所有权语义被擦除 |
| `&str` / `str` | `string` | 切片也映射为 string |
| `char` | `string` | TS 无字符类型 |
| `i8`..`i64` / `u8`..`u64` | `number` | 忽略整数溢出风险，统一用 double |
| `f32` / `f64` | `number` | - |
| `bool` | `boolean` | - |
| `()` (Unit) | `void` | 仅用于返回值，变量中可忽略 |

### 2.2 复合类型

| Nu v1.6 类型 | TypeScript 目标 | 映射策略 |
| --- | --- | --- |
| **Vec**`<T>` | `Array<T>` | 使用原生数组 |
| **Option**`<T>` | `T \| null` | 使用 `null` 代表 None (不使用 undefined) |
| **Result**`<T, E>` | `Result<T, E>` | 使用 Micro-Runtime 定义 |
| **(A, B)** (Tuple) | `[A, B]` | 固定长度数组 |
| `HashMap<K, V>` | `Map<K, V>` | 原生 Map |

### 2.3 智能指针 (完全擦除)

| Nu v1.6 写法 | TypeScript 目标 | 编译器动作 |
| --- | --- | --- |
| **Box**`<T>` | `T` | 移除包装 |
| **Arc**`<T>` | `T` | 移除包装 |
| **Mutex**`<T>` | `T` | **警告**: 移除锁语义，可能需在代码中注入 `// WARNING: Unsafe concurrency` |
| `&T`, `&mut T` | `T` | 移除所有引用符号 |
| `'a` | *(None)* | 删除所有生命周期标注 |

---

## 3. 错误处理与流控 (Error Handling & Flow)

这是本次修正的重点，解决了 `?` 操作符和 `unwrap` 的安全性。

### 3.1 运算符映射

| Nu v1.6 符号 | 上下文 | TypeScript 生成代码 (AST 展开) | 备注 |
| --- | --- | --- | --- |
| `val`**!** | **Option** | `val!` | TS 非空断言 (仅限 Option) |
| `val`**!** | **Result** | `$unwrap(val)` | **修复**: 调用 Runtime helper，保留错误信息 |
| **?** (Try) | 行末 `func()?` | `const _tmp = func();`<br>`if(_tmp.tag==='err') return _tmp;`<br>`const val = _tmp.val;` | **修复**: 编译器必须进行 AST 展开 (Desugaring)，模拟 Rust 的 `?` 行为 |
| **<** `val` | 行首 | `return val;` | - |
| **<** | 行首 | `return;` | - |
| **br** | 循环内 | `break;` | v1.6 规范 |
| **ct** | 循环内 | `continue;` | v1.6 规范 |

### 3.2 ? 操作符的 AST 展开示例

**Nu Code:**
```rust
F process() -> Result<i32, String> {
    l val = dangerous_op()?;
    < Ok(val * 2)
}
```

**生成的 TypeScript (展开后):**
```typescript
export function process(): Result<number, string> {
    const _tmp0 = dangerous_op();
    if (_tmp0.tag === 'err') return _tmp0;
    const val = _tmp0.val;
    
    return Ok(val * 2);
}
```

### 3.3 模式匹配 (M) 重构策略

针对 TS `switch` 不支持解构的问题，编译器需根据匹配复杂度选择生成策略。

| Nu v1.6 源码 | 匹配类型 | TypeScript 生成代码 |
| --- | --- | --- |
| **M** `x { 1 => A, 2 => B }` | 字面量 | `switch(x) { case 1: A; break; ... }` |
| **M** `res { Ok(v) => ..., Err(e) => ... }` | **枚举解构** | **AST 重写为 If-Chain**: <br>`if(res.tag==='ok') { const v=res.val; ... }`<br>`else if(res.tag==='err') { const e=res.err; ... }` |
| **M** `opt { Some(x) => ... }` | Option | `if(opt !== null) { const x = opt; ... }` |

**完整示例:**

```rust
// Nu
M compute(x) {
    Ok(val): {
        println!("Success: {}", val);
    },
    Err(e): {
        println!("Error: {}", e);
    }
}
```

```typescript
// Generated TS
const _match0 = compute(x);
if (_match0.tag === 'ok') {
    const val = _match0.val;
    console.log($fmt("Success: {}", val));
} else if (_match0.tag === 'err') {
    const e = _match0.err;
    console.log($fmt("Error: {}", e));
}
```

---

## 4. 链式调用剥离 (Chain Peeling)

| Nu 方法 | 处理策略 | TypeScript 结果 | 备注 |
| --- | --- | --- | --- |
| `.iter()` | **剔除** | *(ignored)* | - |
| `.into_iter()` | **剔除** | *(ignored)* | - |
| `.collect::<T>()` | **剔除** | *(ignored)* | Turbofish 直接丢弃 |
| `.map(f)` | 保留 | `.map(f)` | - |
| `.filter(f)` | 保留 | `.filter(f)` | - |
| `.enumerate()` | **参数重写** | `.map((val, idx) => ...)` | 需交换闭包参数顺序 `(i,x)` -> `(x,i)` |
| `.clone()` | **替换** | `structuredClone(x)` | 或根据配置降级为浅拷贝 `{...x}` |
| `.len()` | **属性化** | `.length` | 去掉 `()` |
| `.unwrap()` | **条件替换** | Option: `!`, Result: `$unwrap(...)` | 根据类型上下文 |
| `.unwrap_or(v)` | **替换** | `?? v` | Nullish coalescing |

**示例:**

```rust
// Nu
l nums = vec![1, 2, 3];
l doubled = nums.iter()
    .filter(|x| x > 1)
    .map(|x| x * 2)
    .collect::<Vec<i32>>();
```

```typescript
// Generated TS
const nums = [1, 2, 3];
const doubled = nums
    .filter(x => x > 1)
    .map(x => x * 2);
// .iter() and .collect() removed
```

---

## 5. 宏与元编程 (Macros)

| Nu v1.6 宏 | TypeScript 目标 | 转换细节 |
| --- | --- | --- |
| `println!("Msg")` | `console.log("Msg")` | - |
| `println!("{}", x)` | `console.log(x)` | 优化：单参数直接打印 |
| `println!("A:{}", x)` | `console.log($fmt("A:{}", x))` | **修复**: 调用 Runtime `$fmt` |
| `format!("A:{}", x)` | `$fmt("A:{}", x)` | **修复**: 保持格式化语义 |
| `vec![1, 2]` | `[1, 2]` | - |
| `panic!("E")` | `throw new Error("E")` | - |
| `assert!(x)` | `if(!x) throw new Error("Assert failed")` | - |
| `assert_eq!(a, b)` | `if(a !== b) throw new Error(\`Assert failed: \${a} !== \${b}\`)` | - |
| `todo!()` | `throw new Error("TODO: Not implemented")` | - |
| `unimplemented!()` | `throw new Error("Unimplemented")` | - |

---

## 6. 定义与模块 (Definitions)

| Nu v1.6 关键词 | 示例 | TypeScript 目标 |
| --- | --- | --- |
| **F** (Pub Fn) | `F run()` | `export function run()` |
| **f** (Priv Fn) | `f tool()` | `function tool()` |
| **S** (Struct) | `S User` | `export class User` (首字母大写) |
| **S** (Struct) | `S conf` | `class conf` (首字母小写) |
| **D** (Mod) | `D api` | `export namespace api` 或文件导入 |
| **E** (Enum) | `E State` | `export type State = ...` (Union Type) |
| **TR** (Trait) | `TR Fly` | `export interface Fly` |
| **I** (Impl) | `I User` | 合并到 class 定义中 |
| **u** (Use) | `u ./util` | `import * as util from "./util"` |
| **t** (Type) | `t UserId = i32` | `type UserId = number` |
| **C** (Const) | `C MAX = 100` | `const MAX = 100` |
| **ST** (Static) | `ST mut COUNTER = 0` | `let COUNTER = 0` |

---

## 7. 异步与并发 (Async/Concurrency)

| Nu v1.6 符号 | TypeScript 映射 | 备注 |
| --- | --- | --- |
| **~** `F run()` | `export async function run()` | Async 定义 |
| **~** `{ }` | `async () => { }` | Async block |
| `fut`**. ~** | `await fut` | Await 后缀操作符 |
| **@** `block` | `(async () => block)()` | Spawn 映射为立即执行的异步函数 |
| **@@** `block` | **Error** | TS 不支持 OS 线程，编译器应报错或退化为异步 |
| **<<** (Channel) | **Error/Polyfill** | 需要额外的 Channel 运行时或报错 |

**Async 示例:**

```rust
// Nu
~F fetchData(url: String) -> Result<String, String> {
    l response = http::get(url).~?;
    < Ok(response.text().~?)
}
```

```typescript
// Generated TS
export async function fetchData(url: string): Promise<Result<string, string>> {
    const _tmp0 = await http.get(url);
    if (_tmp0.tag === 'err') return _tmp0;
    const response = _tmp0.val;
    
    const _tmp1 = await response.text();
    if (_tmp1.tag === 'err') return _tmp1;
    
    return Ok(_tmp1.val);
}
```

---

## 8. 标准库映射表 (Std Lib Allowlist)

编译器应内置一份白名单，仅允许部分 `std` 调用，其余报错。

| Nu Std | TypeScript 对应 | 兼容性 |
| --- | --- | --- |
| `std::collections::HashMap` | `Map` | ✅ |
| `std::collections::HashSet` | `Set` | ✅ |
| `std::cmp::min/max` | `Math.min/max` | ✅ |
| `std::option::Option` | `T \| null` | ✅ |
| `std::result::Result` | `Result<T,E>` (runtime) | ✅ |
| `std::string::String` | `string` | ✅ |
| `std::vec::Vec` | `Array` | ✅ |
| `std::thread` | *(Unsupported)* | ❌ 编译报错 |
| `std::sync::Mutex` | *(Warning)* | ⚠️ 锁语义被擦除 |
| `std::fs` | `fs` (Node) / *(Error)* (Web) | ⚠️ 需指定 `--target` |
| `std::io` | `console/fs` | ⚠️ 平台相关 |

---

## 9. 编译器配置选项

| 选项 | 默认值 | 说明 |
| --- | --- | --- |
| `--runtime` | `inline` | `inline`: 注入到每个文件, `import`: 外部模块引用 |
| `--target` | `node` | `node`, `browser`, `deno` |
| `--strict` | `true` | 严格模式：不允许的std库调用会报错 |
| `--no-format` | `false` | 禁用 `$fmt`，所有 format! 替换为字符串拼接 |
| `--source-map` | `false` | 生成 .map 文件 |

---

## 10. 完整转换示例

### 输入 (Nu v1.6)

```rust
// calculator.nu
u std::collections::HashMap

S Calculator {
    cache: HashMap<String, i32>
}

I Calculator {
    F new() -> Self {
        Calculator { cache: HashMap::new() }
    }
    
    F compute(x: i32) -> Result<i32, String> {
        ? x < 0 {
            < Err("Negative input".to_string())
        }
        
        l cached = self.cache.get(&x.to_string());
        M cached {
            Some(val): { < Ok(*val) },
            None: {
                l result = x * 2;
                self.cache.insert(x.to_string(), result);
                < Ok(result)
            }
        }
    }
}

f main() {
    l mut calc = Calculator::new();
    
    M calc.compute(10) {
        Ok(val): {
            println!("Result: {}", val);
        },
        Err(e): {
            println!("Error: {}", e);
        }
    }
}
```

### 输出 (TypeScript)

```typescript
// ==================== Nu2TS Micro-Runtime ====================
export type Result<T, E> = { tag: 'ok'; val: T } | { tag: 'err'; err: E };
export const Ok = <T>(val: T): Result<T, any> => ({ tag: 'ok', val });
export const Err = <E>(err: E): Result<any, E> => ({ tag: 'err', err });
export function $unwrap<T, E>(r: Result<T, E>): T {
  if (r.tag === 'err') throw new Error(`${r.err}`);
  return r.val;
}
export function $fmt(s: string, ...a: any[]): string {
  return s.replace(/{}/g, () => String(a.shift()));
}
// ============================================================

export class Calculator {
    cache: Map<string, number>;
    
    constructor() {
        this.cache = new Map();
    }
    
    static new(): Calculator {
        return new Calculator();
    }
    
    compute(x: number): Result<number, string> {
        if (x < 0) {
            return Err("Negative input");
        }
        
        const cached = this.cache.get(x.toString());
        if (cached !== null && cached !== undefined) {
            const val = cached;
            return Ok(val);
        } else {
            const result = x * 2;
            this.cache.set(x.toString(), result);
            return Ok(result);
        }
    }
}

function main(): void {
    let calc = Calculator.new();
    
    const _match0 = calc.compute(10);
    if (_match0.tag === 'ok') {
        const val = _match0.val;
        console.log($fmt("Result: {}", val));
    } else if (_match0.tag === 'err') {
        const e = _match0.err;
        console.log($fmt("Error: {}", e));
    }
}

main();
```

---

## 总结

这份修正版映射清单通过引入**微运行时（约40行代码）**，在保持编译产物轻量的同时，解决了以下关键问题：

1. ✅ **Result 类型安全**: 通过 discriminated union 和 helper 函数
2. ✅ **? 操作符语义**: 通过 AST 展开实现早返回
3. ✅ **格式化字符串**: 通过 $fmt 运行时函数
4. ✅ **模式匹配**: 根据复杂度选择 switch 或 if-chain
5. ✅ **String 命名**: 恢复使用 `String`，保持与 Rust 一致性

这是一个在**理想主义**（零运行时）与**工程现实**（正确性、可维护性）之间取得平衡的务实方案。
