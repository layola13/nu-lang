# Nu Language Specification

[![CI](https://github.com/YOUR_USERNAME/nu/actions/workflows/ci.yml/badge.svg)](https://github.com/YOUR_USERNAME/nu/actions/workflows/ci.yml)
[![Release](https://github.com/YOUR_USERNAME/nu/actions/workflows/release.yml/badge.svg)](https://github.com/YOUR_USERNAME/nu/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Version:** 1.7 (Production Standard)
**Date:** 2025-12-24
**Status:** Frozen / Implementation Ready
**Target:** AI-Native Systems Programming
**License:** MIT

[中文文档 (Chinese Documentation)](./ReadMeCN.md)

---

## Quick Start

### Download Pre-built Binaries

Download the latest release for your platform from the [Releases page](https://github.com/YOUR_USERNAME/nu/releases).

**Available platforms:**
- Linux (x86_64, ARM64)
- macOS (Intel, Apple Silicon)
- Windows (x86_64)

**Installation:**
```bash
# Linux/macOS
tar xzf nu-compiler-*.tar.gz
cd release
sudo cp rust2nu nu2rust cargo2nu nu2cargo /usr/local/bin/

# Or add to your PATH
export PATH=$PATH:/path/to/release
```

**Windows:**
```powershell
# Extract the archive and add the directory to your PATH
```

### Build from Source

```bash
# Clone the repository
git clone https://github.com/YOUR_USERNAME/nu.git
cd nu

# Build all tools
cargo build --release

# Build specific tools
cargo build --release --bin rust2nu
cargo build --release --bin nu2rust
cargo build --release --bin cargo2nu
cargo build --release --bin nu2cargo

# Binaries will be in target/release/
```

### Toolchain

The Nu compiler provides four command-line tools for bidirectional conversion:

#### 1. `rust2nu` - Rust to Nu Converter

Convert standard Rust code to Nu high-density syntax.

**Usage:**
```bash
# Convert single file
./target/release/rust2nu examples/hello.rs -v

# Convert with overwrite
./target/release/rust2nu examples/hello.rs -f -v

# Convert entire directory
./target/release/rust2nu examples/ -o output/ -v

# Recursive conversion
./target/release/rust2nu examples/ -r -v
```

**Options:**
- `INPUT`: Input Rust file or directory
- `-o, --output <OUTPUT>`: Output Nu file or directory (optional)
- `-r, --recursive`: Process directories recursively
- `-f, --force`: Overwrite existing files
- `-v, --verbose`: Verbose output

#### 2. `nu2rust` - Nu to Rust Converter

Convert Nu code back to standard Rust.

**Usage:**
```bash
# Convert Nu file to Rust
./target/release/nu2rust example.nu -o example.rs

# Convert directory
./target/release/nu2rust src_nu/ -o src_rs/ -r
```

#### 3. `cargo2nu` - Cargo Project to Nu Converter

Convert entire Cargo projects to Nu format.

**Usage:**
```bash
# Convert Cargo project
./target/release/cargo2nu /path/to/rust-project -o /path/to/nu-project
```

This tool will:
- Convert all `.rs` files to `.nu` files
- Preserve project structure
- Maintain `Cargo.toml` configuration
- Handle workspace projects

#### 4. `nu2cargo` - Nu Project to Cargo Converter

Convert Nu projects back to standard Cargo projects.

**Usage:**
```bash
# Convert Nu project back to Rust
./target/release/nu2cargo /path/to/nu-project -o /path/to/rust-project
```

### Conversion Example

**Rust Code:**
```rust
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub struct Person {
    pub name: String,
    pub age: u32,
}

impl Person {
    pub fn new(name: String, age: u32) -> Self {
        Person { name, age }
    }
}

fn main() {
    let person = Person::new("Alice".to_string(), 30);
    println!("{}", person.name);
}
```

**Converted to Nu:**
```nu
F add(a: i32, b: i32) -> i32 {
    a + b
}

S Person {
    name: String,
    age: u32,
}

I Person {
    F new(name: String, age: u32) -> Self {
        Person { name, age }
    }
}

f main() {
    l person = Person::new("Alice".to_string(), 30);
    println!("{}", person.name);
}
```

### Compression Statistics

Based on real-world testing, Nu achieves:

- **Token Density**: ~100% improvement
- **Average Compression**: ~55%
- **Lines of Code**: ~40-50% reduction

**Examples:**
- `hello.rs`: 31 lines → `hello.nu`: 28 lines (10% compression)
- `structs.rs`: 100 lines → `structs.nu`: 77 lines (23% compression)
- `closures.rs`: 504 lines → `closures.nu`: ~450 lines (11% compression)

---

## 1. Core Design

**Nu** is a high-density dialect of Rust.

* **Compression Strategy**: Compresses only **definition keywords** (`struct` -> `S`) and **control flow** (`return` -> `<`), while preserving **semantic core** (macros, generics, lifetimes).
* **Compatibility**: 100% compatible with Rust AST.

---

## 2. Lexical & Visibility

### 2.1 Visibility Rules

* **Functions**: Determined by **keyword case**.
  * **`F`** → `pub fn`
  * **`f`** → `fn` (Private)

* **Types**: Determined by **identifier's first letter** (Go-style).
  * `S User` → `pub struct User`
  * `S internal` → `struct internal`

### 2.2 Keyword Mapping Table

| Category | Keyword | Rust Original | Note |
| --- | --- | --- | --- |
| **Definition** | **S** | `struct` |  |
|  | **E** | `enum` |  |
|  | **F** | `pub fn` | **Uppercase=Pub** |
|  | **f** | `fn` | **Lowercase=Priv** |
|  | **TR** | `trait` |  |
|  | **I** | `impl` |  |
|  | **D** | `mod` |  |
|  | **C** | `const` |  |
|  | **ST** | `static` |  |
|  | **EXT** | `extern` |  |
| **Atomic** | **l** | `let` |  |
|  | **v** | `let mut` |  |
|  | **wh** | `where` | **wh** (avoid w conflict) |
|  | **a** | `as` |  |
|  | **u** | `use` |  |
|  | **t** | `type` |  |
|  | **br** | `break` |  |
|  | **ct** | `continue` |  |

---

## 3. Macros & Attributes - **v1.6 Updated**

### 3.1 Macros

**Rule: Keep Rust native syntax completely.**  
No symbolic compression to ensure compatibility and eliminate ambiguity.

| Nu Syntax | Rust Original | Note |
| --- | --- | --- |
| `println!("Msg")` | `println!("Msg")` | **Keep as-is** (no longer use `>`) |
| `vec![1, 2]` | `vec![1, 2]` | Keep as-is |
| `panic!("Err")` | `panic!("Err")` | Keep as-is |
| `format!("{}", x)` | `format!("{}", x)` | Keep as-is |

### 3.2 Attributes

Mixed strategy: compress only high-frequency attributes.

| Nu Syntax | Rust Original | Strategy |
| --- | --- | --- |
| **#D(...)** | `#[derive(...)]` | **Abbreviated** |
| **#I** | `#[inline]` | **Abbreviated** |
| **#[test]** | `#[test]` | **Standard** (tool compat) |
| **#[cfg(...)]** | `#[cfg(...)]` | **Standard** |
| **#[...]** | `#[...]` | Other attributes pass-through |

---

## 4. Type System

### 4.1 String Types

* **String** → `String` (**Owned**, no abbreviation in v1.7)
* **str** → `str` (**Slice**, keep as-is)

### 4.2 Common Abbreviations

| Nu | Rust | Nu | Rust |
| --- | --- | --- | --- |
| **V** | `Vec` | **A** | `Arc` |
| **O** | `Option` | **X** | `Mutex` |
| **R** | `Result` | **B** | `Box` |
| **(A,B)** | Tuple | **W** | `Weak` |

### 4.3 Generics & Turbofish - **v1.6 Updated**

* **Generic Definition**: `S Box<T>` (keep `<T>`)
* **Turbofish**: `iter.collect::<Str>()` (**Mandatory preservation** of `::<T>`, compression forbidden)

---

## 5. Symbols & Flow Control

### 5.1 Core Operators

| Symbol | Meaning | Rust Original | Syntax Rule |
| --- | --- | --- | --- |
| **<** | Return | `return` | Statement start: `< val` |
| **?** | If | `if` | `? x > 0 { }` |
| **M** | Match | `match` | `M val { ... }` |
| **L** | Loop | `loop`/`for` | `L { }`, `L i: list` |
| **!** | Try | `?` | **Suffix only**: `func()!` |

> **Note**: Since macros are restored to original form, **`>` symbol no longer represents Print, only "greater than"**. This completely eliminates ambiguity.

### 5.2 Memory Modifiers

| Symbol | Meaning | Rule |
| --- | --- | --- |
| **!** | **Mut** | **Prefix only**: `&!self` (&mut self), `*!ptr` |
| **U** | **Unsafe** | `U { ... }` |
| **&** | Ref | `&x` |
| ***** | Deref | `*ptr` |

### 5.3 Closures - **v1.6 Updated**

Support return type definitions.

| Syntax | Rust Original |
| --- | --- |
| `|x| x + 1` | `|x| x + 1` |
| `|x| -> i32 { x }` | `|x| -> i32 { x }` |
| `$|x| { x }` | `move |x| { x }` |

### 5.4 Concurrency & Async

| Symbol | Meaning | Equivalent Code |
| --- | --- | --- |
| **@** | Spawn | `tokio::spawn(async move { ... })` |
| **@@** | Thread | `thread::spawn(move |...| ...)` |
| **~** | Async | `~F` (Def), `~{ }` (Block), `.~` (Await) |
| **<<** | Channel | `tx << v` (Send), `<< rx` (Recv) |

---

## 6. Complete Implementation Example

This code covers **Turbofish**, **closure return types**, **macros**, **keyword safety**, and **Try operator**.

```rust
// Module definition
D data_processor

// Import standard library
u std::collections::HashMap

// Struct (S)
// Uppercase first letter -> pub struct
#D(Debug, Clone)
S Processor {
    id: u64,
    cache: HashMap<String, i32> // String = String (no abbreviation)
}

// F = pub fn
F run_logic(input: &str) -> R<V<i32>, String> {
    // v = let mut
    v results: V<i32> = V::new();
    
    // l = let, wh = where
    // Macro (println!) keeps original form, no ambiguity
    println!("Processing: {}", input);

    // Closure with return type
    l parse = |s: &str| -> R<i32, String> {
        // ! suffix = Try, macro (format!) keeps original form
        l val = s.parse::<i32>().map_err(|_| format!("Bad num: {}", s))!;
        < Ok(val)
    };

    // Loop L item: list
    L part: input.split(',') {
        // Try (!) with Match (M)
        M parse(part) {
            Ok(n): {
                // < between variables = less than
                ? n < 0 { 
                    println!("Skipping negative: {}", n);
                    c // continue
                }
                results.push(n);
            },
            Err(e): {
                // < at line start = Return
                < Err(e)
            }
        }
    }

    // vec! macro keeps original form
    l defaults = vec![1, 2, 3];
    results.extend(defaults);

    < Ok(results)
}

// Unit tests (standard syntax)
#[cfg(test)]
D tests {
    u super::*

    #[test]
    F test_run() {
        l data = "10,20,-5,abc";
        // Macro (assert!) keeps original form
        assert!(run_logic(data).is_err());
    }
}
```

---

## 7. Transpiler Implementation Guide

### 7.1 Parser Logic Simplification

Since macros are restored to native syntax, the Parser no longer needs complex Lookahead to distinguish whether `>` is Print or Greater Than.

* Encounter `Ident` + `!` (e.g., `println!`) → parse as macro invocation.
* Encounter `Expr` + `!` (e.g., `func()!`) → parse as Try operator.
* Encounter `>` → **always parse as greater than**.

### 7.2 Type Folding

* Must preserve `::<T>` (Turbofish) structure.
* Recursively convert types: `Vec` -> `V`, `Option` -> `O`, `Result` -> `R`.
* **String remains as `String`** (no abbreviation in v1.7).

### 7.3 Lexer

* Keywords: `wh` (where), `S`, `F`, `f`.
* Identifiers: `w`, `s` as regular variable names.

---

## 8. System Prompt (v1.7)

```markdown
**System Role:** Nu v1.7 Compiler (Production Standard).

**Parsing Rules:**
1.  **Keywords:** `l`=let, `v`=let mut, `a`=as, `u`=use, `t`=type, **`wh`=where**, `br`=break, `ct`=continue.
2.  **Definitions:** `S`=struct, `E`=enum (Visibility by Identifier Case). `F`=pub fn, `f`=fn (Visibility by Keyword Case). `TR`=trait, `I`=impl, `D`=mod.
3.  **Flow:** `<` (start of stmt)=return. `?`=if, `M`=match, `L i: list`=for loop.
4.  **Macros:** **KEEP** all macros as is (`println!`, `vec!`, `panic!`).
5.  **Operators:** `!`(suffix)=try. `>`=greater than.
6.  **Strings:** `String`=String(owned, no abbreviation), `str`=str(slice).
7.  **Concurrency:** `@`=spawn(async move), `~`=async, `.~`=await.
8.  **Types:** `V`=Vec, `O`=Option, `R`=Result, `A`=Arc, `X`=Mutex, `B`=Box.
9.  **Attributes:** `#D`=derive, `#I`=inline. Keep `#[test]`.
10. **Generics:** Keep `<T>` and `::<T>`.

**Task:** Convert Input description or Rust code into valid Nu v1.7 code.
```

Nu v1.7 is a mature version that balances ideals (high density) with reality (engineering compatibility). **Key improvement in v1.7**: Removed `Str` type abbreviation to eliminate conversion complexity and improve reliability.

---

## Development & Contributing

### Continuous Integration

This project uses GitHub Actions for automated testing and releases:

- **CI Workflow**: Runs on every push/PR to main branches
  - Tests on Linux, macOS, and Windows
  - Code formatting checks (`cargo fmt`)
  - Linting with Clippy (`cargo clippy`)
  - Builds all binaries

- **Release Workflow**: Automatically triggered by version tags
  - Builds binaries for 5 platforms (Linux x64/ARM64, macOS Intel/ARM, Windows x64)
  - Creates GitHub release with all binaries
  - Generates SHA256 checksums for verification

### Creating a Release

```bash
# Update version in Cargo.toml
# Current version: 1.3.1

# Create and push tag
git tag v1.3.1
git push origin v1.3.1

# GitHub Actions will automatically:
# 1. Build binaries for all platforms
# 2. Create a release with version from Cargo.toml
# 3. Upload all binaries and checksums
```

See [.github/RELEASE.md](.github/RELEASE.md) for detailed release instructions.

### Git Hooks

This project includes a pre-push hook to ensure code quality before pushing:

**Setup (one-time):**
```bash
./setup-hooks.sh
```

The pre-push hook will automatically:
- Check code formatting with `cargo fmt --check`
- Run linter with `cargo clippy`
- Run all tests

**Bypass hook (not recommended):**
```bash
git push --no-verify
```

### Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Run setup script: `./setup-hooks.sh` (first time only)
4. Make your changes
5. The pre-push hook will automatically check:
   - Code formatting (`cargo fmt`)
   - Linting (`cargo clippy`)
   - Tests (`cargo test`)
6. Submit a pull request

**Manual checks:**
```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Run tests
cargo test
```

---

## License

MIT License

Copyright (c) 2025

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.