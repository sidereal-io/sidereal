// The server closes the events socket with this code when the session that
// opened it is gone.
const POLICY_CLOSE = 1008;
// The server sends a heartbeat every 30 s, so two missed ones mean the
// connection is dead even if no close has arrived.
const SILENCE_MS = 60_000;
const RETRY_MS = 5_000;

// The events socket's address on the page's own origin.
export function eventsUrl(page: string): string {
  const url = new URL("/api/events", page);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  return url.href;
}

// Hold the events socket open while the session lives. `onSignedOut` runs
// once the server says the session is gone. The returned function stops
// everything, and nothing fires after it.
//
// A browser cannot see the status of a refused upgrade, so after any close
// but 1008 this asks GET /api/session whether the session still lives.
export function watchSession(onSignedOut: () => void): () => void {
  let stopped = false;
  let socket: WebSocket | undefined;
  let silence: ReturnType<typeof setTimeout> | undefined;
  let retry: ReturnType<typeof setTimeout> | undefined;
  let asking: AbortController | undefined;

  function stop() {
    stopped = true;
    clearTimeout(silence);
    clearTimeout(retry);
    asking?.abort();
    const current = socket;
    socket = undefined;
    current?.close();
  }

  function signedOut() {
    stop();
    onSignedOut();
  }

  function open() {
    const current = new WebSocket(eventsUrl(location.href));
    socket = current;
    hush();
    current.onmessage = hush;
    current.onclose = (event) => {
      // A socket this module already let go of reports nothing.
      if (stopped || socket !== current) return;
      socket = undefined;
      clearTimeout(silence);
      if (event.code === POLICY_CLOSE) signedOut();
      else later();
    };
  }

  // Restart the silence timer.
  function hush() {
    clearTimeout(silence);
    silence = setTimeout(() => {
      const current = socket;
      socket = undefined;
      current?.close();
      later();
    }, SILENCE_MS);
  }

  function later() {
    retry = setTimeout(() => void ask(), RETRY_MS);
  }

  async function ask() {
    asking = new AbortController();
    let status: number | undefined;
    try {
      status = (await fetch("/api/session", { signal: asking.signal })).status;
    } catch {
      status = undefined;
    }
    if (stopped) return;
    if (status === 401) signedOut();
    else if (status === 200) open();
    else later();
  }

  open();
  return stop;
}
