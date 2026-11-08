import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { eventsUrl, watchSession } from "./eventsSocket.ts";
import { stubWebSocket, type FakeWebSocket } from "./testing/fakeWebSocket.ts";

let sockets: FakeWebSocket[];
type Fetch = (input: string, init?: RequestInit) => Promise<Response>;
let fetch: ReturnType<typeof vi.fn<Fetch>>;
const onSignedOut = vi.fn();

beforeEach(() => {
  vi.useFakeTimers();
  sockets = stubWebSocket();
  fetch = vi.fn<Fetch>(() => Promise.reject(new TypeError("network error")));
  vi.stubGlobal("fetch", fetch);
  onSignedOut.mockReset();
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

const lastSocket = () => {
  const socket = sockets.at(-1);
  if (!socket) throw new Error("no socket was opened");
  return socket;
};

describe("eventsUrl", () => {
  it("uses ws: for a page served over http", () => {
    expect(eventsUrl("http://localhost:5173/")).toBe(
      "ws://localhost:5173/api/events",
    );
  });

  it("uses wss: for a page served over https", () => {
    expect(eventsUrl("https://photos.example/some/page?x=1")).toBe(
      "wss://photos.example/api/events",
    );
  });
});

describe("watchSession", () => {
  it("opens one socket to /api/events on the page's origin", () => {
    watchSession(onSignedOut);
    expect(sockets.map((socket) => socket.url)).toEqual([
      `ws://${location.host}/api/events`,
    ]);
  });

  it("calls onSignedOut on a 1008 close and opens nothing more", async () => {
    watchSession(onSignedOut);
    lastSocket().serverClose(1008);
    await vi.advanceTimersByTimeAsync(120_000);
    expect(onSignedOut).toHaveBeenCalledOnce();
    expect(sockets).toHaveLength(1);
    expect(fetch).not.toHaveBeenCalled();
  });

  it("closes the socket when stopped, and nothing fires after", async () => {
    const stop = watchSession(onSignedOut);
    const socket = lastSocket();
    stop();
    expect(socket.closed).toBe(true);
    socket.serverClose(1008);
    await vi.advanceTimersByTimeAsync(120_000);
    expect(onSignedOut).not.toHaveBeenCalled();
    expect(sockets).toHaveLength(1);
    expect(fetch).not.toHaveBeenCalled();
  });

  // Answer each GET /api/session in turn with these statuses. `undefined`
  // fails like a network error.
  function answerSession(...statuses: (number | undefined)[]) {
    fetch.mockImplementation(() => {
      const status = statuses.shift();
      return status === undefined
        ? Promise.reject(new TypeError("network error"))
        : Promise.resolve(new Response(null, { status }));
    });
  }

  it("asks GET /api/session 5 s after another close, and reopens on a 200", async () => {
    answerSession(200);
    watchSession(onSignedOut);
    lastSocket().serverClose(1006);
    await vi.advanceTimersByTimeAsync(4_999);
    expect(fetch).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(fetch).toHaveBeenCalledOnce();
    expect(fetch.mock.calls[0]?.[0]).toBe("/api/session");
    expect(sockets).toHaveLength(2);
    expect(lastSocket().url).toBe(`ws://${location.host}/api/events`);
    expect(onSignedOut).not.toHaveBeenCalled();
  });

  it("calls onSignedOut when the ask answers 401", async () => {
    answerSession(401);
    watchSession(onSignedOut);
    lastSocket().serverClose(1006);
    await vi.advanceTimersByTimeAsync(5_000);
    expect(onSignedOut).toHaveBeenCalledOnce();
    await vi.advanceTimersByTimeAsync(120_000);
    expect(sockets).toHaveLength(1);
    expect(fetch).toHaveBeenCalledOnce();
  });

  it("asks again 5 s after any other answer or a network error", async () => {
    answerSession(503, undefined, 200);
    watchSession(onSignedOut);
    lastSocket().serverClose(1011);
    await vi.advanceTimersByTimeAsync(5_000);
    expect(fetch).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(5_000);
    expect(fetch).toHaveBeenCalledTimes(2);
    expect(sockets).toHaveLength(1);
    await vi.advanceTimersByTimeAsync(5_000);
    expect(fetch).toHaveBeenCalledTimes(3);
    expect(sockets).toHaveLength(2);
    expect(onSignedOut).not.toHaveBeenCalled();
  });

  it("closes a socket silent for 60 s, then asks 5 s later", async () => {
    answerSession(200);
    watchSession(onSignedOut);
    const silent = lastSocket();
    await vi.advanceTimersByTimeAsync(59_999);
    expect(silent.closed).toBe(false);
    await vi.advanceTimersByTimeAsync(1);
    expect(silent.closed).toBe(true);
    await vi.advanceTimersByTimeAsync(4_999);
    expect(fetch).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(fetch).toHaveBeenCalledOnce();
    expect(sockets).toHaveLength(2);
  });

  it("keeps a socket that gets a heartbeat every 30 s", async () => {
    watchSession(onSignedOut);
    const socket = lastSocket();
    for (let beat = 0; beat < 6; beat++) {
      await vi.advanceTimersByTimeAsync(30_000);
      socket.receive('{"type":"heartbeat"}');
    }
    expect(socket.closed).toBe(false);
    expect(sockets).toHaveLength(1);
    expect(fetch).not.toHaveBeenCalled();
  });

  it("aborts an ask in flight when stopped, and opens nothing after", async () => {
    let answer: ((response: Response) => void) | undefined;
    let signal: AbortSignal | undefined;
    fetch.mockImplementation((_: string, init?: RequestInit) => {
      signal = init?.signal ?? undefined;
      return new Promise<Response>((resolve) => {
        answer = resolve;
      });
    });
    const stop = watchSession(onSignedOut);
    lastSocket().serverClose(1006);
    await vi.advanceTimersByTimeAsync(5_000);
    expect(fetch).toHaveBeenCalledOnce();
    stop();
    expect(signal?.aborted).toBe(true);
    answer?.(new Response(null, { status: 200 }));
    await vi.advanceTimersByTimeAsync(120_000);
    expect(sockets).toHaveLength(1);
    expect(onSignedOut).not.toHaveBeenCalled();
  });

  it("clears the wait to ask when stopped", async () => {
    const stop = watchSession(onSignedOut);
    lastSocket().serverClose(1006);
    stop();
    await vi.advanceTimersByTimeAsync(120_000);
    expect(fetch).not.toHaveBeenCalled();
    expect(sockets).toHaveLength(1);
  });
});
