import { createContext, use, type ReactNode } from "react";
import { createPortal } from "react-dom";

// The two places in the top bar that a screen fills: `start`, after the
// wordmark, for the screen's name, and `end`, after the health, for its
// actions. App owns the bar; a screen deep inside Access puts its items
// there, so the bar and health stay mounted when the screen changes.
export interface BarSlots {
  start: HTMLElement | null;
  end: HTMLElement | null;
}

export const BarContext = createContext<BarSlots>({ start: null, end: null });

// Shows its children in one place of the top bar. With no bar, as in a test
// that renders one screen on its own, they show where the screen puts them.
export function InBar({
  at,
  children,
}: {
  at: keyof BarSlots;
  children: ReactNode;
}) {
  const target = use(BarContext)[at];
  return target ? createPortal(children, target) : children;
}
