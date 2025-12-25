# Match 语句快速修复实现方案

## 一、核心实现

### 1.1 数据结构定义

在 `src/nu2ts/types.rs` 中添加:

```rust
// Match AST 结构
#[derive(Debug, Clone)]
pub(crate) struct MatchAst {
    pub target: String,              // 匹配目标表达式
    pub target_type: Option<String>, // 推断的类型 (Result/Option/Enum)
    pub arms: Vec<MatchArm>,         // 分支列表
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct MatchArm {
    pub pattern: MatchPattern,
    pub guard: Option<String>,       // 守卫条件 (暂不支持)
    pub body: String,                // 分支体代码
}

#[derive(Debug, Clone)]
pub enum MatchPattern {
    // Ok(binding)
    ResultOk { binding: String },
    // Err(binding)
    ResultErr { binding: String },
    // Some(binding)
    OptionSome { binding: String },
    // None
    OptionNone,
    // 字面量: 1, "abc", true
    Literal { value: String },
    // 通配符: _
    Wildcard,
}
```

---

### 1.2 Match 解析函数

在 `src/nu2ts/converter.rs` 中添加:

```rust
impl Nu2TsConverter {
    /// 解析 Match 语句
    /// 
    /// 输入格式:
    /// ```nu
    /// M expr {
    ///     Ok(val): { body },
    ///     Err(e): { body }
    /// }
    /// ```
    fn parse_match_statement(
        &self,
        lines: &[&str],
        start: usize,
    ) -> Result<MatchAst> {
        let first_line = lines[start].trim();
        
        // 提取匹配目标: "M expr {" -> "expr"
        let target = if let Some(pos) = first_line.find('{') {
            first_line[2..pos].trim().to_string()
        } else {
            // 下一行是 {
            first_line[2..].trim().to_string()
        };
        
        // 收集所有行直到匹配的 }
        let mut brace_count = if first_line.contains('{') { 1 } else { 0 };
        let mut i = start + 1;
        
        if brace_count == 0 {
            if i < lines.len() && lines[i].trim() == "{" {
                brace_count = 1;
                i += 1;
            }
        }
        
        let mut arms = Vec::new();
        let mut current_pattern: Option<String> = None;
        let mut current_body = String::new();
        let mut arm_brace_count = 0;
        
        while i < lines.len() && brace_count > 0 {
            let line = lines[i];
            let trimmed = line.trim();
            
            // 更新大括号计数
            for ch in trimmed.chars() {
                match ch {
                    '{' => brace_count += 1,
                    '}' => brace_count -= 1,
                    _ => {}
                }
            }
            
            // 检测分支开始: "Ok(val):" 或 "Err(e):"
            if trimmed.contains(':') && trimmed.ends_with(':') {
                // 保存上一个分支
                if let Some(pat) = current_pattern.take() {
                    arms.push(MatchArm {
                        pattern: self.parse_match_pattern(&pat)?,
                        guard: None,
                        body: current_body.trim().to_string(),
                    });
                    current_body.clear();
                }
                
                // 开始新分支
                current_pattern = Some(trimmed.trim_end_matches(':').to_string());
            } else if trimmed.contains(":{") {
                // 单行形式: "Ok(val): { body }"
                let parts: Vec<&str> = trimmed.splitn(2, ":{").collect();
                current_pattern = Some(parts[0].trim().to_string());
                arm_brace_count = 1;
                current_body.push_str(parts[1].trim_end_matches('}'));
                
                if !parts[1].ends_with('}') || parts[1].matches('{').count() > 1 {
                    arm_brace_count += parts[1].matches('{').count();
                    arm_brace_count -= parts[1].matches('}').count();
                }
            } else if current_pattern.is_some() {
                // 收集分支体
                if trimmed == "{" {
                    arm_brace_count = 1;
                } else if arm_brace_count > 0 {
                    arm_brace_count += trimmed.matches('{').count();
                    arm_brace_count -= trimmed.matches('}').count();
                    
                    if arm_brace_count == 0 {
                        // 分支结束
                        if let Some(pat) = current_pattern.take() {
                            arms.push(MatchArm {
                                pattern: self.parse_match_pattern(&pat)?,
                                guard: None,
                                body: current_body.trim().to_string(),
                            });
                            current_body.clear();
                        }
                    } else {
                        current_body.push_str(line);
                        current_body.push('\n');
                    }
                } else {
                    current_body.push_str(line);
                    current_body.push('\n');
                }
            }
            
            i += 1;
        }
        
        // 推断目标类型
        let target_type = self.infer_match_target_type(&arms);
        
        Ok(MatchAst {
            target,
            target_type,
            arms,
            start_line: start,
            end_line: i - 1,
        })
    }
    
    /// 解析匹配模式
    fn parse_match_pattern(&self, pattern: &str) -> Result<MatchPattern> {
        let trimmed = pattern.trim();
        
        if trimmed.starts_with("Ok(") && trimmed.ends_with(')') {
            let binding = trimmed[3..trimmed.len()-1].trim().to_string();
            return Ok(MatchPattern::ResultOk { binding });
        }
        
        if trimmed.starts_with("Err(") && trimmed.ends_with(')') {
            let binding = trimmed[4..trimmed.len()-1].trim().to_string();
            return Ok(MatchPattern::ResultErr { binding });
        }
        
        if trimmed.starts_with("Some(") && trimmed.ends_with(')') {
            let binding = trimmed[5..trimmed.len()-1].trim().to_string();
            return Ok(MatchPattern::OptionSome { binding });
        }
        
        if trimmed == "None" {
            return Ok(MatchPattern::OptionNone);
        }
        
        if trimmed == "_" {
            return Ok(MatchPattern::Wildcard);
        }
        
        // 字面量
        Ok(MatchPattern::Literal {
            value: trimmed.to_string(),
        })
    }
    
    /// 根据分支模式推断目标类型
    fn infer_match_target_type(&self, arms: &[MatchArm]) -> Option<String> {
        for arm in arms {
            match &arm.pattern {
                MatchPattern::ResultOk { .. } | MatchPattern::ResultErr { .. } => {
                    return Some("Result".to_string());
                }
                MatchPattern::OptionSome { .. } | MatchPattern::OptionNone => {
                    return Some("Option".to_string());
                }
                _ => {}
            }
        }
        None
    }
}
```

---

### 1.3 代码生成函数

```rust
impl Nu2TsConverter {
    /// 生成 TypeScript if-chain
    fn generate_match_ifchain(
        &self,
        match_ast: &MatchAst,
        context: &mut ConversionContext,
    ) -> Result<String> {
        let temp_var = format!("_match{}", context.next_temp_counter());
        let mut output = String::new();
        
        // 计算匹配目标
        output.push_str(&format!(
            "const {} = {};\n",
            temp_var,
            self.convert_expression(&match_ast.target, context)?
        ));
        
        // 生成分支
        let match_type = match_ast.target_type.as_deref();
        let mut is_first = true;
        
        for arm in &match_ast.arms {
            let condition = self.generate_match_condition(
                &temp_var,
                &arm.pattern,
                match_type,
            )?;
            
            let prefix = if is_first {
                "if"
            } else {
                "else if"
            };
            is_first = false;
            
            // 生成绑定变量
            let binding = self.generate_pattern_binding(&arm.pattern, &temp_var)?;
            
            // 转换分支体
            let body = self.convert_expression(&arm.body, context)?;
            
            output.push_str(&format!(
                "{} ({}) {{\n{}\n    {}\n}}\n",
                prefix,
                condition,
                binding,
                body.lines()
                    .map(|l| format!("    {}", l))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        
        Ok(output.trim_end().to_string())
    }
    
    /// 生成匹配条件
    fn generate_match_condition(
        &self,
        temp_var: &str,
        pattern: &MatchPattern,
        match_type: Option<&str>,
    ) -> Result<String> {
        Ok(match pattern {
            MatchPattern::ResultOk { .. } => {
                format!("{}.tag === 'ok'", temp_var)
            }
            MatchPattern::ResultErr { .. } => {
                format!("{}.tag === 'err'", temp_var)
            }
            MatchPattern::OptionSome { .. } => {
                format!("{} !== null", temp_var)
            }
            MatchPattern::OptionNone => {
                format!("{} === null", temp_var)
            }
            MatchPattern::Literal { value } => {
                format!("{} === {}", temp_var, value)
            }
            MatchPattern::Wildcard => {
                "true".to_string()
            }
        })
    }
    
    /// 生成模式绑定代码
    fn generate_pattern_binding(
        &self,
        pattern: &MatchPattern,
        temp_var: &str,
    ) -> Result<String> {
        Ok(match pattern {
            MatchPattern::ResultOk { binding } => {
                format!("    const {} = {}.val;", binding, temp_var)
            }
            MatchPattern::ResultErr { binding } => {
                format!("    const {} = {}.err;", binding, temp_var)
            }
            MatchPattern::OptionSome { binding } => {
                format!("    const {} = {};", binding, temp_var)
            }
            _ => String::new(),
        })
    }
}
```

---

## 二、集成到主流程

修改 `converter.rs` 的 `convert` 方法 (替换第171-193行):

```rust
// 检测match块
if trimmed.starts_with("M ") {
    // ✅ 新实现: 解析并转换
    let match_ast = self.parse_match_statement(&lines, i)?;
    let converted = self.generate_match_ifchain(&match_ast, &mut context)?;
    output.push_str(&converted);
    output.push('\n');
    i = match_ast.end_line;
    i += 1;
    continue;
}
```

---

## 三、测试用例

### 测试 1: 简单 Result 匹配

**输入**:
```nu
l result = compute(10);
M result {
    Ok(val): {
        println!("Success: {}", val);
    },
    Err(e): {
        println!("Error: {}", e);
    }
}
```

**预期输出**:
```typescript
const result = compute(10);
const _match0 = result;
if (_match0.tag === 'ok') {
    const val = _match0.val;
    console.log($fmt("Success: {}", val));
} else if (_match0.tag === 'err') {
    const e = _match0.err;
    console.log($fmt("Error: {}", e));
}
```

---

### 测试 2: Option 匹配

**输入**:
```nu
M user.find(id) {
    Some(u): { < u.name },
    None: { < "Unknown" }
}
```

**预期输出**:
```typescript
const _match0 = user.find(id);
if (_match0 !== null) {
    const u = _match0;
    return u.name;
} else if (_match0 === null) {
    return "Unknown";
}
```

---

### 测试 3: 字面量匹配

**输入**:
```nu
M status {
    1: { println!("Init") },
    2: { println!("Running") },
    _: { println!("Unknown") }
}
```

**预期输出**:
```typescript
const _match0 = status;
if (_match0 === 1) {
    console.log("Init");
} else if (_match0 === 2) {
    console.log("Running");
} else if (true) {
    console.log("Unknown");
}
```

---

### 测试 4: 嵌套表达式

**输入**:
```nu
M compute(x + 1) {
    Ok(val): { < val * 2 },
    Err(_): { < 0 }
}
```

**预期输出**:
```typescript
const _match0 = compute(x + 1);
if (_match0.tag === 'ok') {
    const val = _match0.val;
    return val * 2;
} else if (_match0.tag === 'err') {
    const _ = _match0.err;
    return 0;
}
```

---

### 测试 5: 单行形式

**输入**:
```nu
M x { Ok(v): { v }, Err(_): { 0 } }
```

**预期输出**:
```typescript
const _match0 = x;
if (_match0.tag === 'ok') {
    const v = _match0.val;
    v;
} else if (_match0.tag === 'err') {
    const _ = _match0.err;
    0;
}
```

---

## 四、边界条件处理

### 4.1 未匹配的大括号

```rust
// 在 parse_match_statement 中添加:
if brace_count != 0 {
    anyhow::bail!("Unmatched braces in match statement starting at line {}", start);
}
```

### 4.2 空 Match 块

```rust
if arms.is_empty() {
    anyhow::bail!("Match statement has no arms at line {}", start);
}
```

### 4.3 未知模式

```rust
fn parse_match_pattern(&self, pattern: &str) -> Result<MatchPattern> {
    // ... 现有逻辑 ...
    
    // 兜底: 警告但仍尝试处理
    eprintln!("Warning: Unknown match pattern '{}', treating as literal", pattern);
    Ok(MatchPattern::Literal {
        value: pattern.to_string(),
    })
}
```

---

## 五、集成检查清单

```markdown
- [ ] 在 `types.rs` 中添加 `MatchAst`, `MatchArm`, `MatchPattern` 定义
- [ ] 在 `converter.rs` 中添加 4 个新方法:
  - [ ] `parse_match_statement`
  - [ ] `generate_match_ifchain`
  - [ ] `parse_match_pattern`
  - [ ] `infer_match_target_type`
  - [ ] `generate_match_condition`
  - [ ] `generate_pattern_binding`
- [ ] 替换主流程中的 TODO 注释 (171-193行)
- [ ] 添加单元测试到 `tests/nu2ts/match_tests.rs`
- [ ] 运行回归测试确保无破坏
- [ ] 更新文档说明已支持的 Match 特性
```

---

## 六、已知限制

当前实现 **不支持**:

1. **守卫条件** (`Ok(x) if x > 10: { ... }`)
2. **嵌套模式** (`Ok((a, b)): { ... }`)
3. **范围模式** (`1..=10: { ... }`)
4. **多模式合并** (`Ok(1) | Ok(2): { ... }`)

这些特性可在阶段 2 (AST 重构) 后实现。

---

## 七、性能考虑

- **时间复杂度**: O(n) (n = match 块内行数)
- **空间复杂度**: O(m) (m = 分支数量)
- **预计影响**: 转换速度提升 < 5% (Match 通常占代码 10%)

---

## 八、向后兼容性

✅ **完全兼容**: 新实现替换了原先跳过的逻辑,不影响其他代码路径。
