//! Guards the rules in `docs/clean-room.md` that a machine can check: no
//! driver files, firmware or other binaries in the repository, and no
//! analysis material.

use std::path::{Path, PathBuf};

/// File types that have no business in this repository.
const FORBIDDEN_EXTENSIONS: [&str; 21] = [
    "sys", "inf", "cat", "spt", "hex", "bin", "dll", "exe", "msi", "cab", "zip", "7z", "syx", "pdb", "gzf", "rep",
    "png", "jpg", "ico", "ttf", "otf",
];

/// Folders that hold build output or analysis material and are not tracked.
const SKIPPED_FOLDERS: [&str; 5] = ["target", "dist", ".git", "evidence", "re-tools"];

/// Marks of decompiler output.
const DECOMPILER_MARKS: [&str; 4] = ["FUN_000", "param_1", "local_res", "undefined8"];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            if !SKIPPED_FOLDERS.contains(&name.as_str()) {
                walk(&path, files);
            }
        } else {
            files.push(path);
        }
    }
}

fn tracked_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk(&repository_root(), &mut files);
    assert!(files.len() > 20, "the repository walk found too few files to be believable");
    files
}

#[test]
fn no_driver_files_or_binaries() {
    let offenders: Vec<_> = tracked_files()
        .into_iter()
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| FORBIDDEN_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        })
        .collect();
    assert!(offenders.is_empty(), "files that must not be in the repository: {offenders:?}");
}

#[test]
fn no_decompiler_output() {
    let this_file = Path::new(file!()).file_name().map(|n| n.to_owned());
    let mut offenders = Vec::new();
    for path in tracked_files() {
        if path.file_name().map(|n| n.to_owned()) == this_file {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        if DECOMPILER_MARKS.iter().any(|mark| text.contains(mark)) {
            offenders.push(path);
        }
    }
    assert!(offenders.is_empty(), "files that look like decompiler output: {offenders:?}");
}
