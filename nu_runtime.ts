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
