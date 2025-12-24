# Nu v1.6 Complete Feature Coverage for TypeScript Compiler

**Reference**: [README.md Section 2.2, 5.1-5.4](file:///home/sonygod/projects/nu/README.md)

## Missing Features Identified

### 1. Control Flow Keywords

| Nu Keyword | Rust Original | TypeScript Output | Status |
|------------|---------------|-------------------|--------|
| **br** | `break` | `break;` | ⚠️ **MISSING** |
| **ct** | `continue` | `continue;` | ⚠️ **MISSING** |
| **b** | `break` | `break;` | ✅ Covered |
| **c** | `continue` | `continue;` | ✅ Covered |

> **Note**: Nu v1.6 uses **`br`** and **`ct`** (not `b`/`c`) per README.md line 236-237!

### 2. Concurrency & Async Symbols

| Nu Symbol | Meaning | Rust Equivalent | TypeScript Output | Status |
|-----------|---------|-----------------|-------------------|--------|
| **@** | Spawn | `tokio::spawn(async move { ... })` | `Promise` or async IIFE | ⚠️ **MISSING** |
| **@@** | Thread | `thread::spawn(move \|...\| ...)` | `new Worker()` (Node.js) | ⚠️ **MISSING** |
| **~** | Async | `async fn` / `async { }` | `async function` / `async () => {}` | ⚠️ **MISSING** |
| **.~** | Await | `.await` | `await expr` | ⚠️ **MISSING** |
| **<<** | Channel | Send/Recv | EventEmitter or Channel polyfill | ⚠️ **MISSING** |

### 3. Memory & Safety Keywords

| Nu Keyword | Rust Original | TypeScript Handling | Status |
|------------|---------------|---------------------|--------|
| **U** | `unsafe` | Add `// @ts-ignore` comment or keep as-is | ⚠️ **MISSING** |
| **EXT** | `extern` | `declare` or skip | ⚠️ **MISSING** |
| **ST** | `static` | `static` (in class) or `const` | ⚠️ **MISSING** |

### 4. Closure Variants

| Nu Syntax | Rust | TypeScript | Status |
|-----------|------|------------|--------|
| `\|x\| expr` | `\|x\| expr` | `(x) => expr` | ✅ Covered |
| `\|x\| -> T { }` | `\|x\| -> T { }` | `(x): T => { }` | ✅ Covered |
| **`$\|x\| { }`** | `move \|x\| { }` | `(x) => { }` (ignore move) | ⚠️ **MISSING** |

### 5. Memory Modifiers

| Nu Syntax | Rust | TypeScript | Status |
|-----------|------|------------|--------|
| `&!self` | `&mut self` | `this` (ignore mutability) | ⚠️ **MISSING** |
| `*!ptr` | `*mut ptr` | `ptr` (dereference) | ⚠️ **MISSING** |
| `&x` | `&x` | `x` (remove ref) | ✅ Covered |
| `*x` | `*x` | `x` (remove deref) | ✅ Covered |

### 6. Type Aliases & Attributes

| Nu Syntax | Rust | TypeScript | Status |
|-----------|------|------------|--------|
| **t** Name = Type | `type Name = Type` | `type Name = Type` | ⚠️ **MISSING** |
| **#D(...)** | `#[derive(...)]` | *(ignore or comment)* | ⚠️ **MISSING** |
| **#I** | `#[inline]` | `// @inline` comment | ⚠️ **MISSING** |
| **#[test]** | `#[test]` | *(skip or convert to Jest)* | ⚠️ **MISSING** |

---

## Complete Mapping Table

### Keywords (Definition)

| Nu | Rust | TS | Priority |
|----|------|----|----------|
| S | struct | interface/type | P0 |
| E | enum | enum/union | P0 |
| F | pub fn | export function | P0 |
| f | fn | function | P0 |
| TR | trait | interface | P1 |
| I | impl | namespace/class | P1 |
| D | mod | module | P1 |
| C | const | const | P1 |
| **ST** | **static** | **static/const** | **P2** ⚠️ |
| **EXT** | **extern** | **declare** | **P3** ⚠️ |

### Keywords (Atomic)

| Nu | Rust | TS | Priority |
|----|------|----|----------|
| l | let | const | P0 |
| v | let mut | let | P0 |
| **t** | **type** | **type** | **P1** ⚠️ |
| **a** | **as** | **as** | **P1** ⚠️ |
| u | use | import | P1 |
| wh | where | (generic constraint) | P2 |
| **br** | **break** | **break** | **P0** ⚠️ |
| **ct** | **continue** | **continue** | **P0** ⚠️ |

### Symbols (Flow Control)

| Nu | Rust | TS | Priority |
|----|------|----|----------|
| < | return | return | P0 |
| ? | if | if | P0 |
| M | match | switch/if | P0 |
| L | loop/for | while(true)/for | P0 |
| ! (suffix) | ? (try) | ! (non-null assert) | P1 |

### Symbols (Concurrency)

| Nu | Rust | TS | Priority |
|----|------|----|----------|
| **@** | **tokio::spawn** | **Promise/async** | **P2** ⚠️ |
| **@@** | **thread::spawn** | **Worker** | **P3** ⚠️ |
| **~** | **async** | **async** | **P1** ⚠️ |
| **.~** | **.await** | **await** | **P1** ⚠️ |
| **<<** | **channel** | **EventEmitter** | **P3** ⚠️ |

### Symbols (Memory)

| Nu | Rust | TS | Priority |
|----|------|----|----------|
| & | ref | (remove) | P0 |
| * | deref | (remove) | P0 |
| **!** (prefix) | **mut** | **(remove)** | **P1** ⚠️ |
| **U** | **unsafe** | **// @ts-ignore** | **P2** ⚠️ |

### Closures

| Nu | Rust | TS | Priority |
|----|------|----|----------|
| \|x\| expr | \|x\| expr | (x) => expr | P0 |
| \|x\| -> T { } | \|x\| -> T { } | (x): T => { } | P1 |
| **$\|x\| { }** | **move \|x\| { }** | **(x) => { }** | **P1** ⚠️ |

---

## Implementation Priority

### Phase 1 (P0) - MVP Critical
- [x] Basic keywords: S, E, F, f, l, v
- [x] Flow control: <, ?, M, L
- [x] Memory erasure: &, *
- [ ] **⚠️ br, ct (break/continue)**

### Phase 2 (P1) - Core Features
- [ ] **⚠️ Async: ~, .~**
- [ ] **⚠️ Move closures: $|x|**
- [ ] **⚠️ Type alias: t**
- [ ] **⚠️ As casting: a**
- [ ] **⚠️ Mut prefix: !**
- [ ] TR (trait), I (impl)

### Phase 3 (P2) - Advanced
- [ ] **⚠️ Spawn: @**
- [ ] **⚠️ Static: ST**
- [ ] **⚠️ Unsafe: U**
- [ ] Attributes: #D, #I

### Phase 4 (P3) - Future
- [ ] **⚠️ Thread spawn: @@**
- [ ] **⚠️ Channels: <<**
- [ ] EXT (extern)

---

## Quick Fix Checklist

### Immediate Additions Needed

1. **Control Flow** (P0)
   ```rust
   // Nu
   L x: list {
       ? x > 10 { br; }
       ? x < 0 { ct; }
   }
   ```
   ```typescript
   // TS
   for (const x of list) {
       if (x > 10) { break; }
       if (x < 0) { continue; }
   }
   ```

2. **Async/Await** (P1)
   ```rust
   // Nu
   ~F fetchData() -> Str {
       l data = fetch("url").~;
       < data
   }
   ```
   ```typescript
   // TS
   async function fetchData(): Promise<string> {
       const data = await fetch("url");
       return data;
   }
   ```

3. **Move Closures** (P1)
   ```rust
   // Nu
   l handler = $|x| { process(x) };
   ```
   ```typescript
   // TS
   const handler = (x) => { process(x) };  // ignore 'move'
   ```

4. **Type Alias** (P1)
   ```rust
   // Nu
   t UserId = i32;
   ```
   ```typescript
   // TS
   type UserId = number;
   ```

5. **Mut References** (P1)
   ```rust
   // Nu
   F update(&!self, val: i32) { ... }
   ```
   ```typescript
   // TS
   update(val: number) { ... }  // remove &!self
   ```
