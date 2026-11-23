import { useEffect, useState, type CSSProperties, type ReactNode } from "react";
import { InBar } from "./Bar.tsx";
import "./Library.css";

interface Entry {
  id: string;
  name: string;
  originalUrl: string;
}

type List = "loading" | "failed" | Entry[];

// A list request that the server refused because the session has ended.
const sessionEnded = Symbol("session ended");

// An original is always fetched from this server's API, never from an
// address the list names elsewhere.
const ORIGINAL_URL = /^\/api\/assets\/[\w-]+\/original$/;

function isObject(body: unknown): body is Record<string, unknown> {
  return typeof body === "object" && body !== null;
}

// The entries of a list body, or null when the body is not a list. Nothing
// else from the body is kept, and the names are only ever shown as text.
function entries(body: unknown): Entry[] | null {
  if (!isObject(body) || !Array.isArray(body.assets)) return null;
  const list: Entry[] = [];
  for (const item of body.assets as unknown[]) {
    if (
      !isObject(item) ||
      typeof item.id !== "string" ||
      typeof item.name !== "string" ||
      typeof item.original_url !== "string" ||
      !ORIGINAL_URL.test(item.original_url)
    ) {
      return null;
    }
    list.push({ id: item.id, name: item.name, originalUrl: item.original_url });
  }
  return list;
}

async function fetchList(
  signal: AbortSignal,
): Promise<Entry[] | "failed" | typeof sessionEnded> {
  try {
    const response = await fetch("/api/assets", { signal });
    if (response.status === 401) return sessionEnded;
    if (response.status !== 200) return "failed";
    return entries(await response.json()) ?? "failed";
  } catch {
    return "failed";
  }
}

// One image, whole, at its own shape once it loads. Until then its frame
// keeps the 3:2 shape that Library.css gives it.
function Picture({ entry }: { entry: Entry }) {
  const [ratio, setRatio] = useState<number | null>(null);
  const [broken, setBroken] = useState(false);
  // The ratio is a number the browser measured, never text from the server.
  const style = ratio
    ? ({ "--library-ratio": String(ratio) } as CSSProperties)
    : undefined;
  return (
    <figure className="library-item" style={style}>
      {broken ? (
        <p className="library-frame library-broken">
          This image could not be displayed.
        </p>
      ) : (
        <img
          className="library-frame"
          src={entry.originalUrl}
          alt={entry.name}
          onLoad={(event) => {
            const { naturalWidth, naturalHeight } = event.currentTarget;
            if (naturalWidth > 0 && naturalHeight > 0) {
              setRatio(naturalWidth / naturalHeight);
            }
          }}
          onError={() => {
            setBroken(true);
          }}
        />
      )}
      <figcaption className="library-name">{entry.name}</figcaption>
    </figure>
  );
}

function Library({
  onSessionLost,
  notice,
  children,
}: {
  // Called when the server says the session has ended.
  onSessionLost: () => void;
  // A fixed message from the account, such as a failed sign-out.
  notice?: string | null;
  // The account's items, shown in the top bar after the Library's own.
  children?: ReactNode;
}) {
  const [list, setList] = useState<List>("loading");
  // Each change loads the list again.
  const [generation, setGeneration] = useState(0);

  useEffect(() => {
    // Aborted when the Library goes away or loads again, so a late answer
    // never shows data the user can no longer see.
    const controller = new AbortController();
    void fetchList(controller.signal).then((result) => {
      if (controller.signal.aborted) return;
      if (result === sessionEnded) onSessionLost();
      else setList(result);
    });
    return () => {
      controller.abort();
    };
  }, [generation, onSessionLost]);

  function retry() {
    setList("loading");
    setGeneration((value) => value + 1);
  }

  return (
    <section className="library" aria-label="Library">
      <InBar at="start">
        <span className="library-title">Library</span>
      </InBar>
      <InBar at="end">{children}</InBar>
      {notice && <p className="library-status">{notice}</p>}
      {list === "loading" && (
        <p className="library-status">Loading the library…</p>
      )}
      {list === "failed" && (
        <p className="library-status">
          The library could not be loaded.{" "}
          <button className="button-quiet" type="button" onClick={retry}>
            Retry
          </button>
        </p>
      )}
      {Array.isArray(list) && list.length === 0 && (
        <p className="library-status">
          No images yet. Upload a PNG or JPEG to start.
        </p>
      )}
      {Array.isArray(list) && list.length > 0 && (
        <div className="library-grid">
          {list.map((item) => (
            <Picture key={item.id} entry={item} />
          ))}
        </div>
      )}
    </section>
  );
}

export default Library;
