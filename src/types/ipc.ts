// Mirror of src-tauri/src/db/schema.rs and all Rust IPC types.

export interface Task {
  short_id: string;
  notion_page_id: string;
  title: string;
  status: string;
  priority: string | null;
  last_synced_at: number;
}

export interface Repo {
  id: string;
  host: string;
  group_path: string;
  project: string;
  local_path: string;
}

export interface Worktree {
  id: string;
  task_id: string;
  repo_id: string;
  branch: string;
  path: string;
  is_active: number;
  created_at: number;
  /** Review sessions: the MR's target branch (diff/log base = origin/<base_ref>). */
  base_ref: string | null;
}

export interface Mr {
  id: string;
  worktree_id: string;
  platform: string;
  remote_id: string;
  url: string;
  state: string;
}

/** One note within an MR discussion thread (GitLab/GitHub-normalized, loose). */
export interface ThreadNote {
  author?: { username?: string; name?: string };
  body?: string;
  resolved?: boolean;
  resolvable?: boolean;
  position?: {
    new_path?: string;
    new_line?: number;
    line_range?: { end?: { new_line?: number } };
  };
}

/** An MR discussion thread as returned by `get_mr_threads`. */
export interface MrThread {
  id?: string;
  notes?: ThreadNote[];
}

export interface Annotation {
  id: string;
  task_id: string;
  repo_id: string;
  file_path: string;
  /** Anchor line (== start_line). */
  line_num: number;
  /** First line of the annotated range (new-side). */
  start_line: number;
  /** Last line of the range; single-line annotations have start == end. */
  end_line: number;
  content: string;
  author: string;
  status: 'open' | 'resolved';
  created_at: number;
}

// git_engine types
export interface DiffLine {
  num: number;
  content: string;
  type: 'add' | 'del' | 'ctx';
}

export interface Hunk {
  header: string;
  lines: DiffLine[];
}

/** Mirror of git_engine::types::BlameLine. `uncommitted` = git's all-zero sha: the
 *  line exists only on disk, so there is no commit to open. */
export interface BlameLine {
  line: number;
  sha: string;
  short_sha: string;
  author: string;
  time: number;
  summary: string;
  uncommitted: boolean;
}

export interface FileDiff {
  path: string;
  added: number;
  deleted: number;
  /** Git status letter: "A" | "M" | "D" (meaningful in the summary payload). */
  status: string;
  /** Working-tree staged state: true = staged, false = unstaged, null/undefined =
   *  no local change (committed-only — no stage checkbox). */
  staged?: boolean | null;
  /** Empty in the summary payload — fetched lazily per file via get_file_diff. */
  hunks: Hunk[];
}

export interface RepoDiff {
  repo_id: string;
  branch: string;
  fetch_status: string;
  files: FileDiff[];
}

export interface DiffResult {
  task_id: string;
  repos: RepoDiff[];
}

export interface WorktreeStatus {
  worktree_id: string;
  modified: number;
  staged: number;
  ahead: number;
  behind: number;
  remote_branch_gone: boolean;
}

export interface CommitEntry {
  sha: string;
  short_sha: string;
  message: string;
  author: string;
  timestamp: number;
  /** True = upstream base history; false = the task's own commits. */
  is_base: boolean;
}

/** Rich MR/PR fields from `get_mr_details` — normalized across GitLab/GitHub. */
export interface MrDetails {
  title: string;
  description: string;
  author: string;
  source_branch: string;
  target_branch: string;
  state: string;
  draft: boolean;
  created_at: string;
  web_url: string;
  /** Approval state, folded in from the approvals endpoint. */
  approved?: boolean;
  /** True when the CURRENT user is among the approvers (GitLab only). */
  approved_by_me?: boolean;
  approved_by?: string[];
}

export interface SearchMatch {
  file: string;
  line: number;
  content: string;
}

// confirmation_bridge types
export interface ConfirmationDto {
  id: string;
  task_id: string | null;
  op_type: string;
  payload: unknown;
  origin: 'ui' | 'mcp';
}

// task_manager config types
export interface PropertyNames {
  status: string;
  priority: string | null;
  sprint: string | null;
  project: string | null;
  assignee: string | null;
}

/** The three status values the app writes. Reading a status needs no map — see
 *  lib/taskStatus.ts, which classifies whatever label Notion returns. */
export interface StatusMap {
  ready: string;
  in_progress: string;
  done: string;
}

export interface FilterConfig {
  exclude_statuses: string[];
  filter_by_assignee: boolean;
}

export interface NotionConfig {
  database_id: string;
  user_id: string;
  properties: PropertyNames;
  status_map: StatusMap;
  filters: FilterConfig;
  task_template_page_id?: string | null;
  default_project_id?: string | null;
}

// ─── Notion database schema (mirror of src-tauri/src/task_manager/schema.rs) ──

/** One property of the task database, as Notion describes it. The property panel
 *  renders by `kind` and offers `options` / `relation_db` as choices, so a new
 *  Notion property becomes editable with no code change. */
export interface PropertySchema {
  name: string;
  /** Notion's type string: select, status, number, multi_select, relation, … */
  kind: string;
  options: string[];
  relation_db: string | null;
  /** False for formulas, rollups and timestamps — shown, but not settable. */
  editable: boolean;
}

export interface TaskSchema {
  database_id: string;
  /** Differs per database ("Task name" here). */
  title_property: string;
  properties: PropertySchema[];
}

/** A page in a relation's target database, offered as a choice. */
export interface RelationOption {
  id: string;
  title: string;
}

/** One property with its current value (mirror of properties.rs).
 *
 *  `value` is the canonical shape for the property's type and is what you send
 *  back to `update_task_property`:
 *    select/status/url/date → string|null · number → number|null ·
 *    checkbox → boolean · multi_select → string[] · relation → string[] (page ids)
 *  `display` is the read-only rendering, used for formulas, rollups and people. */
export interface PropertyValue {
  name: string;
  kind: string;
  value: unknown;
  display: string;
}

/** Locally measured time on a task (mirror of hours.rs). Never written to Notion
 *  without an explicit log. */
export interface TaskTime {
  task_id: string;
  tracked_seconds: number;
  logged_seconds: number;
  today_seconds: number;
  /** tracked − logged: what the log button offers. */
  unlogged_seconds: number;
}

// ─── Home snapshot (mirror of src-tauri/src/home/mod.rs) ─────────────────────

/** The MR facts Home shows per repo; `ci`/`unresolved` are cached forge reads. */
export interface HomeMr {
  id: string;
  /** "gitlab" | "github" — decides the reference sigil. */
  platform: string;
  remote_id: string;
  state: string;
  url: string;
  ci: string | null;
  unresolved: number;
  /** Carries at least one approval — rendered as a pill on Home. */
  approved: boolean;
}

/** One repo of a live session: provisioning + working-tree state. */
export interface HomeRepo {
  repo_id: string;
  project: string;
  worktree_id: string | null;
  branch: string | null;
  provisioned: boolean;
  /** Provisioned but the directory is gone (stale row). */
  missing: boolean;
  modified: number;
  staged: number;
  conflicted: number;
  ahead: number;
  behind: number;
  added: number;
  deleted: number;
  files_changed: number;
  /** Files ticked off in a review session (drives the viewed progress bar). */
  files_reviewed: number;
  mr: HomeMr | null;
}

/** A session with a local footprint: a provisioned task, an explorer, a review. */
export interface HomeEntry {
  short_id: string;
  title: string;
  status: string;
  /** Notion priority ("High"/"Medium"/"Low"); null for synthetic sessions. */
  priority: string | null;
  kind: 'task' | 'explorer' | 'review';
  repos: HomeRepo[];
}

/** An open MR where the user is a reviewer (from `list_review_mrs`). */
export interface ReviewMr {
  /** "gitlab" | "github" — decides the reference sigil (`!42` vs `#42`). */
  platform: string;
  /** Full project path on the forge, e.g. "wiremind/devops/gitlab-ci-common". */
  project_full: string;
  iid: number;
  title: string;
  author: string;
  source_branch: string;
  target_branch: string;
  draft: boolean;
  web_url: string;
  updated_at: string;
  /** MAIN clone path when the project is already cloned locally. */
  local_path: string | null;
  /** Already approved (by anyone) but not merged. */
  approved: boolean;
}

/** One viewed-file row from `get_reviewed_files`. */
export interface ReviewedFile {
  repo_id: string;
  file_path: string;
}

/** A clone under `<worktree_root>/MAIN` — the repo pool the pickers list. */
export interface MainRepo {
  url: string;
  local_path: string;
  /** Path relative to MAIN, e.g. "DevOps/mayo". */
  slug: string;
}

export interface GitConfig {
  worktree_root: string;
}

export type ThemeName = 'frappe' | 'latte' | 'onedark' | 'onelight';

export const THEMES: { id: ThemeName; label: string }[] = [
  { id: 'frappe',   label: 'Catppuccin Frappé' },
  { id: 'latte',    label: 'Catppuccin Latte' },
  { id: 'onedark',  label: 'One Dark' },
  { id: 'onelight', label: 'One Light' },
];

export const DEFAULT_THEME: ThemeName = 'frappe';
/** Base UI font size in px. Mirrors `default_font_size()` in task_manager/config.rs —
 *  the whole type ramp is derived from it (lib/theme.ts::applyFontSize). */
export const DEFAULT_FONT_SIZE = 15;

export interface UiConfig {
  font_size: number;
  theme: ThemeName;
  /** Monospace family, named as fontconfig reports it (`fc-list : family`). */
  font_family: string;
}

export interface Config {
  notion: NotionConfig;
  git: GitConfig;
  ui: UiConfig;
}

// Event payload types (Tauri events emitted from Rust)
export interface WorkspaceStubEvent {
  task: Task;
  kind?: 'task' | 'explorer' | 'review';
}

export interface WorkspaceReadyEvent {
  task: Task;
  worktrees: Worktree[];
  repos: Repo[];
  kind?: 'task' | 'explorer' | 'review';
}

export interface ConfirmationRequestedEvent {
  id: string;
  task_id: string | null;
  op_type: string;
  payload: unknown;
  origin: 'ui' | 'mcp';
}

export interface ConfirmationResolvedEvent {
  id: string;
  task_id: string | null;
  approved: boolean;
  op_type: string;
  result: unknown;
  /** Non-null = the op was approved but FAILED (the confirmation row is gone; no
   *  retry). Null on a successful/denied resolution. */
  error: string | null;
}

export interface FileChangedEvent {
  paths: string[];
}

export interface RebaseConflictEvent {
  worktree_id: string;
  files: string[];
}

export interface RebaseDoneEvent {
  worktree_id: string;
  aborted?: true;
}

export interface WorktreeClosedEvent {
  worktree_id: string;
  task_id: string;
  repo_id: string;
}

export interface PtyOutputEvent {
  session_id: string;
  data: number[];
}

export interface PtyExitEvent {
  session_id: string;
}

// ─── Agent activity (mirror of src-tauri/src/agent_hooks/mod.rs) ─────────────

/** `idle` = between turns, free to take a prompt · `working` = mid-turn ·
 *  `waiting` = blocked on the user in its own terminal. */
export type AgentState = 'idle' | 'working' | 'waiting';

export interface AgentToolCall {
  name: string;
  /** One-line rendering of the tool's input, e.g. `cargo test --all`. */
  detail: string | null;
}

/** What one agent is doing, keyed by task short id. Reported by Claude Code's
 *  hooks, so it survives the agent's tab being closed. */
export interface AgentActivity {
  task_id: string;
  state: AgentState;
  /** In-flight tool, or the one awaiting approval when state is `waiting`. */
  tool: AgentToolCall | null;
  last_message: string | null;
  since: number;
}

export interface PtyStartedEvent {
  session_id: string;
  task_id: string;
  pty_type: 'agent' | 'terminal';
}

export interface TaskPausedEvent {
  short_id: string;
}

export interface TaskFinishedEvent {
  short_id: string;
  done_status: string;
}
