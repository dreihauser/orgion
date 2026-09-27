import type { AgendaResponse, FileSummary, MeResponse, OrgNode, User } from "./types";

// Empty (= same-origin, relative paths) by default: next.config.mjs
// proxies /api/* to orgion-server, which is what makes the SameSite=Lax
// session cookie work at all from browser JS (see next.config.mjs's
// comment). Only set NEXT_PUBLIC_ORGION_API_URL to bypass the proxy and
// hit the backend directly — auth won't survive a page reload if the
// two origins differ, since the cookie won't be sent on that cross-site
// fetch.
export const API_BASE = process.env.NEXT_PUBLIC_ORGION_API_URL ?? "";

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

// The API and the web app run on different ports/origins in dev
// (`next dev` on :3000, `orgion serve` on :3030), so every request needs
// `credentials: "include"` for the session cookie to be sent/stored —
// the server's CORS layer mirrors the request origin and allows
// credentials specifically to support this (apps/server/src/api/mod.rs).
function request(path: string, init?: RequestInit): Promise<Response> {
  return fetch(`${API_BASE}${path}`, { credentials: "include", ...init });
}

export function login(username: string, password: string): Promise<User> {
  return request("/api/auth/login", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ username, password }),
  }).then((r) => json<User>(r));
}

export function logout(): Promise<void> {
  return request("/api/auth/logout", { method: "POST" }).then(() => undefined);
}

export function getMe(): Promise<MeResponse> {
  return request("/api/auth/me").then((r) => json<MeResponse>(r));
}

export function listFiles(): Promise<FileSummary[]> {
  return request("/api/files").then((r) => json<FileSummary[]>(r));
}

export function listNodesForFile(fileId: string): Promise<OrgNode[]> {
  return request(`/api/nodes?file=${encodeURIComponent(fileId)}`).then((r) => json<OrgNode[]>(r));
}

export function getAgenda(from?: string, to?: string): Promise<AgendaResponse> {
  const params = new URLSearchParams();
  if (from) params.set("from", from);
  if (to) params.set("to", to);
  const qs = params.toString();
  return request(`/api/agenda${qs ? `?${qs}` : ""}`).then((r) => json<AgendaResponse>(r));
}

export function search(q: string): Promise<{ results: { node: OrgNode; snippet: string }[] }> {
  return request(`/api/search?q=${encodeURIComponent(q)}`).then((r) =>
    json<{ results: { node: OrgNode; snippet: string }[] }>(r),
  );
}

export function patchNode(
  id: string,
  expected_version: string,
  fields: Partial<{ title: string; todo_state: string; tags: string[]; properties: Record<string, string> }>,
): Promise<OrgNode> {
  return request(`/api/nodes/${encodeURIComponent(id)}`, {
    method: "PATCH",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ expected_version, ...fields }),
  }).then((r) => json<OrgNode>(r));
}

export function wsUrl(): string {
  if (API_BASE) {
    return `${API_BASE.replace(/^http/, "ws")}/api/ws`;
  }
  // Same-origin: go through the current page's host so the proxy (or,
  // in production, orgion-server itself) handles the upgrade.
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${protocol}//${window.location.host}/api/ws`;
}
