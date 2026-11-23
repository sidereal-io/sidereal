import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import Library from "./Library.tsx";

type Answer = [status: number, body?: unknown];

// One answer, or a list of answers given in turn (the last one repeats).
// A request with no answer fails like a network error.
type Answers = Partial<Record<string, Answer | Answer[]>>;

function stubServer(answers: Answers) {
  const asked = new Map<string, number>();
  const fetch = vi.fn((input: string, init?: RequestInit) => {
    const key = `${init?.method ?? "GET"} ${input}`;
    const entry = answers[key];
    if (!entry) return Promise.reject(new TypeError("network error"));
    const turn = asked.get(key) ?? 0;
    asked.set(key, turn + 1);
    const list = (
      Array.isArray(entry[0]) ? entry : [entry]
    ) as readonly Answer[];
    const [status, body] = list[Math.min(turn, list.length - 1)];
    return Promise.resolve(
      body === undefined
        ? new Response(null, { status })
        : Response.json(body, { status }),
    );
  });
  vi.stubGlobal("fetch", fetch);
  return fetch;
}

// A request that answers only when the test says so.
function deferred() {
  let answer: (response: Response) => void = () => undefined;
  const promise = new Promise<Response>((resolve) => {
    answer = resolve;
  });
  return { promise, answer };
}

const entry = (id: string, name: string) => ({
  id,
  kind: "image",
  name,
  current_version_id: `${id}-v`,
  original_url: `/api/assets/${id}/original`,
});

const nebula = entry("a1", "nebula.png");
const andromeda = entry("a2", "m31.jpg");

const listAnswer = (...assets: unknown[]): Answer => [200, { assets }];

const button = (name: string) => screen.queryByRole("button", { name });

function showLibrary(onSessionLost = vi.fn()) {
  render(<Library onSessionLost={onSessionLost} />);
  return onSessionLost;
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("Library", () => {
  it("says it is loading before the list answers", async () => {
    const fetch = vi.fn(() => new Promise<Response>(() => undefined));
    vi.stubGlobal("fetch", fetch);
    showLibrary();
    await screen.findByText("Loading the library…");
    expect(fetch).toHaveBeenCalledWith("/api/assets", expect.anything());
  });

  it("shows the empty-library message for an empty list", async () => {
    stubServer({ "GET /api/assets": listAnswer() });
    showLibrary();
    await screen.findByText("No images yet. Upload a PNG or JPEG to start.");
    expect(screen.queryByText("Loading the library…")).toBeNull();
    expect(button("Retry")).toBeNull();
  });

  it("shows each image with its name, from its original URL", async () => {
    stubServer({ "GET /api/assets": listAnswer(nebula, andromeda) });
    showLibrary();
    const image = await screen.findByRole("img", { name: "nebula.png" });
    expect(image.getAttribute("src")).toBe("/api/assets/a1/original");
    screen.getByText("nebula.png");
    screen.getByText("m31.jpg");
    expect(
      screen.getByRole("img", { name: "m31.jpg" }).getAttribute("src"),
    ).toBe("/api/assets/a2/original");
    expect(
      screen.queryByText("No images yet. Upload a PNG or JPEG to start."),
    ).toBeNull();
  });

  it("shows a name holding markup as literal text", async () => {
    stubServer({
      "GET /api/assets": listAnswer(entry("a3", "<b>bold</b>.png")),
    });
    const { container } = render(<Library onSessionLost={vi.fn()} />);
    await screen.findByText("<b>bold</b>.png");
    expect(container.querySelector("b")).toBeNull();
  });

  it("keeps a 3:2 frame until the image loads, then takes its own shape", async () => {
    stubServer({ "GET /api/assets": listAnswer(nebula) });
    showLibrary();
    const image = await screen.findByRole("img", { name: "nebula.png" });
    const figure = image.closest("figure");
    expect(figure?.style.getPropertyValue("--library-ratio")).toBe("");
    Object.defineProperty(image, "naturalWidth", { value: 2000 });
    Object.defineProperty(image, "naturalHeight", { value: 1000 });
    fireEvent.load(image);
    expect(figure?.style.getPropertyValue("--library-ratio")).toBe("2");
  });

  it.each([
    ["a 503", { "GET /api/assets": [503, { error: "unavailable" }] }],
    ["a network error", {}],
    ["a body with no list", { "GET /api/assets": [200, { other: [] }] }],
    [
      "an entry with no name",
      { "GET /api/assets": listAnswer({ ...nebula, name: 3 }) },
    ],
    [
      "an original URL off the API",
      {
        "GET /api/assets": listAnswer({
          ...nebula,
          original_url: "https://elsewhere.invalid/x.png",
        }),
      },
    ],
  ] satisfies [string, Answers][])(
    "shows a list error with Retry, not the empty message, on %s",
    async (_, answers) => {
      stubServer(answers);
      showLibrary();
      await screen.findByText("The library could not be loaded.");
      screen.getByRole("button", { name: "Retry" });
      expect(
        screen.queryByText("No images yet. Upload a PNG or JPEG to start."),
      ).toBeNull();
      expect(screen.queryByRole("img")).toBeNull();
      expect(document.body.textContent).not.toContain("unavailable");
    },
  );

  it("loads the list again on Retry", async () => {
    const fetch = stubServer({
      "GET /api/assets": [[503, { error: "unavailable" }], listAnswer(nebula)],
    });
    showLibrary();
    fireEvent.click(await screen.findByRole("button", { name: "Retry" }));
    await screen.findByRole("img", { name: "nebula.png" });
    expect(screen.queryByText("The library could not be loaded.")).toBeNull();
    expect(fetch).toHaveBeenCalledTimes(2);
  });

  it("keeps the name and says so when an image cannot display", async () => {
    stubServer({ "GET /api/assets": listAnswer(nebula, andromeda) });
    showLibrary();
    const image = await screen.findByRole("img", { name: "nebula.png" });
    fireEvent.error(image);
    screen.getByText("This image could not be displayed.");
    screen.getByText("nebula.png");
    expect(screen.queryByRole("img", { name: "nebula.png" })).toBeNull();
    screen.getByRole("img", { name: "m31.jpg" });
  });

  it("reports a lost session when the list answers 401", async () => {
    stubServer({ "GET /api/assets": [401, { error: "unauthenticated" }] });
    const onSessionLost = showLibrary();
    await waitFor(() => {
      expect(onSessionLost).toHaveBeenCalledTimes(1);
    });
    expect(screen.queryByText("The library could not be loaded.")).toBeNull();
  });

  it("ignores a list that answers after the library is gone", async () => {
    const late = deferred();
    vi.stubGlobal(
      "fetch",
      vi.fn(() => late.promise),
    );
    const onSessionLost = vi.fn();
    const { unmount } = render(<Library onSessionLost={onSessionLost} />);
    await screen.findByText("Loading the library…");
    unmount();
    // Even an answer that would end the session is ignored.
    await act(async () => {
      late.answer(Response.json({ error: "unauthenticated" }, { status: 401 }));
      await late.promise;
    });
    expect(onSessionLost).not.toHaveBeenCalled();
  });
});
