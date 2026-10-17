import { describe, expect, test } from "bun:test";

import { type FetchFn, MaintenanceApi, Refused } from "./api";

const ROOT = new URL("http://127.0.0.1:8000/");

interface Call {
  url: string;
  init?: RequestInit | undefined;
}

function fakeFetch(answer: Response): { fetch: FetchFn; calls: Call[] } {
  const calls: Call[] = [];
  return { calls, fetch: async (url, init) => (calls.push({ url, init }), answer) };
}

const json = (status: number, body: unknown) => new Response(JSON.stringify(body), { status });

describe("MaintenanceApi", () => {
  test("list", async () => {
    const f = fakeFetch(json(200, { assets: [{ path: "assets/a.png", size: 2048 }] }));
    const assets = await new MaintenanceApi(ROOT, f.fetch).list();
    expect(assets).toEqual([{ path: "assets/a.png", size: 2048 }]);
    expect(f.calls[0]!.url).toBe("http://127.0.0.1:8000/_hermetic/unused-assets");
    expect(f.calls[0]!.init?.method ?? "GET").toBe("GET");
  });

  test("trash posts the paths with the edit header", async () => {
    const result = { trashed: ["assets/a.png"], trash: ".trash/2026-10-17T10-00-00Z", skipped: [{ path: "x", reason: "not an unused asset" }] };
    const f = fakeFetch(json(200, result));
    expect(await new MaintenanceApi(ROOT, f.fetch).trash(["assets/a.png", "x"])).toEqual(result);
    const init = f.calls[0]!.init!;
    expect(init.method).toBe("POST");
    expect((init.headers as Record<string, string>)["X-Hermetic-Edit"]).toBe("1");
    expect(JSON.parse(init.body as string)).toEqual({ paths: ["assets/a.png", "x"] });
  });

  test("refusals throw", async () => {
    const f = fakeFetch(new Response("", { status: 403 }));
    const error = await new MaintenanceApi(ROOT, f.fetch).trash([]).then(
      () => null,
      (e: unknown) => e,
    );
    expect(error instanceof Refused && error.status === 403).toBe(true);
  });
});
