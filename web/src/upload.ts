import { useCallback, useEffect, useRef, useState } from "react";
import { isObject, toEntry } from "./entry.ts";

// One picked file's line: waiting, on its way, or its answer.
export type Line =
  | { name: string; kind: "waiting" | "sending" | "saved" | "unconfirmed" }
  | { name: string; kind: "refused"; reason: string }
  | { name: string; kind: "found"; existing: string | null };

// An upload that the server refused because the session has ended.
const sessionEnded = Symbol("session ended");

// The words of one line. Every name is shown as text only.
export function lineText(line: Line): string {
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

// The signed-in view's uploads. They live above the screen switch, so a
// pick goes on while another screen shows, and only one pick is ever on its
// way. Its requests stop when the signed-in view goes away, and no more
// files are sent once `session` is aborted, even before it goes away.
export function useUpload(
  onSessionLost: () => void,
  session: AbortSignal,
): {
  // One line for each file of the last pick, in pick order.
  lines: Line[];
  // True until every file of the pick has its answer.
  busy: boolean;
  // Sends the files of one pick in turn, unless a pick is still on its way.
  send: (files: File[]) => void;
  // Goes up after each upload the server may have saved, so the Library
  // loads its list again.
  saved: number;
} {
  const [lines, setLines] = useState<Line[]>([]);
  const [saved, setSaved] = useState(0);
  // Set while a pick is on its way, so a second choice sends nothing.
  const sendingRef = useRef(false);
  // Aborted when the signed-in view goes away, so a late answer changes
  // nothing.
  const controllerRef = useRef<AbortController>(null);
  // The latest callback and session, so a new one never restarts the
  // controller.
  const sessionLostRef = useRef(onSessionLost);
  const sessionRef = useRef(session);

  useEffect(() => {
    sessionLostRef.current = onSessionLost;
    sessionRef.current = session;
  }, [onSessionLost, session]);

  useEffect(() => {
    const controller = new AbortController();
    controllerRef.current = controller;
    return () => {
      controller.abort();
    };
  }, []);

  // Each file goes only after the one before it has an answer or has ended
  // without one.
  const send = useCallback((files: File[]) => {
    const uploads = controllerRef.current?.signal;
    if (sendingRef.current || !uploads) return;
    const signal = AbortSignal.any([sessionRef.current, uploads]);
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
    void (async () => {
      for (const [index, file] of files.entries()) {
        if (stopped()) return;
        const name = file.name;
        mark(index, { name, kind: "sending" });
        const result = await sendFile(file, signal);
        if (stopped()) return;
        if (result === sessionEnded) {
          sessionLostRef.current();
          return;
        } else if (result === "saved") {
          mark(index, { name, kind: "saved" });
          setSaved((value) => value + 1);
        } else if (result === "unconfirmed") {
          // The file may have been saved, so the list loads again to show it.
          mark(index, { name, kind: "unconfirmed" });
          setSaved((value) => value + 1);
        } else if ("found" in result) {
          // Nothing changed in the library, so the list is not loaded again.
          mark(index, { name, kind: "found", existing: result.found });
        } else {
          mark(index, { name, kind: "refused", reason: result.refused });
        }
      }
      sendingRef.current = false;
    })();
  }, []);

  const busy = lines.some(
    (line) => line.kind === "waiting" || line.kind === "sending",
  );

  return { lines, busy, send, saved };
}
