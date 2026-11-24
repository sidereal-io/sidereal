import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
} from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import SignedIn from "./SignedIn.tsx";

// A request that answers only when the test says so.
function deferred() {
  let answer: (response: Response) => void = () => undefined;
  const promise = new Promise<Response>((resolve) => {
    answer = resolve;
  });
  return { promise, answer };
}

const picker = () => {
  const input = document.querySelector<HTMLInputElement>('input[type="file"]');
  if (!input) throw new Error("no file input");
  return input;
};

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("SignedIn", () => {
  it("shows the Library with the account's items and the notice", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(() => Promise.resolve(Response.json({ assets: [] }))),
    );
    render(
      <SignedIn onSessionLost={vi.fn()} notice="Something went wrong">
        <span>nova</span>
      </SignedIn>,
    );
    await screen.findByText("No images yet. Upload a PNG or JPEG to start.");
    screen.getByRole("region", { name: "Library" });
    screen.getByRole("button", { name: "Upload" });
    screen.getByText("nova");
    screen.getByText("Something went wrong");
  });

  it("keeps an upload through a re-render, and aborts it when it unmounts", async () => {
    const pending = deferred();
    const fetch = vi.fn((_input: string, init?: RequestInit) =>
      init?.method === "POST"
        ? pending.promise
        : Promise.resolve(Response.json({ assets: [] })),
    );
    vi.stubGlobal("fetch", fetch);
    const onSessionLost = vi.fn();
    const { rerender, unmount } = render(
      <SignedIn onSessionLost={onSessionLost} />,
    );
    await screen.findByRole("button", { name: "Upload" });
    fireEvent.change(picker(), {
      target: { files: [new File(["x"], "m31.png", { type: "image/png" })] },
    });
    await screen.findByText("Uploading m31.png…");
    const post = fetch.mock.calls.find(([, init]) => init?.method === "POST");
    const signal = post?.[1]?.signal;
    rerender(<SignedIn onSessionLost={onSessionLost} notice="Signed out" />);
    expect(signal?.aborted).toBe(false);
    screen.getByText("Uploading m31.png…");
    unmount();
    expect(signal?.aborted).toBe(true);
    await act(async () => {
      pending.answer(new Response(null, { status: 401 }));
      await pending.promise;
    });
    expect(onSessionLost).not.toHaveBeenCalled();
  });
});
