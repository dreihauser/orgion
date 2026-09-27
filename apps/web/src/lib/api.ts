import type { AgendaResponse, FileSummary, OrgNode } from "./types";

// Same-origin by default (the server serves the built web UI itself in
// production, see docker/); override for `next dev` against a
// separately-running `orgion serve`.
export const API_BASE = process.env.NEXT_PUBLIC_ORGION_API_URL ?? "http://localhost:3030";

async function json<T>(res: Response): Promise<T> {
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    const message = body?.error?.message ?? res.statusText;
    throw new ApiConflictOrError(res.status, message, body);
  }
  return res.json() as Promise<T>;
}

export class ApiConflictOrError extends Error {
  status: number;
  body: unknown;
  constructor(status: number, message: string, body: unknown) {
    super(message);
    this.status = status;
    this.body = body;
  }
}

export function listFiles(): Promise<FileSummary[]> {
  return fetch(`${API_BASE}/api/files`).then((r) => json<FileSummary[]>(r));
}

export function listNodesForFile(fileId: string): Promise<OrgNode[]> {
  return fetch(`${API_BASE}/api/nodes?file=${encodeURIComponent(fileId)}`).then((r) => json<OrgNode[]>(r));
}

export function getAgenda(from?: string, to?: string): Promise<AgendaResponse> {
  const params = new URLSearchParams();
  if (from) params.set("from", from);
  if (to) params.set("to", to);
  const qs = params.toString();
  return fetch(`${API_BASE}/api/agenda${qs ? `?${qs}` : ""}`).then((r) => json<AgendaResponse>(r));
}

export function search(q: string): Promise<{ results: { node: OrgNode; snippet: string }[] }> {
  return fetch(`${API_BASE}/api/search?q=${encodeURIComponent(q)}`).then((r) =>
    json<{ results: { node: OrgNode; snippet: string }[] }>(r),
  );
}

export function patchNode(
  id: string,
  expected_version: string,
  fields: Partial<{ title: string; todo_state: string; tags: string[]; properties: Record<string, string> }>,
): Promise<OrgNode> {
  return fetch(`${API_BASE}/api/nodes/${encodeURIComponent(id)}`, {
    method: "PATCH",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ expected_version, ...fields }),
  }).then((r) => json<OrgNode>(r));
}

export function wsUrl(): string {
  const base = API_BASE.replace(/^http/, "ws");
  return `${base}/api/ws`;
}
