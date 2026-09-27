"use client";

import { Suspense, useCallback, useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";
import { listNodesForFile, patchNode } from "@/lib/api";
import type { OrgNode } from "@/lib/types";
import { useWorkspaceData } from "@/lib/workspaceData";
import { NodeTable } from "@/components/NodeTable";

function WorkspacePageInner() {
  const searchParams = useSearchParams();
  const fileId = searchParams.get("file");
  const { files, revision, refreshFiles, refreshAgenda } = useWorkspaceData();

  const [nodes, setNodes] = useState<OrgNode[]>([]);
  const [error, setError] = useState<string | null>(null);

  const refreshNodes = useCallback(async () => {
    if (!fileId) return;
    setNodes(await listNodesForFile(fileId));
  }, [fileId]);

  useEffect(() => {
    refreshNodes().catch((e) => setError(String(e)));
  }, [refreshNodes]);

  // Refetch this file's nodes whenever the shared WebSocket connection
  // (mounted once in the shell layout) sees any change.
  useEffect(() => {
    if (revision > 0) refreshNodes().catch(() => {});
  }, [revision, refreshNodes]);

  async function handleCycleTodo(node: OrgNode, next: string | null) {
    try {
      await patchNode(node.id, node.version, { todo_state: next ?? "" });
      await refreshNodes();
      await refreshAgenda();
      await refreshFiles();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      await refreshNodes(); // resync after a conflict
    }
  }

  const selectedFile = files.find((f) => f.id === fileId) ?? null;

  return (
    <div className="h-full overflow-auto p-6">
      <h1 className="mb-4 text-lg font-semibold">{selectedFile?.path ?? "Select a file from the sidebar"}</h1>
      {error && (
        <div className="mb-3 rounded bg-red-100 px-3 py-2 text-sm text-red-800 dark:bg-red-900/40 dark:text-red-300">
          {error}
        </div>
      )}
      {fileId ? (
        <NodeTable nodes={nodes} onCycleTodo={handleCycleTodo} />
      ) : (
        <p className="text-sm text-neutral-400">Pick a file from the tree on the left to see its headings.</p>
      )}
    </div>
  );
}

export default function WorkspacePage() {
  return (
    <Suspense fallback={<div className="p-6 text-sm text-neutral-400">Loading…</div>}>
      <WorkspacePageInner />
    </Suspense>
  );
}
