import { Copy } from "@phosphor-icons/react";
import type { BuildInvocation } from "@/lib/api";
import styles from "./CommandPreview.module.css";

function shellQuote(arg: string): string {
  if (arg === "") return "''";
  const safe = /^[A-Za-z0-9\-_./:=@%+,]+$/.test(arg);
  return safe ? arg : `'${arg.replace(/'/g, "'\\''")}'`;
}

function renderCommand(inv: BuildInvocation): string {
  return [inv.executable, ...inv.args].map(shellQuote).join(" ");
}

export function CommandPreview({ invocation }: { invocation: BuildInvocation }): JSX.Element {
  const commandText = renderCommand(invocation);
  const fullText = `cd ${shellQuote(invocation.working_dir)} && ${commandText}`;

  return (
    <div className={styles.wrap}>
      <div className={styles.headerRow}>
        <span className={styles.label}>Exact command (reproducible in a shell)</span>
        <button
          className={styles.copyButton}
          type="button"
          onClick={() => void navigator.clipboard.writeText(fullText)}
          aria-label="Copy command"
          title="Copy command"
        >
          <Copy size={14} />
          Copy
        </button>
      </div>
      <pre className={styles.pre}>{fullText}</pre>
      {invocation.env_overrides.length > 0 && (
        <div className={styles.envList}>
          {invocation.env_overrides.map((e) => (
            <div key={e.key} className={styles.envRow}>
              <span className={styles.envKey}>{e.key}</span>
              <span className={styles.envValue}>{e.likely_secret ? "[REDACTED]" : e.value}</span>
            </div>
          ))}
        </div>
      )}
      {invocation.shell_mode && (
        <p className={styles.shellWarning}>
          Shell command mode: this string is interpreted by a shell, not run as a plain argument
          array. Only enabled because you turned it on in Settings &gt; Safety.
        </p>
      )}
    </div>
  );
}
