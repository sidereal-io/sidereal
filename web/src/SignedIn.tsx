import type { ReactNode } from "react";
import AssetPage from "./AssetPage.tsx";
import Library from "./Library.tsx";
import { assetIdOf, usePath } from "./navigation.tsx";
import { useUpload } from "./upload.ts";

// What a signed-in administrator sees: the asset page at an asset page
// address, and the Library at any other. It holds the one upload above the
// screens, so a pick goes on whichever screen shows.
function SignedIn({
  onSessionLost,
  session,
  notice,
  children,
}: {
  // Called when the server says the session has ended.
  onSessionLost: () => void;
  // Aborted as soon as the account learns the session has ended, so no
  // more files are sent, even before the signed-in view goes away.
  session: AbortSignal;
  // A fixed message from the account, such as a failed sign-out.
  notice?: string | null;
  // The account's items, shown at the end of the top bar.
  children?: ReactNode;
}) {
  const { lines, busy, send, saved } = useUpload(onSessionLost, session);
  const id = assetIdOf(usePath());
  if (id !== null) {
    // Keyed on the id, so another Asset starts from its own loading state.
    return (
      <AssetPage key={id} id={id} onSessionLost={onSessionLost} notice={notice}>
        {children}
      </AssetPage>
    );
  }
  return (
    <Library
      onSessionLost={onSessionLost}
      notice={notice}
      lines={lines}
      busy={busy}
      onUpload={send}
      saved={saved}
    >
      {children}
    </Library>
  );
}

export default SignedIn;
