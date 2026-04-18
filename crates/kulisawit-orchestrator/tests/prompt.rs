#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_core::{ColumnId, ProjectId, TaskId};
use kulisawit_db::task::Task;
use kulisawit_orchestrator::prompt::compose_prompt;

fn base_task(title: &str, description: Option<&str>) -> Task {
    Task {
        id: TaskId::new(),
        project_id: ProjectId::new(),
        column_id: ColumnId::new(),
        title: title.to_owned(),
        description: description.map(str::to_owned),
        position: 0,
        tags: vec![],
        linked_files: vec![],
        created_at: 0,
        updated_at: 0,
    }
}

#[test]
fn title_and_description_only() {
    let t = base_task("Add rate limit", Some("Describe the endpoint."));
    let p = compose_prompt(&t, None);
    assert_eq!(p, "# Add rate limit\n\nDescribe the endpoint.\n");
}

#[test]
fn title_only_omits_description_section() {
    let t = base_task("Title-only", None);
    let p = compose_prompt(&t, None);
    assert_eq!(p, "# Title-only\n");
}

#[test]
fn with_linked_files_renders_section() {
    let mut t = base_task("Refactor auth", Some("Split auth into modules."));
    t.linked_files = vec!["src/auth.rs".into(), "src/db.rs".into()];
    let p = compose_prompt(&t, None);
    assert_eq!(
        p,
        "# Refactor auth\n\nSplit auth into modules.\n\n## Linked files\n- src/auth.rs\n- src/db.rs\n"
    );
}

#[test]
fn with_tags_renders_section() {
    let mut t = base_task("Refactor auth", None);
    t.tags = vec!["auth".into(), "security".into()];
    let p = compose_prompt(&t, None);
    assert_eq!(p, "# Refactor auth\n\n## Tags\nauth, security\n");
}

#[test]
fn with_variant_renders_trailing_note() {
    let t = base_task("Refactor auth", Some("Do it."));
    let p = compose_prompt(&t, Some("diff-first"));
    assert_eq!(p, "# Refactor auth\n\nDo it.\n\n(variant: diff-first)\n");
}

#[test]
fn with_all_four_sections_in_order() {
    let mut t = base_task("Refactor auth", Some("Split auth into modules."));
    t.linked_files = vec!["src/auth.rs".into()];
    t.tags = vec!["auth".into(), "security".into()];
    let p = compose_prompt(&t, Some("diff-first"));
    assert_eq!(
        p,
        "# Refactor auth\n\nSplit auth into modules.\n\n## Linked files\n- src/auth.rs\n\n## Tags\nauth, security\n\n(variant: diff-first)\n"
    );
}

#[test]
fn empty_tags_and_files_are_omitted() {
    let mut t = base_task("T", Some("D"));
    t.tags = vec![];
    t.linked_files = vec![];
    let p = compose_prompt(&t, None);
    assert_eq!(p, "# T\n\nD\n");
}
