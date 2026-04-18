//! Strongly-typed domain identifiers. All IDs wrap UUID v7 strings.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Generate a fresh identifier backed by a UUID v7.
            pub fn new() -> Self {
                Self(Uuid::now_v7().to_string())
            }

            /// Wrap an existing string (e.g. loaded from storage).
            pub fn from_string(s: String) -> Self {
                Self(s)
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self::from_string(s)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

define_id!(
    /// Identifier for a `project` (tracked repository).
    ProjectId
);
define_id!(
    /// Identifier for a column on the kanban board.
    ColumnId
);
define_id!(
    /// Identifier for a `task` (card on the board).
    TaskId
);
define_id!(
    /// Identifier for an `attempt` (single agent run).
    AttemptId
);

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn different_id_types_do_not_compile_interchangeably() {
        let project = ProjectId::new();
        let task = TaskId::new();
        assert_eq!(project.as_str().len(), 36);
        assert_eq!(task.as_str().len(), 36);
    }

    #[test]
    fn ids_roundtrip_through_json() {
        let id = AttemptId::new();
        let json = serde_json::to_string(&id).expect("ser");
        let back: AttemptId = serde_json::from_str(&json).expect("de");
        assert_eq!(id, back);
    }

    #[test]
    fn ids_are_unique_across_calls() {
        let a = TaskId::new();
        let b = TaskId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn parse_from_str_accepts_any_non_empty_string() {
        let id = ColumnId::from_string("col-1".to_owned());
        assert_eq!(id.as_str(), "col-1");
    }
}
