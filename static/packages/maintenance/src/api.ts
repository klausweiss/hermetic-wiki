// The maintenance API of `hermetic serve`: the unused assets, and moving them to the trash.

export const UNUSED_ASSETS_PATH = "_hermetic/unused-assets";

export interface Asset {
  /** Relative to the notes folder, `/`-separated. */
  path: string;
  size: number;
}

export interface TrashResult {
  trashed: string[];
  /** The trash folder, relative to the notes folder. */
  trash: string;
  skipped: { path: string; reason: string }[];
}

export type FetchFn = (input: string, init?: RequestInit) => Promise<Response>;

/** The server answered, but not with success. */
export class Refused extends Error {
  constructor(readonly status: number) {
    super(`the server refused the request (${status})`);
  }
}

export class MaintenanceApi {
  constructor(
    private readonly root: URL,
    private readonly fetchFn: FetchFn = (input, init) => fetch(input, init),
  ) {}

  private get url(): string {
    return new URL(UNUSED_ASSETS_PATH, this.root).href;
  }

  /** The unused assets, ordered by path. */
  async list(): Promise<Asset[]> {
    const r = await this.fetchFn(this.url, { cache: "no-store" });
    if (!r.ok) throw new Refused(r.status);
    return ((await r.json()) as { assets: Asset[] }).assets;
  }

  /** Moves the unused assets among `paths` to the trash; the others are skipped. */
  async trash(paths: string[]): Promise<TrashResult> {
    const r = await this.fetchFn(this.url, {
      method: "POST",
      headers: { "X-Hermetic-Edit": "1", "Content-Type": "application/json" },
      body: JSON.stringify({ paths }),
    });
    if (!r.ok) throw new Refused(r.status);
    return (await r.json()) as TrashResult;
  }
}
