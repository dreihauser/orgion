// Mirrors apps/server's DTOs (src/dto.rs) and docs/api.md's Node shape.

export interface TimestampRecord {
  date: string; // YYYY-MM-DD
  time: string | null;
  active: boolean;
  repeater: string | null;
}

export interface OrgNode {
  id: string;
  org_id: string | null;
  file_id: string;
  file_path: string;
  parent_id: string | null;
  level: number;
  title: string;
  todo_state: string | null;
  todo_type: "todo" | "done" | null;
  priority: string | null;
  tags: string[];
  scheduled: TimestampRecord | null;
  deadline: TimestampRecord | null;
  closed: TimestampRecord | null;
  properties: Record<string, string>;
  body: string;
  version: string;
}

export interface AgendaItem extends OrgNode {
  agenda_date: string;
  agenda_kind: "scheduled" | "deadline";
}

export interface AgendaResponse {
  today: AgendaItem[];
  overdue: AgendaItem[];
  upcoming: Record<string, AgendaItem[]>;
}

export interface FileSummary {
  id: string;
  path: string;
  node_count: number;
  updated_at: string;
}

export type WsEvent =
  | { type: "node.updated"; node: OrgNode }
  | { type: "node.created"; node: OrgNode }
  | { type: "node.deleted"; node_id: string }
  | { type: "file.changed"; file_id: string; file_path: string };
