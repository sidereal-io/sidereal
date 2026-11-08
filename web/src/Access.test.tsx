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
import "./styles/tokens.css";
import { stubWebSocket, type FakeWebSocket } from "./testing/fakeWebSocket.ts";

type Answer = [status: number, body?: unknown];

// Answer each "METHOD /path" request with its status and JSON body. A request
// with no answer fails like a network error.
function stubServer(answers: Partial<Record<string, Answer>>) {
  const fetch = vi.fn((input: string, init?: RequestInit) => {
    const answer = answers[`${init?.method ?? "GET"} ${input}`];
    if (!answer) return Promise.reject(new TypeError("network error"));
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

const button = (name: string) => screen.queryByRole("button", { name });

const signedOut: Record<string, Answer> = {
  "GET /api/claim": [200, { open: false }],
  "GET /api/session": [401, { error: "unauthenticated" }],
};

// jsdom applies stylesheets but does not resolve var(), so follow one var()
// to its value on :root, as a browser would.
function computed(element: Element, property: string): string {
  const value = getComputedStyle(element).getPropertyValue(property).trim();
  const token = /^var\((--[\w-]+)\)$/.exec(value)?.[1];
  if (!token) return value;
  return getComputedStyle(document.documentElement)
    .getPropertyValue(token)
    .trim();
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

  it("styles the submit button from the tokens", async () => {
    stubServer({ "GET /api/claim": [200, { open: true }] });
    render(<Access />);
    const submitButton = await screen.findByRole("button", { name: "Claim" });
    expect(rgb(computed(submitButton, "background-color"))).toBe(
      "rgb(124, 58, 237)",
    );
    expect(computed(submitButton, "border-radius")).toBe("4px");
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
