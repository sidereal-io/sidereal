import {
  useSyncExternalStore,
  type CSSProperties,
  type MouseEvent,
  type ReactNode,
} from "react";

// Moving between the signed-in screens without a router library: the
// address is the state, `navigate` adds a history entry, and the browser's
// Back and Forward actions arrive as `popstate`.

// Told when `navigate` changes the address, which fires no `popstate`.
const listeners = new Set<() => void>();

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  window.addEventListener("popstate", listener);
  return () => {
    listeners.delete(listener);
    window.removeEventListener("popstate", listener);
  };
}

function currentPath(): string {
  return window.location.pathname;
}

// The address's path, kept up to date.
export function usePath(): string {
  return useSyncExternalStore(subscribe, currentPath);
}

// Shows `path` without loading the document again, as one new history entry.
export function navigate(path: string): void {
  window.history.pushState(null, "", path);
  for (const listener of listeners) listener();
}

// The same address the server answers with the web shell: a lowercase UUID,
// with nothing after it.
const ASSET_PAGE =
  /^\/assets\/([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})$/;

// The Asset id in an asset page address, or null for any other path.
export function assetIdOf(path: string): string | null {
  return ASSET_PAGE.exec(path)?.[1] ?? null;
}

// A real link, so the browser can open it in a new tab. A plain left click
// shows the address in this tab without a document load; a click with a
// modifier key or another button is left to the browser.
export function Link({
  to,
  className,
  style,
  "aria-label": label,
  children,
}: {
  to: string;
  className?: string;
  style?: CSSProperties;
  "aria-label"?: string;
  children: ReactNode;
}) {
  function follow(event: MouseEvent<HTMLAnchorElement>) {
    if (
      event.defaultPrevented ||
      event.button !== 0 ||
      event.metaKey ||
      event.ctrlKey ||
      event.shiftKey ||
      event.altKey
    ) {
      return;
    }
    event.preventDefault();
    navigate(to);
  }
  return (
    <a
      className={className}
      style={style}
      href={to}
      aria-label={label}
      onClick={follow}
    >
      {children}
    </a>
  );
}
