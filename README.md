# TBB Workbench

A local desktop workbench for inspecting and driving an **existing** local
`tor-browser-build`-style checkout. **Not affiliated with or endorsed by The
Tor Project.** Tor and Tor Browser are trademarks of The Tor Project. This
app follows Tor's real, documented "Marble" design specification (Space
Grotesk/Inter type scale, Bootstrap-derived component states, Phosphor
iconography) but does not use Tor Project logo assets and does not present
itself as an official Tor Project product.

## What's implemented (real, not mocked)

**Backend (`crates/tbb-core`, Tauri-independent, unit tested):**
- Repository open/validate via `git2`: branch, commit, ahead/behind,
  dirty/clean with per-file status, tags, submodules, remotes (credential-
  redacted), read-only.
- Git operations (clone/fetch/checkout/submodule update/diff) as argv-only
  invocation builders using the system `git`, executed through the same
  process supervisor as builds — nothing is shelled out as a string.
- Real subprocess runner (`tokio::process`): batched live stdout/stderr
  streaming, output parsing (warning/error counts, phase detection, a
  heuristic-and-labeled-as-such progress estimate), SIGTERM-then-force-kill
  cancellation, raw log persisted to disk immediately.
- File editor: repo-aware file tree (respects `.gitignore`), read/write with
  mtime-based external-change detection, real diffing (`similar`), YAML
  validation, path-traversal/symlink-escape guarded.
- RBM best-effort YAML inspector: variables, targets, input files, includes
  — honestly reports when a file isn't parseable as plain YAML instead of
  guessing.
- Makefile target discovery: real parsing of `.PHONY`/`##` help comments/
  target rules, never executes anything to discover commands.
- Host diagnostics: OS/CPU/memory/disk, tool version checks (git, make,
  python3, perl, podman/docker/newuidmap/newgidmap on Linux), repository
  health, JSON/text export.
- Run history: persisted, redacted run records, retention by count/age,
  diagnostic bundle export.
- Settings persistence (theme, safety toggles, retention, editor prefs,
  recent projects) to local JSON, no secrets, no cloud.
- Preflight report: blocks on real issues (non-git checkout, disabled shell
  mode), warns on low disk space, flags dirty-checkout confirmation.
- Artifact scanning of expected output directories with optional SHA-256.

71 Rust unit/integration tests pass (`cargo test -p tbb-core`), including
cancellation and force-kill against real long-running subprocesses, and
`cargo clippy -p tbb-core --all-targets` is clean. **The `src-tauri` shell
crate (the Tauri glue around tbb-core) has not been compiled in this
environment** — it's written against tbb-core's real API, but you should run
`cargo build` on it yourself first.

**Frontend (React + TypeScript + Vite, strict TS):**
- Full Overview / Clone wizard / Dashboard / Build / Config Editor /
  Diagnostics / Logs / Artifacts / Settings screens, all wired to the real
  IPC commands above (no mock data path anywhere).
- Build page: real Makefile-target discovery feeding preset selection, a
  custom/advanced argv mode, environment override editor with secret
  redaction, real preflight modal, live streaming log pane with
  warning/error/progress badges sourced from actual parsed output.
- Config Editor: file tree, CodeMirror 6 editor with real YAML/JSON/
  Markdown/Shell/Perl highlighting plus a hand-written Makefile mode
  (CodeMirror's legacy-modes bundle doesn't ship one), unsaved-change
  indicators, save/revert/save-all, external-change conflict handling,
  RBM inspector panel, diff-vs-disk and diff-vs-HEAD.
- Diagnostics, Logs (history, full log viewer, filter/wrap/export/delete,
  retention), Artifacts (scan + reveal-in-file-manager), Settings (theme,
  editor, retention, safety toggles including the shell-mode gate) all real.
- Marble-derived design tokens (exact documented type scale and spacing
  math) plus a small Bootstrap-style component kit (Button/Card/Badge/
  Alert/Modal/Tabs) with real hover/active/disabled state rules, Phosphor
  icons throughout.

Verified in this environment: `npm install`, `npx tsc -b --noEmit` (clean),
`npx vite build` (succeeds, 4669 modules, real `dist/` output).

## What's intentionally out of scope for this pass

- Test fixture repository + Rust integration tests specifically for the
  `src-tauri` command layer (tbb-core itself is thoroughly tested)
- Frontend component/E2E tests, ESLint config, CI workflow files
- CONTRIBUTING.md, SECURITY.md, LICENSE, NOTICE, ADR directory
- Command palette (Ctrl/Cmd+K) is a visual affordance only, not yet wired
- "Attach to an in-progress build" after app restart (interrupted runs are
  honestly recorded as `interrupted_by_app_exit`, not silently reattached)

## Running it locally

```bash
# Rust toolchain: https://rustup.rs (1.80+)
# Tauri prerequisites for your OS: https://tauri.app/start/prerequisites/
#   macOS: Xcode Command Line Tools (xcode-select --install)
#   Linux: webkit2gtk-4.1, gtk3, librsvg2, libsoup3 dev packages
#   Windows: WebView2 (preinstalled on most Windows 11/up-to-date 10)

npm install
```

Then pick one:

```bash
npm run app:dev      # launches the app in dev mode (equivalent to: npx tauri dev)
npm run app:build    # builds a release bundle/installer (equivalent to: npx tauri build)
```

**Do not run `npm run tauri` with no subcommand** — that just prints the CLI's
help text, which is expected Tauri CLI behavior, not an error. If you want
to call the CLI directly instead of the npm scripts above:

```bash
npx tauri dev
npx tauri build
```

Frontend-only checks (no Rust involved, fast):
```bash
npm run typecheck    # tsc -b --noEmit
npm run build         # tsc -b && vite build -> dist/
```

### A note on Tauri version pinning

`src-tauri/Cargo.toml` and `package.json` pin exact (not `^`-range) versions
for the paired `tauri`/`@tauri-apps/api`, `tauri-plugin-dialog`/
`@tauri-apps/plugin-dialog`, `tauri-plugin-fs`/`@tauri-apps/plugin-fs`, and
`tauri-plugin-opener`/`@tauri-apps/plugin-opener` packages (currently
2.12.0 / 2.8.0 / 2.6.0 / 2.7.0 respectively). Tauri's CLI hard-errors on
`tauri build`/`tauri dev` if the JS and Rust versions of these don't match,
and loose `^2` ranges on both sides drift independently depending on
exactly when `npm install` vs `cargo build` last ran against each registry.
If you bump one side (`npm update @tauri-apps/api`, `cargo update -p
tauri`), bump its Rust/JS counterpart to the same version in the same
commit, or you'll hit the same "version mismatched Tauri packages" error.

Rust tests (tbb-core):
```bash
cd crates/tbb-core && cargo test
```

## Safety model

- No destructive git operations exist anywhere (no `reset --hard`,
  `clean -fdx`, `checkout` outside an explicit user action, `rebase`,
  `pull`). Clone/fetch/checkout/submodule-update are all explicit,
  previewed-before-running, user-triggered actions.
- Every filesystem write goes through `security::path_guard`, which
  canonicalizes and rejects `..` traversal and symlink escape.
- Standard runs are argv-only via `tokio::process`; shell-string execution
  only happens if a `BuildInvocation` explicitly sets `shell_mode: true`,
  and the Tauri command layer refuses that unless the user has turned on
  "allow shell command mode" in Settings > Safety (off by default).
- Environment values are redacted by key-name heuristics and credential-URL
  pattern matching before ever reaching a log, preview, or diagnostic
  bundle, unless the user explicitly opts into revealing them.
- No telemetry, no automatic network activity beyond git/build commands the
  user explicitly triggers.

## Not affiliated

This project is not affiliated with, endorsed by, or produced by The Tor
Project. It does not use Tor Project logo assets. The color tokens and type
scale are original values informed by publicly documented Marble design
guidance and the product brief, not copied brand assets.
