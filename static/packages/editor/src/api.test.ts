import { describe, expect, test } from "bun:test";

import { Conflict, type FetchFn, Refused, SaveQueue, type SaveState, SourceApi } from "./api";
import type { Payload } from "./model";

const ROOT = new URL("http://127.0.0.1:8000/");

function payload(version: string, text = ""): Payload {
  return { exists: true, file: "pages/a.md", version, text, indent: "  ", blocks: [] };
}

interface Call {
  url: string;
  init?: RequestInit | undefined;
}

/** A fake server: answers PUTs from `answer`, recording every call. */
function fakeFetch(answer: (call: Call, n: number) => Response | "offline"): { fetch: FetchFn; calls: Call[] } {
  const calls: Call[] = [];
  return {
    calls,
    fetch: async (url, init) => {
      const call = { url, init };
      calls.push(call);
      const r = answer(call, calls.length);
      if (r === "offline") throw new TypeError("Failed to fetch");
      return r;
    },
  };
}

const json = (status: number, body: unknown) =>
  new Response(JSON.stringify(body), { status, headers: { "Content-Type": "application/json" } });

describe("SourceApi", () => {
  test("requests carry the path, the edit header, and the version", async () => {
    const f = fakeFetch(() => json(200, payload("v2")));
    const api = new SourceApi(ROOT, f.fetch);
    await api.get("pages/Project X/A b.html");
    expect(f.calls[0]!.url).toBe("http://127.0.0.1:8000/_hermetic/source?path=pages%2FProject%20X%2FA%20b.html");
    await api.put("pages/a.html", "v1", "- a\n");
    const init = f.calls[1]!.init!;
    expect(init.method).toBe("PUT");
    expect((init.headers as Record<string, string>)["X-Hermetic-Edit"]).toBe("1");
    expect((init.headers as Record<string, string>)["X-Hermetic-Version"]).toBe("v1");
    expect(init.body).toBe("- a\n");
  });

  test("uploads post the image with its name for the page", async () => {
    const f = fakeFetch(() => json(201, { file: "assets/a_1_0.png", markdown: "![a.png](../assets/a_1_0.png)" }));
    const api = new SourceApi(ROOT, f.fetch);
    const image = new Blob(["png"], { type: "image/png" });
    const r = await api.upload("pages/Project X/A.html", image, "Screen shot é.png");
    expect(r.markdown).toBe("![a.png](../assets/a_1_0.png)");
    expect(f.calls[0]!.url).toBe("http://127.0.0.1:8000/_hermetic/asset?path=pages%2FProject%20X%2FA.html");
    const init = f.calls[0]!.init!;
    expect(init.method).toBe("POST");
    expect(init.body).toBe(image);
    const headers = init.headers as Record<string, string>;
    expect(headers["X-Hermetic-Edit"]).toBe("1");
    expect(headers["X-Hermetic-Name"]).toBe("Screen%20shot%20%C3%A9.png");
    await api.upload("index.html", image);
    expect("X-Hermetic-Name" in (f.calls[1]!.init!.headers as Record<string, string>)).toBeFalse();
  });

  test("refused uploads carry their status; network errors are TypeErrors", async () => {
    for (const status of [413, 415, 404]) {
      const api = new SourceApi(ROOT, fakeFetch(() => new Response("", { status })).fetch);
      let caught: unknown;
      try {
        await api.upload("index.html", new Blob(["x"]));
      } catch (e) {
        caught = e;
      }
      expect(caught instanceof Refused).toBeTrue();
      expect((caught as Refused).status).toBe(status);
    }
    const offline = new SourceApi(ROOT, fakeFetch(() => "offline").fetch);
    let caught: unknown;
    try {
      await offline.upload("index.html", new Blob(["x"]));
    } catch (e) {
      caught = e;
    }
    expect(caught instanceof TypeError).toBeTrue();
  });

  test("409 is a conflict with the current page", async () => {
    const api = new SourceApi(ROOT, fakeFetch(() => json(409, payload("v9", "- theirs\n"))).fetch);
    let caught: unknown;
    try {
      await api.put("pages/a.html", "v1", "- mine\n");
    } catch (e) {
      caught = e;
    }
    expect(caught instanceof Conflict).toBeTrue();
    expect((caught as Conflict).payload.text).toBe("- theirs\n");
  });
});

function queue(f: FetchFn, texts: string[]) {
  const states: SaveState[] = [];
  const saved: string[] = [];
  const conflicts: string[] = [];
  let i = 0;
  const sleeps: number[] = [];
  const q = new SaveQueue(
    new SourceApi(ROOT, f),
    "pages/a.html",
    "v1",
    {
      text: () => texts[Math.min(i++, texts.length - 1)]!,
      saved: (_p, t) => saved.push(t),
      conflict: (_p, t) => conflicts.push(t),
      failed: () => {},
      state: (s) => states.push(s),
    },
    async (ms) => {
      sleeps.push(ms);
    },
  );
  return { q, states, saved, conflicts, sleeps };
}

describe("SaveQueue", () => {
  test("one save at a time; requests during a save become one more save", async () => {
    let release: () => void = () => {};
    const f = fakeFetch((_c, n) => json(200, payload(`v${n + 1}`)));
    const slow: FetchFn = async (url, init) => {
      if (f.calls.length === 0) await new Promise<void>((r) => (release = r));
      return f.fetch(url, init);
    };
    const { q, saved, states } = queue(slow, ["one", "two", "three"]);
    const first = q.request();
    q.request();
    q.request();
    release();
    await first;
    expect(saved).toEqual(["one", "two"]);
    expect(q.version).toBe("v3");
    expect(states.at(-1)).toBe("saved");
    expect(q.dirty).toBeFalse();
  });

  test("versions advance with each save", async () => {
    const f = fakeFetch((_c, n) => json(200, payload(`v${n + 1}`)));
    const { q } = queue(f.fetch, ["a", "b"]);
    await q.request();
    await q.request();
    const versions = f.calls.map((c) => (c.init!.headers as Record<string, string>)["X-Hermetic-Version"]);
    expect(versions).toEqual(["v1", "v2"]);
  });

  test("server stopped: the change is kept, retried with backoff, then saved (Server stopped)", async () => {
    const f = fakeFetch((_c, n) => (n <= 3 ? "offline" : json(200, payload("v2"))));
    const { q, saved, states, sleeps } = queue(f.fetch, ["typed while offline"]);
    await q.request();
    expect(sleeps).toEqual([1000, 2000, 4000]);
    expect(saved).toEqual(["typed while offline"]);
    expect(states).toEqual(["unsaved", "saving", "unsaved", "saving", "unsaved", "saving", "unsaved", "saving", "saved"]);
  });

  test("a conflict stops the queue and hands over the refused text", async () => {
    const f = fakeFetch(() => json(409, payload("v7", "- theirs\n")));
    const { q, conflicts, saved, states } = queue(f.fetch, ["- mine\n"]);
    await q.request();
    expect(conflicts).toEqual(["- mine\n"]);
    expect(saved).toEqual([]);
    expect(q.version).toBe("v7");
    expect(states.at(-1)).toBe("unsaved");
    expect(f.calls).toHaveLength(1);
  });
});
