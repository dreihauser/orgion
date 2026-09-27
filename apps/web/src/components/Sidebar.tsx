"use client";

import type { FileSummary } from "@/lib/types";

export function Sidebar({
  files,
  selectedId,
  onSelect,
}: {
  files: FileSummary[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}) {
  return (
    <nav className="w-56 shrink-0 border-r border-neutral-200 p-3 dark:border-neutral-800">
      <div className="mb-3 px-1 text-sm font-semibold">Orgion</div>
      <div className="mb-1 px-1 text-xs font-semibold uppercase tracking-wide text-neutral-400">Files</div>
      <ul className="space-y-0.5">
        {files.map((f) => (
          <li key={f.id}>
            <button
              type="button"
              onClick={() => onSelect(f.id)}
              className={`w-full truncate rounded px-2 py-1 text-left text-sm ${
                selectedId === f.id
                  ? "bg-neutral-200 dark:bg-neutral-800"
                  : "hover:bg-neutral-100 dark:hover:bg-neutral-900"
              }`}
            >
              {f.path}
              <span className="ml-1 text-xs text-neutral-400">({f.node_count})</span>
            </button>
          </li>
        ))}
        {files.length === 0 && <li className="px-2 py-1 text-sm text-neutral-400">No files indexed yet.</li>}
      </ul>
    </nav>
  );
}
