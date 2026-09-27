"use client";

import { useEffect, useRef } from "react";
import { wsUrl } from "./api";
import type { WsEvent } from "./types";

/**
 * Subscribes to Orgion's WebSocket event stream (docs/api.md
 * "WebSocket") and calls `onEvent` for every message. Used to refetch
 * on `file.changed`/`node.updated` so an external Emacs edit shows up
 * without a manual page reload — the reactive half of the v0.1 success
 * condition (docs/mvp.md).
 */
export function useOrgionSocket(onEvent: (event: WsEvent) => void) {
  const handlerRef = useRef(onEvent);
  handlerRef.current = onEvent;

  useEffect(() => {
    let socket: WebSocket | null = null;
    let closedByCleanup = false;
    let retryTimer: ReturnType<typeof setTimeout> | null = null;

    function connect() {
      socket = new WebSocket(wsUrl());
      socket.onmessage = (ev) => {
        try {
          const parsed = JSON.parse(ev.data) as WsEvent;
          handlerRef.current(parsed);
        } catch {
          // ignore malformed frames
        }
      };
      socket.onclose = () => {
        if (!closedByCleanup) {
          retryTimer = setTimeout(connect, 2000);
        }
      };
    }
    connect();

    return () => {
      closedByCleanup = true;
      if (retryTimer) clearTimeout(retryTimer);
      socket?.close();
    };
  }, []);
}
