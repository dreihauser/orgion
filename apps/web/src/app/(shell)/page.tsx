"use client";

import Link from "next/link";
import { useMemo } from "react";
import { useAuth } from "@/lib/auth";
import { useWorkspaceData } from "@/lib/workspaceData";
import { AgendaPanel } from "@/components/AgendaPanel";

function StatCard({ label, value }: { label: string; value: number | string }) {
  return (
    <div className="rounded-lg border border-neutral-200 p-4 dark:border-neutral-800">
      <div className="text-2xl font-semibold">{value}</div>
      <div className="text-xs text-neutral-500">{label}</div>
    </div>
  );
}

export default function DashboardPage() {
  const { user, workspaces } = useAuth();
  const { files, agenda } = useWorkspaceData();

  const totalNodes = useMemo(() => files.reduce((sum, f) => sum + f.node_count, 0), [files]);
  const recentFiles = useMemo(
    () =>
      [...files]
        .sort((a, b) => (a.updated_at < b.updated_at ? 1 : -1))
        .slice(0, 8),
    [files],
  );

  const todayCount = agenda?.today.length ?? 0;
  const overdueCount = agenda?.overdue.length ?? 0;

  return (
    <div className="h-full overflow-auto p-8">
      <h1 className="text-xl font-semibold">
        Welcome back, {user?.display_name ?? user?.username}
      </h1>
      <p className="mt-1 text-sm text-neutral-500">
        {workspaces[0]?.name ?? "Workspace"} · {files.length} file{files.length === 1 ? "" : "s"}
      </p>

      <div className="mt-6 grid grid-cols-2 gap-3 sm:grid-cols-4">
        <StatCard label="Files" value={files.length} />
        <StatCard label="Headings" value={totalNodes} />
        <StatCard label="Due today" value={todayCount} />
        <StatCard label="Overdue" value={overdueCount} />
      </div>

      <div className="mt-8 grid grid-cols-1 gap-8 lg:grid-cols-2">
        <section>
          <h2 className="mb-3 text-sm font-semibold">Agenda</h2>
          <AgendaPanel agenda={agenda} />
        </section>

        <section>
          <h2 className="mb-3 text-sm font-semibold">Recent files</h2>
          <ul className="space-y-1">
            {recentFiles.map((f) => (
              <li key={f.id}>
                <Link
                  href={`/workspace?file=${encodeURIComponent(f.id)}`}
                  className="flex items-center justify-between rounded px-2 py-1.5 text-sm hover:bg-neutral-100 dark:hover:bg-neutral-900"
                >
                  <span className="truncate">{f.path}</span>
                  <span className="shrink-0 text-xs text-neutral-400">{f.node_count}</span>
                </Link>
              </li>
            ))}
            {recentFiles.length === 0 && <li className="text-sm text-neutral-400">No files yet.</li>}
          </ul>
        </section>
      </div>
    </div>
  );
}
