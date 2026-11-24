import { useCallback, useEffect, useRef, useState } from "react";

// One upload as the Library shows it.
export type Upload =
  | { kind: "idle" }
  | { kind: "sending"; name: string }
  | { kind: "refused"; name: string; reason: string }
  | { kind: "unconfirmed"; name: string };

// An upload that the server refused because the session has ended.
const sessionEnded = Symbol("session ended");

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
// so no answer at all is "unconfirmed", never a refusal.
async function sendFile(
  file: File,
  signal: AbortSignal,
): Promise<
  "saved" | "unconfirmed" | { refused: string } | typeof sessionEnded
> {
  try {
    const body = new FormData();
    body.append("file", file);
    const response = await fetch("/api/assets", {
      method: "POST",
      body,
      signal,
    });
    if (response.status === 201) return "saved";
    if (response.status === 401) return sessionEnded;
    return { refused: refusal(response.status) };
  } catch {
    return "unconfirmed";
  }
}

// The signed-in view's one upload. It lives above the screen switch, so an
// upload goes on while another screen shows, and only one is ever on its
// way. Its request is aborted only when the signed-in view goes away: on
// sign-out or a lost session.
export function useUpload(onSessionLost: () => void): {
  upload: Upload;
  // Sends one file, unless another is still on its way.
  send: (file: File) => void;
  // Goes up after each upload the server may have saved, so the Library
  // loads its list again.
  saved: number;
} {
  const [upload, setUpload] = useState<Upload>({ kind: "idle" });
  const [saved, setSaved] = useState(0);
  // Set while a file is on its way, so a second choice sends nothing.
  const sendingRef = useRef(false);
  // Aborted when the signed-in view goes away, so a late answer changes
  // nothing.
  const controllerRef = useRef<AbortController>(null);
  // The latest callback, so a new one never restarts the controller.
  const sessionLostRef = useRef(onSessionLost);

  useEffect(() => {
    sessionLostRef.current = onSessionLost;
  }, [onSessionLost]);

  useEffect(() => {
    const controller = new AbortController();
    controllerRef.current = controller;
    return () => {
      controller.abort();
    };
  }, []);

  const send = useCallback((file: File) => {
    const signal = controllerRef.current?.signal;
    if (sendingRef.current || !signal) return;
    sendingRef.current = true;
    setUpload({ kind: "sending", name: file.name });
    void sendFile(file, signal).then((result) => {
      sendingRef.current = false;
      if (signal.aborted) return;
      if (result === sessionEnded) {
        sessionLostRef.current();
      } else if (result === "saved") {
        setUpload({ kind: "idle" });
        setSaved((value) => value + 1);
      } else if (result === "unconfirmed") {
        // The file may have been saved, so the list loads again to show it.
        setUpload({ kind: "unconfirmed", name: file.name });
        setSaved((value) => value + 1);
      } else {
        setUpload({ kind: "refused", name: file.name, reason: result.refused });
      }
    });
  }, []);

  return { upload, send, saved };
}
