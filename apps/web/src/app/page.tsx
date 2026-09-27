"use client";

import { useCallback, useEffect, useState } from "react";
import { getAgenda, listFiles, listNodesForFile, patchNode } from "@/lib/api";
import type { AgendaResponse, FileSummary, OrgNode } from "@/lib/types";
import { useOrgionSocket } from "@/lib/useOrgionSocket";
import { Sidebar } from "@/components/Sidebar";
import { NodeTable } from "@/components/NodeTable";
import { AgendaPanel } from "@/components/AgendaPanel";

export default function DashboardPage() {
  const [files, setFiles] = useState<FileSummary[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [nodes, setNodes] = useState<OrgNode[]>([]);
  const [agenda, setAgenda] = useState<AgendaResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refreshFiles = useCallback(async () => {
    const fs = await listFiles();
    setFiles(fs);
    setSelectedId((current) => current ?? fs[0]?.id ?? null);
  }, []);

  const refreshNodes = useCallback(async (fileId: string) => {
    setNodes(await listNodesForFile(fileId));
  }, []);

  const refreshAgenda = useCallback(async () => {
    setAgenda(await getAgenda());
  }, []);

  useEffect(() => {
    refreshFiles().catch((e) => setError(String(e)));
    refreshAgenda().catch((e) => setError(String(e)));
  }, [refreshFiles, refreshAgenda]);

  useEffect(() => {
    if (selectedId) {
      refreshNodes(selectedId).catch((e) => setError(String(e)));
    }
  }, [selectedId, refreshNodes]);

  // Live updates: an external (e.g. Emacs) edit reindexes server-side and
  // broadcasts `file.changed`; a same-origin API edit broadcasts
  // `node.updated`. Either way, just refetch — see docs/mvp.md's success
  // condition ("Web UI updates without a manual refresh").
  useOrgionSocket(
    useCallback(
      (event) => {
        if (event.type === "file.changed" || event.type === "node.updated" || event.type === "node.created") {
          if (selectedId) refreshNodes(selectedId).catch(() => {});
          refreshAgenda().catch(() => {});
          refreshFiles().catch(() => {});
        }
      },
      [selectedId, refreshNodes, refreshAgenda, refreshFiles],
    ),
  );

  async function handleCycleTodo(node: OrgNode, next: string | null) {
    try {
      await patchNode(node.id, node.version, { todo_state: next ?? "" });
      if (selectedId) await refreshNodes(selectedId);
      await refreshAgenda();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      if (selectedId) await refreshNodes(selectedId); // resync after a conflict
    }
  }

  const selectedFile = files.find((f) => f.id === selectedId) ?? null;

  return (
    <div className="flex h-screen">
      <Sidebar files={files} selectedId={selectedId} onSelect={setSelectedId} />
      <main className="flex flex-1 overflow-hidden">
        <section className="flex-1 overflow-auto p-6">
          <h1 className="mb-4 text-lg font-semibold">{selectedFile?.path ?? "Select a file"}</h1>
          {error && (
            <div className="mb-3 rounded bg-red-100 px-3 py-2 text-sm text-red-800 dark:bg-red-900/40 dark:text-red-300">
              {error}
            </div>
          )}
          <NodeTable nodes={nodes} onCycleTodo={handleCycleTodo} />
        </section>
        <aside className="w-72 shrink-0 overflow-auto border-l border-neutral-200 p-4 dark:border-neutral-800">
          <h2 className="mb-3 text-sm font-semibold">Agenda</h2>
          <AgendaPanel agenda={agenda} />
        </aside>
      </main>
    </div>
  );
}
