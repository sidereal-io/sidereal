import {
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";
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
// address the list names elsewhere, and only from its own Asset's route.
const ORIGINAL_URL = /^\/api\/assets\/[\w-]+\/original$/;

function isObject(body: unknown): body is Record<string, unknown> {
  return typeof body === "object" && body !== null;
}

// One Asset entry, as the list and a repeat upload's answer hold it, or
// null when it is not a valid entry. Nothing else from it is kept, and the
// name is only ever shown as text.
function toEntry(item: unknown): Entry | null {
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
  return { id: item.id, name: item.name, originalUrl: item.original_url };
}

// The entries of a list body, or null when the body is not a list or holds
// an entry that is not valid.
function entries(body: unknown): Entry[] | null {
  if (!isObject(body) || !Array.isArray(body.assets)) return null;
  const list: Entry[] = [];
  for (const item of body.assets as unknown[]) {
    const checked = toEntry(item);
    if (!checked) return null;
    list.push(checked);
  }
  return list;
}

// The name of the Asset a repeat upload found, or null when the answer's
// body holds no valid entry or names it with nothing to show.
async function foundName(response: Response): Promise<string | null> {
  try {
    const body: unknown = await response.json();
    return isObject(body) ? toEntry(body.asset)?.name || null : null;
  } catch {
    return null;
  }
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

// Why an upload was refused, in fixed words. The server's own words never
// reach the screen.
function refusal(status: number): string {
  switch (status) {
    case 413:
      return "it is larger than the 100 MiB limit.";
    case 415:
      return "only PNG and JPEG images are supported.";
    case 400:
      return "choose one PNG or JPEG file and try again.";
    default:
      return "the server could not save it. Try again later.";
  }
}

// Sends one file as the multipart field `file`. Never retried: a lost
// answer after the server saved the file cannot be told from a failure,
// so no answer at all is "unconfirmed", never a refusal. A 409 means the
// bytes are already in the library: `found` holds that Asset's name.
async function sendFile(
  file: File,
  signal: AbortSignal,
): Promise<
  | "saved"
  | "unconfirmed"
  | { refused: string }
  | { found: string | null }
  | typeof sessionEnded
> {
  let response: Response;
  try {
    const body = new FormData();
    body.append("file", file);
    response = await fetch("/api/assets", {
      method: "POST",
      body,
      signal,
    });
  } catch {
    return "unconfirmed";
  }
  if (response.status === 201) return "saved";
  if (response.status === 401) return sessionEnded;
  if (response.status === 409) return { found: await foundName(response) };
  return { refused: refusal(response.status) };
}

// One picked file's line: waiting, on its way, or its answer.
type Line =
  | { name: string; kind: "waiting" | "sending" | "saved" | "unconfirmed" }
  | { name: string; kind: "refused"; reason: string }
  | { name: string; kind: "found"; existing: string | null };

// The words of one line. Every name is shown as text only.
function lineText(line: Line): string {
  switch (line.kind) {
    case "waiting":
      return `${line.name} is waiting.`;
    case "sending":
      return `Uploading ${line.name}…`;
    case "saved":
      return `${line.name} was uploaded.`;
    case "unconfirmed":
      return `No answer came back for ${line.name}. It is safe to upload it again: if it was saved, Sidereal will find it.`;
    case "refused":
      return `${line.name} was not uploaded: ${line.reason}`;
    case "found":
      return line.existing === null || line.existing === line.name
        ? `${line.name} is already in the library.`
        : `${line.name} is already in the library as ${line.existing}.`;
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
  // One line for each file of the last pick, in pick order.
  const [lines, setLines] = useState<Line[]>([]);
  const pickerRef = useRef<HTMLInputElement>(null);
  // Set while a pick is on its way, so a second choice sends nothing.
  const sendingRef = useRef(false);
  // Aborted when the Library goes away, so a late answer changes nothing.
  const uploadsRef = useRef<AbortController>(null);

  useEffect(() => {
    const controller = new AbortController();
    uploadsRef.current = controller;
    return () => {
      controller.abort();
    };
  }, []);

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

  // Upload waits until every file of the pick has its answer.
  const busy = lines.some(
    (line) => line.kind === "waiting" || line.kind === "sending",
  );

  // Sends the files of one pick in turn, each only after the one before it
  // has an answer or has ended without one.
  async function send(files: File[]) {
    const signal = uploadsRef.current?.signal;
    if (sendingRef.current || !signal) return;
    // Read afresh after each await: an abort can come at any time.
    const stopped = () => signal.aborted;
    sendingRef.current = true;
    // A new pick replaces the last pick's lines.
    setLines(files.map((file) => ({ name: file.name, kind: "waiting" })));
    const mark = (index: number, line: Line) => {
      setLines((current) =>
        current.map((old, at) => (at === index ? line : old)),
      );
    };
    for (const [index, file] of files.entries()) {
      if (stopped()) return;
      const name = file.name;
      mark(index, { name, kind: "sending" });
      const result = await sendFile(file, signal);
      if (stopped()) return;
      if (result === sessionEnded) {
        onSessionLost();
        return;
      } else if (result === "saved") {
        mark(index, { name, kind: "saved" });
        setGeneration((value) => value + 1);
      } else if (result === "unconfirmed") {
        // The file may have been saved, so the list loads again to show it.
        mark(index, { name, kind: "unconfirmed" });
        setGeneration((value) => value + 1);
      } else if ("found" in result) {
        // Nothing changed in the library, so the list is not loaded again.
        mark(index, { name, kind: "found", existing: result.found });
      } else {
        mark(index, { name, kind: "refused", reason: result.refused });
      }
    }
    sendingRef.current = false;
  }

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
          disabled={busy}
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
        multiple
        hidden
        onChange={(event) => {
          // Copied first: clearing the value can empty the list it read.
          const files = [...(event.currentTarget.files ?? [])];
          // Cleared, so the same files can be chosen again after a refusal.
          event.currentTarget.value = "";
          if (files.length > 0) void send(files);
        }}
      />
      {lines.length > 0 && (
        <ul className="library-uploads" aria-label="Uploads">
          {lines.map((line, index) => (
            // eslint-disable-next-line @eslint-react/no-array-index-key -- A pick's lines are only ever replaced whole, so an index is stable.
            <li key={index} className="library-status">
              {lineText(line)}
            </li>
          ))}
        </ul>
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
