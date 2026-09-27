"use client";

import type { OrgNode } from "@/lib/types";
import { TodoBadge } from "./TodoBadge";

/**
 * Table view over one file's headings (docs/org-mapping.md §1: heading
 * -> row). Clicking a Status badge cycles through the TODO keywords
 * actually observed in this file (derived from the loaded nodes rather
 * than a separate API call — see apps/web README for why).
 */
export function NodeTable({
  nodes,
  onCycleTodo,
}: {
  nodes: OrgNode[];
  onCycleTodo: (node: OrgNode, next: string | null) => void;
}) {
  const cycle = deriveCycle(nodes);

  return (
    <table className="w-full border-collapse text-sm">
      <thead>
        <tr className="border-b border-neutral-200 text-left text-xs uppercase tracking-wide text-neutral-500 dark:border-neutral-800">
          <th className="py-2 pr-4 font-medium">Status</th>
          <th className="py-2 pr-4 font-medium">Title</th>
          <th className="py-2 pr-4 font-medium">Tags</th>
          <th className="py-2 pr-4 font-medium">Scheduled</th>
          <th className="py-2 pr-4 font-medium">Deadline</th>
        </tr>
      </thead>
      <tbody>
        {nodes.map((node) => (
          <tr
            key={node.id}
            className="border-b border-neutral-100 dark:border-neutral-900"
            style={{ paddingLeft: `${(node.level - 1) * 12}px` }}
          >
            <td className="py-2 pr-4">
              <TodoBadge
                state={node.todo_state}
                type={node.todo_type}
                title="Click to cycle status"
                onClick={() => onCycleTodo(node, nextInCycle(cycle, node.todo_state))}
              />
            </td>
            <td className="py-2 pr-4" style={{ paddingLeft: `${(node.level - 1) * 16}px` }}>
              {node.title}
            </td>
            <td className="py-2 pr-4 text-neutral-500">{node.tags.join(", ")}</td>
            <td className="py-2 pr-4 text-neutral-500">{node.scheduled?.date ?? ""}</td>
            <td className="py-2 pr-4 text-neutral-500">{node.deadline?.date ?? ""}</td>
          </tr>
        ))}
        {nodes.length === 0 && (
          <tr>
            <td colSpan={5} className="py-6 text-center text-neutral-400">
              No headings in this file yet.
            </td>
          </tr>
        )}
      </tbody>
    </table>
  );
}

function deriveCycle(nodes: OrgNode[]): (string | null)[] {
  const seen = new Set<string>();
  const order: string[] = [];
  for (const n of nodes) {
    if (n.todo_state && !seen.has(n.todo_state)) {
      seen.add(n.todo_state);
      order.push(n.todo_state);
    }
  }
  return [null, ...order];
}

function nextInCycle(cycle: (string | null)[], current: string | null): string | null {
  const idx = cycle.indexOf(current);
  return cycle[(idx + 1) % cycle.length];
}
