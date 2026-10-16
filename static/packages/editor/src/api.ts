// Talking to the edit API of `hermetic serve`, and saving one page at a time.

import type { Payload } from "./model";

export const SOURCE_PATH = "_hermetic/source";
export const ASSET_PATH = "_hermetic/asset";

/** An image stored by the server: its file in the notes and the markdown that shows it. */
export interface Uploaded {
  file: string;
  markdown: string;
}

export type FetchFn = (input: string, init?: RequestInit) => Promise<Response>;

/** A save was refused because the file changed since `payload`'s version was loaded. */
export class Conflict extends Error {
  constructor(readonly payload: Payload) {
    super("the page changed elsewhere");
  }
}

/** The server answered, but not with success (not retried). */
export class Refused extends Error {
  constructor(readonly status: number) {
    super(`the server refused the request (${status})`);
  }
}

export class SourceApi {
  constructor(
    private readonly root: URL,
    private readonly fetchFn: FetchFn = (input, init) => fetch(input, init),
  ) {}

  private url(path: string): string {
    return `${new URL(SOURCE_PATH, this.root).href}?path=${encodeURIComponent(path)}`;
  }

  /** The source of the page at site path `path` (e.g. `pages/auth.html`). */
  async get(path: string): Promise<Payload> {
    const r = await this.fetchFn(this.url(path), { cache: "no-store" });
    if (!r.ok) throw new Refused(r.status);
    return (await r.json()) as Payload;
  }

  /** Moves the page's file to the trash, if it is still at `version`. */
  async delete(path: string, version: string): Promise<void> {
    const r = await this.fetchFn(this.url(path), {
      method: "DELETE",
      headers: { "X-Hermetic-Edit": "1", "X-Hermetic-Version": version },
    });
    if (r.status === 409) throw new Conflict((await r.json()) as Payload);
    if (!r.ok) throw new Refused(r.status);
  }

  /**
   * Stores an image for the page at site path `path` in the notes' `assets/` folder.
   * Refusals (e.g. 413 too large, 415 not an image) throw `Refused`; network errors throw
   * TypeError.
   */
  async upload(path: string, image: Blob, name = ""): Promise<Uploaded> {
    const url = `${new URL(ASSET_PATH, this.root).href}?path=${encodeURIComponent(path)}`;
    const headers: Record<string, string> = { "X-Hermetic-Edit": "1" };
    if (name) headers["X-Hermetic-Name"] = encodeURIComponent(name);
    const r = await this.fetchFn(url, { method: "POST", headers, body: image });
    if (!r.ok) throw new Refused(r.status);
    return (await r.json()) as Uploaded;
  }

  /** Saves `text` as the page's source, based on `version`. Network errors throw TypeError. */
  async put(path: string, version: string, text: string): Promise<Payload> {
    const r = await this.fetchFn(this.url(path), {
      method: "PUT",
      headers: {
        "Content-Type": "text/plain; charset=utf-8",
        "X-Hermetic-Edit": "1",
        "X-Hermetic-Version": version,
      },
      body: text,
    });
    if (r.status === 409) throw new Conflict((await r.json()) as Payload);
    if (!r.ok) throw new Refused(r.status);
    return (await r.json()) as Payload;
  }
}

export type SaveState = "saved" | "saving" | "unsaved";

export interface SaveHooks {
  /** The current text of the page, read when a save starts. */
  text(): string;
  /** A save succeeded; `payload` is the page as now stored. */
  saved(payload: Payload, text: string): void;
  /** A save was refused because the file changed elsewhere. */
  conflict(payload: Payload, text: string): void;
  /** The server refused the save for another reason (it is not retried). */
  failed(error: Refused, text: string): void;
  state(state: SaveState): void;
}

/** Waits `ms` milliseconds; replaceable in tests. */
export type Sleep = (ms: number) => Promise<void>;

const BACKOFF_MS = [1000, 2000, 4000, 8000, 10000];

/**
 * Saves of one page, one at a time. `request()` marks the page as changed; while a save
 * runs, further requests are folded into one more save of the latest text. Network
 * failures are retried with backoff (and at once on `retryNow()`); a conflict or a
 * refusal stops the queue until the next request.
 */
export class SaveQueue {
  private running = false;
  private pending = false;
  private wake: (() => void) | null = null;
  private current: SaveState = "saved";

  constructor(
    private readonly api: SourceApi,
    private readonly path: string,
    public version: string,
    private readonly hooks: SaveHooks,
    private readonly sleep: Sleep = (ms) => new Promise((r) => setTimeout(r, ms)),
  ) {}

  get state(): SaveState {
    return this.current;
  }

  /** Whether changes are not yet stored. */
  get dirty(): boolean {
    return this.pending || this.running;
  }

  request(): Promise<void> {
    this.pending = true;
    this.set("unsaved");
    return this.run();
  }

  /** Retries a waiting save immediately (e.g. when the server is back). */
  retryNow(): void {
    this.wake?.();
  }

  private set(state: SaveState): void {
    if (state !== this.current) {
      this.current = state;
      this.hooks.state(state);
    }
  }

  private async run(): Promise<void> {
    if (this.running) return;
    this.running = true;
    let attempt = 0;
    try {
      while (this.pending) {
        this.pending = false;
        const text = this.hooks.text();
        this.set("saving");
        try {
          const payload = await this.api.put(this.path, this.version, text);
          attempt = 0;
          this.version = payload.version;
          this.hooks.saved(payload, text);
        } catch (e) {
          if (e instanceof Conflict) {
            this.version = e.payload.version;
            this.pending = false;
            this.set("unsaved");
            this.hooks.conflict(e.payload, text);
            return;
          }
          if (e instanceof Refused) {
            this.pending = false;
            this.set("unsaved");
            this.hooks.failed(e, text);
            return;
          }
          // Network error: keep the change and try again later.
          this.pending = true;
          this.set("unsaved");
          const delay = BACKOFF_MS[Math.min(attempt++, BACKOFF_MS.length - 1)]!;
          await Promise.race([this.sleep(delay), new Promise<void>((r) => (this.wake = r))]);
          this.wake = null;
        }
      }
      this.set("saved");
    } finally {
      this.running = false;
    }
  }
}
