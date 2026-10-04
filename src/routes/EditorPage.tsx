import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  FloppyDisk,
  ArrowCounterClockwise,
  MagnifyingGlass,
  Warning,
  X,
  Circle,
} from "@phosphor-icons/react";
import { useProjectStore } from "@/features/projects/projectStore";
import { useSettingsStore } from "@/features/settings/settingsStore";
import { FileTree } from "@/components/FileTree";
import { DiffView } from "@/components/DiffView";
import { Button } from "@/components/ui/Button";
import { Alert } from "@/components/ui/Alert";
import { Modal } from "@/components/ui/Modal";
import { Tabs } from "@/components/ui/Tabs";
import { CodeEditor, detectLanguage } from "@/features/editor/CodeEditor";
import { RbmInspectorPanel } from "@/features/editor/RbmInspectorPanel";
import {
  diffProjectFileAgainstDisk,
  diffProjectFileAgainstHead,
  inspectRbm,
  listFileTree,
  readProjectFile,
  validateYamlContent,
  writeProjectFile,
  type DiffLine,
  type FileTreeEntry,
  type RbmInspection,
} from "@/lib/api";
import styles from "./EditorPage.module.css";

interface OpenBuffer {
  relPath: string;
  original: string;
  content: string;
  mtimeMs: number;
  protected: boolean;
}

type InspectorTab = "inspector" | "diff-disk" | "diff-head";

// Hard cap on simultaneously open buffers. Each open file holds its full
// text content (potentially large) in memory twice (original + working
// copy) for diffing/dirty-tracking, so an unbounded number of open tabs
// over a long session is a real memory growth path. Closing tabs (below)
// is the primary way this stays bounded; this is a defense-in-depth cap
// for sessions where the user never manually closes anything — evicts the
// least-recently-used CLEAN (non-dirty) buffer, never a dirty one.
const MAX_OPEN_BUFFERS = 24;

export function EditorPage(): JSX.Element {
  const { currentProject } = useProjectStore();
  const { settings } = useSettingsStore();

  const [entries, setEntries] = useState<FileTreeEntry[]>([]);
  const [filter, setFilter] = useState("");
  const [buffers, setBuffers] = useState<Record<string, OpenBuffer>>({});
  // Tab order is tracked separately from the buffers dict (whose key order
  // isn't guaranteed stable/meaningful) so the tab strip has a predictable,
  // most-recently-opened-last ordering, and so LRU eviction has something
  // principled to evict by.
  const [openOrder, setOpenOrder] = useState<string[]>([]);
  const [activePath, setActivePath] = useState<string | null>(null);
  const [tab, setTab] = useState<InspectorTab>("inspector");
  const [rbmInspection, setRbmInspection] = useState<RbmInspection | null>(null);
  const [diskDiff, setDiskDiff] = useState<DiffLine[] | null>(null);
  const [headDiff, setHeadDiff] = useState<DiffLine[] | null>(null);
  const [yamlError, setYamlError] = useState<string | null>(null);
  const [externalChangeError, setExternalChangeError] = useState<string | null>(null);
  const [saveMessage, setSaveMessage] = useState<string | null>(null);
  const [protectedAck, setProtectedAck] = useState<Record<string, boolean>>({});
  const [closeConfirmPath, setCloseConfirmPath] = useState<string | null>(null);

  useEffect(() => {
    if (!currentProject) return;
    listFileTree(currentProject.id).then(setEntries).catch(() => setEntries([]));
  }, [currentProject]);

  const dirtyPaths = useMemo(() => {
    const set = new Set<string>();
    for (const [path, buf] of Object.entries(buffers)) {
      if (buf.content !== buf.original) set.add(path);
    }
    return set;
  }, [buffers]);

  const openFile = useCallback(
    async function openFile(relPath: string): Promise<void> {
      if (!currentProject) return;
      setActivePath(relPath);
      setExternalChangeError(null);
      setSaveMessage(null);
      setOpenOrder((order) => [...order.filter((p) => p !== relPath), relPath]);
      if (buffers[relPath]) return;

      try {
        const file = await readProjectFile(currentProject.id, relPath);
        setBuffers((b) => {
          const next = {
            ...b,
            [relPath]: {
              relPath,
              original: file.content,
              content: file.content,
              mtimeMs: file.mtime_ms,
              protected: file.protected,
            },
          };
          // LRU eviction, skipping dirty buffers and the one just opened.
          const keys = Object.keys(next);
          if (keys.length > MAX_OPEN_BUFFERS) {
            setOpenOrder((order) => {
              const evictable = order.filter(
                (p) => p !== relPath && next[p] && next[p]!.content === next[p]!.original,
              );
              const toEvict = evictable[0];
              if (toEvict) {
                setBuffers((b2) => {
                  const { [toEvict]: _removed, ...rest } = b2;
                  return rest;
                });
                return order.filter((p) => p !== toEvict);
              }
              return order;
            });
          }
          return next;
        });
      } catch (e) {
        setExternalChangeError(String(e));
      }
    },
    [currentProject, buffers],
  );

  const activeBuffer = activePath ? buffers[activePath] : null;
  const language = activePath ? detectLanguage(activePath) : "plain";
  const isYamlLike = language === "yaml";

  useEffect(() => {
    setYamlError(null);
    setRbmInspection(null);
    setDiskDiff(null);
    setHeadDiff(null);
  }, [activePath]);

  // Guards against race conditions when switching files/tabs rapidly: if
  // the user clicks three files in quick succession, earlier in-flight
  // requests resolving late must not overwrite the display for whatever
  // file is ACTUALLY active by the time they come back.
  useEffect(() => {
    if (!activeBuffer || !currentProject) return;
    let cancelled = false;
    const requestedPath = activeBuffer.relPath;

    if (tab === "inspector" && isYamlLike) {
      inspectRbm(currentProject.id, requestedPath)
        .then((result) => {
          if (!cancelled && activePath === requestedPath) setRbmInspection(result);
        })
        .catch(() => {
          if (!cancelled && activePath === requestedPath) setRbmInspection(null);
        });
    }
    if (tab === "diff-disk") {
      diffProjectFileAgainstDisk(currentProject.id, requestedPath, activeBuffer.content)
        .then((result) => {
          if (!cancelled && activePath === requestedPath) setDiskDiff(result);
        })
        .catch(() => {
          if (!cancelled && activePath === requestedPath) setDiskDiff(null);
        });
    }
    if (tab === "diff-head") {
      diffProjectFileAgainstHead(currentProject.id, requestedPath)
        .then((result) => {
          if (!cancelled && activePath === requestedPath) setHeadDiff(result);
        })
        .catch(() => {
          if (!cancelled && activePath === requestedPath) setHeadDiff(null);
        });
    }

    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tab, activeBuffer?.content, activePath]);

  function updateContent(relPath: string, content: string): void {
    setBuffers((b) => ({ ...b, [relPath]: { ...b[relPath]!, content } }));
    if (detectLanguage(relPath) === "yaml") {
      validateYamlContent(content)
        .then(() => setYamlError(null))
        .catch((e) => setYamlError(String(e)));
    }
  }

  const saveBuffer = useCallback(
    async function saveBuffer(relPath: string, force = false): Promise<void> {
      if (!currentProject) return;
      const buf = buffers[relPath];
      if (!buf) return;
      setExternalChangeError(null);
      setSaveMessage(null);
      try {
        const result = await writeProjectFile(
          currentProject.id,
          relPath,
          buf.content,
          buf.mtimeMs,
          force,
        );
        setBuffers((b) => ({
          ...b,
          [relPath]: { ...b[relPath]!, original: buf.content, mtimeMs: result.mtime_ms },
        }));
        setSaveMessage(`Saved ${relPath}`);
      } catch (e) {
        setExternalChangeError(String(e));
      }
    },
    [currentProject, buffers],
  );

  function revertBuffer(relPath: string): void {
    setBuffers((b) => ({ ...b, [relPath]: { ...b[relPath]!, content: b[relPath]!.original } }));
  }

  async function saveAll(): Promise<void> {
    for (const path of dirtyPaths) {
      await saveBuffer(path);
    }
  }

  function requestCloseTab(relPath: string): void {
    const buf = buffers[relPath];
    if (buf && buf.content !== buf.original) {
      setCloseConfirmPath(relPath);
      return;
    }
    closeTab(relPath);
  }

  function closeTab(relPath: string): void {
    setBuffers((b) => {
      const { [relPath]: _removed, ...rest } = b;
      return rest;
    });
    setOpenOrder((order) => order.filter((p) => p !== relPath));
    setCloseConfirmPath(null);
    if (activePath === relPath) {
      const remaining = openOrder.filter((p) => p !== relPath);
      setActivePath(remaining.length > 0 ? remaining[remaining.length - 1]! : null);
    }
  }

  // Cmd/Ctrl+S saves the active buffer from anywhere on the page.
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent): void {
      const isSaveCombo = (e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "s";
      if (!isSaveCombo || !activeBuffer) return;
      e.preventDefault();
      if (activeBuffer.content === activeBuffer.original) return;
      if (activeBuffer.protected && !protectedAck[activeBuffer.relPath]) {
        setProtectedAck((a) => ({ ...a, [activeBuffer.relPath]: true }));
        return;
      }
      void saveBuffer(activeBuffer.relPath);
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [activeBuffer, protectedAck, saveBuffer]);

  // Warn (via the browser's native unload confirmation) if the user tries
  // to close the window/reload with unsaved changes. Belt-and-suspenders
  // alongside the in-app close-tab confirmation.
  const hasDirtyRef = useRef(false);
  hasDirtyRef.current = dirtyPaths.size > 0;
  useEffect(() => {
    function onBeforeUnload(e: BeforeUnloadEvent): void {
      if (hasDirtyRef.current) {
        e.preventDefault();
        e.returnValue = "";
      }
    }
    window.addEventListener("beforeunload", onBeforeUnload);
    return () => window.removeEventListener("beforeunload", onBeforeUnload);
  }, []);

  if (!currentProject) {
    return <p className={styles.empty}>Open a project first.</p>;
  }

  return (
    <div className={styles.page}>
      <div className={styles.header}>
        <h2>Config Editor</h2>
        <Button
          variant="secondary"
          size="sm"
          disabled={dirtyPaths.size === 0}
          onClick={() => void saveAll()}
        >
          Save All ({dirtyPaths.size})
        </Button>
      </div>

      <div className={styles.layout}>
        <div className={styles.treeColumn}>
          <div className={styles.searchRow}>
            <MagnifyingGlass size={14} />
            <input
              className={styles.searchInput}
              placeholder="Filter files"
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
            />
          </div>
          <div className={styles.treeBody}>
            <FileTree
              entries={entries}
              dirtyPaths={dirtyPaths}
              selectedPath={activePath}
              onSelect={(p) => void openFile(p)}
              filter={filter}
            />
          </div>
        </div>

        <div className={styles.editorColumn}>
          {openOrder.length > 0 && (
            <div className={styles.tabStrip} role="tablist" aria-label="Open files">
              {openOrder.map((path) => {
                const buf = buffers[path];
                const isDirty = buf && buf.content !== buf.original;
                const name = path.split("/").pop() ?? path;
                return (
                  <div
                    key={path}
                    role="tab"
                    aria-selected={path === activePath}
                    className={path === activePath ? `${styles.tabItem} ${styles.tabItemActive}` : styles.tabItem}
                    title={path}
                  >
                    <button className={styles.tabSelect} onClick={() => void openFile(path)}>
                      {isDirty && <Circle size={6} weight="fill" className={styles.tabDirtyDot} />}
                      {name}
                    </button>
                    <button
                      className={styles.tabClose}
                      aria-label={`Close ${name}`}
                      onClick={() => requestCloseTab(path)}
                    >
                      <X size={11} />
                    </button>
                  </div>
                );
              })}
            </div>
          )}

          {!activeBuffer && <p className={styles.emptyEditor}>Select a file to edit.</p>}
          {activeBuffer && (
            <>
              <div className={styles.editorToolbar}>
                <span className={styles.activePath}>{activeBuffer.relPath}</span>
                {activeBuffer.protected && (
                  <span className={styles.protectedTag}>
                    <Warning size={12} weight="fill" />
                    Protected build-control file
                  </span>
                )}
                <div className={styles.spacer} />
                <Button
                  size="sm"
                  variant="ghost"
                  icon={<ArrowCounterClockwise size={14} />}
                  disabled={activeBuffer.content === activeBuffer.original}
                  onClick={() => revertBuffer(activeBuffer.relPath)}
                >
                  Revert
                </Button>
                <Button
                  size="sm"
                  variant="primary"
                  icon={<FloppyDisk size={14} />}
                  disabled={activeBuffer.content === activeBuffer.original || (isYamlLike && !!yamlError)}
                  onClick={() => {
                    const needsAck = activeBuffer.protected && !protectedAck[activeBuffer.relPath];
                    if (needsAck) {
                      setProtectedAck((a) => ({ ...a, [activeBuffer.relPath]: true }));
                      return;
                    }
                    void saveBuffer(activeBuffer.relPath);
                  }}
                  title="Cmd/Ctrl+S"
                >
                  {activeBuffer.protected && !protectedAck[activeBuffer.relPath]
                    ? "Confirm & Save"
                    : "Save"}
                </Button>
              </div>

              {yamlError && (
                <Alert tone="warning" title="YAML validation">
                  {yamlError} — you can still save; this is a lint warning, not a hard block.
                </Alert>
              )}
              {externalChangeError && (
                <Alert tone="danger" title="Could not save">
                  <p>{externalChangeError}</p>
                  {externalChangeError.includes("EXTERNAL_CHANGE") && (
                    <Button size="sm" variant="danger" onClick={() => void saveBuffer(activeBuffer.relPath, true)}>
                      Save anyway (overwrite disk)
                    </Button>
                  )}
                </Alert>
              )}
              {saveMessage && <Alert tone="success">{saveMessage}</Alert>}

              <div className={styles.editorBody}>
                <CodeEditor
                  key={activeBuffer.relPath}
                  value={activeBuffer.content}
                  language={language}
                  tabSize={settings?.editor.tab_size ?? 2}
                  insertSpaces={settings?.editor.insert_spaces ?? true}
                  showWhitespace={settings?.editor.show_whitespace ?? true}
                  onChange={(next) => updateContent(activeBuffer.relPath, next)}
                />
              </div>
            </>
          )}
        </div>

        <div className={styles.inspectorColumn}>
          <Tabs
            tabs={[
              { id: "inspector", label: "RBM Inspector" },
              { id: "diff-disk", label: "Diff vs Disk" },
              { id: "diff-head", label: "Diff vs HEAD" },
            ]}
            active={tab}
            onChange={(id) => setTab(id as InspectorTab)}
          />
          <div className={styles.inspectorBodyWrap}>
            <div className={styles.inspectorBody}>
              {!activeBuffer && <p className={styles.emptyEditor}>No file open.</p>}
              {activeBuffer && tab === "inspector" && (
                isYamlLike && rbmInspection ? (
                  <RbmInspectorPanel inspection={rbmInspection} />
                ) : (
                  <p className={styles.emptyEditor}>
                    RBM inspection is available for YAML-like config files.
                  </p>
                )
              )}
              {activeBuffer && tab === "diff-disk" && diskDiff && <DiffView lines={diskDiff} />}
              {activeBuffer && tab === "diff-head" &&
                (headDiff ? (
                  <DiffView lines={headDiff} />
                ) : (
                  <p className={styles.emptyEditor}>File is untracked, or repository has no commits yet.</p>
                ))}
            </div>
          </div>
        </div>
      </div>

      {closeConfirmPath && (
        <Modal
          title="Unsaved changes"
          onClose={() => setCloseConfirmPath(null)}
          footer={
            <>
              <Button variant="secondary" onClick={() => setCloseConfirmPath(null)}>
                Keep editing
              </Button>
              <Button variant="danger" onClick={() => closeTab(closeConfirmPath)}>
                Discard and close
              </Button>
            </>
          }
        >
          <p>
            <span className={styles.mono}>{closeConfirmPath}</span> has unsaved changes. Closing
            this tab discards them — they stay on disk as last saved, but your edits in this
            session will be lost.
          </p>
        </Modal>
      )}
    </div>
  );
}
