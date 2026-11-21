use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=JUMPCHAMP_APP_VERSION");
    emit_git_rerun_hints();

    let version = resolve_version();
    println!("cargo:rustc-env=JUMPCHAMP_APP_VERSION={version}");
}

fn resolve_version() -> String {
    if let Ok(val) = std::env::var("JUMPCHAMP_APP_VERSION") {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            return normalize_tag(trimmed);
        }
    }

    if let Some(tag) = git_output(&[
        "-c",
        "versionsort.suffix=-",
        "tag",
        "--points-at",
        "HEAD",
        "--sort=-v:refname",
    ])
    .and_then(|s| {
        s.lines()
            .next()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
    }) {
        return normalize_tag(&tag);
    }

    if let Some(desc) = git_output(&["describe", "--tags", "--always"]) {
        return normalize_tag(&desc);
    }

    format!("v{}", env!("CARGO_PKG_VERSION"))
}

fn normalize_tag(raw: &str) -> String {
    let s = raw.trim();
    if s.starts_with('v') {
        s.to_string()
    } else if s.chars().next().is_some_and(|c| c.is_ascii_digit()) && s.contains('.') {
        format!("v{s}")
    } else {
        s.to_string()
    }
}

fn git_output(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn emit_git_rerun_hints() {
    let Some(raw_git_dir) = git_output(&["rev-parse", "--git-dir"]).map(PathBuf::from) else {
        return;
    };
    let git_dir = std::fs::canonicalize(&raw_git_dir).unwrap_or(raw_git_dir);

    let head = git_dir.join("HEAD");
    if head.exists() {
        println!("cargo:rerun-if-changed={}", head.display());
        if let Ok(contents) = std::fs::read_to_string(&head) {
            if let Some(rel_ref) = contents.trim().strip_prefix("ref: ") {
                let ref_path = git_dir.join(rel_ref.trim());
                if ref_path.exists() {
                    println!("cargo:rerun-if-changed={}", ref_path.display());
                }
            }
        }
    }

    for sub in ["packed-refs", "refs/tags"] {
        let path = git_dir.join(sub);
        if path.exists() {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}
