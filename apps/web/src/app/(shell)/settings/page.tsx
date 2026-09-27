"use client";

import { useRouter } from "next/navigation";
import { useAuth } from "@/lib/auth";
import { useTheme, type ThemeMode } from "@/lib/theme";

const THEME_OPTIONS: { value: ThemeMode; label: string }[] = [
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
  { value: "system", label: "System" },
];

export default function SettingsPage() {
  const { user, workspaces, logout } = useAuth();
  const { mode, setMode } = useTheme();
  const router = useRouter();

  async function handleLogout() {
    await logout();
    router.push("/login");
  }

  return (
    <div className="h-full overflow-auto p-8">
      <h1 className="mb-6 text-lg font-semibold">Settings</h1>

      <section className="mb-8 max-w-md">
        <h2 className="mb-2 text-sm font-semibold uppercase tracking-wide text-neutral-500">Appearance</h2>
        <div className="flex gap-2">
          {THEME_OPTIONS.map((opt) => (
            <button
              key={opt.value}
              type="button"
              onClick={() => setMode(opt.value)}
              className={`rounded border px-3 py-1.5 text-sm ${
                mode === opt.value
                  ? "border-neutral-900 bg-neutral-900 text-white dark:border-neutral-100 dark:bg-neutral-100 dark:text-neutral-900"
                  : "border-neutral-300 hover:bg-neutral-100 dark:border-neutral-700 dark:hover:bg-neutral-900"
              }`}
            >
              {opt.label}
            </button>
          ))}
        </div>
      </section>

      <section className="mb-8 max-w-md">
        <h2 className="mb-2 text-sm font-semibold uppercase tracking-wide text-neutral-500">Account</h2>
        <dl className="space-y-1 text-sm">
          <div className="flex justify-between">
            <dt className="text-neutral-500">Username</dt>
            <dd>{user?.username}</dd>
          </div>
          <div className="flex justify-between">
            <dt className="text-neutral-500">Display name</dt>
            <dd>{user?.display_name ?? "—"}</dd>
          </div>
        </dl>
      </section>

      <section className="mb-8 max-w-md">
        <h2 className="mb-2 text-sm font-semibold uppercase tracking-wide text-neutral-500">Workspace</h2>
        {workspaces.map((w) => (
          <div key={w.id} className="flex justify-between text-sm">
            <dt className="text-neutral-500">Name</dt>
            <dd>{w.name}</dd>
          </div>
        ))}
        {workspaces.length === 0 && <p className="text-sm text-neutral-400">No workspace membership found.</p>}
      </section>

      <button
        type="button"
        onClick={handleLogout}
        className="rounded border border-red-300 px-3 py-1.5 text-sm text-red-700 hover:bg-red-50 dark:border-red-900 dark:text-red-400 dark:hover:bg-red-900/20"
      >
        Log out
      </button>
    </div>
  );
}
