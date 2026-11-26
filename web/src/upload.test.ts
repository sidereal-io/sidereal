import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useUpload } from "./upload.ts";

// A request that answers only when the test says so.
function deferred() {
  let answer: (response: Response) => void = () => undefined;
  const promise = new Promise<Response>((resolve) => {
    answer = resolve;
  });
  return { promise, answer };
}

const png = (name = "m31.png") =>
  new File([new Uint8Array([0x89, 0x50, 0x4e, 0x47])], name, {
    type: "image/png",
  });

// Each POST waits for its own deferred answer, in the order sent.
function stubUploads(...answers: ReturnType<typeof deferred>[]) {
  let turn = 0;
  const fetch = vi.fn<(input: string, init?: RequestInit) => Promise<Response>>(
    () => {
      const answer = turn < answers.length ? answers[turn] : null;
      turn += 1;
      return answer
        ? answer.promise
        : Promise.reject(new TypeError("network error"));
    },
  );
  vi.stubGlobal("fetch", fetch);
  return fetch;
}

// A session signal that Access never aborts.
const live = () => new AbortController().signal;

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("useUpload", () => {
  it("starts with no lines, not busy, with nothing saved", () => {
    const { result } = renderHook(() => useUpload(vi.fn(), live()));
    expect(result.current.lines).toEqual([]);
    expect(result.current.busy).toBe(false);
    expect(result.current.saved).toBe(0);
  });

  it("sends one file, ignores a second while it is on its way, and counts the save", async () => {
    const pending = deferred();
    const fetch = stubUploads(pending);
    const { result } = renderHook(() => useUpload(vi.fn(), live()));
    act(() => {
      result.current.send([png()]);
    });
    expect(result.current.lines).toEqual([
      { name: "m31.png", kind: "sending" },
    ]);
    expect(result.current.busy).toBe(true);
    act(() => {
      result.current.send([png("m42.png")]);
    });
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(fetch).toHaveBeenCalledWith(
      "/api/assets",
      expect.objectContaining({ method: "POST" }),
    );
    await act(async () => {
      pending.answer(new Response(null, { status: 201 }));
      await pending.promise;
    });
    await waitFor(() => {
      expect(result.current.lines).toEqual([
        { name: "m31.png", kind: "saved" },
      ]);
    });
    expect(result.current.saved).toBe(1);
  });

  it("counts an upload with no answer as maybe saved", async () => {
    stubUploads();
    const { result } = renderHook(() => useUpload(vi.fn(), live()));
    act(() => {
      result.current.send([png()]);
    });
    await waitFor(() => {
      expect(result.current.lines).toEqual([
        { name: "m31.png", kind: "unconfirmed" },
      ]);
    });
    expect(result.current.saved).toBe(1);
  });

  it("does not count a refused upload", async () => {
    const refused = deferred();
    stubUploads(refused);
    const { result } = renderHook(() => useUpload(vi.fn(), live()));
    act(() => {
      result.current.send([png()]);
    });
    await act(async () => {
      refused.answer(new Response(null, { status: 415 }));
      await refused.promise;
    });
    await waitFor(() => {
      expect(result.current.lines).toEqual([
        {
          name: "m31.png",
          kind: "refused",
          reason: "its format is not supported.",
        },
      ]);
    });
    expect(result.current.saved).toBe(0);
  });

  it("gives a 415's reason from the server's answer", async () => {
    const refused = deferred();
    stubUploads(refused);
    const { result } = renderHook(() => useUpload(vi.fn(), live()));
    act(() => {
      result.current.send([png()]);
    });
    await act(async () => {
      refused.answer(
        Response.json(
          {
            error:
              "unsupported format: only PNG, JPEG and TIFF images are supported",
            reason: "only PNG, JPEG and TIFF images are supported",
          },
          { status: 415 },
        ),
      );
      await refused.promise;
    });
    await waitFor(() => {
      expect(result.current.lines).toEqual([
        {
          name: "m31.png",
          kind: "refused",
          reason: "only PNG, JPEG and TIFF images are supported.",
        },
      ]);
    });
  });

  it("does not count a repeat upload, and keeps the found Asset's name", async () => {
    const found = deferred();
    stubUploads(found);
    const { result } = renderHook(() => useUpload(vi.fn(), live()));
    act(() => {
      result.current.send([png()]);
    });
    await act(async () => {
      found.answer(
        Response.json(
          {
            asset: {
              id: "a1",
              kind: "image",
              name: "nebula.png",
              current_version_id: "v1",
              original_url: "/api/assets/a1/original",
              preview_url: null,
            },
          },
          { status: 409 },
        ),
      );
      await found.promise;
    });
    await waitFor(() => {
      expect(result.current.lines).toEqual([
        { name: "m31.png", kind: "found", existing: "nebula.png" },
      ]);
    });
    expect(result.current.saved).toBe(0);
  });

  it("reports a lost session when the upload answers 401", async () => {
    const ended = deferred();
    stubUploads(ended);
    const onSessionLost = vi.fn();
    const { result } = renderHook(() => useUpload(onSessionLost, live()));
    act(() => {
      result.current.send([png()]);
    });
    await act(async () => {
      ended.answer(new Response(null, { status: 401 }));
      await ended.promise;
    });
    await waitFor(() => {
      expect(onSessionLost).toHaveBeenCalledTimes(1);
    });
  });

  it("keeps its request through a re-render, and aborts it only on unmount", async () => {
    const pending = deferred();
    const fetch = stubUploads(pending);
    const onSessionLost = vi.fn();
    const session = live();
    const { result, rerender, unmount } = renderHook(
      (callback: () => void) => useUpload(callback, session),
      { initialProps: onSessionLost },
    );
    act(() => {
      result.current.send([png()]);
    });
    const signal = (fetch.mock.calls[0]?.[1] as RequestInit).signal;
    rerender(vi.fn());
    expect(signal?.aborted).toBe(false);
    unmount();
    expect(signal?.aborted).toBe(true);
    await act(async () => {
      pending.answer(new Response(null, { status: 401 }));
      await pending.promise;
    });
    expect(onSessionLost).not.toHaveBeenCalled();
  });
});
