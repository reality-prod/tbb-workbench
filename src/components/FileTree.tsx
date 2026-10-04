import { useMemo, useState } from "react";
import { CaretRight, CaretDown, File, Folder, Lock } from "@phosphor-icons/react";
import type { FileTreeEntry } from "@/lib/api";
import styles from "./FileTree.module.css";

interface TreeNode {
  name: string;
  relPath: string;
  isDir: boolean;
  protected: boolean;
  dirty: boolean;
  children: TreeNode[];
}

function buildTree(entries: FileTreeEntry[], dirtyPaths: Set<string>): TreeNode[] {
  const root: TreeNode[] = [];
  const dirMap = new Map<string, TreeNode>();

  const sorted = [...entries].sort((a, b) => a.rel_path.localeCompare(b.rel_path));
  for (const entry of sorted) {
    const parts = entry.rel_path.split("/");
    let currentList = root;
    let currentPath = "";
    for (let i = 0; i < parts.length; i++) {
      const part = parts[i]!;
      currentPath = currentPath ? `${currentPath}/${part}` : part;
      const isLast = i === parts.length - 1;
      if (isLast && !entry.is_dir) {
        currentList.push({
          name: part,
          relPath: currentPath,
          isDir: false,
          protected: entry.protected,
          dirty: dirtyPaths.has(currentPath),
          children: [],
        });
      } else {
        let node = dirMap.get(currentPath);
        if (!node) {
          node = {
            name: part,
            relPath: currentPath,
            isDir: true,
            protected: false,
            dirty: false,
            children: [],
          };
          dirMap.set(currentPath, node);
          currentList.push(node);
        }
        currentList = node.children;
      }
    }
  }
  return root;
}

function TreeRow({
  node,
  depth,
  selectedPath,
  onSelect,
}: {
  node: TreeNode;
  depth: number;
  selectedPath: string | null;
  onSelect: (relPath: string) => void;
}): JSX.Element {
  const [expanded, setExpanded] = useState(depth < 1);

  if (node.isDir) {
    return (
      <div>
        <button
          className={styles.row}
          style={{ paddingLeft: `${depth * 14 + 8}px` }}
          onClick={() => setExpanded((e) => !e)}
        >
          {expanded ? <CaretDown size={12} /> : <CaretRight size={12} />}
          <Folder size={14} weight="fill" className={styles.folderIcon} />
          <span className={styles.name}>{node.name}</span>
        </button>
        {expanded && (
          <div>
            {node.children.map((child) => (
              <TreeRow
                key={child.relPath}
                node={child}
                depth={depth + 1}
                selectedPath={selectedPath}
                onSelect={onSelect}
              />
            ))}
          </div>
        )}
      </div>
    );
  }

  const isSelected = selectedPath === node.relPath;
  return (
    <button
      className={isSelected ? `${styles.row} ${styles.selected}` : styles.row}
      style={{ paddingLeft: `${depth * 14 + 22}px` }}
      onClick={() => onSelect(node.relPath)}
      title={node.relPath}
    >
      <File size={14} />
      <span className={styles.name}>{node.name}</span>
      {node.protected && <Lock size={11} className={styles.lockIcon} aria-label="Protected build-control file" />}
      {node.dirty && <span className={styles.dirtyDot} aria-label="Unsaved changes" />}
    </button>
  );
}

export function FileTree({
  entries,
  dirtyPaths,
  selectedPath,
  onSelect,
  filter,
}: {
  entries: FileTreeEntry[];
  dirtyPaths: Set<string>;
  selectedPath: string | null;
  onSelect: (relPath: string) => void;
  filter: string;
}): JSX.Element {
  const filtered = useMemo(() => {
    if (!filter.trim()) return entries;
    const needle = filter.toLowerCase();
    return entries.filter((e) => e.rel_path.toLowerCase().includes(needle));
  }, [entries, filter]);

  const tree = useMemo(() => buildTree(filtered, dirtyPaths), [filtered, dirtyPaths]);

  return (
    <div className={styles.tree} role="tree" aria-label="Repository files">
      {tree.map((node) => (
        <TreeRow key={node.relPath} node={node} depth={0} selectedPath={selectedPath} onSelect={onSelect} />
      ))}
    </div>
  );
}
