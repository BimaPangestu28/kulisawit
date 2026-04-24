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
