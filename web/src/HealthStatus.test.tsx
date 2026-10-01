import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, it, vi } from "vitest";
import HealthStatus from "./HealthStatus.tsx";

// Answer every health request with this status and JSON body.
function stubHealth(status: number, body: unknown) {
  vi.stubGlobal(
    "fetch",
    vi.fn(() => Promise.resolve(Response.json(body, { status }))),
  );
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("HealthStatus", () => {
  it('shows "healthy" for status 200 with {"status":"ok"}', async () => {
    stubHealth(200, { status: "ok" });
    render(<HealthStatus />);
    await screen.findByText("healthy");
  });

  it('shows "unreachable" for status 500', async () => {
    stubHealth(500, { status: "ok" });
    render(<HealthStatus />);
    await screen.findByText("unreachable");
  });
});
