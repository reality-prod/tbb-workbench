/**
 * Typed bridge to the real Rust backend commands defined in
 * `src-tauri/src/commands`. Every function here invokes an actual
 * `#[tauri::command]` backed by `tbb-core`. There is no mocked data path.
 *
 * Tauri camelCases Rust snake_case argument names automatically, so every
 * invoke() call below uses camelCase keys matching what the generated JS
 * bindings expect (e.g. Rust `project_id` -> JS `projectId`).
 */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// ---------------------------------------------------------------------------
// Shared / project models
// ---------------------------------------------------------------------------

export interface ManagedProject {
  id: string;
  name: string;
  root_path: string;
  opened_at: string;
}

export interface RemoteInfo {
  name: string;
  url: string;
}

export interface ChangedFile {
  path: string;
  status: string;
  staged: boolean;
}

export interface RepositoryStatus {
  is_git_repo: boolean;
  looks_like_tor_browser_build: boolean;
  detected_markers: string[];
  remote_origin_url: string | null;
  remotes: RemoteInfo[];
  current_branch: string | null;
  head_detached: boolean;
  current_commit: string | null;
  ahead: number | null;
  behind: number | null;
  is_dirty: boolean;
  changed_file_count: number;
  changed_files: ChangedFile[];
  tags: string[];
  submodules: string[];
  last_fetch_ms: number | null;
  validation_errors: string[];
}

export interface RecentProject {
  path: string;
  name: string;
  last_opened_ms: number;
}

// ---------------------------------------------------------------------------
// Invocations / runs
// ---------------------------------------------------------------------------

export interface EnvironmentOverride {
  key: string;
  value: string;
  likely_secret: boolean;
}

export type RunKind =
  | "build"
  | "git_clone"
  | "git_fetch"
  | "git_checkout"
  | "git_submodule"
  | "custom";

export interface BuildInvocation {
  executable: string;
  args: string[];
  working_dir: string;
  env_overrides: EnvironmentOverride[];
  label: string;
  shell_mode: boolean;
  expected_output_dirs: string[];
  preset_id: string | null;
  kind: RunKind;
}

export type BuildState =
  | "idle"
  | "preparing"
  | "checking_prerequisites"
  | "fetching"
  | "building"
  | "packaging"
  | "verifying"
  | "running"
  | "cancelling"
  | "cancelled"
  | "failed"
  | "succeeded"
  | "interrupted_by_app_exit";

export function isTerminalState(state: BuildState): boolean {
  return (
    state === "cancelled" ||
    state === "failed" ||
    state === "succeeded" ||
    state === "interrupted_by_app_exit"
  );
}

export type OutputStream = "stdout" | "stderr";
export type LineLevel = "info" | "warning" | "error";

export interface BuildOutputEvent {
  run_id: string;
  seq: number;
  stream: OutputStream;
  text: string;
  timestamp_ms: number;
  level: LineLevel;
}

export interface PhaseMark {
  name: string;
  at_ms: number;
}

export interface ProgressInfo {
  current: number;
  total: number;
  percent: number;
  source: string;
}

export interface RunStats {
  warnings: number;
  errors: number;
  downloads: number;
  lines_total: number;
  current_phase: string | null;
  phases: PhaseMark[];
  progress: ProgressInfo | null;
  first_fatal: string | null;
}

export interface OutputBatch {
  run_id: string;
  events: BuildOutputEvent[];
  stats: RunStats;
  state: BuildState;
}

export type TerminationConfirmation = "not_requested" | "confirmed" | "unconfirmed";

export interface CompletionEvent {
  run_id: string;
  state: BuildState;
  exit_code: number | null;
  signal: number | null;
  duration_ms: number;
  cancel_requested: boolean;
  termination: TerminationConfirmation;
  stats: RunStats;
  log_file: string;
}

export interface RunRecord {
  id: string;
  kind: RunKind;
  project_id: string;
  project_name: string;
  project_root: string | null;
  label: string;
  preset_id: string | null;
  executable: string;
  args: string[];
  working_dir: string;
  env_keys: string[];
  env_redacted: EnvironmentOverride[];
  shell_mode: boolean;
  state: BuildState;
  started_at_ms: number;
  finished_at_ms: number | null;
  duration_ms: number | null;
  exit_code: number | null;
  signal: number | null;
  cancel_requested: boolean;
  termination: TerminationConfirmation;
  repo_branch: string | null;
  repo_commit: string | null;
  repo_dirty: boolean;
  pid: number | null;
  log_file: string;
  stats: RunStats;
  expected_output_dirs: string[];
}

export async function previewBuildCommand(invocation: BuildInvocation): Promise<BuildInvocation> {
  return invoke<BuildInvocation>("preview_build_command", { invocation });
}

export async function runInvocation(projectId: string, invocation: BuildInvocation): Promise<string> {
  return invoke<string>("run_invocation", { projectId, invocation });
}

export async function cancelRun(projectId: string): Promise<void> {
  return invoke<void>("cancel_run", { projectId });
}

export async function forceKillRun(projectId: string): Promise<void> {
  return invoke<void>("force_kill_run", { projectId });
}

export async function hasActiveRun(projectId: string): Promise<boolean> {
  return invoke<boolean>("has_active_run", { projectId });
}

export async function reconcileInterruptedRuns(): Promise<string[]> {
  return invoke<string[]>("reconcile_interrupted_runs");
}

export function subscribeToRunOutput(
  runId: string,
  onEvent: (batch: OutputBatch) => void,
): Promise<UnlistenFn> {
  return listen<OutputBatch>(`run-output://${runId}`, (e) => onEvent(e.payload));
}

export function subscribeToRunCompletion(
  runId: string,
  onEvent: (event: CompletionEvent) => void,
): Promise<UnlistenFn> {
  return listen<CompletionEvent>(`run-complete://${runId}`, (e) => onEvent(e.payload));
}

// ---------------------------------------------------------------------------
// Project commands
// ---------------------------------------------------------------------------

export async function openProject(path: string): Promise<ManagedProject> {
  return invoke<ManagedProject>("open_project", { path });
}

export async function listProjects(): Promise<ManagedProject[]> {
  return invoke<ManagedProject[]>("list_projects");
}

export async function getProject(projectId: string): Promise<ManagedProject> {
  return invoke<ManagedProject>("get_project", { projectId });
}

export async function listRecentProjects(): Promise<RecentProject[]> {
  return invoke<RecentProject[]>("list_recent_projects");
}

export async function getRepositoryStatus(projectId: string): Promise<RepositoryStatus> {
  return invoke<RepositoryStatus>("get_repository_status", { projectId });
}

// ---------------------------------------------------------------------------
// Git commands (clone / fetch / checkout / submodule / diff)
// ---------------------------------------------------------------------------

export type RefKind = "default_branch" | "branch" | "tag" | "commit";

export interface CloneRequest {
  source_url: string;
  destination: string;
  shallow: boolean;
  git_ref: string | null;
  ref_kind: RefKind | null;
}

export async function validateCloneDestination(destination: string): Promise<void> {
  return invoke<void>("validate_clone_destination", { destination });
}

export async function previewCloneInvocation(request: CloneRequest): Promise<BuildInvocation> {
  return invoke<BuildInvocation>("preview_clone_invocation", { request });
}

export async function previewCheckoutInvocation(
  repoRoot: string,
  gitRef: string,
): Promise<BuildInvocation> {
  return invoke<BuildInvocation>("preview_checkout_invocation", { repoRoot, gitRef });
}

export async function previewFetchInvocation(
  repoRoot: string,
  remote: string,
): Promise<BuildInvocation> {
  return invoke<BuildInvocation>("preview_fetch_invocation", { repoRoot, remote });
}

export async function previewSubmoduleUpdateInvocation(repoRoot: string): Promise<BuildInvocation> {
  return invoke<BuildInvocation>("preview_submodule_update_invocation", { repoRoot });
}

export async function previewDiffPatchInvocation(
  repoRoot: string,
  stagedToo: boolean,
): Promise<BuildInvocation> {
  return invoke<BuildInvocation>("preview_diff_patch_invocation", { repoRoot, stagedToo });
}

// ---------------------------------------------------------------------------
// Editor commands
// ---------------------------------------------------------------------------

export interface FileTreeEntry {
  name: string;
  rel_path: string;
  is_dir: boolean;
  size: number | null;
  protected: boolean;
}

export interface FileContent {
  rel_path: string;
  content: string;
  mtime_ms: number;
  protected: boolean;
  size: number;
}

export interface SaveResult {
  rel_path: string;
  mtime_ms: number;
  bytes_written: number;
}

export interface DiffLine {
  tag: "context" | "insert" | "delete";
  text: string;
}

export async function listFileTree(projectId: string): Promise<FileTreeEntry[]> {
  return invoke<FileTreeEntry[]>("list_file_tree", { projectId });
}

export async function readProjectFile(projectId: string, relPath: string): Promise<FileContent> {
  return invoke<FileContent>("read_project_file", { projectId, relPath });
}

/** Throws with a message prefixed `EXTERNAL_CHANGE:` if the file changed on disk. */
export async function writeProjectFile(
  projectId: string,
  relPath: string,
  content: string,
  expectedMtimeMs: number | null,
  force: boolean,
): Promise<SaveResult> {
  return invoke<SaveResult>("write_project_file", {
    projectId,
    relPath,
    content,
    expectedMtimeMs,
    force,
  });
}

export async function deleteProjectFile(projectId: string, relPath: string): Promise<void> {
  return invoke<void>("delete_project_file", { projectId, relPath });
}

export async function renameProjectFile(
  projectId: string,
  fromRel: string,
  toRel: string,
): Promise<void> {
  return invoke<void>("rename_project_file", { projectId, fromRel, toRel });
}

export async function diffProjectFileAgainstDisk(
  projectId: string,
  relPath: string,
  proposedContent: string,
): Promise<DiffLine[]> {
  return invoke<DiffLine[]>("diff_project_file_against_disk", {
    projectId,
    relPath,
    proposedContent,
  });
}

export async function diffProjectFileAgainstHead(
  projectId: string,
  relPath: string,
): Promise<DiffLine[] | null> {
  return invoke<DiffLine[] | null>("diff_project_file_against_head", { projectId, relPath });
}

/** Throws with the parser's error message if invalid. */
export async function validateYamlContent(content: string): Promise<void> {
  return invoke<void>("validate_yaml_content", { content });
}

export async function isProtectedPath(relPath: string): Promise<boolean> {
  return invoke<boolean>("is_protected_path", { relPath });
}

// ---------------------------------------------------------------------------
// RBM inspector
// ---------------------------------------------------------------------------

export interface RbmField {
  key: string;
  value_preview: string;
  kind: string;
}

export interface RbmInspection {
  parsed_ok: boolean;
  parse_error: string | null;
  top_level_fields: RbmField[];
  variables: RbmField[];
  input_files: string[];
  targets_or_platforms: string[];
  includes: string[];
  notes: string[];
}

export async function inspectRbm(projectId: string, relPath: string): Promise<RbmInspection> {
  return invoke<RbmInspection>("inspect_rbm", { projectId, relPath });
}

// ---------------------------------------------------------------------------
// Discovery (real Makefile targets, docs, wrapper scripts)
// ---------------------------------------------------------------------------

export interface DiscoveredTarget {
  name: string;
  help_text: string | null;
  source_file: string;
  line: number;
  availability_note: string;
}

export interface DiscoveredScript {
  rel_path: string;
  executable: boolean;
}

export interface DiscoveryReport {
  targets: DiscoveredTarget[];
  wrapper_scripts: DiscoveredScript[];
  doc_files_found: string[];
}

export async function discoverSupportedCommands(projectId: string): Promise<DiscoveryReport> {
  return invoke<DiscoveryReport>("discover_supported_commands", { projectId });
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

export type CapabilityStatus = "passed" | "warning" | "missing" | "unknown" | "not_applicable";

export interface ToolCheck {
  name: string;
  status: CapabilityStatus;
  detail: string;
  version: string | null;
}

export interface HostProfile {
  os: string;
  os_version: string | null;
  kernel_version: string | null;
  arch: string;
  cpu_cores: number;
  total_memory_bytes: number;
  available_memory_bytes: number;
  hostname: string | null;
}

export interface DiskInfo {
  mount_point: string;
  total_bytes: number;
  available_bytes: number;
}

export interface RepositoryHealth {
  is_git_repo: boolean;
  looks_like_tor_browser_build: boolean;
  detected_markers: string[];
  submodule_count: number;
  validation_errors: string[];
}

export interface DiagnosticsReport {
  generated_at_ms: number;
  host: HostProfile;
  disk: DiskInfo | null;
  tools: ToolCheck[];
  repository: RepositoryHealth | null;
  readiness_note: string;
}

export async function runHostDiagnostics(projectId: string | null): Promise<DiagnosticsReport> {
  return invoke<DiagnosticsReport>("run_host_diagnostics", { projectId });
}

export async function diagnosticsAsText(report: DiagnosticsReport): Promise<string> {
  return invoke<string>("diagnostics_as_text", { report });
}

export async function diagnosticsAsJson(report: DiagnosticsReport): Promise<string> {
  return invoke<string>("diagnostics_as_json", { report });
}

export async function runDiagnosticToolChecks(): Promise<ToolCheck[]> {
  return invoke<ToolCheck[]>("run_diagnostic_tool_checks");
}

export async function saveDiagnosticsJson(
  report: DiagnosticsReport,
  destination: string,
): Promise<void> {
  return invoke<void>("save_diagnostics_json", { report, destination });
}

// ---------------------------------------------------------------------------
// Logs / run history
// ---------------------------------------------------------------------------

export interface DiskUsage {
  total_bytes: number;
  file_count: number;
}

export async function listRunHistory(): Promise<RunRecord[]> {
  return invoke<RunRecord[]>("list_run_history");
}

export async function getLogTail(runId: string, maxLines: number): Promise<string[]> {
  return invoke<string[]>("get_log_tail", { runId, maxLines });
}

export async function getFullLog(runId: string): Promise<string> {
  return invoke<string>("get_full_log", { runId });
}

export async function getLogFilePath(runId: string): Promise<string> {
  return invoke<string>("get_log_file_path", { runId });
}

export async function deleteRunRecord(runId: string): Promise<void> {
  return invoke<void>("delete_run_record", { runId });
}

export async function getLogsDiskUsage(): Promise<DiskUsage> {
  return invoke<DiskUsage>("get_logs_disk_usage");
}

export async function applyLogRetention(): Promise<string[]> {
  return invoke<string[]>("apply_log_retention");
}

export async function buildDiagnosticBundle(runId: string): Promise<string> {
  return invoke<string>("build_diagnostic_bundle", { runId });
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

export type ThemePreference = "system" | "light" | "dark";

export interface SafetySettings {
  allow_advanced_commands: boolean;
  allow_shell_command_mode: boolean;
  reveal_env_values: boolean;
}

export interface RetentionSettings {
  max_logs: number;
  max_age_days: number;
}

export interface EditorSettings {
  tab_size: number;
  insert_spaces: boolean;
  show_whitespace: boolean;
}

export interface ApplicationSettings {
  theme: ThemePreference;
  safety: SafetySettings;
  retention: RetentionSettings;
  editor: EditorSettings;
  recent_projects: RecentProject[];
  default_clone_parent_dir: string | null;
}

export async function getSettings(): Promise<ApplicationSettings> {
  return invoke<ApplicationSettings>("get_settings");
}

export async function updateSettings(newSettings: ApplicationSettings): Promise<void> {
  return invoke<void>("update_settings", { newSettings });
}

// ---------------------------------------------------------------------------
// Preflight
// ---------------------------------------------------------------------------

export type IssueSeverity = "info" | "warning" | "error";

export interface AppIssue {
  severity: IssueSeverity;
  code: string;
  summary: string;
  details: string | null;
  suggested_action: string | null;
  overridable: boolean;
}

export interface PreflightReport {
  repository: RepositoryStatus;
  resolved_command: string;
  working_dir: string;
  available_disk_bytes: number | null;
  low_disk_space_warning: boolean;
  issues: AppIssue[];
  requires_dirty_confirmation: boolean;
  blocked: boolean;
}

export interface PerlModuleCheck {
  module: string;
  status: CapabilityStatus;
  detail: string;
}

export interface PackageManagerInfo {
  id: string;
  name: string;
  path: string;
}

export interface CpanmInstallOption {
  id: string;
  label: string;
  package_manager: string;
  display_command: string;
  can_run_in_app: boolean;
  requires_admin: boolean;
  auth_note: string;
}

export interface PlatformInfo {
  os: string;
  os_version: string | null;
  arch: string;
  perl_path: string | null;
  package_managers: PackageManagerInfo[];
  cpanm_install_options: CpanmInstallOption[];
}

export interface PerlPreflightReport {
  platform: PlatformInfo;
  checks: PerlModuleCheck[];
  missing_modules: string[];
  blocking_modules: string[];
  cpanm_available: boolean;
  cpanm_path: string | null;
  local_lib_root: string | null;
  install_command: string | null;
}

export interface PerlInstallResult {
  checks: PerlModuleCheck[];
  output: string;
  local_lib_root: string | null;
}

export async function getPreflightReport(
  projectId: string,
  invocation: BuildInvocation,
): Promise<PreflightReport> {
  return invoke<PreflightReport>("get_preflight_report", { projectId, invocation });
}

export async function getPerlPreflight(): Promise<PerlPreflightReport> {
  return invoke<PerlPreflightReport>("get_perl_preflight");
}

export async function installCpanm(method: string): Promise<string> {
  return invoke<string>("install_cpanm", { method });
}

export async function installRbmPerlModules(): Promise<PerlInstallResult> {
  return invoke<PerlInstallResult>("install_rbm_perl_modules");
}

// ---------------------------------------------------------------------------
// Artifacts
// ---------------------------------------------------------------------------

export interface BuildArtifact {
  path: string;
  size: number;
  modified_ms: number;
  kind: string;
  sha256: string | null;
}

export interface ArtifactScanResult {
  searched_dirs: string[];
  artifacts: BuildArtifact[];
  any_dir_existed: boolean;
}

export async function scanProjectArtifacts(
  projectId: string,
  expectedDirs: string[],
  computeHashes: boolean,
): Promise<ArtifactScanResult> {
  return invoke<ArtifactScanResult>("scan_project_artifacts", {
    projectId,
    expectedDirs,
    computeHashes,
  });
}

// ---------------------------------------------------------------------------
// System integration
// ---------------------------------------------------------------------------

export async function revealPathInFileManager(path: string): Promise<void> {
  return invoke<void>("reveal_path_in_file_manager", { path });
}

export async function openTerminalHere(workingDir: string): Promise<void> {
  return invoke<void>("open_terminal_here", { workingDir });
}
