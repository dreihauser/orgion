"use client";

import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { Suspense, useMemo, useState } from "react";
import { useAuth } from "@/lib/auth";
import { buildFileTree, type TreeNode } from "@/lib/fileTree";
import { useWorkspaceData } from "@/lib/workspaceData";
import { FileTree } from "./FileTree";

const NAV_LINKS = [
  { href: "/", label: "Dashboard" },
  { href: "/settings", label: "Settings" },
];

export function AppShell({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const router = useRouter();
  const { user, workspaces, logout } = useAuth();
  const { files } = useWorkspaceData();
  const [userMenuOpen, setUserMenuOpen] = useState(false);

  const tree = useMemo(() => buildFileTree(files), [files]);
  const workspaceName = workspaces[0]?.name;

  function handleSelectFile(id: string) {
    router.push(`/workspace?file=${encodeURIComponent(id)}`);
  }

  async function handleLogout() {
    await logout();
    router.push("/login");
  }

  return (
    <div className="flex h-screen">
      <nav className="flex w-64 shrink-0 flex-col border-r border-neutral-200 dark:border-neutral-800">
        <div className="shrink-0 border-b border-neutral-200 p-3 dark:border-neutral-800">
          <div className="text-sm font-semibold">Orgion</div>
          {workspaceName && <div className="truncate text-xs text-neutral-400">{workspaceName}</div>}
        </div>

        <div className="shrink-0 space-y-0.5 p-2">
          {NAV_LINKS.map((link) => {
            const active = pathname === link.href;
            return (
              <Link
                key={link.href}
                href={link.href}
                className={`block rounded px-2 py-1.5 text-sm ${
                  active
                    ? "bg-neutral-200 font-medium dark:bg-neutral-800"
                    : "hover:bg-neutral-100 dark:hover:bg-neutral-900"
                }`}
              >
                {link.label}
              </Link>
            );
          })}
        </div>

        <div className="shrink-0 px-3 pb-1 pt-2 text-xs font-semibold uppercase tracking-wide text-neutral-400">
          Files
        </div>
        {/* This is the scrollable region the user asked for — the rest of
            the sidebar (brand, nav, account) stays fixed. */}
        <div className="min-h-0 flex-1 overflow-y-auto px-1 pb-2">
          <Suspense fallback={<FileTree tree={tree} selectedFileId={null} onSelectFile={handleSelectFile} />}>
            <ScopedFileTree tree={tree} onSelectFile={handleSelectFile} />
          </Suspense>
        </div>

        <div className="relative shrink-0 border-t border-neutral-200 p-2 dark:border-neutral-800">
          <button
            type="button"
            onClick={() => setUserMenuOpen((v) => !v)}
            className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-sm hover:bg-neutral-100 dark:hover:bg-neutral-900"
          >
            <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-neutral-300 text-xs font-medium text-neutral-700 dark:bg-neutral-700 dark:text-neutral-200">
              {(user?.display_name ?? user?.username ?? "?").slice(0, 1).toUpperCase()}
            </span>
            <span className="truncate">{user?.display_name ?? user?.username ?? "Anonymous"}</span>
          </button>
          {userMenuOpen && (
            <div className="absolute bottom-full left-2 mb-1 w-56 rounded border border-neutral-200 bg-white py-1 shadow-lg dark:border-neutral-800 dark:bg-neutral-900">
              <button
                type="button"
                onClick={handleLogout}
                className="block w-full px-3 py-1.5 text-left text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800"
              >
                Log out
              </button>
            </div>
          )}
        </div>
      </nav>

      <main className="min-w-0 flex-1 overflow-hidden">{children}</main>
    </div>
  );
}

/** Isolated so `useSearchParams()` only forces a Suspense boundary around
 * this small piece of the sidebar, not the whole shell. */
function ScopedFileTree({ tree, onSelectFile }: { tree: TreeNode[]; onSelectFile: (id: string) => void }) {
  const searchParams = useSearchParams();
  const selectedFileId = searchParams.get("file");
  return <FileTree tree={tree} selectedFileId={selectedFileId} onSelectFile={onSelectFile} />;
}
