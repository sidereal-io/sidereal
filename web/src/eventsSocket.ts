// The server closes the events socket with this code when the session that
// opened it is gone.
const POLICY_CLOSE = 1008;

// The events socket's address on the page's own origin.
export function eventsUrl(page: string): string {
  const url = new URL("/api/events", page);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  return url.href;
}

// Hold the events socket open while the session lives. `onSignedOut` runs
// once the server says the session is gone. The returned function stops
// everything, and nothing fires after it.
export function watchSession(onSignedOut: () => void): () => void {
  let stopped = false;
  let socket: WebSocket | undefined;

  function stop() {
    stopped = true;
    const current = socket;
    socket = undefined;
    current?.close();
  }

  function open() {
    const current = new WebSocket(eventsUrl(location.href));
    socket = current;
    current.onclose = (event) => {
      // A socket this module already let go of reports nothing.
      if (stopped || socket !== current) return;
      socket = undefined;
      if (event.code === POLICY_CLOSE) {
        stop();
        onSignedOut();
      }
    };
  }

  open();
  return stop;
}
