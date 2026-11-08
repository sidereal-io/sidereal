import { vi } from "vitest";

// A WebSocket the test plays the server for. Like a browser's, a close the
// page asks for is reported to `onclose` later, not at once.
export class FakeWebSocket {
  static instances: FakeWebSocket[] = [];
  readonly url: string;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;
  closed = false;

  constructor(url: string | URL) {
    this.url = String(url);
    FakeWebSocket.instances.push(this);
  }

  close() {
    if (this.closed) return;
    this.closed = true;
    queueMicrotask(() =>
      this.onclose?.(new CloseEvent("close", { code: 1005 })),
    );
  }

  receive(data: string) {
    this.onmessage?.(new MessageEvent("message", { data }));
  }

  serverClose(code: number) {
    this.closed = true;
    this.onclose?.(new CloseEvent("close", { code }));
  }
}

// Replace the global WebSocket, and answer the list of sockets opened.
export function stubWebSocket(): FakeWebSocket[] {
  FakeWebSocket.instances = [];
  vi.stubGlobal("WebSocket", FakeWebSocket);
  return FakeWebSocket.instances;
}
