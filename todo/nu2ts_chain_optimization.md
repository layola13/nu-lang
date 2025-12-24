# Chain Method Optimization Guide (Onion Peeling)

**Reference**: [`nu2ts.md` Section 8](file:///home/sonygod/projects/nu/docs/cn/nu2ts.md#8-链式调用处理-chain-handling---the-onion-peeling)

## Overview

Rust iterator chains like `.iter().map().collect()` need special handling to generate idiomatic TypeScript.

## Ghost Nodes (Delete Entirely)

These methods are **removed** from the output - the receiver is "promoted" directly to the next method:

```rust
// Nu
nums.iter().map(|x| x * 2).collect::<Vec<i32>>()

// Generated TS (iter and collect removed)
nums.map(x => x * 2)
```

**List of Ghost Nodes:**
- `.iter()` → *(delete)*
- `.into_iter()` → *(delete)*
- `.collect::<T>()` → *(delete)*

**Compiler Logic:**
```rust
fn is_ghost_method(method_name: &str) -> bool {
    matches!(method_name, "iter" | "into_iter" | "collect")
}

fn optimize_chain(expr: &MethodCall) -> Expr {
    if is_ghost_method(&expr.method) {
        // Skip this method, return receiver directly
        optimize_chain(expr.receiver)
    } else {
        // Keep this method
        transform_method(expr)
    }
}
```

## Method Transformations

These methods are **transformed** to their TypeScript equivalents:

| Rust Method | TypeScript Equivalent | Type |
|-------------|----------------------|------|
| `.len()` | `.length` | Property (remove `()`) |
| `.unwrap()` | `!` | Non-null assertion |
| `.unwrap_or(v)` | `?? v` | Nullish coalescing |
| `.contains(x)` | `.includes(x)` | Rename |
| `.to_string()` | `.toString()` | Rename |
| `.clone()` | `structuredClone()` | Runtime function |

## Special Cases

### .unwrap() Handling

Context-dependent transformation:

```rust
// Option<T> context
let val = maybe_value.unwrap();  // → const val = maybeValue!;

// Result<T, E> context  
let val = result.unwrap();       // → if (result.tag === 'ok') { result.val }
```

For Result, prefer pattern matching:
```typescript
if (result.tag === 'ok') {
  const val = result.val;  // Type-safe
}
```

### .len() → .length

```rust
// Nu
let count = vec.len();

// TS
const count = vec.length;
```

**AST Transformation:**
- `MethodCall { receiver, method: "len", args: [] }` 
- → `Member { object: receiver, property: "length" }`

## Complex Example

**Input (Nu):**
```rust
l nums = vec![1, 2, 3, 4, 5];
l doubled = nums.iter()
    .filter(|x| x > 2)
    .map(|x| x * 2)
    .collect::<Vec<i32>>();
l size = doubled.len();
```

**Output (TS):**
```typescript
const nums = [1, 2, 3, 4, 5];
const doubled = nums
    .filter(x => x > 2)
    .map(x => x * 2);
    // .iter() removed, .collect() removed
const size = doubled.length;  // .len() -> .length
```

## Implementation Checklist

- [ ] Detect ghost methods in method chain
- [ ] Remove ghost nodes from AST
- [ ] Transform `.len()` to `.length` property access
- [ ] Transform `.unwrap()` based on type context
- [ ] Transform `.unwrap_or(v)` to nullish coalescing
- [ ] Rename methods (`.contains()` → `.includes()`)
- [ ] Handle nested chains recursively
- [ ] Preserve comments and formatting
