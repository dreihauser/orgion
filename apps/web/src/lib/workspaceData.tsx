"use client";

import { createContext, useCallback, useContext, useEffect, useState } from "react";
import { getAgenda, listFiles } from "./api";
import type { AgendaResponse, FileSummary, WsEvent } from "./types";
import { useOrgionSocket } from "./useOrgionSocket";

interface WorkspaceDataState {
  files: FileSummary[];
  agenda: AgendaResponse | null;
  refreshFiles: () => Promise<void>;
  refreshAgenda: () => Promise<void>;
  /** Bumped on every WebSocket event; pages that need to refetch
   * something more specific (e.g. one file's node list) watch this. */
  revision: number;
  lastEvent: WsEvent | null;
}

const WorkspaceDataContext = createContext<WorkspaceDataState | null>(null);

/**
 * Fetches the file list + agenda once and keeps them live via the
 * WebSocket (docs/mvp.md's success condition: external edits and
 * same-origin API edits both show up without a manual reload). Mounted
 * once in the authenticated shell so every page shares one socket
 * connection instead of each page opening its own.
 */
export function WorkspaceDataProvider({ children }: { children: React.ReactNode }) {
  const [files, setFiles] = useState<FileSummary[]>([]);
  const [agenda, setAgenda] = useState<AgendaResponse | null>(null);
  const [revision, setRevision] = useState(0);
  const [lastEvent, setLastEvent] = useState<WsEvent | null>(null);

  const refreshFiles = useCallback(async () => {
    setFiles(await listFiles());
  }, []);

  const refreshAgenda = useCallback(async () => {
    setAgenda(await getAgenda());
  }, []);

  useEffect(() => {
    refreshFiles().catch(() => {});
    refreshAgenda().catch(() => {});
  }, [refreshFiles, refreshAgenda]);

  useOrgionSocket(
    useCallback(
      (event) => {
        setLastEvent(event);
        setRevision((r) => r + 1);
        if (event.type === "file.changed" || event.type === "node.updated" || event.type === "node.created") {
          refreshFiles().catch(() => {});
          refreshAgenda().catch(() => {});
        }
      },
      [refreshFiles, refreshAgenda],
    ),
  );

  return (
    <WorkspaceDataContext.Provider value={{ files, agenda, refreshFiles, refreshAgenda, revision, lastEvent }}>
      {children}
    </WorkspaceDataContext.Provider>
  );
}

export function useWorkspaceData(): WorkspaceDataState {
  const ctx = useContext(WorkspaceDataContext);
  if (!ctx) throw new Error("useWorkspaceData must be used within WorkspaceDataProvider");
  return ctx;
}
