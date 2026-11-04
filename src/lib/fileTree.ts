// The sidebar's view of the changed files (PRD §6.5): a tree of folders, with
// chains of single-child folders compacted, or one flat list of paths.
import type { FileEntry } from './types';

export type FolderNode = {
  kind: 'folder';
  /** One folder, or a compacted chain such as `src/app/core`. */
  name: string;
  /** The full path of the last folder in the chain, which identifies the node. */
  path: string;
  children: TreeNode[];
};

export type FileNode = { kind: 'file'; name: string; file: FileEntry };

export type TreeNode = FolderNode | FileNode;

/** A visible line of the tree, with its nesting depth. */
export type TreeRow = { node: TreeNode; depth: number };

const collator = new Intl.Collator('en', { numeric: true, sensitivity: 'base' });

export function comparePaths(a: string, b: string): number {
  return collator.compare(a, b) || (a < b ? -1 : a > b ? 1 : 0);
}

/** Folders first, then files, each in path order, as GitHub's file tree shows them. */
function compareNodes(a: TreeNode, b: TreeNode): number {
  if (a.kind !== b.kind) return a.kind === 'folder' ? -1 : 1;
  return comparePaths(a.name, b.name);
}

export function buildTree(files: FileEntry[]): TreeNode[] {
  const root: FolderNode = { kind: 'folder', name: '', path: '', children: [] };
  const folders = new Map<string, FolderNode>([['', root]]);

  for (const file of files) {
    const parts = file.path.split('/');
    let parent = root;
    for (let i = 0; i < parts.length - 1; i++) {
      const path = parts.slice(0, i + 1).join('/');
      let folder = folders.get(path);
      if (!folder) {
        folder = { kind: 'folder', name: parts[i], path, children: [] };
        folders.set(path, folder);
        parent.children.push(folder);
      }
      parent = folder;
    }
    parent.children.push({ kind: 'file', name: parts[parts.length - 1], file });
  }

  return root.children.map(compact).sort(compareNodes);
}

/** Merges a folder whose only child is a folder into that child, and sorts. */
function compact(node: TreeNode): TreeNode {
  if (node.kind === 'file') return node;
  let folder = node;
  while (folder.children.length === 1 && folder.children[0].kind === 'folder') {
    const child = folder.children[0];
    folder = { ...child, name: `${folder.name}/${child.name}` };
  }
  return { ...folder, children: folder.children.map(compact).sort(compareNodes) };
}

/** The rows to show: folders in `collapsed` hide what they contain. */
export function treeRows(nodes: TreeNode[], collapsed: ReadonlySet<string>): TreeRow[] {
  const rows: TreeRow[] = [];
  const visit = (nodes: TreeNode[], depth: number) => {
    for (const node of nodes) {
      rows.push({ node, depth });
      if (node.kind === 'folder' && !collapsed.has(node.path)) visit(node.children, depth + 1);
    }
  };
  visit(nodes, 0);
  return rows;
}

/** The files in the order the sidebar lists them, which the diff list follows (PRD §6.6). */
export function orderedFiles(files: FileEntry[], mode: 'tree' | 'flat'): FileEntry[] {
  if (mode === 'flat') return [...files].sort((a, b) => comparePaths(a.path, b.path));
  const ordered: FileEntry[] = [];
  const visit = (nodes: TreeNode[]) => {
    for (const node of nodes) {
      if (node.kind === 'file') ordered.push(node.file);
      else visit(node.children);
    }
  };
  visit(buildTree(files));
  return ordered;
}
