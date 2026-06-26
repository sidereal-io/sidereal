import { useEffect, useRef } from 'react';

class WebSocketManager {
  private ws: WebSocket | null = null;
  private listeners = new Map<string, Set<(data: any) => void>>();
  private reconnectDelay = 1000;
  private maxReconnectDelay = 30000;
  private shouldReconnect = true;

  constructor() {
    this.connect();
  }

  private connect() {
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    this.ws = new WebSocket(`${protocol}//${location.host}/ws`);

    this.ws.onopen = () => {
      console.log('Connected to server via WebSocket');
      this.reconnectDelay = 1000;
    };

    this.ws.onclose = () => {
      console.log('Disconnected from server');
      if (this.shouldReconnect) {
        setTimeout(() => {
          this.reconnectDelay = Math.min(this.reconnectDelay * 2, this.maxReconnectDelay);
          this.connect();
        }, this.reconnectDelay);
      }
    };

    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };

    this.ws.onmessage = (event) => {
      try {
        const { event: eventName, data } = JSON.parse(event.data);
        const handlers = this.listeners.get(eventName);
        if (handlers) {
          for (const handler of handlers) {
            handler(data);
          }
        }
      } catch (err) {
        console.error('Failed to parse WebSocket message:', err);
      }
    };
  }

  on(event: string, handler: (data: any) => void) {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event)!.add(handler);
  }

  off(event: string, handler: (data: any) => void) {
    this.listeners.get(event)?.delete(handler);
  }

  disconnect() {
    this.shouldReconnect = false;
    this.ws?.close();
  }
}

// Singleton instance shared across all hook consumers
let manager: WebSocketManager | null = null;
let refCount = 0;

function getManager(): WebSocketManager {
  if (!manager) {
    manager = new WebSocketManager();
  }
  refCount++;
  return manager;
}

function releaseManager() {
  refCount--;
  if (refCount <= 0 && manager) {
    manager.disconnect();
    manager = null;
    refCount = 0;
  }
}

export function useSocket() {
  const managerRef = useRef<WebSocketManager | null>(null);

  useEffect(() => {
    managerRef.current = getManager();

    return () => {
      releaseManager();
      managerRef.current = null;
    };
  }, []);

  return managerRef.current;
}

export function usePlateSolvingUpdates(callback: (data: any) => void) {
  const socket = useSocket();

  useEffect(() => {
    if (!socket) return;

    socket.on('plate-solving-update', callback);

    return () => {
      socket.off('plate-solving-update', callback);
    };
  }, [socket, callback]);
}

export function useSourceSyncUpdates(callback: (data: any) => void) {
  const socket = useSocket();

  useEffect(() => {
    if (!socket) return;

    socket.on('source-sync-complete', callback);

    return () => {
      socket.off('source-sync-complete', callback);
    };
  }, [socket, callback]);
}
