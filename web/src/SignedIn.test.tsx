import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import SignedIn from "./SignedIn.tsx";

type Answer = [status: number, body?: unknown];

// Answers each "METHOD /path" request from the table, looked up when the
// request is made, so a test can change an answer on the way. A pending
// promise stands for a request the test holds. A request with no answer
// fails like a network error.
type Answers = Partial<Record<string, Answer | Promise<Response>>>;

function stubServer(answers: Answers) {
  const fetch = vi.fn((input: string, init?: RequestInit) => {
    const answer = answers[`${init?.method ?? "GET"} ${input}`];
    if (!answer) return Promise.reject(new TypeError("network error"));
    if (answer instanceof Promise) return answer;
    const [status, body] = answer;
    return Promise.resolve(
      body === undefined
        ? new Response(null, { status })
        : Response.json(body, { status }),
    );
  });
  vi.stubGlobal("fetch", fetch);
  return fetch;
}

const nebulaId = "0b6f3c2e-8d1a-4f7b-9c3e-2a5d6e7f8091";
const m31Id = "5e1d7a40-2b3c-4d5e-8f60-718293a4b5c6";

const entry = (id: string, name: string) => ({
  id,
  kind: "image",
  name,
  current_version_id: `${id.slice(0, 8)}-0000-4000-8000-000000000000`,
  original_url: `/api/assets/${id}/original`,
  preview_url: null,
});

const withFacts = (id: string, name: string, value: number) => ({
  asset: {
    ...entry(id, name),
    media_type: "image/png",
    facets: [
      { name: "image.file-size", label: "File size", value, unit: "bytes" },
    ],
  },
});

const list = (...assets: unknown[]): Answer => [200, { assets }];

const uploads = (fetch: ReturnType<typeof stubServer>) =>
  fetch.mock.calls.filter(
    ([url, init]) => url === "/api/assets" && init?.method === "POST",
  );

const choose = (name: string) => {
  fireEvent.change(picker(), {
    target: { files: [new File(["x"], name, { type: "image/png" })] },
  });
};

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

beforeEach(() => {
  history.replaceState(null, "", "/");
});

// A session signal that Access never aborts.
const session = new AbortController().signal;

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
      <SignedIn
        session={session}
        onSessionLost={vi.fn()}
        notice="Something went wrong"
      >
        <span>nova</span>
      </SignedIn>,
    );
    await screen.findByText("No images yet. Upload an image to start.");
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
      <SignedIn session={session} onSessionLost={onSessionLost} />,
    );
    await screen.findByRole("button", { name: "Upload" });
    fireEvent.change(picker(), {
      target: { files: [new File(["x"], "m31.png", { type: "image/png" })] },
    });
    await screen.findByText("Uploading m31.png…");
    const post = fetch.mock.calls.find(([, init]) => init?.method === "POST");
    const signal = post?.[1]?.signal;
    rerender(
      <SignedIn
        session={session}
        onSessionLost={onSessionLost}
        notice="Signed out"
      />,
    );
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

  describe("the screen switch", () => {
    it("shows an Asset's page at its address, with the account's items, and no Library", async () => {
      history.replaceState(null, "", `/assets/${nebulaId}`);
      const fetch = stubServer({
        [`GET /api/assets/${nebulaId}`]: [
          200,
          withFacts(nebulaId, "nebula.png", 1234567),
        ],
      });
      render(
        <SignedIn session={session} onSessionLost={vi.fn()}>
          <span>nova</span>
        </SignedIn>,
      );
      await screen.findByRole("heading", { name: "nebula.png" });
      screen.getByText("1,234,567 bytes");
      screen.getByText("nova");
      expect(screen.queryByRole("region", { name: "Library" })).toBeNull();
      expect(screen.queryByRole("button", { name: "Upload" })).toBeNull();
      expect(fetch).not.toHaveBeenCalledWith("/api/assets", expect.anything());
    });

    it("shows the account's notice on the asset page", async () => {
      history.replaceState(null, "", `/assets/${nebulaId}`);
      stubServer({
        [`GET /api/assets/${nebulaId}`]: [
          200,
          withFacts(nebulaId, "nebula.png", 1),
        ],
      });
      render(
        <SignedIn
          session={session}
          onSessionLost={vi.fn()}
          notice="Something went wrong"
        />,
      );
      await screen.findByRole("heading", { name: "nebula.png" });
      screen.getByText("Something went wrong");
    });

    it.each([["/elsewhere"], [`/assets/${nebulaId.toUpperCase()}`]])(
      "shows the Library at %s",
      async (path) => {
        history.replaceState(null, "", path);
        stubServer({ "GET /api/assets": list() });
        render(<SignedIn session={session} onSessionLost={vi.fn()} />);
        await screen.findByRole("region", { name: "Library" });
        expect(location.pathname).toBe(path);
      },
    );

    it.each([["image"], ["name"]])(
      "opens an Asset's page from its %s in the Library, and Back returns",
      async (part) => {
        stubServer({
          "GET /api/assets": list(entry(nebulaId, "nebula.png")),
          [`GET /api/assets/${nebulaId}`]: [
            200,
            withFacts(nebulaId, "nebula.png", 1234567),
          ],
        });
        render(<SignedIn session={session} onSessionLost={vi.fn()} />);
        const image = await screen.findByRole("img", { name: "nebula.png" });
        fireEvent.click(
          part === "image" ? image : screen.getByText("nebula.png"),
        );
        await screen.findByRole("heading", { name: "nebula.png" });
        expect(location.pathname).toBe(`/assets/${nebulaId}`);
        history.back();
        await screen.findByRole("region", { name: "Library" });
        expect(location.pathname).toBe("/");
      },
    );

    it("returns to the Library by the asset page's Library link", async () => {
      history.replaceState(null, "", `/assets/${nebulaId}`);
      stubServer({
        "GET /api/assets": list(entry(nebulaId, "nebula.png")),
        [`GET /api/assets/${nebulaId}`]: [
          200,
          withFacts(nebulaId, "nebula.png", 1),
        ],
      });
      render(<SignedIn session={session} onSessionLost={vi.fn()} />);
      await screen.findByRole("heading", { name: "nebula.png" });
      fireEvent.click(screen.getByRole("link", { name: "Library" }));
      await screen.findByRole("img", { name: "nebula.png" });
      expect(location.pathname).toBe("/");
    });

    it("shows only Loading after a history jump to another Asset whose answer is pending", async () => {
      const m31 = deferred();
      stubServer({
        [`GET /api/assets/${nebulaId}`]: [
          200,
          withFacts(nebulaId, "nebula.png", 1234567),
        ],
        [`GET /api/assets/${m31Id}`]: m31.promise,
      });
      history.replaceState(null, "", `/assets/${m31Id}`);
      history.pushState(null, "", `/assets/${nebulaId}`);
      render(<SignedIn session={session} onSessionLost={vi.fn()} />);
      await screen.findByRole("heading", { name: "nebula.png" });
      history.back();
      await waitFor(() => {
        expect(location.pathname).toBe(`/assets/${m31Id}`);
      });
      await screen.findByText("Loading the image…");
      expect(screen.queryByText("nebula.png")).toBeNull();
      expect(screen.queryByRole("img")).toBeNull();
      expect(screen.queryByText("1,234,567 bytes")).toBeNull();
      await act(async () => {
        m31.answer(Response.json(withFacts(m31Id, "m31.png", 42)));
        await m31.promise;
      });
      await screen.findByRole("heading", { name: "m31.png" });
    });
  });

  describe("an upload while the asset page shows", () => {
    async function uploadThenOpenNebula(post: Promise<Response>) {
      const answers: Answers = {
        "GET /api/assets": list(entry(nebulaId, "nebula.png")),
        "POST /api/assets": post,
        [`GET /api/assets/${nebulaId}`]: [
          200,
          withFacts(nebulaId, "nebula.png", 1234567),
        ],
      };
      const fetch = stubServer(answers);
      render(<SignedIn session={session} onSessionLost={vi.fn()} />);
      await screen.findByRole("img", { name: "nebula.png" });
      choose("m31.png");
      await screen.findByText("Uploading m31.png…");
      fireEvent.click(screen.getByRole("link", { name: "nebula.png" }));
      await screen.findByRole("heading", { name: "nebula.png" });
      return { answers, fetch };
    }

    it("goes on, shows no status on the page, and still blocks a second upload", async () => {
      const post = deferred();
      const { fetch } = await uploadThenOpenNebula(post.promise);
      expect(screen.queryByText(/Uploading/)).toBeNull();
      const signal = uploads(fetch)[0]?.[1]?.signal;
      expect(signal?.aborted).toBe(false);
      history.back();
      await screen.findByText("Uploading m31.png…");
      expect(
        screen.getByRole("button", { name: "Upload" }).hasAttribute("disabled"),
      ).toBe(true);
      choose("m42.png");
      expect(uploads(fetch)).toHaveLength(1);
      expect(signal?.aborted).toBe(false);
    });

    it("lists the new image when the upload ended while the asset page showed", async () => {
      const post = deferred();
      const { answers } = await uploadThenOpenNebula(post.promise);
      answers["GET /api/assets"] = list(
        entry(nebulaId, "nebula.png"),
        entry(m31Id, "m31.png"),
      );
      await act(async () => {
        post.answer(
          Response.json({ asset: entry(m31Id, "m31.png") }, { status: 201 }),
        );
        await post.promise;
      });
      history.back();
      await screen.findByRole("img", { name: "m31.png" });
      expect(screen.queryByText(/Uploading/)).toBeNull();
    });
  });
});
