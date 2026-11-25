import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import Access from "./Access.tsx";
import accessCss from "./Access.css?raw";
import "./styles/tokens.css";
import { stubWebSocket, type FakeWebSocket } from "./testing/fakeWebSocket.ts";

type Answer = [status: number, body?: unknown, headers?: HeadersInit];

// Answer each "METHOD /path" request with its status, JSON body and headers.
// A request with no answer fails like a network error.
function stubServer(answers: Partial<Record<string, Answer>>) {
  const fetch = vi.fn((input: string, init?: RequestInit) => {
    const answer = answers[`${init?.method ?? "GET"} ${input}`];
    if (!answer) return Promise.reject(new TypeError("network error"));
    const [status, body, headers] = answer;
    return Promise.resolve(
      body === undefined
        ? new Response(null, { status, headers })
        : Response.json(body, { status, headers }),
    );
  });
  vi.stubGlobal("fetch", fetch);
  return fetch;
}

const button = (name: string) => screen.queryByRole("button", { name });

const signedOut: Record<string, Answer> = {
  "GET /api/claim": [200, { open: false }],
  "GET /api/session": [401, { error: "unauthenticated" }],
};

// jsdom applies stylesheets but does not resolve var(), so follow each var()
// to its value on :root, as a browser would. A semantic token is var() of a
// primitive, so the chain can be more than one step long.
function computed(element: Element, property: string): string {
  let value = getComputedStyle(element).getPropertyValue(property).trim();
  for (let step = 0; step < 10; step++) {
    const token = /^var\((--[\w-]+)\)$/.exec(value)?.[1];
    if (!token) return value;
    value = getComputedStyle(document.documentElement)
      .getPropertyValue(token)
      .trim();
  }
  throw new Error(`${property} does not resolve to a value`);
}

// The rgb() form a browser reports for a color.
function rgb(color: string): string {
  const probe = document.createElement("div");
  probe.style.color = color;
  return probe.style.color;
}

// Fill in the shown form and press its submit button.
async function submit(label: string) {
  const submitButton = await screen.findByRole("button", { name: label });
  fireEvent.change(screen.getByLabelText("Username"), {
    target: { value: "sam" },
  });
  fireEvent.change(screen.getByLabelText("Password"), {
    target: { value: "correct-horse-battery" },
  });
  fireEvent.click(submitButton);
}

let sockets: FakeWebSocket[];

beforeEach(() => {
  sockets = stubWebSocket();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe("Access", () => {
  it("shows the claim form while the claim is open", async () => {
    stubServer({ "GET /api/claim": [200, { open: true }] });
    render(<Access />);
    await screen.findByRole("button", { name: "Claim" });
    expect(button("Sign in")).toBeNull();
    expect(button("Sign out")).toBeNull();
  });

  it("shows the sign-in form while signed out", async () => {
    stubServer({
      "GET /api/claim": [200, { open: false }],
      "GET /api/session": [401, { error: "unauthenticated" }],
    });
    render(<Access />);
    await screen.findByRole("button", { name: "Sign in" });
    expect(button("Claim")).toBeNull();
  });

  it("shows the username while signed in", async () => {
    stubServer({
      "GET /api/claim": [200, { open: false }],
      "GET /api/session": [200, { username: "sam", role: "admin" }],
    });
    render(<Access />);
    await screen.findByRole("button", { name: "Sign out" });
    screen.getByText("sam");
    expect(screen.queryByRole("textbox")).toBeNull();
  });

  it("shows nothing before the first answer", async () => {
    const fetch = vi.fn(() => new Promise<Response>(() => undefined));
    vi.stubGlobal("fetch", fetch);
    render(<Access />);
    await waitFor(() => {
      expect(fetch).toHaveBeenCalled();
    });
    expect(screen.queryAllByRole("button")).toEqual([]);
    expect(screen.queryByText("Something went wrong")).toBeNull();
  });

  it("shows a failure when the claim state is unavailable", async () => {
    stubServer({ "GET /api/claim": [503, { error: "unavailable" }] });
    render(<Access />);
    await screen.findByText("Something went wrong");
    expect(screen.queryAllByRole("button")).toEqual([]);
  });

  it("shows a failure when the session check cannot reach the server", async () => {
    stubServer({ "GET /api/claim": [200, { open: false }] });
    render(<Access />);
    await screen.findByText("Something went wrong");
    expect(screen.queryAllByRole("button")).toEqual([]);
  });

  it("sends the typed credentials with a claim", async () => {
    const fetch = stubServer({
      "GET /api/claim": [200, { open: true }],
      "POST /api/claim": [201, { username: "sam", role: "admin" }],
    });
    render(<Access />);
    await submit("Claim");
    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith(
        "/api/claim",
        expect.objectContaining({ method: "POST" }),
      );
    });
    const init = fetch.mock.calls.find(
      ([, init]) => init?.method === "POST",
    )?.[1];
    expect(typeof init?.body === "string" && JSON.parse(init.body)).toEqual({
      username: "sam",
      password: "correct-horse-battery",
    });
  });

  it("moves to the sign-in form after a claim", async () => {
    stubServer({
      "GET /api/claim": [200, { open: true }],
      "POST /api/claim": [201, { username: "sam", role: "admin" }],
    });
    render(<Access />);
    await submit("Claim");
    await screen.findByRole("button", { name: "Sign in" });
  });

  it("shows a fixed message when a sign-in is refused", async () => {
    stubServer({
      ...signedOut,
      "POST /api/session": [401, { error: "<b>nope</b>" }],
    });
    render(<Access />);
    await submit("Sign in");
    await screen.findByText("Invalid username or password");
    screen.getByRole("button", { name: "Sign in" });
    expect(document.body.textContent).not.toContain("nope");
  });

  it("shows a failure when a sign-in meets an unexpected status", async () => {
    stubServer({
      ...signedOut,
      "POST /api/session": [503, { error: "unavailable" }],
    });
    render(<Access />);
    await submit("Sign in");
    await screen.findByText("Something went wrong");
    screen.getByRole("button", { name: "Sign in" });
  });

  describe("a limited sign-in", () => {
    const limited = (headers?: HeadersInit) => {
      stubServer({
        ...signedOut,
        "POST /api/session": [429, { error: "<b>nope</b>" }, headers],
      });
    };

    it("shows the wait in seconds and keeps the form usable", async () => {
      limited({ "Retry-After": "30" });
      render(<Access />);
      await submit("Sign in");
      await screen.findByText(
        "Too many failed sign-ins. Try again in 30 seconds.",
      );
      expect(button("Sign in")?.hasAttribute("disabled")).toBe(false);
      expect(document.body.textContent).not.toContain("nope");
    });

    it.each([
      ["1", "1 second"],
      ["59", "59 seconds"],
      ["60", "1 minute"],
      ["61", "2 minutes"],
      ["3300", "55 minutes"],
    ])("shows Retry-After: %s as %s", async (retryAfter, wait) => {
      limited({ "Retry-After": retryAfter });
      render(<Access />);
      await submit("Sign in");
      await screen.findByText(
        `Too many failed sign-ins. Try again in ${wait}.`,
      );
    });

    it.each([
      ["missing", undefined],
      ["a fraction", { "Retry-After": "1.5" }],
      ["negative", { "Retry-After": "-3" }],
      ["a date", { "Retry-After": "Wed, 11 Nov 2026 07:28:00 GMT" }],
    ])("shows a wait of later when Retry-After is %s", async (_, headers) => {
      limited(headers);
      render(<Access />);
      await submit("Sign in");
      await screen.findByText("Too many failed sign-ins. Try again later.");
      screen.getByRole("button", { name: "Sign in" });
    });

    it("leaves a limited claim to the usual failure", async () => {
      stubServer({
        "GET /api/claim": [200, { open: true }],
        "POST /api/claim": [429, { error: "too many attempts" }],
      });
      render(<Access />);
      await submit("Claim");
      await screen.findByText("Something went wrong");
    });
  });

  it("shows the username after a sign-in", async () => {
    stubServer({
      ...signedOut,
      "POST /api/session": [200, { username: "sam", role: "admin" }],
    });
    render(<Access />);
    await submit("Sign in");
    await screen.findByRole("button", { name: "Sign out" });
    screen.getByText("sam");
  });

  it("moves to the sign-in form after a sign-out", async () => {
    stubServer({
      "GET /api/claim": [200, { open: false }],
      "GET /api/session": [200, { username: "sam", role: "admin" }],
      "DELETE /api/session": [204],
    });
    render(<Access />);
    fireEvent.click(await screen.findByRole("button", { name: "Sign out" }));
    await screen.findByRole("button", { name: "Sign in" });
  });

  it("shows the library while signed in", async () => {
    stubServer({
      "GET /api/claim": [200, { open: false }],
      "GET /api/session": [200, { username: "sam", role: "admin" }],
      "GET /api/assets": [
        200,
        {
          assets: [
            {
              id: "a1",
              kind: "image",
              name: "nebula.png",
              current_version_id: "v1",
              original_url: "/api/assets/a1/original",
            },
          ],
        },
      ],
    });
    render(<Access />);
    await screen.findByRole("img", { name: "nebula.png" });
    screen.getByText("sam");
  });

  it("moves to the sign-in form when the library finds the session ended", async () => {
    stubServer({
      "GET /api/claim": [200, { open: false }],
      "GET /api/session": [200, { username: "sam", role: "admin" }],
      "GET /api/assets": [401, { error: "unauthenticated" }],
    });
    render(<Access />);
    await screen.findByRole("button", { name: "Sign in" });
    expect(screen.queryByRole("region", { name: "Library" })).toBeNull();
  });

  it("hides the library on sign-out, and its late answer shows nothing", async () => {
    let answerList: (response: Response) => void = () => undefined;
    const list = new Promise<Response>((resolve) => {
      answerList = resolve;
    });
    const answers: Partial<Record<string, Answer>> = {
      "GET /api/claim": [200, { open: false }],
      "GET /api/session": [200, { username: "sam", role: "admin" }],
      "DELETE /api/session": [204],
    };
    vi.stubGlobal(
      "fetch",
      vi.fn((input: string, init?: RequestInit) => {
        if (input === "/api/assets") return list;
        const answer = answers[`${init?.method ?? "GET"} ${input}`];
        if (!answer) return Promise.reject(new TypeError("network error"));
        const [status, body] = answer;
        return Promise.resolve(
          body === undefined
            ? new Response(null, { status })
            : Response.json(body, { status }),
        );
      }),
    );
    render(<Access />);
    await screen.findByText("Loading the library…");
    fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
    await screen.findByRole("button", { name: "Sign in" });
    await act(async () => {
      answerList(
        Response.json({
          assets: [
            {
              id: "a1",
              kind: "image",
              name: "nebula.png",
              current_version_id: "v1",
              original_url: "/api/assets/a1/original",
            },
          ],
        }),
      );
      await list;
    });
    expect(screen.queryByText("nebula.png")).toBeNull();
    expect(screen.queryByRole("region", { name: "Library" })).toBeNull();
  });

  it("uses no primitive whose role has a semantic token", () => {
    expect(
      accessCss.match(
        /var\(--sr-(violet-600|white|obsidian-100|obsidian-400|obsidian-800)\)/g,
      ),
    ).toBeNull();
  });

  it("styles the submit button from the tokens", async () => {
    stubServer({ "GET /api/claim": [200, { open: true }] });
    render(<Access />);
    const submitButton = await screen.findByRole("button", { name: "Claim" });
    expect(rgb(computed(submitButton, "background-color"))).toBe(
      "rgb(124, 58, 237)",
    );
    expect(computed(submitButton, "border-radius")).toBe("4px");
  });

  describe("a pick while the session ends", () => {
    // A request that answers, or fails like a network error, only when the
    // test says so.
    function held() {
      let answer: (response: Response) => void = () => undefined;
      let fail: () => void = () => undefined;
      const promise = new Promise<Response>((resolve, reject) => {
        answer = resolve;
        fail = () => {
          reject(new TypeError("network error"));
        };
      });
      return { promise, answer, fail };
    }

    type Held = ReturnType<typeof held>;

    // A server where sam is signed in, the list is empty, and every upload
    // and sign-out waits for the test.
    function heldServer() {
      const uploads: (Held & { name: string })[] = [];
      const signOuts: (Held & { signal: AbortSignal | null | undefined })[] =
        [];
      const fetch = vi.fn((input: string, init?: RequestInit) => {
        const key = `${init?.method ?? "GET"} ${input}`;
        if (key === "POST /api/assets") {
          const file = (init?.body as FormData).get("file") as File;
          const request = { ...held(), name: file.name };
          uploads.push(request);
          return request.promise;
        }
        if (key === "DELETE /api/session") {
          const request = { ...held(), signal: init?.signal };
          signOuts.push(request);
          return request.promise;
        }
        const answer = {
          "GET /api/claim": [200, { open: false }],
          "GET /api/session": [200, { username: "sam", role: "admin" }],
          "POST /api/session": [200, { username: "sam", role: "admin" }],
          "GET /api/assets": [200, { assets: [] }],
        }[key] as Answer | undefined;
        if (!answer) return Promise.reject(new TypeError("network error"));
        return Promise.resolve(Response.json(answer[1], { status: answer[0] }));
      });
      vi.stubGlobal("fetch", fetch);
      const lists = () =>
        fetch.mock.calls.filter(
          ([url, init]) => url === "/api/assets" && !init?.method,
        ).length;
      const sent = () => uploads.map((request) => request.name);
      const upload = (index: number) => {
        const request = uploads.at(index);
        if (!request) throw new Error(`upload ${String(index)} was not sent`);
        return request;
      };
      const signOut = (index = 0) => {
        const request = signOuts.at(index);
        if (!request) throw new Error("no sign-out was sent");
        return request;
      };
      return { lists, sent, upload, signOut };
    }

    const saved = () => Response.json({}, { status: 201 });

    function pick(...names: string[]) {
      const input =
        document.querySelector<HTMLInputElement>('input[type="file"]');
      if (!input) throw new Error("no file input");
      const files = names.map(
        (name) => new File(["x"], name, { type: "image/png" }),
      );
      fireEvent.change(input, { target: { files } });
    }

    const lines = () =>
      Array.from(
        screen
          .queryByRole("list", { name: "Uploads" })
          ?.querySelectorAll("li") ?? [],
        (item) => item.textContent,
      );

    async function signedInAsSam() {
      render(<Access />);
      await screen.findByText("No images yet. Upload a PNG or JPEG to start.");
    }

    // Nova picks the files and presses Sign out while the first one is on
    // its way; then the first file is saved while the sign-out is pending.
    async function signOutAfterFirstSaved(
      server: ReturnType<typeof heldServer>,
      names: string[],
    ) {
      await signedInAsSam();
      pick(...names);
      await waitFor(() => {
        expect(server.sent()).toEqual([names[0]]);
      });
      fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
      const lists = server.lists();
      await act(async () => {
        server.upload(0).answer(saved());
        await server.upload(0).promise;
      });
      await waitFor(() => {
        expect(server.sent()).toEqual(names.slice(0, 2));
        expect(server.lists()).toBe(lists + 1);
      });
    }

    it("sends no more files once a sign-out answers, even in the same step as an upload", async () => {
      const server = heldServer();
      await signOutAfterFirstSaved(server, ["a.png", "b.png", "c.png"]);
      const lists = server.lists();
      await act(async () => {
        server.signOut().answer(new Response(null, { status: 204 }));
        server.upload(1).answer(saved());
        await server.upload(1).promise;
      });
      await screen.findByRole("button", { name: "Sign in" });
      expect(screen.queryByRole("region", { name: "Library" })).toBeNull();
      expect(lines()).toEqual([]);
      expect(server.sent()).toEqual(["a.png", "b.png"]);
      expect(server.lists()).toBe(lists);
    });

    it("shows nothing for an upload that answers after the sign-out", async () => {
      const server = heldServer();
      await signOutAfterFirstSaved(server, ["a.png", "b.png", "c.png"]);
      const lists = server.lists();
      await act(async () => {
        server.signOut().answer(new Response(null, { status: 204 }));
        await server.signOut().promise;
      });
      await screen.findByRole("button", { name: "Sign in" });
      await act(async () => {
        server.upload(1).answer(saved());
        await server.upload(1).promise;
      });
      expect(screen.queryByRole("region", { name: "Library" })).toBeNull();
      expect(document.body.textContent).not.toContain("b.png");
      expect(server.sent()).toEqual(["a.png", "b.png"]);
      expect(server.lists()).toBe(lists);
    });

    it("sends no more files once the events socket ends the session", async () => {
      const server = heldServer();
      await signedInAsSam();
      pick("a.png", "b.png");
      await waitFor(() => {
        expect(server.sent()).toEqual(["a.png"]);
      });
      // Both in one act scope, so React has not yet unmounted the Library
      // when the upload loop sees the answer.
      await act(async () => {
        sockets.at(0)?.serverClose(1008);
        server.upload(0).answer(saved());
        await server.upload(0).promise;
      });
      await screen.findByRole("button", { name: "Sign in" });
      expect(server.sent()).toEqual(["a.png"]);
    });

    it.each([
      [
        "answers 500",
        (request: Held) => {
          request.answer(new Response(null, { status: 500 }));
        },
      ],
      [
        "gets no answer",
        (request: Held) => {
          request.fail();
        },
      ],
    ])("goes on with the pick when the sign-out %s", async (_, end) => {
      const server = heldServer();
      await signedInAsSam();
      pick("a.png", "b.png");
      await waitFor(() => {
        expect(server.sent()).toEqual(["a.png"]);
      });
      fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
      await act(async () => {
        end(server.signOut());
        await server.signOut().promise.catch(() => undefined);
      });
      await screen.findByText("Something went wrong");
      await act(async () => {
        server.upload(0).answer(saved());
        await server.upload(0).promise;
      });
      await waitFor(() => {
        expect(server.sent()).toEqual(["a.png", "b.png"]);
      });
      await act(async () => {
        server.upload(1).answer(saved());
        await server.upload(1).promise;
      });
      await screen.findByText("b.png was uploaded.");
      expect(lines()).toEqual(["a.png was uploaded.", "b.png was uploaded."]);
      screen.getByRole("region", { name: "Library" });
      screen.getByText("Something went wrong");
    });

    it("ends the pick at a 401 after a sign-out with no answer", async () => {
      const server = heldServer();
      await signedInAsSam();
      pick("a.png", "b.png", "c.png");
      await waitFor(() => {
        expect(server.sent()).toEqual(["a.png"]);
      });
      fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
      await act(async () => {
        server.signOut().fail();
        await server.signOut().promise.catch(() => undefined);
      });
      await screen.findByText("Something went wrong");
      await act(async () => {
        server.upload(0).answer(saved());
        await server.upload(0).promise;
      });
      await waitFor(() => {
        expect(server.sent()).toEqual(["a.png", "b.png"]);
      });
      await act(async () => {
        server
          .upload(1)
          .answer(Response.json({ error: "unauthenticated" }, { status: 401 }));
        await server.upload(1).promise;
      });
      await screen.findByRole("button", { name: "Sign in" });
      expect(server.sent()).toEqual(["a.png", "b.png"]);
    });

    // Nova presses Sign out, and the events socket ends the session before
    // the sign-out answers. Then Nova signs in again.
    async function signInAgainBeforeTheSignOutAnswers(
      server: ReturnType<typeof heldServer>,
    ) {
      await signedInAsSam();
      pick("a.png", "b.png");
      await waitFor(() => {
        expect(server.sent()).toEqual(["a.png"]);
      });
      fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
      act(() => {
        sockets.at(0)?.serverClose(1008);
      });
      await submit("Sign in");
      await screen.findByText("No images yet. Upload a PNG or JPEG to start.");
    }

    it("lets a late sign-out answer leave the next sign-in alone", async () => {
      const server = heldServer();
      await signInAgainBeforeTheSignOutAnswers(server);
      pick("c.png", "d.png");
      await waitFor(() => {
        expect(server.sent()).toEqual(["a.png", "c.png"]);
      });
      await act(async () => {
        server.signOut().answer(new Response(null, { status: 204 }));
        await server.signOut().promise;
      });
      await act(async () => {
        server.upload(1).answer(saved());
        await server.upload(1).promise;
      });
      await waitFor(() => {
        expect(server.sent()).toEqual(["a.png", "c.png", "d.png"]);
      });
      await act(async () => {
        server.upload(2).answer(saved());
        await server.upload(2).promise;
      });
      await screen.findByText("d.png was uploaded.");
      expect(lines()).toEqual(["c.png was uploaded.", "d.png was uploaded."]);
      screen.getByRole("region", { name: "Library" });
      expect(screen.queryByText("Something went wrong")).toBeNull();
    });

    // The browser applies a cancelled request's answer to no cookie, so the
    // old sign-out cannot clear the cookie of the next sign-in.
    it("cancels a pending sign-out once its signed-in view has ended", async () => {
      const server = heldServer();
      await signedInAsSam();
      fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
      await waitFor(() => {
        expect(server.signOut().signal?.aborted).toBe(false);
      });
      act(() => {
        sockets.at(0)?.serverClose(1008);
      });
      await screen.findByRole("button", { name: "Sign in" });
      expect(server.signOut().signal?.aborted).toBe(true);
    });

    it("shows no notice for a late failed sign-out", async () => {
      const server = heldServer();
      await signInAgainBeforeTheSignOutAnswers(server);
      await act(async () => {
        server.signOut().answer(new Response(null, { status: 500 }));
        await server.signOut().promise;
      });
      screen.getByRole("region", { name: "Library" });
      expect(screen.queryByText("Something went wrong")).toBeNull();
    });

    it("gives a new sign-in a fresh session signal", async () => {
      const server = heldServer();
      await signedInAsSam();
      fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
      await act(async () => {
        server.signOut().answer(new Response(null, { status: 204 }));
        await server.signOut().promise;
      });
      await submit("Sign in");
      await screen.findByText("No images yet. Upload a PNG or JPEG to start.");
      pick("c.png", "d.png");
      await waitFor(() => {
        expect(server.sent()).toEqual(["c.png"]);
      });
      await act(async () => {
        server.upload(0).answer(saved());
        await server.upload(0).promise;
      });
      await waitFor(() => {
        expect(server.sent()).toEqual(["c.png", "d.png"]);
      });
    });
  });

  describe("the events socket", () => {
    const signedIn: Record<string, Answer> = {
      "GET /api/claim": [200, { open: false }],
      "GET /api/session": [200, { username: "sam", role: "admin" }],
      "DELETE /api/session": [204],
    };

    // Waits for testing-library still need time to pass, so fake timers
    // also move with the clock.
    beforeEach(() => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
    });

    const sessionAsks = (fetch: ReturnType<typeof stubServer>) =>
      fetch.mock.calls.filter(
        ([url, init]) => url === "/api/session" && !init?.method,
      ).length;

    const theSocket = () => {
      expect(sockets).toHaveLength(1);
      const socket = sockets.at(0);
      if (!socket) throw new Error("no socket was opened");
      return socket;
    };

    async function showingSam(answers: Record<string, Answer>) {
      const fetch = stubServer(answers);
      render(<Access />);
      await screen.findByRole("button", { name: "Sign out" });
      return fetch;
    }

    it("opens one socket to /api/events after a sign-in", async () => {
      stubServer({
        ...signedOut,
        "POST /api/session": [200, { username: "sam", role: "admin" }],
      });
      render(<Access />);
      expect(sockets).toHaveLength(0);
      await submit("Sign in");
      await screen.findByRole("button", { name: "Sign out" });
      expect(theSocket().url).toBe(`ws://${location.host}/api/events`);
    });

    it("opens no socket while signed out", async () => {
      stubServer(signedOut);
      render(<Access />);
      await screen.findByRole("button", { name: "Sign in" });
      await vi.advanceTimersByTimeAsync(120_000);
      expect(sockets).toHaveLength(0);
    });

    it("closes the socket on sign-out and opens no other", async () => {
      await showingSam({ ...signedIn });
      const socket = theSocket();
      fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
      await screen.findByRole("button", { name: "Sign in" });
      expect(socket.closed).toBe(true);
      await vi.advanceTimersByTimeAsync(120_000);
      expect(sockets).toHaveLength(1);
    });

    it("shows the sign-in form with no message on a 1008 close", async () => {
      await showingSam({ ...signedIn });
      act(() => {
        theSocket().serverClose(1008);
      });
      await screen.findByRole("button", { name: "Sign in" });
      expect(button("Sign out")).toBeNull();
      expect(document.querySelector(".access-message")).toBeNull();
      await vi.advanceTimersByTimeAsync(120_000);
      expect(sockets).toHaveLength(1);
    });

    it("keeps showing sam after another close, and reopens on a 200", async () => {
      const fetch = await showingSam({ ...signedIn });
      act(() => {
        theSocket().serverClose(1006);
      });
      await vi.advanceTimersByTimeAsync(4_999);
      expect(sessionAsks(fetch)).toBe(1);
      screen.getByText("sam");
      await vi.advanceTimersByTimeAsync(1);
      await waitFor(() => {
        expect(sockets).toHaveLength(2);
      });
      screen.getByText("sam");
    });

    it("shows the sign-in form when the ask after a close answers 401", async () => {
      const answers = { ...signedIn };
      await showingSam(answers);
      answers["GET /api/session"] = [401, { error: "unauthenticated" }];
      act(() => {
        theSocket().serverClose(1006);
      });
      await vi.advanceTimersByTimeAsync(5_000);
      await screen.findByRole("button", { name: "Sign in" });
      expect(sockets).toHaveLength(1);
    });

    it("keeps showing sam while the database is down after a 1011 close", async () => {
      const answers = { ...signedIn };
      const fetch = await showingSam(answers);
      answers["GET /api/session"] = [503, { error: "unavailable" }];
      act(() => {
        theSocket().serverClose(1011);
      });
      await vi.advanceTimersByTimeAsync(5_000);
      expect(sessionAsks(fetch)).toBe(2);
      screen.getByText("sam");
      expect(sockets).toHaveLength(1);
      answers["GET /api/session"] = [200, { username: "sam", role: "admin" }];
      await vi.advanceTimersByTimeAsync(5_000);
      await waitFor(() => {
        expect(sockets).toHaveLength(2);
      });
      expect(sessionAsks(fetch)).toBe(3);
      screen.getByText("sam");
      expect(button("Sign in")).toBeNull();
    });

    it("treats a socket silent for 60 s as closed", async () => {
      const fetch = await showingSam({ ...signedIn });
      const socket = theSocket();
      await vi.advanceTimersByTimeAsync(60_000);
      expect(socket.closed).toBe(true);
      await vi.advanceTimersByTimeAsync(5_000);
      expect(sessionAsks(fetch)).toBe(2);
    });

    it("signing out during the wait sends no ask and opens no socket", async () => {
      const fetch = await showingSam({ ...signedIn });
      act(() => {
        theSocket().serverClose(1006);
      });
      await vi.advanceTimersByTimeAsync(2_000);
      fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
      await screen.findByRole("button", { name: "Sign in" });
      await vi.advanceTimersByTimeAsync(120_000);
      expect(sessionAsks(fetch)).toBe(1);
      expect(sockets).toHaveLength(1);
    });
  });
});
