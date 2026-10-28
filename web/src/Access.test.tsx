import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import Access from "./Access.tsx";

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

afterEach(() => {
  cleanup();
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
});
