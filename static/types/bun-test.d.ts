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
    toHaveLength(length: number): void;
    not: Matchers;
  }
  export function expect(actual: unknown): Matchers;
}

// Reading fixture files in tests (Bun implements node:fs).
declare module "node:fs" {
  export function readFileSync(path: string | URL, encoding: "utf8"): string;
  export function readdirSync(path: string | URL): string[];
}
