import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import AssetPage from "./AssetPage.tsx";
import assetPageCss from "./AssetPage.css?raw";
import tokensCss from "./styles/tokens.css?raw";

type Answer = [status: number, body?: unknown];

const id = "0b6f3c2e-8d1a-4f7b-9c3e-2a5d6e7f8091";
const address = `/api/assets/${id}`;

// Answers each request for the Asset with the next answer in turn (the last
// one repeats). A request with no answer left fails like a network error.
function stubServer(...answers: (Answer | "network error")[]) {
  let turn = 0;
  const fetch = vi.fn((input: string) => {
    if (input !== address) {
      return Promise.reject(new TypeError(`unexpected ${input}`));
    }
    const answer = answers[Math.min(turn, answers.length - 1)];
    turn += 1;
    if (answer === "network error") {
      return Promise.reject(new TypeError("network error"));
    }
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

// A request that answers only when the test says so.
function deferred() {
  let answer: (response: Response) => void = () => undefined;
  const promise = new Promise<Response>((resolve) => {
    answer = resolve;
  });
  return { promise, answer };
}

const fileSize = {
  name: "image.file-size",
  label: "File size",
  value: 1234567,
  unit: "bytes",
};

function asset(overrides: Record<string, unknown> = {}) {
  return {
    asset: {
      id,
      kind: "image",
      name: "nebula.png",
      current_version_id: "7d2c9e10-1f3a-4b5c-8d6e-9f0a1b2c3d4e",
      original_url: `/api/assets/${id}/original`,
      facets: [fileSize],
      ...overrides,
    },
  };
}

function showPage(onSessionLost = vi.fn()) {
  const view = render(
    <AssetPage id={id} onSessionLost={onSessionLost}>
      <span>nova</span>
    </AssetPage>,
  );
  return { onSessionLost, ...view };
}

const loaded = () => screen.findByRole("heading", { name: "nebula.png" });
const failedLine = "The image could not be loaded.";
const noFactsLine = "No facts are recorded for this image.";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("AssetPage", () => {
  it("says it is loading before the Asset answers", async () => {
    const fetch = vi.fn(() => new Promise<Response>(() => undefined));
    vi.stubGlobal("fetch", fetch);
    showPage();
    await screen.findByText("Loading the image…");
    expect(fetch).toHaveBeenCalledWith(address, expect.anything());
    expect(screen.queryByRole("img")).toBeNull();
    expect(screen.queryByText(noFactsLine)).toBeNull();
  });

  it("shows the image whole, its name and each fact with its digits grouped", async () => {
    stubServer([200, asset()]);
    showPage();
    await loaded();
    const image = screen.getByRole("img", { name: "nebula.png" });
    expect(image.getAttribute("src")).toBe(`/api/assets/${id}/original`);
    const facts = screen.getByRole("term");
    expect(facts.textContent).toBe("File size");
    expect(screen.getByRole("definition").textContent).toBe("1,234,567 bytes");
    expect(screen.queryByText(noFactsLine)).toBeNull();
  });

  it("lists every fact in the order the server gives", async () => {
    stubServer([
      200,
      asset({
        facets: [
          fileSize,
          { name: "test.width", label: "Width", value: 6000, unit: "px" },
        ],
      }),
    ]);
    showPage();
    await loaded();
    expect(screen.getAllByRole("term").map((term) => term.textContent)).toEqual(
      ["File size", "Width"],
    );
    expect(
      screen.getAllByRole("definition").map((value) => value.textContent),
    ).toEqual(["1,234,567 bytes", "6,000 px"]);
  });

  it("lists the four facts of a new PNG in the server's order", async () => {
    stubServer([
      200,
      asset({
        facets: [
          fileSize,
          { name: "image.format", label: "Format", value: "PNG", unit: "" },
          {
            name: "image.height",
            label: "Height",
            value: 2160,
            unit: "pixels",
          },
          { name: "image.width", label: "Width", value: 4096, unit: "pixels" },
        ],
      }),
    ]);
    showPage();
    await loaded();
    expect(screen.getAllByRole("term").map((term) => term.textContent)).toEqual(
      ["File size", "Format", "Height", "Width"],
    );
    const values = screen.getAllByRole("definition");
    expect(values.map((value) => value.textContent)).toEqual([
      "1,234,567 bytes",
      "PNG",
      "2,160 pixels",
      "4,096 pixels",
    ]);
    expect(values.map((value) => value.tagName)).toEqual([
      "DD",
      "DD",
      "DD",
      "DD",
    ]);
  });

  it("shows a count with no unit without a trailing space", async () => {
    stubServer([
      200,
      asset({ facets: [{ ...fileSize, value: 12, unit: "" }] }),
    ]);
    showPage();
    await loaded();
    expect(screen.getByRole("definition").textContent).toBe("12");
  });

  it("shows markup in a choice as text", async () => {
    stubServer([
      200,
      asset({
        facets: [
          {
            name: "image.format",
            label: "Format",
            value: "<i>PNG</i>",
            unit: "",
          },
        ],
      }),
    ]);
    const { container } = showPage();
    await loaded();
    expect(screen.getByRole("definition").textContent).toBe("<i>PNG</i>");
    expect(container.querySelector("i")).toBeNull();
  });

  it("shows only the file size, and no processing failure, when the header was not read", async () => {
    stubServer([200, asset()]);
    const { container } = showPage();
    await loaded();
    fireEvent.error(screen.getByRole("img", { name: "nebula.png" }));
    await screen.findByText("This image could not be displayed.");
    expect(screen.getAllByRole("term").map((term) => term.textContent)).toEqual(
      ["File size"],
    );
    expect(screen.queryByText(noFactsLine)).toBeNull();
    expect(container.textContent).not.toMatch(/header|failed|run/i);
  });

  it("shows no provenance", async () => {
    stubServer([200, asset()]);
    const { container } = showPage();
    await loaded();
    expect(container.textContent).not.toMatch(/image\.file-size|measure|run/);
  });

  it("puts the account's items and a Library link in the page", async () => {
    stubServer([200, asset()]);
    showPage();
    await loaded();
    expect(screen.getByText("nova")).toBeTruthy();
    expect(
      screen.getByRole("link", { name: "Library" }).getAttribute("href"),
    ).toBe("/");
  });

  it("says no facts are recorded when the server returns none", async () => {
    stubServer([200, asset({ facets: [] })]);
    showPage();
    await loaded();
    expect(screen.getByRole("img", { name: "nebula.png" })).toBeTruthy();
    expect(screen.getByText(noFactsLine)).toBeTruthy();
    expect(screen.queryByRole("term")).toBeNull();
  });

  it("shows markup in a name or a label as text", async () => {
    stubServer([
      200,
      asset({
        name: "<b>bold</b>.png",
        facets: [{ ...fileSize, label: "<i>File</i> size" }],
      }),
    ]);
    const { container } = showPage();
    await screen.findByRole("heading", { name: "<b>bold</b>.png" });
    expect(screen.getByRole("term").textContent).toBe("<i>File</i> size");
    expect(container.querySelector("b, i")).toBeNull();
  });

  it("keeps the name and facts when the image cannot display", async () => {
    stubServer([200, asset()]);
    showPage();
    await loaded();
    fireEvent.error(screen.getByRole("img", { name: "nebula.png" }));
    await screen.findByText("This image could not be displayed.");
    expect(screen.queryByRole("img")).toBeNull();
    expect(screen.getByRole("heading", { name: "nebula.png" })).toBeTruthy();
    expect(screen.getByRole("definition").textContent).toBe("1,234,567 bytes");
  });

  it("says an Asset that does not exist is not in the library, with a Library link", async () => {
    stubServer([404]);
    showPage();
    const line = await screen.findByText("This image is not in the library.", {
      exact: false,
    });
    expect(
      within(line).getByRole("link", { name: "Library" }).getAttribute("href"),
    ).toBe("/");
    expect(screen.queryByRole("button", { name: "Retry" })).toBeNull();
    expect(screen.queryByText(noFactsLine)).toBeNull();
  });

  it("calls onSessionLost when the server says the session has ended", async () => {
    stubServer([401]);
    const { onSessionLost } = showPage();
    await waitFor(() => {
      expect(onSessionLost).toHaveBeenCalledTimes(1);
    });
    expect(screen.queryByText(failedLine)).toBeNull();
  });

  const failures: [string, Answer | "network error"][] = [
    ["a 503", [503, { error: "the library is unavailable" }]],
    ["a network error", "network error"],
    ["a body that is not JSON", [200]],
    ["a body with no asset", [200, { assets: [] }]],
    ["another Asset's id", [200, asset({ id: "another" })]],
    [
      "an original elsewhere",
      [200, asset({ original_url: "https://example.com/nebula.png" })],
    ],
    ["a name that is not text", [200, asset({ name: 7 })]],
    ["no facets", [200, asset({ facets: undefined })]],
    [
      "a facet with no label",
      [200, asset({ facets: [{ ...fileSize, label: null }] })],
    ],
    [
      "a facet with no unit",
      [200, asset({ facets: [{ ...fileSize, unit: 1 }] })],
    ],
    [
      "a facet with no name",
      [200, asset({ facets: [{ ...fileSize, name: undefined }] })],
    ],
    [
      "a negative value",
      [200, asset({ facets: [{ ...fileSize, value: -1 }] })],
    ],
    [
      "a fractional value",
      [200, asset({ facets: [{ ...fileSize, value: 1.5 }] })],
    ],
    [
      "a value past the largest safe integer",
      [200, asset({ facets: [{ ...fileSize, value: 9007199254740992 }] })],
    ],
    [
      "an empty text value",
      [200, asset({ facets: [{ ...fileSize, value: "" }] })],
    ],
    [
      "a value of true",
      [200, asset({ facets: [{ ...fileSize, value: true }] })],
    ],
    ["a null value", [200, asset({ facets: [{ ...fileSize, value: null }] })]],
  ];

  it.each(failures)(
    "says the image could not be loaded after %s, and Retry asks again",
    async (_, failure) => {
      const fetch = stubServer(failure, [200, asset()]);
      showPage();
      await screen.findByText(failedLine, { exact: false });
      expect(screen.queryByText(noFactsLine)).toBeNull();
      expect(screen.queryByRole("img")).toBeNull();
      expect(screen.queryByRole("heading", { name: "nebula.png" })).toBeNull();
      fireEvent.click(screen.getByRole("button", { name: "Retry" }));
      await loaded();
      expect(fetch).toHaveBeenCalledTimes(2);
    },
  );

  it("stores the largest count and shows it grouped", async () => {
    stubServer([
      200,
      asset({ facets: [{ ...fileSize, value: 9007199254740991 }] }),
    ]);
    showPage();
    await loaded();
    expect(screen.getByRole("definition").textContent).toBe(
      "9,007,199,254,740,991 bytes",
    );
  });

  it("shows nothing from an answer that arrives after the page goes away", async () => {
    const pending = deferred();
    const fetch = vi.fn((_: string, init?: RequestInit) => {
      init?.signal?.addEventListener("abort", () => undefined);
      return pending.promise;
    });
    vi.stubGlobal("fetch", fetch);
    const { onSessionLost, unmount } = showPage();
    await screen.findByText("Loading the image…");
    const signal = fetch.mock.calls[0][1]?.signal;
    unmount();
    expect(signal?.aborted).toBe(true);
    await act(async () => {
      pending.answer(new Response(null, { status: 401 }));
      await pending.promise;
    });
    expect(onSessionLost).not.toHaveBeenCalled();
  });
});

// The custom properties tokens.css defines. `pnpm tokens:check` keeps that
// file equal to what DESIGN.md defines.
const tokens = new Set(
  [...tokensCss.matchAll(/(--sr-[\w-]+)\s*:/g)].map((match) => match[1]),
);

// Properties whose values are colors, sizes, spacing or fonts.
const DESIGNED =
  /^(color|background(-color)?|border(-(top|right|bottom|left))?(-(color|width))?|font(-(family|size|weight))?|line-height|gap|row-gap|column-gap|padding(-\w+)?|margin(-\w+)?|width|height|min-(width|height)|max-(width|height)|flex(-basis)?|top|right|bottom|left|inset)$/;

// Each designed declaration whose value holds anything a token should give:
// a length, a color or a font name, or a custom property DESIGN.md does not
// define. Keywords such as `solid`, `auto` and `0`, unitless flex factors,
// and a share of the container such as `100%` are not design values.
function rawValues(css: string): string[] {
  const body = css.replace(/\/\*[\s\S]*?\*\//g, "");
  const found: string[] = [];
  for (const [, property, value] of body.matchAll(
    /([\w-]+)\s*:\s*([^;{}]+);/g,
  )) {
    if (!DESIGNED.test(property)) continue;
    const unknown = [...value.matchAll(/var\(\s*(--[\w-]+)\s*\)/g)]
      .map((match) => match[1])
      .filter((name) => !tokens.has(name));
    const rest = value.replace(/var\(\s*--[\w-]+\s*\)/g, "");
    const raw =
      /\d(px|em|rem|vh|vw|ch)\b|#[0-9a-f]{3,8}\b|(rgb|hsl)a?\(/i.test(rest) ||
      (/^font(-family)?$/.test(property) && /[a-z]/i.test(rest)) ||
      (/^(color|background)/.test(property) &&
        /\b(?!(transparent|inherit|currentcolor)\b)[a-z]+\b/i.test(rest)) ||
      (/^border/.test(property) &&
        /\b(?!(solid|dashed|dotted|none)\b)[a-z]+\b/i.test(rest));
    if (raw || unknown.length > 0) found.push(`${property}: ${value.trim()}`);
  }
  return found;
}

describe("AssetPage.css", () => {
  it("gives every color, size, spacing and font as a DESIGN.md token", () => {
    expect(assetPageCss).toMatch(/var\(--sr-/);
    expect(rawValues(assetPageCss)).toEqual([]);
  });

  it("finds a raw value or an undefined token", () => {
    // Built at run time, so the check that every used token exists does not
    // find this made-up one in the source.
    const prefix = "--sr-";
    const missing = `var(${prefix}spacing-999)`;
    expect(
      rawValues(
        `.a { padding: 12px; color: #fff; gap: ${missing}; margin: 0; border: var(--sr-border-width-1) solid red; font-family: Inter; flex: 1 0 var(--sr-size-300); }`,
      ),
    ).toEqual([
      "padding: 12px",
      "color: #fff",
      `gap: ${missing}`,
      "border: var(--sr-border-width-1) solid red",
      "font-family: Inter",
    ]);
  });
});
