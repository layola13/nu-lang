
# Nu语言编译器开发计划

## 项目概述

**项目名称**: Nu (Neuro-Rust) 编译器
**版本**: v1.3.1 (含工程系统补丁)
**目标**: 实现Nu语言到Rust的双向转换器（Transpiler）+ 工程构建系统
**核心价值**: 将Rust代码压缩50-60%，使AI能处理2倍规模的代码逻辑

### 新增特性 (v1.3.1 补丁)
- ✅ Nu.toml 工程清单文件支持
- ✅ 模块系统 (D/u/U 语法)
- ✅ 导入导出机制
- ✅ 项目构建工具 (nuc)

---

## 技术栈选型

### 核心技术
- **语言**: Rust
- **词法分析**: `logos` (高性能Lexer生成器)
- **语法分析**: `syn` (Rust语法解析库) + 自定义Parse实现
- **代码生成**: `quote` + `prettyplease`
- **AST处理**: `proc-macro2`
- **工程系统**: `toml` (解析Nu.toml) + `cargo_toml` (生成Cargo.toml)
- **CLI工具**: `clap` (命令行参数解析)

### 选型理由
1. Nu是Rust的高密度方言，使用Rust可直接复用`syn`生态
2. 目标产物是标准Rust代码，`syn` AST可无损转换
3. 双向转换更容易实现（Rust2Nu利用现成的`syn::parse_file()`）
4. 生态成熟，避免重复造轮子

---

## 架构设计

### 整体流程

```
Nu项目 (Nu.toml + *.nu)
    ↓
[工程解析 - toml]
    ↓
项目结构 + 依赖图
    ↓
[逐文件编译]
    ↓
Nu源码 (.nu)
    ↓
[词法分析 - logos]
    ↓
Token流
    ↓
[语法分析 - syn + custom Parse]
    ↓
Nu AST
    ↓
[模块解析与路径解析]
    ↓
[语义转换]
    ↓
Rust AST (syn::File)
    ↓
[代码生成 - quote + prettyplease]
    ↓
Rust项目 (Cargo.toml + *.rs)
```

### 反向流程 (Rust2Nu)

```
Rust源码 (.rs)
    ↓
[syn::parse_file()]
    ↓
Rust AST
    ↓
[AST遍历 - Fold/Visit]
    ↓
Nu AST
    ↓
[格式化输出]
    ↓
Nu源码 (.nu)
```

---

## 详细开发计划

### 阶段1: 项目初始化 (1-2天)

**目标**: 搭建基础项目结构和依赖

#### 任务列表
- [x] 创建Cargo工作空间
- [ ] 添加核心依赖
  ```toml
  [dependencies]
  syn = { version = "2.0", features = ["full", "extra-traits", "parsing"] }
  quote = "1.0"
  proc-macro2 = "1.0"
  prettyplease = "0.2"
  logos = "0.13"
  anyhow = "1.0"
  clap = { version = "4.0", features = ["derive"] }
  toml = "0.8"
  serde = { version = "1.0", features = ["derive"] }
  cargo_toml = "0.17"
  walkdir = "2.4"
  ```
- [ ] 设计项目目录结构
  ```
  nu_compiler/
  ├── src/
  │   ├── lexer/       # 词法分析
  │   ├── parser/      # 语法分析
  │   ├── ast/         # AST定义
  │   ├── codegen/     # 代码生成
  │   ├── nu2rust/     # Nu->Rust转换
  │   ├── rust2nu/     # Rust->Nu转换
  │   ├── project/     # 工程系统 (Nu.toml解析)
  │   ├── module/      # 模块系统 (路径解析)
  │   ├── cli/         # CLI命令 (nuc build/run/init)
  │   └── main.rs      # CLI入口
  ├── tests/           # 测试用例
  └── examples/        # 示例代码
  ```

#### 交付物
- 可编译通过的项目骨架
- README.md说明文档

---

### 阶段2: 词法分析器 (2-3天)

**目标**: 使用`logos`实现Nu语言的Token化

#### 核心Token定义

```rust
use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
pub enum Token {
    // 定义关键字 (大写=pub, 小写=private)
    #[token("S")] StructPub,
    #[token("s")] StructPriv,
    #[token("E")] EnumPub,
    #[token("e")] EnumPriv,
    #[token("F")] FnPub,
    #[token("f")] FnPriv,
    #[token("TR")] TraitPub,
    #[token("tr")] TraitPriv,
    #[token("I")] Impl,
    #[token("D")] Mod,
    #[token("C")] Const,
    #[token("ST")] Static,
    #[token("EXT")] Extern,
    
    // 原子关键字
    #[token("l")] Let,
    #[token("v")] LetMut,
    #[token("a")] As,
    #[token("u")] Use,
    #[token("t")] Type,
    #[token("w")] Where,
    #[token("b")] Break,
    #[token("c")] Continue,
    
    // 流控和操作符
    #[token("<")] LessThanOrReturn,  // 需要上下文消歧
    #[token(">")] GreaterThanOrPrint, // 需要上下文消歧
    #[token("?")] If,
    #[token("M")] Match,
    #[token("L")] Loop,
    
    // 修饰符
    #[token("!")] MutOrTry,  // 前缀=mut, 后缀=try
    #[token("U")] Unsafe,
    #[token("&")] Ref,
    #[token("*")] Deref,
    
    // 并发
    #[token("@")] Spawn,
    #[token("@@")] Thread,
    #[token("~")] Async,
    #[token("<<")]  Channel,
    
    // 类型缩写
    #[token("V")] Vec,
    #[token("O")] Option,
    #[token("R")] Result,
    #[token("A")] Arc,
    #[token("X")] Mutex,
    #[token("B")] Box,
    #[token("W")] Weak,
    #[token("Str")] String,
    
    // 字面量和标识符
    #[regex(r"[a-z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    IdentLower(String),
    
    #[regex(r"[A-Z][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    IdentUpper(String),
    
    #[regex(r#""([^"\\]|\\.)*""#, |lex| lex.slice().to_string())]
    StringLit(String),
    
    #[regex(r"\d+", |lex| lex.slice().parse())]
    IntLit(i64),
    
    // 标点符号
    #[token("(")] LParen,
    #[token(")")] RParen,
    #[token("{")] LBrace,
    #[token("}")] RBrace,
    #[token("[")] LBracket,
    #[token("]")] RBracket,
    #[token(",")] Comma,
    #[token(";")] Semi,
    #[token(":")] Colon,
    #[token("::")] PathSep,
    #[token(".")] Dot,
    #[token(".~")] Await,
    
    // 属性
    #[token("#D")] DeriveMacro,
    #[token("#?")] CfgMacro,
    #[token("#T")] TestMacro,
    #[token("#I")] InlineMacro,
    #[token("#!")] MustUseMacro,
    
    // 空白和注释
    #[regex(r"[ \t\r\n]+", logos::skip)]
    #[regex(r"//[^\n]*", logos::skip)]
    Whitespace,
    
    #[error]
    Error,
}
```

#### 二义性处理策略

**`<` 和 `>` 的消歧**:
- 在词法阶段生成`LessThanOrReturn`和`GreaterThanOrPrint`
- 在语法分析阶段根据**位置上下文**确定实际含义:
  - 语句开头 → Return/Print
  - 表达式中间 → 比较运算符

**`!` 的消歧**:
- 词法阶段记录位置
- 语法阶段根据前后文:
  - 前缀(`&!`, `*!`) → Mut
  - 后缀(`.!`, `()!`) → Try

#### 交付物
- 完整的Token定义
- 词法分析器单元测试
- 支持错误位置报告

---

### 阶段3: AST定义 (2-3天)

**目标**: 定义Nu语言的抽象语法树结构

#### 核心AST节点

```rust
// ast/mod.rs

use syn::{Ident, Type, Expr, Block};

/// Nu语言的顶层项
pub enum NuItem {
    Fn(NuFn),
    Struct(NuStruct),
    Enum(NuEnum),
    Trait(NuTrait),
    Impl(NuImpl),
    Mod(NuMod),
    Use(NuUse),
    Const(NuConst),
    Static(NuStatic),
}

/// 函数定义
pub struct NuFn {
    pub visibility: Visibility,  // 大写=pub
    pub is_async: bool,          // ~ 前缀
    pub name: Ident,
    pub generics: syn::Generics,
    pub inputs: Vec<NuFnArg>,
    pub output: Option<Type>,
    pub body: NuBlock,
}

/// 结构体定义
pub struct NuStruct {
    pub visibility: Visibility,
    pub name: Ident,
    pub generics: syn::Generics,
    pub fields: Vec<NuField>,
}

/// 语句
pub enum NuStmt {
    Let(NuLet),           // l x = ...
    LetMut(NuLetMut),     // v x = ...
    Return(Expr),         // < val
    Print(Expr),          // > val
    If(NuIf),             // ? cond { }
    Match(NuMatch),       // M val { }
    Loop(NuLoop),         // L { } 或 L i: list { }
    Spawn(NuSpawn),       // @ { }
    Thread(NuThread),     // @@ { }
    Expr(Expr),
}

/// 表达式扩展
pub enum NuExpr {
    Await(Box<Expr>),     // expr.~
    Try(Box<Expr>),       // expr!
    Channel(NuChannel),   // tx << val 或 << rx
    Closure(NuClosure),   // |x| ... 或 $|x| ...
    Std(Expr),            // 标准Rust表达式
}

pub enum Visibility {
    Public,    // 大写首字母
    Private,   // 小写首字母
}
```

#### 交付物
- 完整的AST类型定义
- AST构建辅助函数
- AST遍历trait (Visitor模式)

---

### 阶段4: 语法分析器 (5-7天)

**目标**: 实现从Token流到Nu AST的解析

#### 核心解析策略

使用`syn`的`ParseStream`实现自定义解析:

```rust
use syn::parse::{Parse, ParseStream, Result};
use syn::Token;

impl Parse for NuStmt {
    fn parse(input: ParseStream) -> Result<Self> {
        // 1. 检查语句开头的特殊符号
        if input.peek(Token![<]) {
            // < 在语句开头 = Return
            input.parse::<Token![<]>()?;
            let expr = input.parse()?;
            return Ok(NuStmt::Return(expr));
        }
        
        if input.peek(Token![>]) {
            // > 在语句开头 = Print
            input.parse::<Token![>]>()?;
            let expr = parse_print_expr(input)?;  // 处理字符串插值
            return Ok(NuStmt::Print(expr));
        }
        
        if input.peek(Token![?]) {
            // ? = If
            return Ok(NuStmt::If(input.parse()?));
        }
        
        // 2. 检查关键字
        let lookahead = input.lookahead1();
        
        if lookahead.peek(kw::l) {  // l = let
            return Ok(NuStmt::Let(input.parse()?));
        }
        
        if lookahead.peek(kw::v) {  // v = let mut
            return Ok(NuStmt::LetMut(input.parse()?));
        }
        
        // 3. 否则解析为表达式
        let expr = input.parse()?;
        Ok(NuStmt::Expr(expr))
    }
}
```

#### 关键难点处理

**1. `<` / `>` 二义性**
- 在语句级别: 行首直接判断为Return/Print
- 在表达式级别: 使用Rust标准表达式解析器(已处理二义性)

**2. `!` 前缀/后缀**
```rust
// 前缀: &!self -> &mut self
if input.peek(Token![&]) && input.peek2(Token![!]) {
    input.parse::<Token![&]>()?;
    input.parse::<Token![!]>()?;
    // 生成 &mut
}

// 后缀: func()! -> func()?
let expr = input.parse()?;
if input.peek(Token![!]) {
    input.parse::<Token![!]>()?;
    // 转换为Try表达式
}
```

**3. 字符串插值**
```rust
// > "Value: {x}" -> println!("Value: {}", x)
fn parse_print_expr(input: ParseStream) -> Result<Expr> {
    let lit: LitStr = input.parse()?;
    let s = lit.value();
    
    // 解析 {...} 

---

### 阶段5: 工程系统实现 (3-4天) 🆕

**目标**: 实现Nu.toml解析和Cargo.toml生成

#### 5.1 Nu.toml结构定义

```rust
// project/nu_manifest.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct NuManifest {
    #[serde(rename = "P")]
    pub package: PackageInfo,
    
    #[serde(rename = "D", default)]
    pub dependencies: HashMap<String, Dependency>,
    
    #[serde(rename = "DD", default)]
    pub dev_dependencies: HashMap<String, Dependency>,
    
    #[serde(rename = "W")]
    pub workspace: Option<Workspace>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PackageInfo {
    pub id: String,        // name
    pub v: String,         // version
    #[serde(default = "default_edition")]
    pub ed: String,        // edition
}

fn default_edition() -> String {
    "2024".to_string()
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Dependency {
    Simple(String),  // "1.0"
    Detailed {
        #[serde(rename = "v")]
        version: Option<String>,
        
        #[serde(rename = "f")]
        features: Option<Vec<String>>,
        
        path: Option<String>,
        git: Option<String>,
        branch: Option<String>,
    },
}
```

#### 5.2 Nu.toml → Cargo.toml转换

```rust
// project/converter.rs

use cargo_toml::{Manifest, Package, Dependency as CargoDep};
use std::collections::BTreeMap;

pub fn nu_to_cargo(nu_manifest: NuManifest) -> Result<Manifest> {
    let mut cargo = Manifest::default();
    
    // 转换Package信息
    cargo.package = Some(Package {
        name: nu_manifest.package.id,
        version: nu_manifest.package.v.parse()?,
        edition: Some(parse_edition(&nu_manifest.package.ed)?),
        ..Default::default()
    });
    
    // 转换Dependencies
    cargo.dependencies = convert_deps(nu_manifest.dependencies)?;
    cargo.dev_dependencies = convert_deps(nu_manifest.dev_dependencies)?;
    
    Ok(cargo)
}

fn convert_deps(nu_deps: HashMap<String, Dependency>) -> Result<BTreeMap<String, CargoDep>> {
    let mut cargo_deps = BTreeMap::new();
    
    for (name, dep) in nu_deps {
        let cargo_dep = match dep {
            Dependency::Simple(ver) => CargoDep::Simple(ver),
            Dependency::Detailed { version, features, path, git, branch } => {
                CargoDep::Detailed(cargo_toml::DependencyDetail {
                    version: version,
                    features: features.unwrap_or_default(),
                    path: path,
                    git: git,
                    branch: branch,
                    ..Default::default()
                })
            }
        };
        cargo_deps.insert(name, cargo_dep);
    }
    
    Ok(cargo_deps)
}
```

#### 5.3 模块路径解析器

```rust
// module/resolver.rs

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct ModuleResolver {
    src_dir: PathBuf,
    module_map: HashMap<String, PathBuf>,
}

impl ModuleResolver {
    pub fn new(src_dir: PathBuf) -> Self {
        let mut resolver = Self {
            src_dir,
            module_map: HashMap::new(),
        };
        resolver.scan_modules();
        resolver
    }
    
    /// 扫描src目录，构建模块映射
    fn scan_modules(&mut self) {
        for entry in WalkDir::new(&self.src_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "nu"))
        {
            let path = entry.path();
            let rel_path = path.strip_prefix(&self.src_dir).unwrap();
            let module_name = self.path_to_module_name(rel_path);
            self.module_map.insert(module_name, path.to_path_buf());
        }
    }
    
    /// 解析模块声明: D Network -> 查找 network.nu 或 network/mod.nu
    pub fn resolve_module(&self, name: &str) -> Option<PathBuf> {
        let snake_case = to_snake_case(name);
        
        // 尝试 network.nu
        let file_path = self.src_dir.join(format!("{}.nu", snake_case));
        if file_path.exists() {
            return Some(file_path);
        }
        
        // 尝试 network/mod.nu
        let mod_path = self.src_dir.join(&snake_case).join("mod.nu");
        if mod_path.exists() {
            return Some(mod_path);
        }
        
        None
    }
    
    /// 将路径转换为模块名: src/handlers/auth.nu -> handlers::auth
    fn path_to_module_name(&self, path: &Path) -> String {
        path.with_extension("")
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect::<Vec<_>>()
            .join("::")
    }
}

/// PascalCase -> snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }
    result
}
```

#### 5.4 CLI命令实现

```rust
// cli/commands.rs

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "nuc")]
#[command(about = "Nu Language Compiler", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 初始化新的Nu项目
    Init {
        /// 项目名称
        name: String,
    },
    
    /// 编译Nu项目
    Build {
        /// 发布模式
        #[arg(short, long)]
        release: bool,
    },
    
    /// 运行Nu项目
    Run {
        /// 传递给程序的参数
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    
    /// 压缩Rust代码为Nu
    Compress {
        /// 输入Rust文件或目录
        input: PathBuf,
        
        /// 输出Nu文件或目录
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

// cli/init.rs
pub fn init_project(name: &str) -> Result<()> {
    let project_dir = Path::new(name);
    
    // 创建目录结构
    fs::create_dir_all(project_dir.join("src"))?;
    
    // 生成Nu.toml
    let nu_toml = format!(
        r#"[P]
id = "{name}"
v = "0.1.0"
ed = "2024"

[D]
# 在这里添加依赖
"#
    );
    fs::write(project_dir.join("Nu.toml"), nu_toml)?;
    
    // 生成main.nu
    let main_nu = r#"~F Main() {
    > "Hello, Nu!";
}
"#;
    fs::write(project_dir.join("src/main.nu"), main_nu)?;
    
    println!("✓ 项目 {name} 创建成功!");
    Ok(())
}
```

#### 5.5 完整编译流程

```rust
// cli/build.rs

pub fn build_project(release: bool) -> Result<()> {
    // 1. 读取Nu.toml
    let nu_manifest = read_nu_manifest("Nu.toml")?;
    
    // 2. 生成Cargo.toml
    let cargo_manifest = nu_to_cargo(nu_manifest)?;
    fs::write("Cargo.toml", toml::to_string(&cargo_manifest)?)?;
    
    // 3. 扫描src目录
    let resolver = ModuleResolver::new(PathBuf::from("src"));
    
    // 4. 编译所有.nu文件
    for entry in WalkDir::new("src")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "nu"))
    {
        let nu_path = entry.path();
        let rs_path = nu_path.with_extension("rs");
        
        println!("Compiling {} -> {}", nu_path.display(), rs_path.display());
        
        // 编译单个文件
        compile_file(nu_path, &rs_path, &resolver)?;
    }
    
    // 5. 调用cargo build
    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    if release {
        cmd.arg("--release");
    }
    
    let status = cmd.status()?;
    if !status.success() {
        bail!("Cargo build failed");
    }
    
    println!("✓ 编译成功!");
    Ok(())
}

fn compile_file(nu_path: &Path, rs_path: &Path, resolver: &ModuleResolver) -> Result<()> {
    // 读取Nu源码
    let nu_code = fs::read_to_string(nu_path)?;
    
    // 词法分析
    let tokens = tokenize(&nu_code)?;
    
    // 语法分析
    let nu_ast = parse(tokens)?;
    
    // 模块解析 (处理 D/u/U 语句)
    let nu_ast = resolve_modules(nu_ast, resolver)?;
    
    // 转换为Rust AST
    let rust_ast = nu_to_rust_ast(nu_ast)?;
    
    // 生成Rust代码
    let rust_code = generate_rust_code(rust_ast)?;
    
    // 写入文件
    fs::write(rs_path, rust_code)?;
    
    Ok(())
}
```

#### 交付物
- Nu.toml解析器
- Cargo.toml生成器
- 模块路径解析器
- CLI工具 (nuc init/build/run/compress)
- 完整的项目编译流程

---

### 阶段6: 模块系统语法支持 (2-3天) 🆕

**目标**: 在Parser中支持D/u/U模块语法

#### 6.1 模块声明解析

```rust
// parser/module.rs

/// 解析模块声明: D Network 或 D utils
impl Parse for NuModDecl {
    fn parse(input: ParseStream) -> Result<Self> {
        // 检查是否是 D
        if !input.peek(kw::D) {
            return Err(input.error("Expected 'D' for module declaration"));
        }
        input.parse::<kw::D>()?;
        
        // 解析模块名
        let ident: Ident = input.parse()?;
        
        // 根据首字母判断可见性
        let visibility = if ident.to_string().chars().next().unwrap().is_uppercase() {
            Visibility::Public
        } else {
            Visibility::Private
        };
        
        // 转换为snake_case作为实际模块名
        let module_name = to_snake_case(&ident.to_string());
        
        Ok(NuModDecl {
            visibility,
            name: Ident::new(&module_name, ident.span()),
        })
    }
}
```

#### 6.2 Use语句解析

```rust
// parser/use_stmt.rs

/// 解析use语句: u std::io 或 U std::io (pub use)
impl Parse for NuUse {
    fn parse(input: ParseStream) -> Result<Self> {
        // u = private use, U = pub use
        let is_pub = if input.peek(kw::U_upper) {
            input.parse::<kw::U_upper>()?;
            true
        } else if input.peek(kw::u_lower) {
            input.parse::<kw::u_lower>()?;
            false
        } else {
            return Err(input.error("Expected 'u' or 'U' for use statement"));
        };
        
        // 解析路径: std::io 或 std::{fs, io}
        let path = parse_use_path(input)?;
        
        // 检查是否有 as 别名
        let alias = if input.peek(kw::a) {
            input.parse::<kw::a>()?;
            Some(input.parse()?)
        } else {
            None
        };
        
        Ok(NuUse {
            visibility: if is_pub { Visibility::Public } else { Visibility::Private },
            path,
            alias,
        })
    }
}

fn parse_use_path(input: ParseStream) -> Result<UsePath> {
    let mut segments = Vec::new();
    
    loop {
        segments.push(input.parse::<Ident>()?);
        
        if !input.peek(Token![::]) {
            break;
        }
        input.parse::<Token![::]>()?;
        
        // 检查是否是组导入: std::{fs, io}
        if input.peek(token::Brace) {
            let content;
            braced!(content in input);
            let items = content.parse_terminated(Ident::parse, Token![,])?;
            return Ok(UsePath::Group {
                base: segments,
                items: items.into_iter().collect(),
            });
        }
        
        // 检查是否是glob: std::*
        if input.peek(Token![*]) {
            input.parse::<Token![*]>()?;
            return Ok(UsePath::Glob(segments));
        }
    }
    
    Ok(UsePath::Simple(segments))
}

#[derive(Debug)]
pub enum UsePath {
    Simple(Vec<Ident>),                    // std::io
    Group { base: Vec<Ident>, items: Vec<Ident> },  // std::{fs, io}
    Glob(Vec<Ident>),                      // std::*
}
```

#### 6.3 模块转换为Rust

```rust
// codegen/module.rs

impl ToRust for NuModDecl {
    fn to_rust(&self) -> TokenStream {
        let vis = match self.visibility {
            Visibility::Public => quote! { pub },
            Visibility::Private => quote! {},
        };
        
        let name = &self.name;
        
        quote! {
            #vis mod #name;
        }
    }
}

impl ToRust for NuUse {
    fn to_rust(&self) -> TokenStream {
        let vis = match self.visibility {
            Visibility::Public => quote! { pub },
            Visibility::Private => quote! {},
        };
        
        let path_tokens = match &self.path {
            UsePath::Simple(segments) => {
                quote! { #(#segments)::* }
            }
            UsePath::Group { base, items } => {
                quote! { #(#base)::*::{#(#items),*} }
            }
            UsePath::Glob(segments) => {
                quote! { #(#segments)::*::* }
            }
        };
        
        if let Some(alias) = &self.alias {
            quote! {
                #vis use #path_tokens as #alias;
            }
        } else {
            quote! {
                #vis use #path_tokens;
            }
        }
    }
}
```

#### 交付物
- 模块声明解析器
- Use语句解析器
- 路径解析逻辑
- Rust代码生成
- 集成测试

---
