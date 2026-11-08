import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { eventsUrl, watchSession } from "./eventsSocket.ts";
import { stubWebSocket, type FakeWebSocket } from "./testing/fakeWebSocket.ts";

let sockets: FakeWebSocket[];
let fetch: ReturnType<typeof vi.fn>;
const onSignedOut = vi.fn();

beforeEach(() => {
  vi.useFakeTimers();
  sockets = stubWebSocket();
  fetch = vi.fn(() => Promise.reject(new TypeError("network error")));
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
});
