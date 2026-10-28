import { useEffect, useState } from "react";

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

function CredentialsForm({ label }: { label: string }) {
  return (
    <form
      className="access-form"
      onSubmit={(event) => {
        event.preventDefault();
      }}
    >
      <label>
        Username
        <input name="username" autoComplete="username" required />
      </label>
      <label>
        Password
        <input name="password" type="password" required />
      </label>
      <button type="submit">{label}</button>
    </form>
  );
}

function Access() {
  const [view, setView] = useState<View>("loading");

  useEffect(() => {
    let stopped = false;
    void probe().then((next) => {
      if (!stopped) setView(next);
    });
    return () => {
      stopped = true;
    };
  }, []);

  if (view === "loading") return null;
  if (view === "failed") return <p>Something went wrong</p>;
  if (view === "unclaimed") return <CredentialsForm label="Claim" />;
  if (view === "signed-out") return <CredentialsForm label="Sign in" />;
  return (
    <p>
      Signed in as <strong>{view.username}</strong>{" "}
      <button type="button">Sign out</button>
    </p>
  );
}

export default Access;
