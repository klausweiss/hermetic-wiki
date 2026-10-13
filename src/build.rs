//! Cargo's build script (see `build` in Cargo.toml), not a module of the crate: records
//! the commit of hermetic's source for the site footer (`HERMETIC_COMMIT`), or nothing
//! when it is not built from a git checkout.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    let commit = Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .unwrap_or_default();
    println!("cargo:rustc-env=HERMETIC_COMMIT={commit}");
    // Rebuild when the checked-out commit moves: `HEAD` names the branch, whose ref (or
    // packed-refs) moves on commit. Asked from git, since in a worktree `.git` is a file.
    let git_path = |what: &str| {
        Command::new("git")
            .args(["rev-parse", "--path-format=absolute", "--git-path", what])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| PathBuf::from(s.trim()))
    };
    let mut watch = vec![git_path("HEAD"), git_path("packed-refs")];
    if let Some(head) = git_path("HEAD").and_then(|p| std::fs::read_to_string(p).ok())
        && let Some(r) = head.strip_prefix("ref: ")
    {
        watch.push(git_path(r.trim()));
    }
    for path in watch.into_iter().flatten().filter(|p| p.exists()) {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    println!("cargo:rerun-if-changed=src/build.rs");
}
