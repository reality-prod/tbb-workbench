import { useState } from "react";
import { FolderOpen, MagnifyingGlass, HashStraight } from "@phosphor-icons/react";
import { useProjectStore } from "@/features/projects/projectStore";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Alert } from "@/components/ui/Alert";
import { Badge } from "@/components/ui/Badge";
import { revealPathInFileManager, scanProjectArtifacts, type ArtifactScanResult } from "@/lib/api";
import styles from "./ArtifactsPage.module.css";

function formatBytes(bytes: number): string {
  if (bytes > 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GiB`;
  if (bytes > 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MiB`;
  return `${(bytes / 1024).toFixed(0)} KiB`;
}

export function ArtifactsPage(): JSX.Element {
  const { currentProject } = useProjectStore();
  const [dirsInput, setDirsInput] = useState("out");
  const [computeHashes, setComputeHashes] = useState(false);
  const [result, setResult] = useState<ArtifactScanResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function scan(): Promise<void> {
    if (!currentProject) return;
    setLoading(true);
    setError(null);
    try {
      const dirs = dirsInput.split(",").map((s) => s.trim()).filter(Boolean);
      const r = await scanProjectArtifacts(currentProject.id, dirs, computeHashes);
      setResult(r);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  if (!currentProject) {
    return <p className={styles.empty}>Open a project first.</p>;
  }

  return (
    <div className={styles.page}>
      <h2>Artifacts</h2>
      <Card title="Scan for build output">
        <div className={styles.row}>
          <label className={styles.field}>
            <span>Directories to search (comma-separated, relative to project root)</span>
            <input value={dirsInput} onChange={(e) => setDirsInput(e.target.value)} />
          </label>
          <label className={styles.checkboxRow}>
            <input type="checkbox" checked={computeHashes} onChange={(e) => setComputeHashes(e.target.checked)} />
            Compute SHA-256 (slower for large files)
          </label>
          <Button variant="primary" icon={<MagnifyingGlass size={14} />} onClick={() => void scan()} disabled={loading}>
            Scan
          </Button>
        </div>
      </Card>

      {error && <Alert tone="danger">{error}</Alert>}

      {result && !result.any_dir_existed && (
        <Alert tone="warning" title="Output location not detected">
          None of the searched directories exist yet ({result.searched_dirs.join(", ")}). Run a
          build first, or adjust the directories above to match what your checkout actually
          produces.
        </Alert>
      )}

      {result && result.artifacts.length > 0 && (
        <Card title={`${result.artifacts.length} artifact(s) found`}>
          <div className={styles.artifactList}>
            {result.artifacts.map((a) => (
              <div key={a.path} className={styles.artifactRow}>
                <div className={styles.artifactMain}>
                  <span className={styles.artifactPath}>{a.path}</span>
                  <div className={styles.artifactMeta}>
                    <Badge tone="accent">{a.kind}</Badge>
                    <span>{formatBytes(a.size)}</span>
                    <span>{new Date(a.modified_ms).toLocaleString()}</span>
                    {a.sha256 && (
                      <span className={styles.hash} title={a.sha256}>
                        <HashStraight size={12} />
                        {a.sha256.slice(0, 12)}\u2026
                      </span>
                    )}
                  </div>
                </div>
                <Button size="sm" variant="ghost" icon={<FolderOpen size={14} />} onClick={() => void revealPathInFileManager(a.path)}>
                  Reveal
                </Button>
              </div>
            ))}
          </div>
        </Card>
      )}
    </div>
  );
}
