import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import HealthStatus from "./HealthStatus.tsx";

// Answer every health request with this status and JSON body.
function stubHealth(status: number, body: unknown) {
  vi.stubGlobal(
    "fetch",
    vi.fn(() => Promise.resolve(Response.json(body, { status }))),
  );
}

// The first check has finished once "checking" is gone, so a test that
// expects "checking" fails instead of matching the starting state.
async function expectState(state: string) {
  await waitFor(() => {
    expect(screen.queryByText("checking")).toBeNull();
  });
  screen.getByText(state);
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("HealthStatus", () => {
  it('shows "healthy" for status 200 with {"status":"ok"}', async () => {
    stubHealth(200, { status: "ok" });
    render(<HealthStatus />);
    await expectState("healthy");
  });

  it('shows "unreachable" for status 500', async () => {
    stubHealth(500, { status: "ok" });
    render(<HealthStatus />);
    await expectState("unreachable");
  });
});
