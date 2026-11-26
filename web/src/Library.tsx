import {
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";
import { InBar } from "./Bar.tsx";
import { isObject, toEntry, type Entry } from "./entry.ts";
import { Link } from "./navigation.tsx";
import { lineText, type Line } from "./upload.ts";
import "./Library.css";

type List = "loading" | "failed" | Entry[];

// A list request that the server refused because the session has ended.
const sessionEnded = Symbol("session ended");

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
// keeps the 3:2 shape that Library.css gives it. The image and its name are
// one link to the image's asset page, named for the image. The image shows
// from its preview, or else its original; the list gives a new card when
// that address changes, so a failure to show the original does not hide a
// preview that arrives later.
function Picture({ entry }: { entry: Entry }) {
  const [ratio, setRatio] = useState<number | null>(null);
  const [broken, setBroken] = useState(false);
  // The ratio is a number the browser measured, never text from the server.
  const style = ratio
    ? ({ "--library-ratio": String(ratio) } as CSSProperties)
    : undefined;
  return (
    <Link
      className="library-item"
      style={style}
      to={`/assets/${entry.id}`}
      aria-label={entry.name}
    >
      <figure className="library-figure">
        {broken ? (
          <p className="library-frame library-broken">
            This image could not be displayed.
          </p>
        ) : (
          <img
            className="library-frame"
            src={entry.imageUrl}
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
    </Link>
  );
}

function Library({
  onSessionLost,
  notice,
  lines,
  busy,
  onUpload,
  saved,
  children,
}: {
  // Called when the server says the session has ended.
  onSessionLost: () => void;
  // A fixed message from the account, such as a failed sign-out.
  notice?: string | null;
  // One line for each file of the signed-in view's last pick, which goes on
  // while another screen shows.
  lines: Line[];
  // True until every file of that pick has its answer.
  busy: boolean;
  // Sends the chosen files, unless a pick is still on its way.
  onUpload: (files: File[]) => void;
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
        aria-label="Image to upload"
        multiple
        hidden
        onChange={(event) => {
          // Copied first: clearing the value can empty the list it read.
          const files = [...(event.currentTarget.files ?? [])];
          // Cleared, so the same files can be chosen again after a refusal.
          event.currentTarget.value = "";
          if (files.length > 0) onUpload(files);
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
          No images yet. Upload an image to start.
        </p>
      )}
      {Array.isArray(list) && list.length > 0 && (
        <div className="library-grid">
          {list.map((item) => (
            <Picture key={`${item.id} ${item.imageUrl}`} entry={item} />
          ))}
        </div>
      )}
    </section>
  );
}

export default Library;
