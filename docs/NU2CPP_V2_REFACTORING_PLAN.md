# Nu2Cpp v2.0 重构计划

基于nu2rust v1.8.15的成功经验，对nu2cpp进行智能转换器架构重构。

## 参考依据

- `docs/NU2CPP_ANALYSIS_SUMMARY.md`: 5个关键架构缺陷分析
- `docs/NU2CPP_CRITICAL_FIXES.md`: 详细修复方案
- `docs/NU2CPP_IMPROVEMENTS.md`: Lambda参数保护等改进
- `src/nu2rust/mod.rs`: 成功的3160行参考实现（16次迭代验证）

## 重构目标

### 1. 增强ConversionContext（优先级：⭐⭐⭐⭐⭐）

**当前状态**（第15-25行）：
```rust
#[derive(Default)]
struct ConversionContext {
    in_trait: bool,
    in_class: bool,
    in_struct_block: bool,
    in_namespace: bool,
    current_class_name: Option<String>,
    has_constructor: bool,
}
```

**目标状态**（参考nu2rust第16-23行）：
```rust
#[derive(Default, Clone)]
struct ConversionContext {
    // C++访问修饰符状态
    in_public_section: bool,
    in_private_section: bool,
    in_protected_section: bool,
    
    // 作用域状态
    in_trait: bool,
    in_class: bool,
    in_struct_block: bool,
    in_namespace: bool,
    in_template: bool,
    in_function: bool,
    
    // C++特有
    current_class_name: Option<String>,
    has_constructor: bool,
}
```

**改进点**：
- 添加访问修饰符跟踪（public/private/protected）
- 添加模板和函数作用域跟踪
- 添加Clone trait支持上下文传递

### 2. 优化convert_line优先级（优先级：⭐⭐⭐⭐⭐）

**问题**（第121-309行）：
- Loop (L) 在Function (F) **之后**检查 → 可能误判 "L {"
- 缺少Unsafe/const函数优先级
- 没有模式守卫边界检查

**解决方案**（参考nu2rust第430-736行）：

```rust
fn convert_line(...) -> Result<Option<String>> {
    let trimmed = line.trim();
    
    // 优先级1: struct字段（最高优先级）
    if context.in_struct_block && ... { }
    
    // 优先级2: Loop (必须在Function之前！)
    if trimmed.starts_with("L ") || trimmed == "L {" { }
    
    // 优先级3: If/If not
    if trimmed.starts_with("?! ") { }  // if not 在 if 之前
    if trimmed.starts_with("if ") { }
    if trimmed.starts_with("? ") { }
    
    // 优先级4: Match
    if trimmed.starts_with("M ") { }
    
    // 优先级5: Unsafe/const函数（在普通函数之前）
    if trimmed.starts_with("unsafe F ") { }
    if trimmed.starts_with("const F ") { }
    
    // 优先级6: 普通函数（带模式守卫）
    if trimmed.starts_with("F ") {
        let after_marker = &trimmed[2..];
        if after_marker.starts_with('(') {
            // 函数调用，不是定义
            return self.convert_expression(trimmed);
        }
        if after_marker.contains('(') {
            // 函数定义
            return self.convert_function(trimmed, context);
        }
    }
    
    // ... 其他模式
}
```

### 3. 增强convert_inline_keywords（优先级：⭐⭐⭐⭐）

**当前状态**（第1483-1558行）：
- ✅ 已有字符串保护（第1491-1505行）
- ✅ 已有br/ct转换
- ❌ 缺少其他关键字转换（if, match等）
- ❌ 缺少边界检查

**目标**（参考nu2rust第1458-2366行，900+行实现）：
- 保持字符串保护机制
- 添加 if/match/use/mod 等关键字的行内转换
- 添加边界检查避免误匹配
- 处理宏调用（println!, V!等）

### 4. Lambda参数保护机制（优先级：⭐⭐⭐）

**当前状态**（第1603-1652行）：
- ✅ 已实现闭包参数保护
- ✅ 使用占位符机制 `__CLOSURE_PARAMS_N__`
- ✅ 支持返回类型检测

**需要增强**：
- C++ Lambda语法：`[capture](params){ body }`
- 需要保护capture list：`[&]`, `[=]`, `[this]`
- 需要保护Lambda参数中的单字母类型

**实现**（参考NU2CPP_IMPROVEMENTS.md第187-256行）：
```rust
// 识别C++ Lambda模式: [capture](params) -> RetType { body }
if chars[i] == '[' {
    // 查找]...( 模式
    let mut j = i + 1;
    while j < chars.len() && chars[j] != ']' { j += 1; }
    if j + 1 < chars.len() && chars[j + 1] == '(' {
        // 这是Lambda，保护整个签名
        protected_lambdas.push(lambda_signature);
        result = result.replacen(&lambda, "__LAMBDA_PARAMS_N__", 1);
    }
}
```

### 5. 边界检查类型转换（优先级：⭐⭐⭐⭐）

**当前状态**（第1560-1600行）：
- ✅ 已实现 `replace_type_with_boundary`
- ✅ 检查前边界（alphanumeric + underscore）

**已经完成，无需修改！**

示例：
```rust
fn replace_type_with_boundary(s: &str, from: &str, to: &str) -> String {
    // 检查前边界
    let has_start_boundary = i == 0 || 
        (!chars[i - 1].is_alphanumeric() && chars[i - 1] != '_');
    
    if has_start_boundary {
        result.push_str(to);
        i += from_chars.len();
        continue;
    }
}
```

使用示例（第1689-1694行）：
```rust
result = Self::replace_type_with_boundary(&result, "V<", "std::vector<");
result = Self::replace_type_with_boundary(&result, "O<", "std::optional<");
// 避免 "YEAR <" 被错误转换为 "YEAstd::expected<"
```

### 6. 字符串保护机制（优先级：⭐⭐⭐）

**当前状态**（第1491-1505行）：
- ✅ 已在convert_inline_keywords中实现
- ✅ 处理转义字符
- ✅ 跳过字符串内容

**已经完成，无需修改！**

```rust
// 跳过字符串字面量，不对其中的内容进行转换
if chars[i] == '"' {
    result.push(chars[i]);
    i += 1;
    let mut prev_char = '"';
    while i < chars.len() {
        let current = chars[i];
        result.push(current);
        i += 1;
        if current == '"' && prev_char != '\\' {
            break;
        }
        prev_char = current;
    }
    continue;
}
```

## 实施步骤

### Phase 1: 核心架构（第1-3天）

1. ✅ **增强ConversionContext**
   - 文件：`src/nu2cpp/mod.rs` 第15-25行
   - 添加访问修饰符和作用域跟踪

2. ✅ **优化convert_line优先级**
   - 文件：`src/nu2cpp/mod.rs` 第121-309行
   - 重新排序检查顺序
   - 添加模式守卫

3. ⏳ **测试核心改进**
   - 创建测试用例验证优先级
   - 测试Loop vs Function误判场景

### Phase 2: 高级功能（第4-5天）

4. ⏳ **增强convert_inline_keywords**
   - 文件：`src/nu2cpp/mod.rs` 第1483-1558行
   - 添加更多关键字支持
   - 增强边界检查

5. ⏳ **Lambda参数保护**
   - 文件：`src/nu2cpp/mod.rs` 第1603-1652行
   - 增强C++ Lambda识别
   - 保护capture list

### Phase 3: 测试和文档（第6-7天）

6. ⏳ **创建测试套件**
   - 文件：`tests/nu2cpp_v2_tests.rs`
   - 覆盖所有改进点
   - 回归测试

7. ⏳ **更新文档**
   - 文件：`docs/NU2CPP_V2_IMPLEMENTATION.md`
   - 说明新架构
   - 提供使用示例

## 成功标准

1. ✅ 所有测试通过（无回归）
2. ✅ Loop vs Function优先级正确
3. ✅ 边界检查避免误转换（如 YEAR → YEA...）
4. ✅ Lambda参数不被误转换
5. ✅ 字符串字面量内容保持不变
6. ✅ 上下文状态机正确跟踪作用域

## 风险评估

### 高风险
- convert_line优先级改动可能影响现有转换
- 缓解：充分测试，保留原有功能

### 中风险
- ConversionContext修改可能需要更新多处调用
- 缓解：使用Default trait保持向后兼容

### 低风险
- 字符串保护和边界检查已经存在
- Lambda保护是新增功能，不影响现有代码

## 参考资料

- Nu2Rust成功案例：3160行，16次迭代
- 关键错误修复：v1.6.7 (&&!), v1.7.6 (M泛型), v1.8.2 (字符串)
- 最佳实践：优先级驱动+边界检查+字符串保护

---

**文档版本**: v2.0.1  
**创建时间**: 2025-12-28  
**参考**: docs/NU2CPP_*.md, src/nu2rust/mod.rs