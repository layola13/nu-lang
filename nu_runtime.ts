// ==================== Nu Runtime Library ====================
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
