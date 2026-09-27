"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { useAuth } from "@/lib/auth";

export default function LoginPage() {
  const { user, loading, login } = useAuth();
  const router = useRouter();
  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    if (!loading && user) router.replace("/");
  }, [loading, user, router]);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setSubmitting(true);
    setError(null);
    try {
      await login(username, password);
      router.push("/");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Login failed");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="flex min-h-screen items-center justify-center px-4">
      <div className="w-full max-w-sm">
        <div className="mb-8 text-center">
          <h1 className="text-2xl font-semibold">Orgion</h1>
          <p className="mt-1 text-sm text-neutral-500">Sign in to your workspace</p>
        </div>
        <form
          onSubmit={handleSubmit}
          className="space-y-4 rounded-lg border border-neutral-200 bg-white p-6 shadow-sm dark:border-neutral-800 dark:bg-neutral-900"
        >
          {error && (
            <div className="rounded bg-red-100 px-3 py-2 text-sm text-red-800 dark:bg-red-900/40 dark:text-red-300">
              {error}
            </div>
          )}
          <div>
            <label className="mb-1 block text-xs font-medium text-neutral-500" htmlFor="username">
              Username
            </label>
            <input
              id="username"
              autoFocus
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              className="w-full rounded border border-neutral-300 bg-transparent px-3 py-2 text-sm outline-none focus:border-neutral-500 dark:border-neutral-700"
            />
          </div>
          <div>
            <label className="mb-1 block text-xs font-medium text-neutral-500" htmlFor="password">
              Password
            </label>
            <input
              id="password"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              className="w-full rounded border border-neutral-300 bg-transparent px-3 py-2 text-sm outline-none focus:border-neutral-500 dark:border-neutral-700"
            />
          </div>
          <button
            type="submit"
            disabled={submitting}
            className="w-full rounded bg-neutral-900 px-3 py-2 text-sm font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-50 dark:bg-neutral-100 dark:text-neutral-900"
          >
            {submitting ? "Signing in…" : "Sign in"}
          </button>
        </form>
        <p className="mt-4 text-center text-xs text-neutral-400">
          Admin account created by <code>orgion init</code> — see your terminal output.
        </p>
      </div>
    </div>
  );
}
