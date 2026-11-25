import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type RefObject,
} from "react";
import "./Access.css";
import { watchSession } from "./eventsSocket.ts";
import Library from "./Library.tsx";

// What the first check finds.
type Found = "unclaimed" | "signed-out" | "failed" | { username: string };

// A signed-in view carries the controller of its own session signal, and
// the number of that controller, which keys its Library.
type View =
  | "loading"
  | Exclude<Found, object>
  | { username: string; session: AbortController; key: number };

// A status this section does not expect means the same as no answer.
class Unexpected extends Error {}

function isObject(body: unknown): body is Record<string, unknown> {
  return typeof body === "object" && body !== null;
}

async function signedInUser(): Promise<Found> {
  const response = await fetch("/api/session");
  if (response.status === 401) return "signed-out";
  if (response.status !== 200) throw new Unexpected();
  const body: unknown = await response.json();
  if (!isObject(body) || typeof body.username !== "string") {
    throw new Unexpected();
  }
  return { username: body.username };
}

// Nothing from a response body is shown on screen except the username.
async function probe(): Promise<Found> {
  try {
    const response = await fetch("/api/claim");
    if (response.status !== 200) return "failed";
    const body: unknown = await response.json();
    if (!isObject(body) || typeof body.open !== "boolean") return "failed";
    return body.open ? "unclaimed" : await signedInUser();
  } catch {
    return "failed";
  }
}

// A signed-in view and its session signal, which becomes the current one.
// Every end of a signed-in view aborts its controller, so one not yet
// aborted belongs to the view on screen. A second sign-in that answers then,
// as after a double submit, keeps it: the Library stays mounted, and a pick
// it started must still stop when the session ends. A new controller gets
// the next number, so its view gets a fresh Library even if React never
// showed the sign-in form in between.
function signedInView(
  current: RefObject<AbortController | null>,
  made: RefObject<number>,
  username: string,
): View {
  const live = current.current;
  const session = live && !live.signal.aborted ? live : new AbortController();
  if (session !== live) made.current += 1;
  current.current = session;
  return { username, session, key: made.current };
}

type Credentials = { username: string; password: string };

// What a refused claim or sign-in shows. The server's own words never reach
// the screen.
function refusal(status: number): string {
  return status === 400 || status === 401
    ? "Invalid username or password"
    : "Something went wrong";
}

// What a sign-in from a limited address shows. Only a whole number of
// seconds in Retry-After is believed.
function limitedMessage(retryAfter: string | null): string {
  if (retryAfter === null || !/^\d+$/.test(retryAfter)) {
    return "Too many failed sign-ins. Try again later.";
  }
  const seconds = Number(retryAfter);
  const [count, unit] =
    seconds < 60 ? [seconds, "second"] : [Math.ceil(seconds / 60), "minute"];
  const wait = `${String(count)} ${unit}${count === 1 ? "" : "s"}`;
  return `Too many failed sign-ins. Try again in ${wait}.`;
}

async function send(
  method: string,
  url: string,
  credentials?: Credentials,
  signal?: AbortSignal,
): Promise<Response | undefined> {
  try {
    return await fetch(url, {
      method,
      headers: credentials ? { "Content-Type": "application/json" } : {},
      body: credentials ? JSON.stringify(credentials) : undefined,
      signal,
    });
  } catch {
    return undefined;
  }
}

function text(form: FormData, name: string): string {
  const value = form.get(name);
  return typeof value === "string" ? value : "";
}

function CredentialsForm({
  label,
  message,
  onSubmit,
}: {
  label: string;
  message: string | null;
  onSubmit: (credentials: Credentials) => void;
}) {
  return (
    <form
      className="access-form"
      onSubmit={(event) => {
        event.preventDefault();
        const form = new FormData(event.currentTarget);
        onSubmit({
          username: text(form, "username"),
          password: text(form, "password"),
        });
      }}
    >
      <label className="access-field">
        Username
        <input name="username" autoComplete="username" required />
      </label>
      <label className="access-field">
        Password
        <input
          name="password"
          type="password"
          autoComplete={label === "Claim" ? "new-password" : "current-password"}
          required
        />
      </label>
      <button className="access-button" type="submit">
        {label}
      </button>
      {message && <p className="access-message">{message}</p>}
    </form>
  );
}

function Access() {
  const [view, setView] = useState<View>("loading");
  const [message, setMessage] = useState<string | null>(null);
  // The current signed-in view's session controller. It is aborted in the
  // same call that learns the session ended, before the view changes, so
  // the Library sends nothing more even before it goes away.
  const sessionRef = useRef<AbortController>(null);
  // How many session controllers this screen has made.
  const madeRef = useRef(0);

  useEffect(() => {
    let stopped = false;
    void probe().then((next) => {
      if (stopped) return;
      setView(
        typeof next === "object"
          ? signedInView(sessionRef, madeRef, next.username)
          : next,
      );
    });
    return () => {
      stopped = true;
    };
  }, []);

  // The session ended, as the events socket or a library request found.
  const sessionLost = useCallback(() => {
    sessionRef.current?.abort();
    setMessage(null);
    setView("signed-out");
  }, []);

  // Keyed on the username, so a re-render of the same user keeps its socket.
  const signedIn = typeof view === "object" ? view.username : null;
  useEffect(() => {
    if (signedIn === null) return;
    return watchSession(sessionLost);
  }, [signedIn, sessionLost]);

  function show(next: View) {
    setMessage(null);
    setView(next);
  }

  async function claim(credentials: Credentials) {
    const response = await send("POST", "/api/claim", credentials);
    if (response?.status === 201) show("signed-out");
    else setMessage(refusal(response?.status ?? 0));
  }

  async function signIn(credentials: Credentials) {
    const response = await send("POST", "/api/session", credentials);
    if (response?.status === 429) {
      setMessage(limitedMessage(response.headers.get("Retry-After")));
      return;
    }
    if (response?.status !== 200) {
      setMessage(refusal(response?.status ?? 0));
      return;
    }
    try {
      const body: unknown = await response.json();
      if (!isObject(body) || typeof body.username !== "string") {
        throw new Unexpected();
      }
      show(signedInView(sessionRef, madeRef, body.username));
    } catch {
      setMessage("Something went wrong");
    }
  }

  async function signOut() {
    // The signed-in view this sign-out comes from.
    const session = sessionRef.current;
    // Cancelled when that view ends, so a late answer cannot clear the
    // cookie of a newer sign-in.
    const response = await send(
      "DELETE",
      "/api/session",
      undefined,
      session?.signal,
    );
    // That view has ended already, so its answer changes nothing.
    if (session !== sessionRef.current || session?.signal.aborted) return;
    // A 401 means the session had already ended.
    if (response?.status === 204 || response?.status === 401) {
      session?.abort();
      show("signed-out");
    } else setMessage("Something went wrong");
  }

  if (view === "loading") return null;
  if (view === "failed") return <p>Something went wrong</p>;
  if (view === "unclaimed") {
    return (
      <CredentialsForm
        label="Claim"
        message={message}
        onSubmit={(credentials) => void claim(credentials)}
      />
    );
  }
  if (view === "signed-out") {
    return (
      <CredentialsForm
        label="Sign in"
        message={message}
        onSubmit={(credentials) => void signIn(credentials)}
      />
    );
  }
  return (
    <Library
      key={view.key}
      onSessionLost={sessionLost}
      session={view.session.signal}
      notice={message}
    >
      <span className="access-user">{view.username}</span>
      <button
        className="button-quiet"
        type="button"
        onClick={() => void signOut()}
      >
        Sign out
      </button>
    </Library>
  );
}

export default Access;
