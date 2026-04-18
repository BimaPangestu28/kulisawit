//! Compose the prompt handed to an `AgentAdapter` from a `Task` row.
//!
//! The prompt is a plain Markdown-ish string:
//!
//! ```text
//! # <title>
//!
//! <description>
//!
//! ## Linked files
//! - <path>
//!
//! ## Tags
//! <tag>, <tag>
//!
//! (variant: <name>)
//! ```
//!
//! Empty sections are omitted. This function is deterministic and allocation-
//! only — no I/O.

use kulisawit_db::task::Task;

pub fn compose_prompt(task: &Task, variant: Option<&str>) -> String {
    let mut sections: Vec<String> = Vec::new();

    sections.push(format!("# {}", task.title));

    if let Some(desc) = task.description.as_deref() {
        if !desc.is_empty() {
            sections.push(desc.to_owned());
        }
    }

    if !task.linked_files.is_empty() {
        let mut block = String::from("## Linked files\n");
        for (idx, f) in task.linked_files.iter().enumerate() {
            if idx > 0 {
                block.push('\n');
            }
            block.push_str("- ");
            block.push_str(f);
        }
        sections.push(block);
    }

    if !task.tags.is_empty() {
        sections.push(format!("## Tags\n{}", task.tags.join(", ")));
    }

    if let Some(v) = variant {
        sections.push(format!("(variant: {v})"));
    }

    let mut out = sections.join("\n\n");
    out.push('\n');
    out
}
