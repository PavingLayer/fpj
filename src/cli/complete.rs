use std::ffi::OsStr;

use clap_complete::engine::{CompletionCandidate, PathCompleter, ValueCompleter as _};

use fpj::database::LayoutDatabase;
use fpj::engine::default_db_path;

fn open_db() -> Option<LayoutDatabase> {
    LayoutDatabase::open(&default_db_path()).ok()
}

pub fn complete_layer_names(current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(db) = open_db() else { return vec![] };
    let Ok(names) = db.list_layers() else {
        return vec![];
    };
    let current = current.to_string_lossy();
    names
        .into_iter()
        .filter(|n| n.starts_with(current.as_ref()))
        .map(CompletionCandidate::new)
        .collect()
}

pub fn complete_layout_names(current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(db) = open_db() else { return vec![] };
    let Ok(names) = db.list_layouts() else {
        return vec![];
    };
    let current = current.to_string_lossy();
    names
        .into_iter()
        .filter(|n| n.starts_with(current.as_ref()))
        .map(CompletionCandidate::new)
        .collect()
}

/// Completer for `--source`: `@layer-name` references and filesystem paths.
pub fn complete_layer_source(current: &OsStr) -> Vec<CompletionCandidate> {
    let current_str = current.to_string_lossy();

    if let Some(prefix) = current_str.strip_prefix('@') {
        let Some(db) = open_db() else { return vec![] };
        let Ok(names) = db.list_layers() else {
            return vec![];
        };
        return names
            .into_iter()
            .filter(|n| n.starts_with(prefix))
            .map(|n| CompletionCandidate::new(format!("@{n}")))
            .collect();
    }

    let mut candidates = Vec::new();

    // When input is empty, suggest @<layer> references alongside paths
    if current_str.is_empty() {
        if let Some(db) = open_db() {
            if let Ok(names) = db.list_layers() {
                for name in names {
                    candidates.push(CompletionCandidate::new(format!("@{name}")));
                }
            }
        }
    }

    candidates.extend(PathCompleter::any().complete(current));
    candidates
}
