// Minimal types for the parts of `bun:test` the unit tests use, so type-checking
// needs no extra dependency (such as @types/bun).
declare module "bun:test" {
  export function test(name: string, fn: () => void | Promise<void>): void;
  export function describe(name: string, fn: () => void): void;
  interface Matchers {
    toBe(expected: unknown): void;
    toEqual(expected: unknown): void;
    toBeNull(): void;
    toBeTrue(): void;
    toBeFalse(): void;
    not: Matchers;
  }
  export function expect(actual: unknown): Matchers;
}
