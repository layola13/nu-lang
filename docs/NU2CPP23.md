这是基于 **Nu v1.6.5** 标准到 **C++23 (GCC 14+)** 的完整特性映射表。

此表假设后端已引入 `<print>`, `<expected>`, `<variant>`, `<format>`, `<memory>` 等 C++23 标准库。

### 1. 基础类型与内存 (Primitives & Memory)

| Nu v1.6.5 类型 | C++23 映射目标 | 语义/备注 |
| --- | --- | --- |
| **i8, i16, i32, i64** | `int8_t`, `int16_t`, `int32_t`, `int64_t` | 标准定长整数 |
| **u8, u16, u32, u64** | `uint8_t`, `uint16_t`, `uint32_t`, `uint64_t` | 无符号整数 |
| **f32, f64** | `float`, `double` | 浮点数 |
| **bool** | `bool` | 布尔值 |
| **char** | `char32_t` | Rust char 为 Unicode Scalar (4字节) |
| **()** (Unit) | `void` (返回值) / `std::monostate` (泛型中) | 根据上下文自动切换 |
| **String** | `std::string` | 拥有所有权 |
| **&str** | `std::string_view` | 零拷贝字符串切片 |
| **V<T>** (Vec) | `std::vector<T>` | 动态数组 |
| **O<T>** (Option) | `std::optional<T>` | 可空类型 |
| **R<T, E>** (Result) | **`std::expected<T, E>`** | **(C++23 核心升级)** 替代旧的 Variant 方案 |
| **B<T>** (Box) | `std::unique_ptr<T>` | 独占智能指针 |
| **A<T>** (Arc) | `std::shared_ptr<T>` | 共享智能指针 (C++ SharedPtr 自带原子计数) |
| **X<T>** (Mutex) | `std::mutex` (需封装) | 需封装为 `nu::Mutex<T>` 以模拟 LockGuard |

---

### 2. 变量定义与可见性 (Variables & Visibility)

| Nu v1.6.5 语法 | C++23 映射 | 备注 |
| --- | --- | --- |
| `l x = val` | `const auto x = val;` | **默认 Const** |
| `v x = val` | `auto x = val;` | **Mutable** |
| `l x: T = val` | `const T x = val;` | 显式类型 |
| `SM Global` | `static inline auto Global` | Static Mut (C++ 全局变量默认可变) |
| `ST Global` | `static inline const auto Global` | Static Const |
| `F func` (Pub) | `public:` (类内) / 全局导出 | 公有函数 |
| `f func` (Priv) | `private:` (类内) / `static` (文件内) | 私有函数 |
| `S User` (Pub Type) | `struct User` | C++ struct 默认 public |
| `S user` (Priv Type) | `struct user` | 语义上作为内部类型处理 |

---

### 3. 结构体与枚举 (Aggregates)

| Nu v1.6.5 语法 | C++23 映射 | 实现策略 |
| --- | --- | --- |
| `S Point { x, y }` | `struct Point { auto x; auto y; };` | 成员变量 |
| `init = Point { x: 1, y: 2 }` | `auto init = Point { .x = 1, .y = 2 };` | **Designated Initializers** (C++20/23) |
| `E Shape { Circle(f32) }` | `using Shape = std::variant<Circle>;` | 需先生成 `struct Circle { float _0; };` |
| `shape.0` (Tuple Access) | `shape._0` | 生成的 Struct 成员命名约定 |

---

### 4. 控制流与模式匹配 (Control Flow)

| Nu v1.6.5 语法 | C++23 映射 | 备注 |
| --- | --- | --- |
| `< val` | `return val;` | 返回语句 |
| `if cond { }` | `if (cond) { }` | 标准 if |
| `L { }` (Loop) | `while (true) { }` | 无限循环 |
| `L i: list` | `for (const auto& i : list)` | **Range-based for** |
| `M val { ... }` | `if (auto* p = std::get_if<T>(&val)) ...` | **If-Chain 策略** (支持 Guard 和 Fallthrough) |
| `val!` (Try Operator) | `NU_TRY(val)` (宏) | 展开为检查 `std::expected::has_value()` |
| `br`, `ct` | `break;`, `continue;` | 标准跳转 |

---

### 5. 函数与闭包 (Functions & Closures)

| Nu v1.6.5 语法 | C++23 映射 | 备注 |
| --- | --- | --- |
| `arg: &T` | `const T& arg` | 常量引用 |
| `arg: &!T` | `T& arg` | 可变引用 |
| `arg: T` | `T arg` | 值传递 (Move) |
| `|x| x+1` | `[&](auto x) { return x+1; }` | Lambda (默认引用捕获) |
| `$ |x| x+1` (Move) | `[=](auto x) { return x+1; }` | Lambda (值捕获/Move) |
| `-> T` | `-> T` | 尾置返回类型 |

---

### 6. 泛型与特质 (Generics & Traits)

| Nu v1.6.5 语法 | C++23 映射 | 关键特性 |
| --- | --- | --- |
| `F foo<T>` | `template <typename T> auto foo` | 模板函数 |
| `TR Graph` | `template <typename T> concept Graph` | **C++20 Concepts** |
| `wh T: Graph` | `requires Graph<T>` | Requires 子句 |
| `t Item` (Assoc Type) | `using Item = ...` | 关联类型别名 |
| `I Graph for MyS` | `struct MyS { ... }` | 直接在类中实现 Concept 要求的接口 |

---

### 7. 宏与内置功能 (Macros & Built-ins)

| Nu v1.6.5 语法 | C++23 映射 | 备注 |
| --- | --- | --- |
| `println!("{}", x)` | **`std::println("{}", x);`** | **(C++23 核心升级)** 取代 iostream |
| `format!("{}", x)` | **`std::format("{}", x)`** | 零开销格式化字符串 |
| `panic!("msg")` | `throw std::runtime_error("msg");` | 或 `std::terminate()` |
| `vec![1, 2]` | `std::vector{1, 2}` | 推导指南初始化 |
| `assert!(x)` | `assert(x);` | C 标准断言 |
| `TODO!()` | `throw std::logic_error("Not impl");` |  |

---

### 8. 并发 (Concurrency)

| Nu v1.6.5 语法 | C++23 映射 | 备注 |
| --- | --- | --- |
| `@ { ... }` (Spawn) | `std::jthread([=]{ ... });` | **C++20 jthread** (自动 Join) |
| `<<` (Channel Send) | `channel.send(val)` | 需在 nu_core 实现 Channel |
| `X<T>` (Mutex) | `std::mutex` |  |

---

### 9. 属性 (Attributes)

| Nu v1.6.5 语法 | C++23 映射 | 备注 |
| --- | --- | --- |
| `#[cfg(linux)]` | `#ifdef __linux__` | 预处理指令转换 |
| `#I` (Inline) | `inline` 或 `__attribute__((always_inline))` | 内联提示 |
| `#D(...)` | (无直接映射) | 需生成对应的 Operator 重载 (如 `operator==`) |


关于智能指针（Smart Pointers），这是 Nu-lang (Rust 语义) 到 C++ 转换中最顺畅、最“门当户对”的部分。

Rust 的所有权模型很大程度上启发了现代 C++ (`<memory>`) 的设计。在 C++23 中，我们有完美的对应物。

以下是 **Nu v1.6.5** 到 **C++23** 的智能指针详细映射策略。

### 1. 核心映射表 (Mapping Table)

| Nu v1.6.5 类型 | C++23 类型 | 语义对比 | 转换策略 |
| --- | --- | --- | --- |
| **`B<T>`** (Box) | **`std::unique_ptr<T>`** | 独占所有权 | 完美映射。赋值时必须使用 `std::move()`。 |
| **`A<T>`** (Arc) | **`std::shared_ptr<T>`** | 线程安全的共享所有权 | C++ `shared_ptr` 控制块默认是原子操作的 (Atomic)，等同于 Rust `Arc`。 |
| **`Rc<T>`** (Rc) | **`std::shared_ptr<T>`** | 非线程安全的共享所有权 | **降级映射**。C++ 标准库没有非原子的 SharedPtr。用 `std::shared_ptr` 虽有轻微性能损耗（原子操作），但语义正确且安全。 |
| **`W<T>`** (Weak) | **`std::weak_ptr<T>`** | 弱引用 | 完美映射。用于打破循环引用。 |
| **`X<T>`** (Mutex) | **`nu::Mutex<T>`** | 内部可变性 | 需封装。Rust Mutex 保护数据，C++ Mutex 保护代码块。 |

---

### 2. 详细实现与转换规则

#### A. 独占指针 `B<T>` -> `std::unique_ptr<T>`

这是最严格的转换。因为 `std::unique_ptr` 禁止拷贝，你的 Transpiler 必须极其小心地插入 `std::move`。

* **Nu 源码:**
```rust
S Node { val: i32 }

f process(n: B<Node>) { ... }

f main() {
    l b1 = B::new(Node { val: 1 });
    process(b1); // b1 被 Move
}

```


* **C++23 生成代码:**
```cpp
struct Node { int32_t val; };

// 参数必须是值传递 (by value)，表示接管所有权
void process(std::unique_ptr<Node> n) { ... }

auto main() -> int {
    // B::new -> std::make_unique
    auto b1 = std::make_unique<Node>(Node{.val = 1});

    // 关键点：Nu 是隐式 Move，C++ 必须显式 std::move
    process(std::move(b1)); 

    return 0;
}

```



#### B. 共享指针 `A<T>` / `Rc<T>` -> `std::shared_ptr<T>`

Rust 需要显式调用 `.clone()` 来增加引用计数，而 C++ 的拷贝构造函数会自动处理。

* **Nu 源码:**
```rust
l a1 = A::new(10);
l a2 = a1.clone(); // 显式 Clone

```


* **C++23 生成代码:**
```cpp
auto a1 = std::make_shared<int32_t>(10);

// 转换策略：遇到 .clone()，直接生成简单的拷贝赋值
auto a2 = a1; // C++ 自动增加引用计数

```



#### C. 内部可变性 `X<T>` (Mutex) -> `nu::Mutex<T>`

这是最难点。Rust 的 `Mutex<T>` 包含了数据 `T`，而 C++ 的 `std::mutex` 只是把锁。
为了模拟 `lock().unwrap()` 返回一个能访问数据的 Guard，我们需要在 `nu_core.hpp` 中手写一个垫片类。

**运行时垫片 (`nu_core.hpp`):**

```cpp
namespace nu {
    template <typename T>
    class Mutex {
        mutable std::mutex _mtx;
        T _data;

    public:
        // 构造函数
        template<typename... Args>
        Mutex(Args&&... args) : _data(std::forward<Args>(args)...) {}

        // RAII Guard 类 (模拟 Rust 的 MutexGuard)
        class LockGuard {
            std::unique_lock<std::mutex> _lock;
            T& _ref;
        public:
            LockGuard(std::mutex& m, T& d) : _lock(m), _ref(d) {}
            
            // 重载 -> 和 * 以便像指针一样访问数据
            T* operator->() { return &_ref; }
            T& operator*() { return _ref; }
        };

        // lock() 方法
        auto lock() -> std::expected<LockGuard, std::string> {
            // 简单起见，这里总是成功。Rust 的 PoisonError 在 C++ 很难模拟
            return LockGuard(_mtx, _data);
        }
    };
}

```

* **Nu 源码:**
```rust
l m = X::new(5);
{
    v guard = m.lock()!; // ! 是 unwrap
    *guard = 10;
} // guard 自动 Drop，解锁

```


* **C++23 生成代码:**
```cpp
// X::new -> nu::Mutex 构造
auto m = nu::Mutex<int32_t>(5);
{
    // lock()! -> NU_TRY(m.lock())
    // 获取到的是 nu::Mutex::LockGuard
    auto guard = NU_TRY(m.lock());

    // 操作 Guard 就像操作数据一样
    *guard = 10;
} // C++ RAII 自动析构 guard，释放 std::mutex

```



### 3. 内存布局可视化

理解这两个指针在内存中的区别对于编写 Transpiler 很有帮助：

* **`B<T>` (`unique_ptr`)**: 非常轻量，只是一个指针的大小（64位系统上是8字节），直接指向堆内存。**零开销**。
* **`A<T>` (`shared_ptr`)**: 比较重，包含两个指针（一个指向数据，一个指向控制块 Control Block）。控制块里维护着 Reference Count (Strong & Weak)。这也是为什么 Nu-lang 推荐默认用 `B<T>`。

### 4. Transpiler (nu2cpp) 开发注意事项

1. **`std::make_unique` 是必须的**: 不要生成 `std::unique_ptr<T>(new T(...))`，这非异常安全。必须生成 `std::make_unique<T>(...)`。
2. **Weak Ptr 升级**:
* Nu: `weak.upgrade()` 返回 `Option<Arc<T>>`。
* C++: `weak.lock()` 返回 `std::shared_ptr<T>` (如果失败则为空)。
* **映射**: `weak.lock()` 的结果可以直接转换为 `std::optional` 语义（通过判断指针是否为 null）。


3. **循环引用**: C++ 的 `shared_ptr` 和 Rust 的 `Arc` 一样，都无法处理循环引用（会导致内存泄漏）。这属于“行为一致”，Transpiler 不需要额外处理，责任在写代码的人（需使用 Weak）。

### 总结

C++23 的智能指针系统非常成熟。

* **Box** -> **unique_ptr** (必须加 `std::move`)
* **Arc/Rc** -> **shared_ptr** (直接拷贝)
* **Mutex** -> **自定义 Wrapper** (核心在于把 mutex 和 data 绑在一起)

只要搞定了 `nu::Mutex` 这个 Wrapper，其他的基本上是文本替换工作。

处理 `#[cfg(...)]`（条件编译）是实现跨平台 `nu2cpp` 的关键。

在 Rust/Nu 中，`cfg` 决定了代码**是否存在**。在 C++ 中，唯一能达到同样效果（让代码完全消失，不参与编译）的工具是 **C 预处理器 (Preprocessor)**，即 `#ifdef`, `#if defined(...)` 等指令。

以下是详细的 **Nu `cfg` 到 C++ 预处理器** 的映射策略。

---

### 1. 基础谓词映射表 (Predicate Mapping)

你需要编写一个转换器，将 Nu 的 `cfg` 键值对翻译成标准的 C++ 宏或编译器预定义宏。

| Nu `cfg` 写法 | C++ 预处理器写法 | 备注 |
| --- | --- | --- |
| **OS 检查** |  |  |
| `target_os = "linux"` | `__linux__` | GCC/Clang 默认定义 |
| `target_os = "windows"` | `_WIN32` | MSVC/MinGW 默认定义 |
| `target_os = "macos"` | `__APPLE__` |  |
| `target_os = "android"` | `__ANDROID__` |  |
| **架构检查** |  |  |
| `target_arch = "x86_64"` | `__x86_64__` 或 `_M_X64` |  |
| `target_arch = "aarch64"` | `__aarch64__` | ARM64 |
| **编译模式** |  |  |
| `debug_assertions` | `!defined(NDEBUG)` | C++ 标准做法：Release 模式定义 `NDEBUG` |
| `not(debug_assertions)` | `defined(NDEBUG)` | 即 Release 模式 |
| **Feature Flags** |  |  |
| `feature = "extra"` | `NU_FEATURE_EXTRA` | **需在编译命令中通过 `-D` 传入** |

---

### 2. 逻辑组合映射 (Logic Combination)

Nu 的 `cfg` 支持嵌套逻辑，需要递归翻译成 C++ 的逻辑运算符。

| Nu 逻辑 | C++ 逻辑 | 转换示例 |
| --- | --- | --- |
| `all(A, B)` | `(A && B)` | `cfg(all(unix, not(macos)))`<br>

<br>⬇️<br>

<br>`#if defined(__unix__) && !defined(__APPLE__)` |
| `any(A, B)` | `(A || B)` | `cfg(any(linux, windows))`<br>

<br>⬇️<br>

<br>`#if defined(**linux**) |
| `not(A)` | `!A` | `cfg(not(windows))`<br>

<br>⬇️<br>

<br>`#if !defined(_WIN32)` |

---

### 3. 实战场景转换 (Code Generation Examples)

`nu2cpp` 需要根据 `cfg` 出现的位置，精准插入 `#if` 和 `#endif`。

#### A. 整个函数/结构体 (Item Level)

这是最简单的，包住整个定义。

**Nu 源码:**

```rust
#[cfg(target_os = "windows")]
F open_window() {
    // Windows logic
}

#[cfg(not(target_os = "windows"))]
F open_window() {
    // Unix logic
}

```

**C++ 生成:**

```cpp
#if defined(_WIN32)
void open_window() {
    // Windows logic code...
}
#endif

#if !defined(_WIN32)
void open_window() {
    // Unix logic code...
}
#endif

```

#### B. 结构体字段 (Struct Fields)

这是 C++ 支持但很丑陋的地方，但必须这样做以保证内存布局一致。

**Nu 源码:**

```rust
S User {
    id: i32,
    #[cfg(feature = "pro")]
    license_key: String,
}

```

**C++ 生成:**

```cpp
struct User {
    int32_t id;

#if defined(NU_FEATURE_PRO)
    std::string license_key;
#endif
};

```

#### C. 语句块/表达式 (Statements)

Rust 允许在 Block 内写 `cfg`。C++ 预处理指令可以出现在任何地方。

**Nu 源码:**

```rust
f init() {
    setup_core();
    #[cfg(debug_assertions)]
    setup_logger(); // 仅 Debug 模式运行
}

```

**C++ 生成:**

```cpp
void init() {
    setup_core();
#if !defined(NDEBUG)
    setup_logger();
#endif
}

```

#### D. `cfg!` 宏 (Expression Level)

Rust 的 `if cfg!(...)` 是编译期常量检查。
在 C++17/20 中，最佳实践是使用 `if constexpr`，但这要求被检查的代码在所有平台都**语法有效**。如果代码调用了不存在的 API，必须回退到 `#ifdef`。

**策略：** 为了安全，`nu2cpp` 统一将 `cfg!` 宏转换为 `#ifdef` 块模拟。

**Nu 源码:**

```rust
if cfg!(target_os = "linux") {
    println!("Linux");
} else {
    println!("Other");
}

```

**C++ 生成:**

```cpp
#if defined(__linux__)
    nu::println("Linux");
#else
    nu::println("Other");
#endif

```

---

### 4. `Cargo.toml` Features 的传递

在 Rust 中，`Cargo.toml` 定义了 features。
在 C++ 中，这些必须通过编译器参数 `-D` 传递进去。

假设 `Cargo.toml` (Nu 项目):

```toml
[features]
gpu_support = []

```

当你用 `nu2cpp` 编译时，需要生成对应的 CMake 或 Makefile：

**CMakelists.txt (生成示例):**

```cmake
option(NU_FEATURE_GPU_SUPPORT "Enable GPU support" OFF)

if (NU_FEATURE_GPU_SUPPORT)
    add_compile_definitions(NU_FEATURE_GPU_SUPPORT)
endif()

```

这样，当用户在 C++ 端开启选项时，代码里的 `#if defined(NU_FEATURE_GPU_SUPPORT)` 就会生效。

### 5. `CodegenCpp` 实现伪代码

你的 Transpiler 在遍历 AST 时，如果发现节点上有 `Attribute` 且名为 `cfg`，需要做如下拦截：

```rust
// Rust (Transpiler Logic)

fn visit_function(&mut self, func: &Function) {
    // 1. 检查是否有 cfg 属性
    if let Some(cfg) = func.attributes.get("cfg") {
        let cpp_condition = parse_cfg_to_cpp(cfg); // 转换逻辑如上表
        self.emit(format!("#if {}\n", cpp_condition));
    }

    // 2. 生成函数本体
    self.emit_function_signature(func);
    self.emit_block(&func.body);

    // 3. 闭合 endif
    if func.attributes.has("cfg") {
        self.emit("#endif\n");
    }
}

```

### 总结

处理 `cfg` 不需要高深的 C++ 模板技巧，反而是最古老的 **C Preprocessor** 最管用。

* **映射核心**：将 Rust 的 Key-Value 映射为 GCC/MSVC 的预定义宏。
* **主要工作**：编写一个小的递归解析器，把 `all(a, not(b))` 变成 `A && !B`。
* **构建系统**：别忘了生成的构建脚本（CMake）需要配合处理 Features (`-D` 标志)。

这正是 C++20/23 最“大显神通”的地方。

在旧版 C++ 中，实现 `Eq`, `Ord`, `Hash` 需要写一大堆样板代码。但在 **C++20/23** 中，编译器为你做了绝大部分工作，这使得 `#[derive(...)]` 到 C++ 的映射变得异常简单和优雅。

以下是 **Nu v1.6.5** `derive` 属性到 **C++23** 的完整映射策略。

### 1. 核心 Derive 映射表

| Nu Attribute | C++23 映射实现 | 复杂度 | 备注 |
| --- | --- | --- | --- |
| **`#D(Clone)`** | `T(const T&) = default;`<br>

<br>`T& operator=(const T&) = default;` | ⭐ | 生成默认拷贝构造函数 |
| **`#D(Copy)`** | (同上) | ⭐ | C++ 不区分 Clone/Copy，都是拷贝构造 |
| **`#D(Debug)`** | **特化 `std::formatter<T>**` | ⭐⭐⭐ | 为了支持 `std::println("{:?}", x)` |
| **`#D(Default)`** | `T() = default;` | ⭐ | 生成默认构造函数 |
| **`#D(PartialEq)`** | **`bool operator==(const T&) const = default;`** | ⭐ | **C++20 神器**：自动生成成员逐个比较 |
| **`#D(Eq)`** | (同上) | ⭐ | C++ 不需要额外标记 Eq |
| **`#D(PartialOrd)`** | **`auto operator<=>(const T&) const = default;`** | ⭐ | **C++20 太空船操作符**：自动生成 `<, <=, >, >=` |
| **`#D(Ord)`** | (同上) | ⭐ |  |
| **`#D(Hash)`** | **特化 `std::hash<T>**` | ⭐⭐⭐ | C++ 没有默认 Hash 实现，需手写代码生成 |

---

### 2. 详细转换代码示例

#### A. 比较与排序 (Eq, Ord) —— C++20 的胜利

这是 Nu2Cpp 最爽的部分。Rust 的 `PartialEq` 和 `PartialOrd` 几乎是 1:1 对应 C++20 的默认操作符。

**Nu 源码:**

```rust
#D(PartialEq, Eq, PartialOrd, Ord)
S Version {
    major: i32,
    minor: i32
}

```

**C++23 生成代码:**

```cpp
struct Version {
    int32_t major;
    int32_t minor;

    // #[derive(PartialEq, Eq)]
    // 自动生成 == 和 !=
    bool operator==(const Version&) const = default;

    // #[derive(PartialOrd, Ord)]
    // 自动生成 <, <=, >, >= (利用 C++20 三路比较)
    auto operator<=>(const Version&) const = default;
};

```

#### B. 复制语义 (Clone/Copy)

Rust 默认是 Move，C++ 默认是 Copy。为了模拟 Nu/Rust 语义，如果**没有** `derive(Clone)`，我们应该**删除** C++ 的拷贝构造函数。

**Nu 源码:**

```rust
// 情况 1: 没有 derive(Clone) -> Move Only
S UniqueToken { id: i32 }

// 情况 2: 有 Clone
#D(Clone)
S Config { mode: i32 }

```

**C++23 生成代码:**

```cpp
struct UniqueToken {
    int32_t id;
    
    // 模拟 Rust: 没有 Clone trait，禁止隐式拷贝
    UniqueToken(const UniqueToken&) = delete;
    UniqueToken& operator=(const UniqueToken&) = delete;
    
    // 允许 Move
    UniqueToken(UniqueToken&&) = default;
    UniqueToken& operator=(UniqueToken&&) = default;
};

struct Config {
    int32_t mode;
    
    // #[derive(Clone)] -> 允许拷贝
    Config(const Config&) = default;
    Config& operator=(const Config&) = default;
};

```

#### C. 调试打印 (Debug)

为了支持 C++23 的 `std::println("{:?}", obj)`，你需要为该类型生成 `std::formatter` 特化。这比 Rust 的 `derive(Debug)` 生成的代码要多一些，但逻辑很固定。

**Nu 源码:**

```rust
#D(Debug)
S Point { x: i32, y: i32 }

```

**C++23 生成代码:**

```cpp
struct Point { int32_t x; int32_t y; };

// 放在全局命名空间或 std 特化中
template <>
struct std::formatter<Point> {
    // 解析格式字符串 (默认直接通过)
    constexpr auto parse(std::format_parse_context& ctx) {
        return ctx.begin();
    }

    // 执行格式化
    auto format(const Point& p, std::format_context& ctx) const {
        // 类似 Rust: Point { x: 10, y: 20 }
        return std::format_to(ctx.out(), "Point {{ x: {}, y: {} }}", p.x, p.y);
    }
};

```

#### D. 哈希 (Hash)

这是最麻烦的一个。C++ 标准库没有提供“自动组合成员 Hash”的功能（boost 有，但我们不想引入 boost）。你需要生成一个简单的 Hash 组合函数。

**Nu 源码:**

```rust
#D(Hash)
S User { id: i32, active: bool }

```

**C++23 生成代码:**

```cpp
struct User { int32_t id; bool active; };

// 辅助函数: Hash Combine (通常放在 nu_core.hpp)
namespace nu {
    inline void hash_combine(std::size_t& seed, std::size_t value) {
        seed ^= value + 0x9e3779b9 + (seed << 6) + (seed >> 2);
    }
}

// std::hash 特化
template <>
struct std::hash<User> {
    std::size_t operator()(const User& u) const {
        std::size_t seed = 0;
        nu::hash_combine(seed, std::hash<int32_t>{}(u.id));
        nu::hash_combine(seed, std::hash<bool>{}(u.active));
        return seed;
    }
};

```

---

### 3. 其他常见 Attribute 映射

除了 `derive`，还有一些常见属性的映射。

| Nu Attribute | C++ 映射 | 备注 |
| --- | --- | --- |
| **`#[inline]`** | `inline` | 建议编译器内联 |
| **`#[inline(always)]`** | `__attribute__((always_inline))` (GCC/Clang)<br>

<br>`__forceinline` (MSVC) | 强制内联 |
| **`#[repr(C)]`** | (通常不需要) | 如果生成的 struct 是标准布局，C++ 默认兼容 C |
| **`#[repr(packed)]`** | `__attribute__((packed))` | 紧凑对齐 |
| **`#[allow(...)]`** | (忽略) | C++ 编译器警告不同，直接忽略即可 |
| **`#[test]`** | (不生成代码 或 生成 GoogleTest 宏) | 通常 `nu2cpp` 会忽略测试代码，除非指定生成测试目标 |

### 总结

对于你的 `CodegenCpp` 模块：

1. **最优先支持**：`PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Clone`。因为在 C++20 里它们只需要一行代码 (`= default`)，性价比极高。
2. **次优先**：`Debug`。因为这是开发中最常用的，虽然生成 `formatter` 代码稍微繁琐一点（需要遍历 AST 字段）。
3. **最后支持**：`Hash`。除非你的 Nu 代码里用了 `HashMap<Key, Val>` 且 Key 是自定义结构体，否则可以先不实现。

要不要看看我怎么写 `CodegenCpp` 里处理 `derive(Debug)` 的 Rust 伪代码？那个字符串拼接逻辑有点小技巧。

这确实是拼图中最后、也是最重要的一块。

如果不处理 `Nu.toml`，你生成的几百个 `.hpp/.cpp` 文件就是一盘散沙，用户根本不知道怎么把它们编译成一个可执行文件。

在 Rust/Nu 中，`Cargo` 兼任了**包管理**和**构建系统**。
在 C++ 中，最标准的做法是将 `Nu.toml` 翻译成 **`CMakeLists.txt`**。

这是 **Nu v1.6.5 `Nu.toml**` 到 **Modern CMake (3.20+)** 的完整映射方案。

---

### 1. 核心映射逻辑：Workspace = `add_subdirectory`

Nu 的 Workspace (工作区) 允许在一个大仓库里管理多个包 (`crates`)。CMake 的机制几乎一模一样。

#### 目录结构对比

**Nu 项目结构:**

```text
/my_project
  ├── Nu.toml       (定义 [W]orkspace)
  ├── crates/
  │     ├── app     (Nu.toml: [P]ackage)
  │     └── logic   (Nu.toml: [P]ackage)

```

**生成的 C++ 项目结构:**

```text
/my_project_cpp
  ├── CMakeLists.txt  (Root: 处理 Workspace)
  ├── crates/
  │     ├── app
  │     │     └── CMakeLists.txt (App: 处理可执行文件)
  │     └── logic
  │           └── CMakeLists.txt (Logic: 处理库)

```

---

### 2. 根级 `Nu.toml` (Workspace) 映射

假设根目录的 `Nu.toml` 如下：

```toml
# Nu v1.6.5
[W]
m = ["crates/logic", "crates/app"]
r = "2"

```

**生成的根 `CMakeLists.txt`:**

```cmake
cmake_minimum_required(VERSION 3.20)

# 1. 项目名称 (通常取目录名，或者 Nu.toml 里定义的顶层名称)
project(MyWorkspace LANGUAGES CXX)

# 2. 强制 C++23 标准 (关键！为了支持 std::expected 和 <print>)
set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF) # 确保使用 ISO C++，而非 GCC 扩展

# 3. 包含 nu_core.hpp 的路径
# 假设你把 nu_core.hpp 放在了构建目录或源码根目录
include_directories("${CMAKE_SOURCE_DIR}/include") 

# 4. [W].m -> add_subdirectory
# Nu 的 workspace members 映射为 CMake 的子目录添加
add_subdirectory(crates/logic)
add_subdirectory(crates/app)

```

---

### 3. 包级 `Nu.toml` (Package) 映射

这里需要区分是 **Library ([L])** 还是 **Binary ([[B]])**。

#### 情况 A: 库包 (Library) - `crates/logic/Nu.toml`

```toml
[P]
id = "logic"
v = "0.1.0"

[D]
serde = { v = "1.0" } # 依赖处理见下文

```

**生成的 `crates/logic/CMakeLists.txt`:**

```cmake
# [P].id -> project name
project(logic VERSION 0.1.0 LANGUAGES CXX)

# 收集源文件 (简单起见，使用 glob，生产环境建议显式列出)
file(GLOB_RECURSE SOURCES "src/*.cpp")

# [L] (默认) -> add_library
add_library(logic STATIC ${SOURCES})

# 设置头文件搜索路径 (让别人能引用我)
target_include_directories(logic PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)

# [D] 依赖映射 (难点)
# 如果依赖的是 C++ 库，使用 find_package
find_package(nlohmann_json QUIET) 
if(nlohmann_json_FOUND)
    target_link_libraries(logic PRIVATE nlohmann_json::nlohmann_json)
endif()

```

#### 情况 B: 应用包 (Binary) - `crates/app/Nu.toml`

```toml
[P]
id = "app"
v = "0.1.0"

[D]
logic = { path = "../logic" } # 依赖内部库

```

**生成的 `crates/app/CMakeLists.txt`:**

```cmake
project(app VERSION 0.1.0 LANGUAGES CXX)

file(GLOB_RECURSE SOURCES "src/*.cpp")

# [[B]] -> add_executable
add_executable(app ${SOURCES})

# [D] logic -> target_link_libraries
# CMake 会自动处理路径依赖，只要名称对得上
target_link_libraries(app PRIVATE logic)

```

---

### 4. 依赖管理 ([D]ependencies) 的映射策略

这是 `nu2cpp` 最棘手的地方。Cargo 自动下载依赖，CMake 默认不会。

对于 MVP，我建议采取 **"Best Effort"** 策略：

1. **Workspace 内部依赖 (`path = "..."`)**:
* **完美映射**。直接转化为 `target_link_libraries(target PRIVATE dependency_name)`。CMake 只要加载了那个子目录，就能找到这个 Target。


2. **外部 Crates.io 依赖 (`v = "1.0"`)**:
* 你无法自动把 Rust crate 变成 C++ 库。
* **策略 1 (白名单映射)**: 如果你在 `Nu.toml` 里看到 `serde`，生成的 CMake 里自动 `find_package(nlohmann_json)`。
* **策略 2 (占位符)**: 生成警告注释。
```cmake
# WARNING: Dependency 'reqwest' found in Nu.toml.
# Please add the equivalent C++ library (e.g., libcurl/cpr) manually here.
# target_link_libraries(app PRIVATE ???)

```


* **策略 3 (FetchContent - 进阶)**: 如果你知道对应的 C++ 库的 GitHub 地址，可以用 CMake 的 `FetchContent` 自动下载源码编译。这最像 Cargo。



---

### 5. `Nu.toml` 到 `CMakeLists.txt` 完整映射表

| Nu.toml Section | Key | CMake Command | 备注 |
| --- | --- | --- | --- |
| **`[P]`** (Package) | `id` | `project(id ...)` |  |
|  | `v` | `project(... VERSION v)` |  |
|  | `ed` (edition) | `set(CMAKE_CXX_STANDARD 23)` | 默认全开 23 |
| **`[W]`** (Workspace) | `m` (members) | `foreach(dir IN LISTS m)`<br>

<br>`add_subdirectory(${dir})`<br>

<br>`endforeach()` | 递归添加 |
| **`[L]`** (Lib) | `name` | `add_library(name ...)` | 通常对应 `[P].id` |
|  | `crate-type` | `STATIC` / `SHARED` | `staticlib`->`STATIC`, `cdylib`->`SHARED` |
| **`[[B]]`** (Bin) | `name` | `add_executable(name ...)` |  |
| **`[D]`** (Dependencies) | `foo = {path=..}` | `target_link_libraries(curr PRIVATE foo)` | 内部依赖 |
|  | `foo = "1.0"` | `find_package(foo)` (需预设映射) | 外部依赖 |
| **`[features]`** | `feat = []` | `option(NU_FEAT_FEAT "..." OFF)` | 生成 CMake 开关 |
| **`build`** | `build = "build.nu"` | `add_custom_command(...)` | **极难**，MVP 建议忽略 |

### 6. 你的下一步

在你的 **`cargo2nu` / `nu2cargo**` 工具集中，你已经有了能够解析 `Nu.toml` 的代码。

你需要为 `nu2cpp` 添加一个子命令 `gen-cmake`：

1. 读取根目录 `Nu.toml`。
2. 生成根 `CMakeLists.txt` (设置 C++23, 包含 `nu_core.hpp`, 处理 Workspace)。
3. 递归遍历所有 members。
4. 读取子目录 `Nu.toml`。
5. 生成子 `CMakeLists.txt` (设置 target, 链接依赖)。

**友情提示：**
生成的 CMakeLists.txt 最好包含这一行，这样生成的二进制文件会自动带有调试信息（类似 `cargo build`），除非用户显式指定 Release：

```cmake
if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE Debug)
endif()

```

这就补全了。现在你有了**语法 (C++23)**、**运行时 (nu_core)**、**构建系统 (CMake)**。你可以开始让你的穿山甲跑起来了！