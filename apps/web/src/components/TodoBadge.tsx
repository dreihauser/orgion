"use client";

const COLORS: Record<string, string> = {
  todo: "bg-amber-100 text-amber-800 dark:bg-amber-900/40 dark:text-amber-300",
  done: "bg-emerald-100 text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-300",
  none: "bg-neutral-100 text-neutral-500 dark:bg-neutral-800 dark:text-neutral-400",
};

export function TodoBadge({
  state,
  type,
  onClick,
  title,
}: {
  state: string | null;
  type: "todo" | "done" | null;
  onClick?: () => void;
  title?: string;
}) {
  const colorKey = type ?? "none";
  return (
    <button
      type="button"
      onClick={onClick}
      title={title}
      className={`rounded px-2 py-0.5 text-xs font-medium tracking-wide transition-opacity hover:opacity-80 ${COLORS[colorKey]}`}
    >
      {state ?? "—"}
    </button>
  );
}
