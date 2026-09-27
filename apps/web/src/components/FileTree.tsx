"use client";

import { useState } from "react";
import type { TreeNode } from "@/lib/fileTree";

function FolderIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" className="shrink-0 text-neutral-400">
      <path
        d="M1.5 3.5a1 1 0 0 1 1-1h3.379a1 1 0 0 1 .707.293l1.121 1.121a1 1 0 0 0 .707.293H13.5a1 1 0 0 1 1 1V12.5a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1v-9Z"
        stroke="currentColor"
        strokeWidth="1.1"
      />
    </svg>
  );
}

function FileIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" className="shrink-0 text-neutral-400">
      <path
        d="M4 1.5h5.5L12.5 4.5V14a.5.5 0 0 1-.5.5H4a.5.5 0 0 1-.5-.5v-12a.5.5 0 0 1 .5-.5Z"
        stroke="currentColor"
        strokeWidth="1.1"
      />
      <path d="M9.25 1.5V4.5h3" stroke="currentColor" strokeWidth="1.1" />
    </svg>
  );
}

function Node({
  node,
  depth,
  selectedFileId,
  onSelectFile,
}: {
  node: TreeNode;
  depth: number;
  selectedFileId: string | null;
  onSelectFile: (id: string) => void;
}) {
  const [expanded, setExpanded] = useState(true);
  const indent = 8 + depth * 14;

  if (node.isFile && node.file) {
    const active = node.file.id === selectedFileId;
    return (
      <button
        type="button"
        onClick={() => onSelectFile(node.file!.id)}
        style={{ paddingLeft: indent }}
        className={`flex w-full items-center gap-1.5 rounded py-1 pr-2 text-left text-sm ${
          active
            ? "bg-neutral-200 font-medium dark:bg-neutral-800"
            : "hover:bg-neutral-100 dark:hover:bg-neutral-900"
        }`}
        title={node.file.path}
      >
        <FileIcon />
        <span className="truncate">{node.name}</span>
      </button>
    );
  }

  return (
    <div>
      <button
        type="button"
        onClick={() => setExpanded((v) => !v)}
        style={{ paddingLeft: indent }}
        className="flex w-full items-center gap-1.5 rounded py-1 pr-2 text-left text-sm text-neutral-500 hover:bg-neutral-100 dark:hover:bg-neutral-900"
      >
        <span className="inline-block w-3 shrink-0 text-[10px]">{expanded ? "▾" : "▸"}</span>
        <FolderIcon />
        <span className="truncate">{node.name}</span>
      </button>
      {expanded && (
        <div>
          {node.children.map((child) => (
            <Node
              key={child.path}
              node={child}
              depth={depth + 1}
              selectedFileId={selectedFileId}
              onSelectFile={onSelectFile}
            />
          ))}
        </div>
      )}
    </div>
  );
}

export function FileTree({
  tree,
  selectedFileId,
  onSelectFile,
}: {
  tree: TreeNode[];
  selectedFileId: string | null;
  onSelectFile: (id: string) => void;
}) {
  if (tree.length === 0) {
    return <p className="px-2 py-1 text-sm text-neutral-400">No files indexed yet.</p>;
  }
  return (
    <div>
      {tree.map((node) => (
        <Node key={node.path} node={node} depth={0} selectedFileId={selectedFileId} onSelectFile={onSelectFile} />
      ))}
    </div>
  );
}
