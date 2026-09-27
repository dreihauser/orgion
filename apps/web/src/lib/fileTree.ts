import type { FileSummary } from "./types";

export interface TreeNode {
  name: string;
  /** Full path from the workspace root, e.g. "projects/hri/tasks.org" or "projects/hri" for a folder. */
  path: string;
  isFile: boolean;
  file?: FileSummary;
  children: TreeNode[];
}

/**
 * Turns the flat file list the API returns into a nested folder/file
 * tree, sorted folders-first then alphabetically at each level — this is
 * what makes a workspace with many subdirectories (book/, projects/,
 * roam/, ...) legible in the sidebar instead of one long flat list.
 */
export function buildFileTree(files: FileSummary[]): TreeNode[] {
  const root: TreeNode[] = [];

  for (const file of files) {
    const parts = file.path.split("/");
    let level = root;
    let pathSoFar = "";

    parts.forEach((part, i) => {
      pathSoFar = pathSoFar ? `${pathSoFar}/${part}` : part;
      const isFile = i === parts.length - 1;
      let node = level.find((n) => n.name === part && n.isFile === isFile);
      if (!node) {
        node = { name: part, path: pathSoFar, isFile, children: [], file: isFile ? file : undefined };
        level.push(node);
      }
      level = node.children;
    });
  }

  sortTree(root);
  return root;
}

function sortTree(nodes: TreeNode[]) {
  nodes.sort((a, b) => {
    if (a.isFile !== b.isFile) return a.isFile ? 1 : -1;
    return a.name.localeCompare(b.name);
  });
  for (const node of nodes) {
    if (!node.isFile) sortTree(node.children);
  }
}
