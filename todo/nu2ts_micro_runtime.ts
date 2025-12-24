// ==================== Nu2TS Micro-Runtime ====================
// Auto-generated polyfill for Nu v1.6 -> TypeScript compilation
// Version: 1.6.2
// Size: ~40 lines, <1KB minified
// Strategy: Type Erasure + Minimal Runtime Helpers
//
// This micro-runtime addresses critical issues identified in zero-runtime approach:
// 1. Result type safety with discriminated unions
// 2. Proper ? operator desugaring with early return
// 3. Format string support for format!() and println!()
// 4. Pattern matching utilities
// ================================================================

// -------------------- Result Type System --------------------

/**
 * Result type - Discriminated union for success/failure
 * Replaces Rust's Result<T, E>
 */
export type Result<T, E> =
    | { tag: 'ok'; val: T }
    | { tag: 'err'; err: E };

/**
 * Ok constructor - Creates successful Result
 */
export const Ok = <T>(val: T): Result<T, any> => ({
    tag: 'ok',
    val
});

/**
 * Err constructor - Creates error Result
 */
export const Err = <E>(err: E): Result<any, E> => ({
    tag: 'err',
    err
});

/**
 * $unwrap - Unwraps Result or throws error
 * 
 * CRITICAL: Prevents incorrect Option-style handling of Result
 * 
 * Usage in generated code:
 *   const val = result.unwrap()  // Nu
 *   const val = $unwrap(result)  // TS
 * 
 * @throws Error if Result is Err variant
 */
export function $unwrap<T, E>(r: Result<T, E>): T {
    if (r.tag === 'err') {
        throw new Error(`Unwrap failed: ${r.err}`);
    }
    return r.val;
}

/**
 * Type guard for Ok variant
 */
export function isOk<T, E>(r: Result<T, E>): r is { tag: 'ok'; val: T } {
    return r.tag === 'ok';
}

/**
 * Type guard for Err variant
 */
export function isErr<T, E>(r: Result<T, E>): r is { tag: 'err'; err: E } {
    return r.tag === 'err';
}

// -------------------- Format String Support --------------------

/**
 * $fmt - Format string helper for format!() and println!()
 * 
 * Replaces Rust's format string syntax with TypeScript template strings
 * 
 * Usage in generated code:
 *   format!("Value: {}", x)      // Nu
 *   $fmt("Value: {}", x)         // TS
 * 
 * Limitations:
 * - Only supports {} placeholders (no position, width, precision)
 * - Uses String() for conversion (not Display/Debug traits)
 */
export function $fmt(template: string, ...args: any[]): string {
    let i = 0;
    return template.replace(/{}/g, () => {
        return i < args.length ? String(args[i++]) : '{}';
    });
}

// -------------------- Option Helpers --------------------

/**
 * Type guard for Some (non-null) values
 */
export function isSome<T>(opt: T | null): opt is T {
    return opt !== null;
}

/**
 * Type guard for None (null) values
 */
export function isNone<T>(opt: T | null): opt is null {
    return opt === null;
}

// -------------------- Pattern Matching Utilities --------------------

/**
 * $match - Runtime helper for complex pattern matching
 * 
 * Used when match expression is too complex for switch/if-chain
 * 
 * Usage:
 *   $match(value, [
 *     [v => v.tag === 'ok', v => v.val],
 *     [v => v.tag === 'err', v => { throw v.err }]
 *   ])
 * 
 * NOTE: Rarely needed - most matches compile to if-chains
 */
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

// ==================== End Micro-Runtime ====================

// Usage Examples:
//
// 1. Result handling with ? operator:
//
//   // Nu:
//   F compute() -> Result<i32, String> {
//       l val = dangerous()?;
//       < Ok(val * 2)
//   }
//
//   // Generated TS (with AST desugaring):
//   export function compute(): Result<number, string> {
//       const _tmp0 = dangerous();
//       if (_tmp0.tag === 'err') return _tmp0;  // Early return
//       const val = _tmp0.val;
//       return Ok(val * 2);
//   }
//
// 2. Format strings:
//
//   // Nu:
//   println!("Result: {}, Error: {}", x, e);
//
//   // Generated TS:
//   console.log($fmt("Result: {}, Error: {}", x, e));
//
// 3. Pattern matching on Result:
//
//   // Nu:
//   M result {
//       Ok(val): { println!("Success: {}", val); },
//       Err(e): { println!("Error: {}", e); }
//   }
//
//   // Generated TS:
//   if (result.tag === 'ok') {
//       const val = result.val;
//       console.log($fmt("Success: {}", val));
//   } else if (result.tag === 'err') {
//       const e = result.err;
//       console.log($fmt("Error: {}", e));
//   }
