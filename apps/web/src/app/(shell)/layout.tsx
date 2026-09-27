"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { useAuth } from "@/lib/auth";
import { WorkspaceDataProvider } from "@/lib/workspaceData";
import { AppShell } from "@/components/AppShell";

export default function ShellLayout({ children }: { children: React.ReactNode }) {
  const { user, loading } = useAuth();
  const router = useRouter();

  useEffect(() => {
    if (!loading && !user) {
      router.replace("/login");
    }
  }, [loading, user, router]);

  if (loading) {
    return (
      <div className="flex h-screen items-center justify-center text-sm text-neutral-400">
        Loading…
      </div>
    );
  }

  if (!user) {
    // Redirect effect above is in flight; render nothing rather than a
    // flash of the authenticated shell.
    return null;
  }

  return (
    <WorkspaceDataProvider>
      <AppShell>{children}</AppShell>
    </WorkspaceDataProvider>
  );
}
