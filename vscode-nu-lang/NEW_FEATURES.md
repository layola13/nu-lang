# VSCode Nu Lang 插件新功能文档

## 版本 0.0.3 新增功能

本版本新增了两个重要功能：**Nu 代码格式化**和**二进制构建**。

---

## 功能 1: Nu 代码格式化 (Format Nu Code)

### 功能描述
通过 Nu → Rust → rustfmt → Rust → Nu 的循环转换，实现 Nu 代码的格式化。

### 使用方法

#### 方法 1: 右键菜单
1. 打开 `.nu` 文件
2. 右键点击编辑器
3. 选择 **"Nu: Format Code"**

#### 方法 2: 命令面板
1. 按 `Ctrl+Shift+P` (Mac: `Cmd+Shift+P`)
2. 输入 `Nu: Format Code`
3. 按回车执行

### 实现原理
```
.nu 文件 → nu2rust → .rs 文件 → rustfmt → 格式化的 .rs → rust2nu → 格式化的 .nu
```

### 配置选项
在 VSCode 设置中配置以下选项：

```json
{
  "nu-lang.nu2rustPath": "",  // 自动检测或指定 nu2rust 路径
  "nu-lang.rust2nuPath": "/usr/local/bin/rust2nu"  // rust2nu 路径
}
```

### 特性
- ✅ 使用临时文件存储中间结果
- ✅ 保留原文件的语义
- ✅ 格式化失败时不修改原文件
- ✅ 显示进度提示
- ✅ 自动清理临时文件

### 错误处理
如果格式化失败，会显示详细的错误信息：
- `nu2rust binary not found` - 需要配置 nu2rust 路径
- `rust2nu binary not found` - 需要安装或配置 rust2nu
- `rustfmt failed` - Rust 代码格式化失败

---

## 功能 2: 编译二进制文件 (Build Binary)

### 功能描述
将 Nu 文件编译为可执行的二进制文件，支持独立文件和 Cargo 项目。

### 使用方法

#### 方法 1: 右键菜单
1. 打开 `.nu` 文件
2. 右键点击编辑器
3. 选择 **"Nu: Build Binary"**

#### 方法 2: 命令面板
1. 按 `Ctrl+Shift+P` (Mac: `Cmd+Shift+P`)
2. 输入 `Nu: Build Binary`
3. 按回车执行

### 编译策略

#### 独立文件模式
当文件不在 Cargo 项目中时：
```bash
rustc file.rs -o file  # 使用 rustc 直接编译
```

#### Cargo 项目模式
当文件在 Cargo 项目中时：
```bash
cargo build  # 或 cargo build --release (release 模式)
```

### 实现流程
1. **检查 .rs 文件** - 如不存在，先调用 nu2rust 编译
2. **检测项目类型** - 判断是否在 Cargo 项目中
3. **选择编译器** - rustc (独立) 或 cargo (项目)
4. **执行编译** - 显示进度
5. **显示结果** - 提示二进制文件路径

### 配置选项
```json
{
  "nu-lang.rustcPath": "rustc",  // rustc 路径
  "nu-lang.cargoPath": "cargo"   // cargo 路径
}
```

### 特性
- ✅ 自动检测项目类型
- ✅ 智能选择编译工具
- ✅ 显示编译进度
- ✅ 提供二进制文件路径
- ✅ 支持 release 模式（可扩展）
- ✅ 点击通知可打开文件所在文件夹

### 输出路径

#### 独立文件
```
/path/to/file.nu → /path/to/file (或 file.exe on Windows)
```

#### Cargo 项目
```
项目根目录/target/debug/项目名
或
项目根目录/target/release/项目名 (release 模式)
```

---

## 技术实现

### 文件结构
```
vscode-nu-lang/src/services/
├── formatService.ts      # 格式化服务
├── buildService.ts       # 构建服务
├── conversionService.ts  # 转换服务（已有）
├── binaryManager.ts      # 二进制管理（已有）
└── cargoService.ts       # Cargo 服务（已有）
```

### 关键类和方法

#### FormatService
```typescript
class FormatService {
  // 格式化 Nu 文件
  async formatNuFile(nuFilePath: string): Promise<FormatResult>
  
  // 格式化当前编辑器文件
  async formatCurrentFile(): Promise<void>
}
```

#### BuildService
```typescript
class BuildService {
  // 构建二进制文件
  async buildBinary(nuFilePath: string, releaseMode?: boolean): Promise<BuildResult>
  
  // 构建当前文件
  async buildCurrentFile(releaseMode?: boolean): Promise<void>
  
  // 构建并运行
  async buildAndRun(nuFilePath: string, args?: string[]): Promise<void>
}
```

---

## 依赖要求

### 必需工具
- **nu2rust** - Nu → Rust 编译器
- **rust2nu** - Rust → Nu 反编译器 (仅格式化功能需要)
- **rustfmt** - Rust 代码格式化工具
- **rustc** - Rust 编译器 (独立文件编译)
- **cargo** - Rust 包管理器 (Cargo 项目编译)

### 安装方法
```bash
# 安装 Rust 工具链 (包含 rustc, cargo, rustfmt)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 编译 nu2rust 和 rust2nu
cd /path/to/nu/project
cargo build --release

# 可选：安装到系统路径
sudo cp target/release/nu2rust /usr/local/bin/
sudo cp target/release/rust2nu /usr/local/bin/
```

---

## 测试步骤

### 测试格式化功能
1. 创建一个 `.nu` 文件，内容如下：
```nu
fn   add(   a:i32  ,  b:i32   )->i32{  a+b  }
fn main(){let x=add(1,2);println!("{}", x);}
```

2. 右键选择 "Nu: Format Code"

3. 期望结果：代码被格式化为规范格式
```nu
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn main() {
    let x = add(1, 2);
    println!("{}", x);
}
```

### 测试构建功能
1. 创建一个简单的 `.nu` 文件：
```nu
fn main() {
    println!("Hello from Nu!");
}
```

2. 右键选择 "Nu: Build Binary"

3. 期望结果：
   - 显示编译进度
   - 编译成功提示，显示二进制路径
   - 可以点击 "Open Folder" 打开文件位置

4. 运行生成的二进制文件：
```bash
./file_name  # 或在 Windows 上运行 file_name.exe
```

5. 期望输出：`Hello from Nu!`

---

## 故障排除

### 格式化失败
**问题**: `nu2rust binary not found`
**解决**: 配置 `nu-lang.nu2rustPath` 或将 nu2rust 添加到 PATH

**问题**: `rust2nu binary not found`
**解决**: 配置 `nu-lang.rust2nuPath` 或安装 rust2nu

**问题**: `rustfmt failed`
**解决**: 检查生成的 Rust 代码是否有语法错误

### 构建失败
**问题**: `rustc not found in PATH`
**解决**: 安装 Rust 工具链或配置 `nu-lang.rustcPath`

**问题**: `cargo not found`
**解决**: 安装 Rust 工具链或配置 `nu-lang.cargoPath`

**问题**: 编译错误
**解决**: 检查 Nu 代码是否正确，先尝试手动编译 .rs 文件

---

## 打包和发布

### 打包扩展
```bash
cd vscode-nu-lang
npm run package
```

这会生成 `nu-lang-0.0.3.vsix` 文件。

### 安装 VSIX
```bash
code --install-extension nu-lang-0.0.3.vsix
```

或在 VSCode 中：
1. 打开扩展面板
2. 点击 `...` 菜单
3. 选择 "Install from VSIX..."
4. 选择生成的 .vsix 文件

---

## 未来改进

### 格式化功能
- [ ] 支持自定义格式化规则
- [ ] 支持部分代码格式化（选中区域）
- [ ] 集成到 VSCode 的格式化 API (Format Document)

### 构建功能
- [ ] 添加 release 模式切换选项
- [ ] 支持自定义编译参数
- [ ] 添加"构建并运行"命令
- [ ] 集成调试功能
- [ ] 支持交叉编译

---

## 贡献

欢迎提交问题和改进建议！

## 许可证

与主项目相同