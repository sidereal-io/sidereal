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
      "an entry with no current version",
      {
        "GET /api/assets": listAnswer({
          ...nebula,
          current_version_id: undefined,
        }),
      },
    ],
    [
      "an original URL that names another Asset",
      {
        "GET /api/assets": listAnswer({
          ...nebula,
          original_url: "/api/assets/a2/original",
        }),
      },
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

  describe("upload", () => {
    const png = () =>
      new File([new Uint8Array([0x89, 0x50, 0x4e, 0x47])], "nebula.png", {
        type: "image/png",
      });

    const named = (name: string) =>
      new File([new Uint8Array([0x89, 0x50, 0x4e, 0x47])], name, {
        type: "image/png",
      });

    // The file input the Upload button opens.
    const picker = () => {
      const input =
        document.querySelector<HTMLInputElement>('input[type="file"]');
      if (!input) throw new Error("no file input");
      return input;
    };

    function choose(...files: File[]) {
      fireEvent.change(picker(), { target: { files } });
    }

    // The text of each line in the list of uploads, in order.
    const lines = () =>
      Array.from(
        screen.getByRole("list", { name: "Uploads" }).querySelectorAll("li"),
        (item) => item.textContent,
      );

    const uploads = (fetch: ReturnType<typeof vi.fn>) =>
      fetch.mock.calls.filter(
        ([url, init]) =>
          url === "/api/assets" &&
          (init as RequestInit | undefined)?.method === "POST",
      );

    it("opens the file picker from one Upload button, for PNG and JPEG", async () => {
      stubServer({ "GET /api/assets": listAnswer() });
      showLibrary();
      const upload = await screen.findByRole("button", { name: "Upload" });
      const click = vi.spyOn(picker(), "click");
      fireEvent.click(upload);
      expect(click).toHaveBeenCalledTimes(1);
      expect(picker().accept).toBe("image/png,image/jpeg");
      expect(picker().multiple).toBe(true);
    });

    it("sends the chosen file at once, as the multipart field file", async () => {
      const fetch = stubServer({
        "GET /api/assets": listAnswer(),
        "POST /api/assets": [201, { asset: nebula }],
      });
      showLibrary();
      await screen.findByRole("button", { name: "Upload" });
      const file = png();
      choose(file);
      await waitFor(() => {
        expect(uploads(fetch)).toHaveLength(1);
      });
      const body = (uploads(fetch)[0]?.[1] as RequestInit).body;
      expect(body).toBeInstanceOf(FormData);
      expect((body as FormData).get("file")).toBe(file);
      expect([...(body as FormData).keys()]).toEqual(["file"]);
    });

    it("sends the files of one pick in turn, one request at a time", async () => {
      const answers = [deferred(), deferred(), deferred()];
      let sent = 0;
      const fetch = vi.fn((_input: string, init?: RequestInit) => {
        if (init?.method !== "POST") {
          return Promise.resolve(Response.json({ assets: [] }));
        }
        const next = answers.at(sent++);
        return next ? next.promise : Promise.reject(new TypeError("extra"));
      });
      vi.stubGlobal("fetch", fetch);
      showLibrary();
      await screen.findByRole("button", { name: "Upload" });
      const files = [named("a.png"), named("b.png"), named("c.png")];
      choose(...files);
      for (const [index, file] of files.entries()) {
        await waitFor(() => {
          expect(uploads(fetch)).toHaveLength(index + 1);
        });
        const body = (uploads(fetch)[index]?.[1] as RequestInit)
          .body as FormData;
        expect(body.get("file")).toBe(file);
        expect([...body.keys()]).toEqual(["file"]);
        // The next file waits until this one has its answer.
        await act(async () => {
          await Promise.resolve();
        });
        expect(uploads(fetch)).toHaveLength(index + 1);
        await act(async () => {
          answers[index]?.answer(
            Response.json({ asset: nebula }, { status: 201 }),
          );
          await answers[index]?.promise;
        });
      }
      await waitFor(() => {
        expect(
          screen
            .getByRole("button", { name: "Upload" })
            .hasAttribute("disabled"),
        ).toBe(false);
      });
      expect(uploads(fetch)).toHaveLength(3);
    });

    it("shows one line for each file of a pick, in pick order", async () => {
      const second = deferred();
      let sent = 0;
      vi.stubGlobal(
        "fetch",
        vi.fn((_input: string, init?: RequestInit) => {
          if (init?.method !== "POST") {
            return Promise.resolve(Response.json({ assets: [] }));
          }
          sent += 1;
          return sent === 1
            ? Promise.resolve(Response.json({ asset: nebula }, { status: 201 }))
            : second.promise;
        }),
      );
      showLibrary();
      await screen.findByRole("button", { name: "Upload" });
      choose(named("a.png"), named("b.png"), named("c.png"));
      await screen.findByText("Uploading b.png…");
      expect(lines()).toEqual([
        "a.png was uploaded.",
        "Uploading b.png…",
        "c.png is waiting.",
      ]);
      const items = screen
        .getByRole("list", { name: "Uploads" })
        .querySelectorAll("li");
      for (const item of items) {
        expect(item.className).toBe("library-status");
      }
    });

    it("shows a waiting filename holding markup as literal text", async () => {
      vi.stubGlobal(
        "fetch",
        vi.fn((_input: string, init?: RequestInit) =>
          init?.method === "POST"
            ? new Promise<Response>(() => undefined)
            : Promise.resolve(Response.json({ assets: [] })),
        ),
      );
      const { container } = render(<Library onSessionLost={vi.fn()} />);
      await screen.findByRole("button", { name: "Upload" });
      choose(
        named("a.png"),
        new File(["x"], "<b>m42</b>.png", { type: "image/png" }),
      );
      await screen.findByText("<b>m42</b>.png is waiting.");
      expect(container.querySelector("b")).toBeNull();
    });

    it("replaces the last pick's lines with the next pick's", async () => {
      stubServer({
        "GET /api/assets": listAnswer(),
        "POST /api/assets": [409],
      });
      showLibrary();
      await screen.findByRole("button", { name: "Upload" });
      choose(named("a.png"), named("b.png"));
      await screen.findByText("b.png is already in the library.");
      choose(named("c.png"));
      await screen.findByText("c.png is already in the library.");
      expect(lines()).toEqual(["c.png is already in the library."]);
    });

    it("keeps the last lines when the picker is cancelled", async () => {
      const fetch = stubServer({
        "GET /api/assets": listAnswer(),
        "POST /api/assets": [409],
      });
      showLibrary();
      await screen.findByRole("button", { name: "Upload" });
      choose(named("a.png"));
      await screen.findByText("a.png is already in the library.");
      choose();
      expect(lines()).toEqual(["a.png is already in the library."]);
      expect(uploads(fetch)).toHaveLength(1);
    });

    it("shows the filename while pending and sends no second request", async () => {
      const pending = deferred();
      const fetch = vi.fn((_input: string, init?: RequestInit) =>
        init?.method === "POST"
          ? pending.promise
          : Promise.resolve(Response.json({ assets: [] })),
      );
      vi.stubGlobal("fetch", fetch);
      showLibrary();
      const upload = await screen.findByRole("button", { name: "Upload" });
      choose(png());
      await screen.findByText("Uploading nebula.png…");
      expect(upload.hasAttribute("disabled")).toBe(true);
      fireEvent.click(upload);
      choose(png());
      expect(uploads(fetch)).toHaveLength(1);
      await act(async () => {
        pending.answer(Response.json({ asset: nebula }, { status: 201 }));
        await pending.promise;
      });
      await waitFor(() => {
        expect(upload.hasAttribute("disabled")).toBe(false);
      });
    });

    it("shows the new image after success, without a page reload", async () => {
      const fetch = stubServer({
        "GET /api/assets": [listAnswer(), listAnswer(nebula)],
        "POST /api/assets": [201, { asset: nebula }],
      });
      showLibrary();
      await screen.findByText("No images yet. Upload a PNG or JPEG to start.");
      choose(png());
      await screen.findByRole("img", { name: "nebula.png" });
      expect(lines()).toEqual(["nebula.png was uploaded."]);
      expect(
        fetch.mock.calls.filter(
          ([url, init]) => url === "/api/assets" && !init?.method,
        ),
      ).toHaveLength(2);
    });

    it.each([
      [
        413,
        "nebula.png was not uploaded: it is larger than the 100 MiB limit.",
      ],
      [
        415,
        "nebula.png was not uploaded: only PNG and JPEG images are supported.",
      ],
      [
        400,
        "nebula.png was not uploaded: choose one PNG or JPEG file and try again.",
      ],
      [
        503,
        "nebula.png was not uploaded: the server could not save it. Try again later.",
      ],
      [
        403,
        "nebula.png was not uploaded: the server could not save it. Try again later.",
      ],
    ])("shows a fixed message for a %i refusal", async (status, message) => {
      const fetch = stubServer({
        "GET /api/assets": listAnswer(),
        "POST /api/assets": [status, { error: "<b>server words</b>" }],
      });
      showLibrary();
      await screen.findByRole("button", { name: "Upload" });
      choose(png());
      await screen.findByText(message);
      expect(document.body.textContent).not.toContain("server words");
      expect(screen.queryByText(/Uploading/)).toBeNull();
      // Nova can choose another file straight away.
      expect(
        screen.getByRole("button", { name: "Upload" }).hasAttribute("disabled"),
      ).toBe(false);
      expect(picker().value).toBe("");
      choose(png());
      await waitFor(() => {
        expect(uploads(fetch)).toHaveLength(2);
      });
    });

    const listRequests = (fetch: ReturnType<typeof vi.fn>) =>
      fetch.mock.calls.filter(
        ([url, init]) =>
          url === "/api/assets" && !(init as RequestInit | undefined)?.method,
      );

    it.each([
      ["nebula.png", "nebula.png is already in the library."],
      [
        "m31-final.png",
        "m31-final.png is already in the library as nebula.png.",
      ],
    ])(
      "says %s is already in the library when the upload answers 409",
      async (chosen, notice) => {
        const fetch = stubServer({
          "GET /api/assets": listAnswer(nebula),
          "POST /api/assets": [409, { asset: nebula }],
        });
        showLibrary();
        await screen.findByRole("img", { name: "nebula.png" });
        choose(named(chosen));
        await screen.findByText(notice);
        expect(document.body.textContent).not.toContain("was not uploaded");
        expect(screen.queryByText(/Uploading/)).toBeNull();
        expect(listRequests(fetch)).toHaveLength(1);
        expect(screen.getAllByRole("img")).toHaveLength(1);
        // Nova can choose another file straight away.
        expect(
          screen
            .getByRole("button", { name: "Upload" })
            .hasAttribute("disabled"),
        ).toBe(false);
        expect(picker().value).toBe("");
        choose(png());
        await waitFor(() => {
          expect(uploads(fetch)).toHaveLength(2);
        });
      },
    );

    it.each([
      ["no body", undefined],
      ["a body with no asset", { other: nebula }],
      ["an entry with no name", { asset: { ...nebula, name: 3 } }],
      ["an entry with an empty name", { asset: { ...nebula, name: "" } }],
      [
        "an entry with no current version",
        { asset: { ...nebula, current_version_id: undefined } },
      ],
      [
        "an original URL that names another Asset",
        { asset: { ...nebula, original_url: "/api/assets/a2/original" } },
      ],
      ["a list instead of an entry", { asset: [nebula] }],
    ])(
      "gives the short notice, with no name from the body, for a 409 with %s",
      async (_, body) => {
        stubServer({
          "GET /api/assets": listAnswer(),
          "POST /api/assets": [409, body],
        });
        showLibrary();
        await screen.findByRole("button", { name: "Upload" });
        choose(named("m31-final.png"));
        await screen.findByText("m31-final.png is already in the library.");
        expect(document.body.textContent).not.toContain("nebula.png");
        expect(document.body.textContent).not.toContain("was not uploaded");
      },
    );

    it("shows an existing name holding markup as literal text", async () => {
      stubServer({
        "GET /api/assets": listAnswer(),
        "POST /api/assets": [409, { asset: entry("a3", "<b>bold</b>.png") }],
      });
      const { container } = render(<Library onSessionLost={vi.fn()} />);
      await screen.findByRole("button", { name: "Upload" });
      choose(named("m31-final.png"));
      await screen.findByText(
        "m31-final.png is already in the library as <b>bold</b>.png.",
      );
      expect(container.querySelector("b")).toBeNull();
    });

    it("says the outcome is unknown, and loads the list again, when no answer comes back", async () => {
      // The server saved the file, but its answer was lost on the way.
      const fetch = stubServer({
        "GET /api/assets": [listAnswer(), listAnswer(nebula)],
      });
      showLibrary();
      await screen.findByRole("button", { name: "Upload" });
      choose(png());
      await screen.findByText(
        "No answer came back for nebula.png. It is safe to upload it again: if it was saved, Sidereal will find it.",
      );
      expect(document.body.textContent).not.toContain("was not uploaded");
      await screen.findByRole("img", { name: "nebula.png" });
      expect(
        fetch.mock.calls.filter(
          ([url, init]) => url === "/api/assets" && !init?.method,
        ),
      ).toHaveLength(2);
      // The screen never sends the file again by itself.
      expect(uploads(fetch)).toHaveLength(1);
    });

    it("shows a filename holding markup as literal text", async () => {
      const pending = deferred();
      vi.stubGlobal(
        "fetch",
        vi.fn((_input: string, init?: RequestInit) =>
          init?.method === "POST"
            ? pending.promise
            : Promise.resolve(Response.json({ assets: [] })),
        ),
      );
      const { container } = render(<Library onSessionLost={vi.fn()} />);
      await screen.findByRole("button", { name: "Upload" });
      choose(new File(["x"], "<b>m42</b>.png", { type: "image/png" }));
      await screen.findByText("Uploading <b>m42</b>.png…");
      expect(container.querySelector("b")).toBeNull();
    });

    it("reports a lost session when the upload answers 401", async () => {
      stubServer({
        "GET /api/assets": listAnswer(),
        "POST /api/assets": [401, { error: "unauthenticated" }],
      });
      const onSessionLost = showLibrary();
      await screen.findByRole("button", { name: "Upload" });
      choose(png());
      await waitFor(() => {
        expect(onSessionLost).toHaveBeenCalledTimes(1);
      });
    });

    it("ignores an upload that answers after the library is gone", async () => {
      const pending = deferred();
      const fetch = vi.fn((_input: string, init?: RequestInit) =>
        init?.method === "POST"
          ? pending.promise
          : Promise.resolve(Response.json({ assets: [] })),
      );
      vi.stubGlobal("fetch", fetch);
      const onSessionLost = vi.fn();
      const { unmount } = render(<Library onSessionLost={onSessionLost} />);
      await screen.findByRole("button", { name: "Upload" });
      choose(png());
      await screen.findByText("Uploading nebula.png…");
      unmount();
      await act(async () => {
        pending.answer(
          Response.json({ error: "unauthenticated" }, { status: 401 }),
        );
        await pending.promise;
      });
      expect(onSessionLost).not.toHaveBeenCalled();
      expect(fetch.mock.calls.filter(([, init]) => !init?.method)).toHaveLength(
        1,
      );
    });
  });
});
