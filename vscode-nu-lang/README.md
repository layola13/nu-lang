# Nu Language Support for Visual Studio Code

<p align="center">
  <img src="icon.png" alt="Nu Language Logo" width="128" height="128">
</p>

为 Visual Studio Code 提供 **Nu v1.5.1** 语言的语法高亮支持。

## 功能特性

- ✨ 完整的 Nu v1.5.1 语法高亮
- 🎨 支持所有 Nu 关键字、操作符和类型
- 📝 智能括号匹配和自动补全
- 💬 注释支持（行注释 `//` 和块注释 `/* */`）

## 支持的语法元素

### 关键字
- **控制流**: `b` (break), `c` (continue), `wh` (while)
- **变量声明**: `l` (let), `v` (var), `a` (auto), `u` (use), `t` (type)
- **流程控制**: `M` (Match), `L` (Loop)
- **条件判断**: `?` (if)
- **异步**: `~` (async)

### 定义关键字
- **结构定义**: `S` (Struct), `E` (Enum), `TR` (Trait), `I` (Impl), `D` (Delegate)
- **函数**: `F` (Fn), `f` (fn)
- **修饰符**: `C` (Const), `ST` (Static), `EXT` (External)

### 类型系统
- **自定义类型**: `Str`, `str`, `V` (Vec), `O` (Option), `R` (Result), `A` (Arc), `X` (Mutex), `B` (Box), `W` (Weak)
- **原始类型**: `u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64`, `f32`, `f64`, `bool`, `char`, `usize`, `isize`

### 操作符
- **返回**: `<` (行首的返回语句)
- **打印**: `>` (行首的打印语句)
- **比较**: `<=`, `>=`, `==`, `!=`
- **并发**: `@` (Spawn), `@@` (Thread)
- **错误处理**: `!` (Try)

### 属性
- `#D` (Derive), `#I` (Inline), `#?` (Test), `#T` (Type), `#!` (Inner), `#[...]` (Attribute)

## 安装方法

### 方法 1: 从源码安装

1. 克隆或下载此仓库
2. 在项目根目录打开终端
3. 按 `F5` 启动扩展开发主机进行测试

### 方法 2: 从 VSIX 安装

1. 打包扩展：
   ```bash
   npm install -g vsce
   vsce package
   ```

2. 在 VSCode 中安装：
   - 按 `Ctrl+Shift+P` (Windows/Linux) 或 `Cmd+Shift+P` (macOS)
   - 输入 "Install from VSIX"
   - 选择生成的 `.vsix` 文件

## 使用方法

1. 创建或打开 `.nu` 文件
2. VSCode 会自动识别文件类型并应用语法高亮
3. 享受清晰的代码高亮体验！

## 示例代码

```nu
// Nu v1.5.1 示例代码
F add(a: i32, b: i32) -> i32 {
    < a + b
}

F main() {
    l x = 10
    l y = 20
    l result = add(x, y)
    > "Result: {}", result
    
    ? result > 25 {
        > "大于25"
    }
}
```

## 调试技巧

如果某些语法高亮不正确：

1. 在代码上按 `Ctrl+Shift+P` -> `Developer: Inspect Editor Tokens and Scopes`
2. 点击代码查看其被匹配到的 token type
3. 确认 scope 名称是否正确

## 技术细节

- 基于 TextMate Grammar 实现
- 使用正则表达式进行语法匹配
- 特别处理了单字母关键字的边界匹配，避免误伤变量名
- 行首操作符 `<` 和 `>` 通过正则 `^(\\s*)` 进行消歧

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可证

MIT

## 更新日志

### 0.0.1
- 初始版本
- 支持 Nu v1.5.1 完整语法高亮