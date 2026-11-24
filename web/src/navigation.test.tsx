import {
  act,
  cleanup,
  fireEvent,
  render,
  renderHook,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { assetIdOf, Link, navigate, usePath } from "./navigation.tsx";

// Whether the last click reached the page with its default action still on.
// The listener then stops the default, which jsdom cannot carry out.
let opened: boolean | null;

function recordDefault(event: MouseEvent) {
  opened = !event.defaultPrevented;
  event.preventDefault();
}

beforeEach(() => {
  history.replaceState(null, "", "/");
  opened = null;
  window.addEventListener("click", recordDefault);
});

afterEach(() => {
  window.removeEventListener("click", recordDefault);
  cleanup();
  vi.restoreAllMocks();
});

describe("usePath", () => {
  it("reads the address and follows navigate", () => {
    const { result } = renderHook(() => usePath());
    expect(result.current).toBe("/");
    act(() => {
      navigate("/assets/x");
    });
    expect(result.current).toBe("/assets/x");
    expect(location.pathname).toBe("/assets/x");
  });

  it("follows the browser's Back action", async () => {
    const { result } = renderHook(() => usePath());
    act(() => {
      navigate("/assets/x");
    });
    history.back();
    await waitFor(() => {
      expect(result.current).toBe("/");
    });
  });
});

describe("navigate", () => {
  it("adds one history entry, so Back returns to the address before", async () => {
    const push = vi.spyOn(history, "pushState");
    navigate("/assets/x");
    expect(push).toHaveBeenCalledTimes(1);
    expect(push).toHaveBeenCalledWith(null, "", "/assets/x");
    history.back();
    await waitFor(() => {
      expect(location.pathname).toBe("/");
    });
  });
});

describe("Link", () => {
  function showLink() {
    const { result } = renderHook(() => usePath());
    render(<Link to="/assets/x">nebula.png</Link>);
    return { path: () => result.current, link: screen.getByRole("link") };
  }

  it("is a real link to its address", () => {
    const { link } = showLink();
    expect(link.getAttribute("href")).toBe("/assets/x");
  });

  it("moves to its address on a plain click, without a page load", () => {
    const { link, path } = showLink();
    fireEvent.click(link);
    expect(opened).toBe(false);
    expect(path()).toBe("/assets/x");
  });

  it.each([
    ["Ctrl", { ctrlKey: true }],
    ["Meta", { metaKey: true }],
    ["Shift", { shiftKey: true }],
    ["Alt", { altKey: true }],
    ["the middle button", { button: 1 }],
  ])("leaves a click with %s to the browser", (_, init) => {
    const { link, path } = showLink();
    fireEvent.click(link, init);
    expect(opened).toBe(true);
    expect(path()).toBe("/");
  });
});

describe("assetIdOf", () => {
  const id = "0b6f3c2e-8d1a-4f7b-9c3e-2a5d6e7f8091";

  it("finds the id in an asset page address", () => {
    expect(assetIdOf(`/assets/${id}`)).toBe(id);
  });

  it.each([
    ["/"],
    ["/assets"],
    [`/assets/${id.toUpperCase()}`],
    [`/assets/${id}/`],
    [`/assets/${id}/original`],
    ["/assets/a1"],
    [`/x/assets/${id}`],
  ])("finds none in %s", (path) => {
    expect(assetIdOf(path)).toBeNull();
  });
});
