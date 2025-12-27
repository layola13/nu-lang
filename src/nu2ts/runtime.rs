// TypeScript微运行时生成器

pub fn generate_micro_runtime() -> &'static str {
    r#"// ==================== Nu2TS Micro-Runtime ====================
// Auto-generated polyfill for Nu v1.6 -> TypeScript compilation
// Size: ~40 lines, <1KB minified

// Result Type
export type Result<T, E> = 
  | { tag: 'ok'; val: T }
  | { tag: 'err'; err: E };

// Option Type
export type Option<T> = T | null;
export type O<T> = Option<T>; // Alias for O<T>

// Result Constructors
export const Ok = <T>(val: T): Result<T, any> => ({ tag: 'ok', val });
export const Err = <E>(err: E): Result<any, E> => ({ tag: 'err', err });

// Unwrap Helper (throws on error)
export function $unwrap<T, E>(r: Result<T, E>): T {
  if (r.tag === 'err') {
    throw new Error(`Unwrap failed: ${r.err}`);
  }
  return r.val;
}

// Format String Helper (simple implementation)
export function $fmt(template: string, ...args: any[]): string {
  let i = 0;
  return template.replace(/{}/g, () => {
    return i < args.length ? String(args[i++]) : '{}';
  });
}

// Result Helpers
export function isOk<T, E>(r: Result<T, E>): boolean {
  return r.tag === 'ok';
}

export function isErr<T, E>(r: Result<T, E>): boolean {
  return r.tag === 'err';
}

export function $expect<T, E>(r: Result<T, E>, msg: string): T {
  if (r.tag === 'err') {
    throw new Error(`${msg}: ${r.err}`);
  }
  return r.val;
}

export function $unwrapOr<T, E>(r: Result<T, E>, defaultValue: T): T {
  return r.tag === 'ok' ? r.val : defaultValue;
}

export function $unwrapOrElse<T, E>(r: Result<T, E>, fn: (e: E) => T): T {
  return r.tag === 'ok' ? r.val : fn(r.err);
}

// Option Helpers
export function isSome<T>(opt: T | null): opt is T {
  return opt !== null;
}

export function isNone<T>(opt: T | null): opt is null {
  return opt === null;
}

// Match Helper for complex patterns
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
// ============================================================

"#
}

pub fn generate_runtime_import() -> &'static str {
    r#"import { Result, Ok, Err, $unwrap, $expect, $unwrapOr, $unwrapOrElse, isOk, isErr, isSome, isNone, $fmt, $match } from './nu_runtime';

"#
}

/// 生成独立的 nu_runtime.ts 文件内容（用于 Import 模式）
pub fn generate_runtime_file_content() -> &'static str {
    r#"// ==================== Nu Runtime Library ====================
// Nu v1.6 TypeScript Runtime
// Version: 1.0.0

// Result Type
export type Result<T, E> = 
  | { tag: 'ok'; val: T }
  | { tag: 'err'; err: E };

// Result Constructors
export const Ok = <T>(val: T): Result<T, any> => ({ tag: 'ok', val });
export const Err = <E>(err: E): Result<any, E> => ({ tag: 'err', err });

// Unwrap Helper (throws on error)
export function $unwrap<T, E>(r: Result<T, E>): T {
  if (r.tag === 'err') {
    throw new Error(`Unwrap failed: ${r.err}`);
  }
  return r.val;
}

// Format String Helper
export function $fmt(template: string, ...args: any[]): string {
  let i = 0;
  return template.replace(/{}/g, () => {
    return i < args.length ? String(args[i++]) : '{}';
  });
}

// Result Helpers
export function isOk<T, E>(r: Result<T, E>): boolean {
  return r.tag === 'ok';
}

export function isErr<T, E>(r: Result<T, E>): boolean {
  return r.tag === 'err';
}

export function $expect<T, E>(r: Result<T, E>, msg: string): T {
  if (r.tag === 'err') {
    throw new Error(`${msg}: ${r.err}`);
  }
  return r.val;
}

export function $unwrapOr<T, E>(r: Result<T, E>, defaultValue: T): T {
  return r.tag === 'ok' ? r.val : defaultValue;
}

export function $unwrapOrElse<T, E>(r: Result<T, E>, fn: (e: E) => T): T {
  return r.tag === 'ok' ? r.val : fn(r.err);
}

// Option Helpers
export function isSome<T>(opt: T | null): opt is T {
  return opt !== null;
}

export function isNone<T>(opt: T | null): opt is null {
  return opt === null;
}

// Match Helper for complex patterns
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
"#
}
