// Hand-rolled mirrors of crates/kulisawit-server/src/wire.rs response shapes.
// Keep in sync when wire.rs changes (the wire.rs ProjectResponse field has a
// comment pointing back here).

export interface Project {
  id: string;
  name: string;
  repo_path: string;
  created_at: number;
  /**
   * Filled with seeded column IDs only on POST /api/projects responses.
   * Always [] on GET /api/projects (list). Empty on board responses too;
   * use the embedded `columns` array for column IDs.
   */
  column_ids: string[];
}

export interface Task {
  id: string;
  project_id: string;
  column_id: string;
  title: string;
  description: string | null;
  position: number;
  tags: string[];
  linked_files: string[];
  created_at: number;
  updated_at: number;
}

export interface BoardColumn {
  id: string;
  name: string;
  position: number;
  tasks: Task[];
}

export interface BoardResponse {
  project: Project;
  columns: BoardColumn[];
}

// ---- Request types ----

export interface CreateTaskRequest {
  project_id: string;
  column_id: string;
  title: string;
  description?: string;
  tags?: string[];
  linked_files?: string[];
}

export interface UpdateTaskRequest {
  title?: string;
  description?: string;
  column_id?: string;
  tags?: string[];
  linked_files?: string[];
}

export interface DispatchRequest {
  agent: string;
  batch: number;
  variants?: string[];
}

// ---- Response types ----

export interface DispatchResponse {
  attempt_ids: string[];
}

export type AttemptStatus =
  | "queued"
  | "running"
  | "completed"
  | "failed"
  | "cancelled";

export type VerificationStatus = "pending" | "passed" | "failed" | "skipped";

export interface Attempt {
  id: string;
  task_id: string;
  agent_id: string;
  status: AttemptStatus;
  prompt_variant: string | null;
  worktree_path: string;
  branch_name: string;
  started_at: number | null;
  completed_at: number | null;
  // NEW (Phase 3.3.1) — keep in sync with kulisawit-server/src/wire.rs::AttemptResponse
  verification_status: VerificationStatus | null;
  verification_output: string | null;
}

// ---- SSE event shapes (mirror kulisawit-core::AgentEvent + EventEnvelope) ----

export type RunStatus =
  | "starting"
  | "in_progress"
  | "succeeded"
  | "failed"
  | "cancelled";

export type AgentEvent =
  | { type: "stdout"; text: string }
  | { type: "stderr"; text: string }
  | { type: "tool_call"; name: string; input: unknown }
  | { type: "tool_result"; name: string; output: unknown }
  | { type: "file_edit"; path: string; diff: string | null }
  | { type: "status"; status: RunStatus; detail: string | null };

export interface EventEnvelope {
  attempt_id: string;
  event: AgentEvent;
  ts_ms: number;
}

export const TERMINAL_RUN_STATUSES: RunStatus[] = [
  "succeeded",
  "failed",
  "cancelled",
];
