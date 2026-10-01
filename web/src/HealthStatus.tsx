import { useEffect, useState } from "react";

type Health = "checking" | "healthy" | "unreachable";

const CHECK_TIMEOUT_MS = 5000;
const CHECK_INTERVAL_MS = 5000;

// Healthy only for status 200 with a JSON body whose `status` is "ok".
// Every other result, including an abort or a bad body, is unreachable.
// Nothing from the body is ever shown on screen.
async function checkHealth(signal: AbortSignal): Promise<Health> {
  try {
    const response = await fetch("/healthz", { signal });
    if (response.status !== 200) return "unreachable";
    const body: unknown = await response.json();
    const isOk =
      typeof body === "object" &&
      body !== null &&
      (body as { status?: unknown }).status === "ok";
    return isOk ? "healthy" : "unreachable";
  } catch {
    return "unreachable";
  }
}

function HealthStatus() {
  const [health, setHealth] = useState<Health>("checking");

  useEffect(() => {
    let stopped = false;
    let controller: AbortController | undefined;
    let timeout: ReturnType<typeof setTimeout> | undefined;
    let next: ReturnType<typeof setTimeout> | undefined;

    // The next check starts 5 seconds after this one finishes, so checks
    // never overlap, even when the server is slow to answer.
    async function run() {
      controller = new AbortController();
      timeout = setTimeout(() => controller?.abort(), CHECK_TIMEOUT_MS);
      const result = await checkHealth(controller.signal);
      clearTimeout(timeout);
      if (stopped) return;
      setHealth(result);
      next = setTimeout(run, CHECK_INTERVAL_MS);
    }

    // run() never rejects: checkHealth turns every error into "unreachable".
    void run();

    return () => {
      stopped = true;
      controller?.abort();
      clearTimeout(timeout);
      clearTimeout(next);
    };
  }, []);

  return (
    <p>
      Server: <strong>{health}</strong>
    </p>
  );
}

export default HealthStatus;
