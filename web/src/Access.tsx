import { useEffect, useState } from "react";
import "./Access.css";

type View =
  "loading" | "unclaimed" | "signed-out" | "failed" | { username: string };

// A status this section does not expect means the same as no answer.
class Unexpected extends Error {}

function isObject(body: unknown): body is Record<string, unknown> {
  return typeof body === "object" && body !== null;
}

async function signedInUser(): Promise<View> {
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
async function probe(): Promise<View> {
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

type Credentials = { username: string; password: string };

// What a refused claim or sign-in shows. The server's own words never reach
// the screen.
function refusal(status: number): string {
  return status === 400 || status === 401
    ? "Invalid username or password"
    : "Something went wrong";
}

async function send(
  method: string,
  url: string,
  credentials?: Credentials,
): Promise<Response | undefined> {
  try {
    return await fetch(url, {
      method,
      headers: credentials ? { "Content-Type": "application/json" } : {},
      body: credentials ? JSON.stringify(credentials) : undefined,
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

  useEffect(() => {
    let stopped = false;
    void probe().then((next) => {
      if (!stopped) setView(next);
    });
    return () => {
      stopped = true;
    };
  }, []);

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
    if (response?.status !== 200) {
      setMessage(refusal(response?.status ?? 0));
      return;
    }
    try {
      const body: unknown = await response.json();
      if (!isObject(body) || typeof body.username !== "string") {
        throw new Unexpected();
      }
      show({ username: body.username });
    } catch {
      setMessage("Something went wrong");
    }
  }

  async function signOut() {
    const response = await send("DELETE", "/api/session");
    // A 401 means the session had already ended.
    if (response?.status === 204 || response?.status === 401)
      show("signed-out");
    else setMessage("Something went wrong");
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
    <div className="access-form">
      <p>
        Signed in as <strong>{view.username}</strong>
      </p>
      <button
        className="access-button"
        type="button"
        onClick={() => void signOut()}
      >
        Sign out
      </button>
      {message && <p className="access-message">{message}</p>}
    </div>
  );
}

export default Access;
