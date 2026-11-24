import {
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";
import { InBar } from "./Bar.tsx";
import type { Upload } from "./upload.ts";
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
// address the list names elsewhere, and only from its own Asset's route.
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
      typeof item.current_version_id !== "string" ||
      typeof item.original_url !== "string" ||
      !ORIGINAL_URL.test(item.original_url) ||
      item.original_url !== `/api/assets/${item.id}/original`
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
  upload,
  onUpload,
  saved,
  children,
}: {
  // Called when the server says the session has ended.
  onSessionLost: () => void;
  // A fixed message from the account, such as a failed sign-out.
  notice?: string | null;
  // The signed-in view's upload, which goes on while another screen shows.
  upload: Upload;
  // Sends a chosen file, unless another is still on its way.
  onUpload: (file: File) => void;
  // Goes up after each upload the server may have saved.
  saved: number;
  // The account's items, shown in the top bar after the Library's own.
  children?: ReactNode;
}) {
  const [list, setList] = useState<List>("loading");
  // Each change loads the list again, as does each upload that may have
  // been saved, even one that ended while another screen showed.
  const [generation, setGeneration] = useState(0);
  const pickerRef = useRef<HTMLInputElement>(null);

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
  }, [generation, saved, onSessionLost]);

  function retry() {
    setList("loading");
    setGeneration((value) => value + 1);
  }

  return (
    <section className="library" aria-label="Library">
      <InBar at="start">
        <span className="library-title">Library</span>
      </InBar>
      <InBar at="end">
        <button
          className="button-primary"
          type="button"
          disabled={upload.kind === "sending"}
          onClick={() => pickerRef.current?.click()}
        >
          Upload
        </button>
        {children}
      </InBar>
      <input
        ref={pickerRef}
        type="file"
        accept="image/png,image/jpeg"
        aria-label="Image to upload"
        hidden
        onChange={(event) => {
          const file = event.currentTarget.files?.[0];
          // Cleared, so the same file can be chosen again after a refusal.
          event.currentTarget.value = "";
          if (file) onUpload(file);
        }}
      />
      {upload.kind === "sending" && (
        <p className="library-status">Uploading {upload.name}…</p>
      )}
      {upload.kind === "refused" && (
        <p className="library-status">
          {upload.name} was not uploaded: {upload.reason}
        </p>
      )}
      {upload.kind === "unconfirmed" && (
        <p className="library-status">
          No answer came back for {upload.name}. It may have been saved: check
          the library before you upload it again.
        </p>
      )}
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
