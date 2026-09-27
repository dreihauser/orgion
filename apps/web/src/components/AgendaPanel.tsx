"use client";

import type { AgendaItem, AgendaResponse } from "@/lib/types";
import { TodoBadge } from "./TodoBadge";

function Row({ item }: { item: AgendaItem }) {
  return (
    <div className="flex items-center gap-2 py-1 text-sm">
      <TodoBadge state={item.todo_state} type={item.todo_type} />
      <span className="flex-1 truncate">{item.title}</span>
      <span className="shrink-0 text-xs uppercase text-neutral-400">{item.agenda_kind}</span>
    </div>
  );
}

export function AgendaPanel({ agenda }: { agenda: AgendaResponse | null }) {
  if (!agenda) {
    return <div className="text-sm text-neutral-400">Loading agenda…</div>;
  }

  const upcomingDates = Object.keys(agenda.upcoming).sort();

  return (
    <div className="space-y-4">
      <section>
        <h3 className="mb-1 text-xs font-semibold uppercase tracking-wide text-neutral-500">Today</h3>
        {agenda.today.length === 0 && <p className="text-sm text-neutral-400">Nothing scheduled today.</p>}
        {agenda.today.map((item) => (
          <Row key={`${item.id}-${item.agenda_kind}`} item={item} />
        ))}
      </section>

      {agenda.overdue.length > 0 && (
        <section>
          <h3 className="mb-1 text-xs font-semibold uppercase tracking-wide text-red-500">Overdue</h3>
          {agenda.overdue.map((item) => (
            <Row key={`${item.id}-overdue`} item={item} />
          ))}
        </section>
      )}

      <section>
        <h3 className="mb-1 text-xs font-semibold uppercase tracking-wide text-neutral-500">Upcoming</h3>
        {upcomingDates.length === 0 && <p className="text-sm text-neutral-400">Nothing in range.</p>}
        {upcomingDates.map((date) => (
          <div key={date} className="mb-2">
            <div className="text-xs text-neutral-400">{date}</div>
            {agenda.upcoming[date].map((item) => (
              <Row key={`${item.id}-${date}-${item.agenda_kind}`} item={item} />
            ))}
          </div>
        ))}
      </section>
    </div>
  );
}
