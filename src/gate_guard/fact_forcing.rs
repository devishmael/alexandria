use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Error produced when a file mutation violates the Fact-Forcing Gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactForcingError {
    UninspectedFileMutation { path: PathBuf, message: String },
}

impl std::fmt::Display for FactForcingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UninspectedFileMutation { path, message } => {
                write!(f, "[E_FACT_FORCING] {}: {}", path.display(), message)
            }
        }
    }
}

impl std::error::Error for FactForcingError {}

/// Tracks inspected/read files within an active agent session to deterministically
/// prevent blind, hallucinated edits to existing files.
#[derive(Debug, Default, Clone)]
pub struct FactForcingTracker {
    inspected_files: HashSet<PathBuf>,
}

impl FactForcingTracker {
    /// Creates a new, empty tracker for a fresh session.
    pub fn new() -> Self {
        Self {
            inspected_files: HashSet::new(),
        }
    }

    /// Normalizes a path for consistent lookup.
    fn normalize_path(path: &Path) -> PathBuf {
        fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }

    /// Records that a file was read or inspected during the session.
    pub fn record_read(&mut self, path: impl AsRef<Path>) {
        let p = path.as_ref();
        self.inspected_files.insert(Self::normalize_path(p));
        self.inspected_files.insert(p.to_path_buf());
    }

    /// Checks if a file was inspected in this session.
    pub fn has_inspected(&self, path: impl AsRef<Path>) -> bool {
        let p = path.as_ref();
        let normalized = Self::normalize_path(p);
        self.inspected_files.contains(p) || self.inspected_files.contains(&normalized)
    }

    /// Verifies whether a planned mutation (edit or write) on `path` is permitted.
    ///
    /// - If the file does not exist on disk, creating a new file is permitted.
    /// - If the file exists, it MUST have been recorded via `record_read`.
    /// - Otherwise, returns `Err(FactForcingError::UninspectedFileMutation)`.
    pub fn verify_mutation(&self, path: impl AsRef<Path>) -> Result<(), FactForcingError> {
        let p = path.as_ref();

        // If the file does not exist yet, brand new file creation is allowed
        if !p.exists() {
            return Ok(());
        }

        // If the file exists, it must have been inspected
        if self.has_inspected(p) {
            Ok(())
        } else {
            Err(FactForcingError::UninspectedFileMutation {
                path: p.to_path_buf(),
                message: format!(
                    "E_FACT_FORCING: File mutation rejected. The file '{}' exists but has not been inspected/read in this session. You must inspect the file and its references before applying modifications.",
                    p.display()
                ),
            })
        }
    }
}
