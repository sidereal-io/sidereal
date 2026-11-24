import type { ReactNode } from "react";
import Library from "./Library.tsx";
import { useUpload } from "./upload.ts";

// What a signed-in administrator sees. It holds the one upload above the
// screens, so an upload goes on whichever screen shows.
function SignedIn({
  onSessionLost,
  notice,
  children,
}: {
  // Called when the server says the session has ended.
  onSessionLost: () => void;
  // A fixed message from the account, such as a failed sign-out.
  notice?: string | null;
  // The account's items, shown at the end of the top bar.
  children?: ReactNode;
}) {
  const { upload, send, saved } = useUpload(onSessionLost);
  return (
    <Library
      onSessionLost={onSessionLost}
      notice={notice}
      upload={upload}
      onUpload={send}
      saved={saved}
    >
      {children}
    </Library>
  );
}

export default SignedIn;
