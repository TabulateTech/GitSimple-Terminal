use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::model::{BranchSync, CmdResult, CommitEntry, DeleteTarget, FileStatus};

pub(crate) fn run_cmd<I, S>(program: &str, args: I) -> CmdResult
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    match Command::new(program).args(args).output() {
        Ok(output) => {
            let mut text = String::from_utf8_lossy(&output.stdout).to_string();
            if text.trim().is_empty() {
                text = String::from_utf8_lossy(&output.stderr).to_string();
            }
            CmdResult {
                ok: output.status.success(),
                text: text.trim().to_string(),
            }
        }
        Err(error) => CmdResult {
            ok: false,
            text: error.to_string(),
        },
    }
}

pub(crate) fn git_ok<I, S>(args: I) -> CmdResult
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    run_cmd("git", args)
}

pub(crate) fn recent_commits() -> Vec<CommitEntry> {
    let result = git_ok([
        "--no-pager",
        "log",
        "--pretty=format:%h%x09%s",
        "--decorate=short",
        "-20",
        "--color=never",
    ]);
    if !result.ok {
        return Vec::new();
    }
    result
        .text
        .lines()
        .filter_map(|line| {
            let (hash, summary) = line.split_once('\t')?;
            Some(CommitEntry {
                hash: hash.to_string(),
                summary: summary.to_string(),
            })
        })
        .collect()
}

pub(crate) fn commit_detail(hash: &str) -> String {
    let result = git_ok([
        "--no-pager",
        "show",
        "--stat",
        "--summary",
        "--color=never",
        "--format=commit %h%nAuthor: %an%nDate:   %ad%n%n%B",
        hash,
    ]);
    if result.ok && !result.text.trim().is_empty() {
        limit_text(&result.text)
    } else {
        format!("No se pudo leer el commit {hash}.")
    }
}

pub(crate) fn pending_push_count() -> usize {
    let result = git_ok(["rev-list", "--count", "@{u}..HEAD"]);
    if result.ok {
        result.text.trim().parse::<usize>().unwrap_or(0)
    } else {
        0
    }
}

pub(crate) fn branch_sync_state() -> BranchSync {
    let upstream = git_ok(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"]);
    if !upstream.ok {
        return BranchSync::default();
    }

    let result = git_ok(["rev-list", "--left-right", "--count", "HEAD...@{u}"]);
    if result.ok {
        parse_branch_sync_counts(&result.text).unwrap_or(BranchSync {
            has_upstream: true,
            ..BranchSync::default()
        })
    } else {
        BranchSync {
            has_upstream: true,
            ..BranchSync::default()
        }
    }
}

pub(crate) fn parse_branch_sync_counts(text: &str) -> Option<BranchSync> {
    let mut parts = text.split_whitespace();
    let ahead = parts.next()?.parse::<usize>().ok()?;
    let behind = parts.next()?.parse::<usize>().ok()?;
    Some(BranchSync {
        has_upstream: true,
        ahead,
        behind,
    })
}

pub(crate) fn rebase_in_progress() -> bool {
    git_path_exists("rebase-merge") || git_path_exists("rebase-apply")
}

pub(crate) fn continue_rebase() -> CmdResult {
    git_ok(["-c", "core.editor=true", "rebase", "--continue"])
}

pub(crate) fn current_local_branch() -> Option<String> {
    git_branch_show_current()
        .or_else(rebase_branch_name)
        .or_else(local_branch_pointing_at_head)
}

pub(crate) fn git_branch_show_current() -> Option<String> {
    let branch = git_text(["branch", "--show-current"], "");
    clean_branch_name(&branch)
}

pub(crate) fn rebase_branch_name() -> Option<String> {
    ["rebase-merge/head-name", "rebase-apply/head-name"]
        .into_iter()
        .filter_map(git_path)
        .find_map(|path| {
            fs::read_to_string(path)
                .ok()
                .and_then(|value| clean_branch_name(&value))
        })
}

pub(crate) fn local_branch_pointing_at_head() -> Option<String> {
    let result = git_ok(["branch", "--format=%(refname:short)", "--points-at", "HEAD"]);
    if !result.ok {
        return None;
    }
    result.text.lines().find_map(clean_branch_name)
}

pub(crate) fn clean_branch_name(value: &str) -> Option<String> {
    let branch = value
        .trim()
        .strip_prefix("refs/heads/")
        .unwrap_or(value.trim())
        .trim();
    if branch.is_empty() || branch == "HEAD" || branch == "sin rama" {
        None
    } else {
        Some(branch.to_string())
    }
}

pub(crate) fn git_path(relative: &str) -> Option<PathBuf> {
    let path = git_text(["rev-parse", "--git-path", relative], "");
    if path.is_empty() {
        return None;
    }
    let path = PathBuf::from(path);
    if path.exists() { Some(path) } else { None }
}

pub(crate) fn git_path_exists(relative: &str) -> bool {
    git_path(relative).is_some()
}

pub(crate) fn setup_upstream_for_branch(branch: &str) -> CmdResult {
    if branch.is_empty() || branch == "HEAD" || branch == "sin rama" {
        return CmdResult {
            ok: false,
            text: "no se pudo detectar una rama local para conectar con GitHub".to_string(),
        };
    }

    let fetch = git_ok(["fetch", "origin"]);
    if !fetch.ok {
        return fetch;
    }

    let remote_branch = if git_ok(["rev-parse", "--verify", &format!("origin/{branch}")]).ok {
        branch.to_string()
    } else if git_ok(["rev-parse", "--verify", "origin/main"]).ok {
        "main".to_string()
    } else if git_ok(["rev-parse", "--verify", "origin/master"]).ok {
        "master".to_string()
    } else {
        return git_ok(["push", "-u", "origin", branch]);
    };

    git_ok([
        "branch",
        "--set-upstream-to",
        &format!("origin/{remote_branch}"),
        branch,
    ])
}

pub(crate) fn delete_repo_confirmation_body(
    target: DeleteTarget,
    repo: Option<&str>,
    root: &str,
) -> String {
    match target {
        DeleteTarget::Local => format!(
            "Esto eliminara el repositorio Git local (.git) en:\n{}\nTus archivos se quedan en la carpeta.",
            short_path(root, 72)
        ),
        DeleteTarget::Github => format!(
            "Esto eliminara el repositorio en GitHub:\n{}\nTus archivos locales se quedan intactos.",
            repo.unwrap_or("sin repo GitHub")
        ),
        DeleteTarget::Both => format!(
            "Esto eliminara el repo en GitHub y luego el Git local (.git).\nGitHub: {}\nLocal: {}\nTus archivos se quedan en la carpeta.",
            repo.unwrap_or("sin repo GitHub"),
            short_path(root, 72)
        ),
    }
}

pub(crate) fn delete_github_repo(repo: &str) -> CmdResult {
    run_cmd("gh", ["repo", "delete", repo, "--yes"])
}

pub(crate) fn delete_local_repo(root: &str) -> CmdResult {
    let root = PathBuf::from(root.trim());
    if root.as_os_str().is_empty() {
        return CmdResult {
            ok: false,
            text: "ruta local vacia".to_string(),
        };
    }

    let git_path = root.join(".git");
    if git_path.is_dir() {
        match fs::remove_dir_all(&git_path) {
            Ok(()) => CmdResult {
                ok: true,
                text: "repo local eliminado; archivos conservados".to_string(),
            },
            Err(error) => CmdResult {
                ok: false,
                text: error.to_string(),
            },
        }
    } else if git_path.is_file() {
        match fs::remove_file(&git_path) {
            Ok(()) => CmdResult {
                ok: true,
                text: "enlace .git eliminado; archivos conservados".to_string(),
            },
            Err(error) => CmdResult {
                ok: false,
                text: error.to_string(),
            },
        }
    } else {
        CmdResult {
            ok: false,
            text: "no se encontro .git en la carpeta del proyecto".to_string(),
        }
    }
}

pub(crate) fn github_repo_slug(remote: &str) -> Option<String> {
    let remote = remote.trim().trim_end_matches('/');
    if remote.is_empty() || remote == "sin origin" {
        return None;
    }

    let slug = if let Some(rest) = remote.strip_prefix("git@github.com:") {
        rest
    } else if let Some(rest) = remote.strip_prefix("ssh://git@github.com/") {
        rest
    } else if let Some((_, rest)) = remote.split_once("github.com/") {
        rest
    } else {
        return None;
    };

    let slug = slug.trim_end_matches(".git").trim_matches('/');
    let mut parts = slug.split('/');
    let owner = parts.next()?.trim();
    let repo = parts.next()?.trim();
    if owner.is_empty() || repo.is_empty() {
        None
    } else {
        Some(format!("{owner}/{repo}"))
    }
}

pub(crate) fn is_push_alignment_error(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    text.contains("[rejected]")
        || text.contains("fetch first")
        || text.contains("non-fast-forward")
        || text.contains("failed to push some refs")
        || text.contains("tip of your current branch is behind")
}

pub(crate) fn staged_preview() -> String {
    let files = staged_file_count();
    let stat = git_ok(["diff", "--cached", "--shortstat", "--color=never"]);
    let stat = stat.text.trim();
    if files == 0 {
        "sin archivos staged".to_string()
    } else if stat.is_empty() {
        format!("{files} archivo(s) staged")
    } else {
        format!("{files} archivo(s) staged, {stat}")
    }
}

pub(crate) fn staged_file_count() -> usize {
    let result = git_ok(["diff", "--cached", "--name-only", "--color=never"]);
    if result.ok {
        result
            .text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count()
    } else {
        0
    }
}

pub(crate) fn unstage_file(path: &str) -> CmdResult {
    if !has_head_commit() {
        return unstage_without_head(path);
    }

    let result = git_ok(["restore", "--staged", "--", path]);
    if result.ok || !is_missing_head_error(&result.text) {
        return result;
    }
    unstage_without_head(path)
}

pub(crate) fn unstage_without_head(path: &str) -> CmdResult {
    let result = git_ok(["rm", "--cached", "-r", "--", path]);
    if result.ok {
        CmdResult {
            ok: true,
            text: "archivo removido del stage; tu archivo sigue en la carpeta".to_string(),
        }
    } else {
        result
    }
}

pub(crate) fn has_head_commit() -> bool {
    git_ok(["rev-parse", "--verify", "HEAD"]).ok
}

pub(crate) fn is_selected_head_commit(selected: &str, head: &str) -> bool {
    !selected.is_empty() && selected == head
}

pub(crate) fn is_missing_head_error(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    let normalized = text.replace(['\'', '"', '`'], "");
    normalized.contains("could not resolve head")
        || text.contains("ambiguous argument 'head'")
        || text.contains("unknown revision or path not in the working tree")
        || normalized.contains("bad revision head")
}

pub(crate) fn git_text<I, S>(args: I, fallback: &str) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let result = git_ok(args);
    if result.ok && !result.text.trim().is_empty() {
        result.text.trim().to_string()
    } else {
        fallback.to_string()
    }
}

pub(crate) fn format_action(action: &str, result: CmdResult) -> String {
    if result.ok {
        if result.text.is_empty() {
            format!("{action}: OK.")
        } else {
            format!("{action}: OK. {}", result.text)
        }
    } else if result.text.is_empty() {
        format!("{action}: ERROR.")
    } else {
        format!("{action}: ERROR. {}", result.text)
    }
}

pub(crate) fn status_files() -> Vec<FileStatus> {
    let output = Command::new("git")
        .args(["status", "--porcelain=v1", "-z", "-uall"])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    let mut files = Vec::new();
    let mut parts = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty());
    while let Some(record) = parts.next() {
        let text = String::from_utf8_lossy(record);
        let xy = text.get(..2).unwrap_or("  ").to_string();
        let path = text.get(3..).unwrap_or("").to_string();
        let mut display = path.clone();
        if matches!(xy.as_bytes().first(), Some(b'R' | b'C')) {
            if let Some(old) = parts.next() {
                let old = String::from_utf8_lossy(old);
                display = format!("{old} -> {path}");
            }
        }
        files.push(FileStatus { xy, path, display });
    }
    files
}

pub(crate) fn file_diff(file: &FileStatus) -> String {
    if file.is_untracked() {
        return preview_untracked(&file.path);
    }

    if file.is_staged() {
        let cached = git_ok([
            "--no-pager",
            "diff",
            "--cached",
            "--color=never",
            "--",
            &file.path,
        ]);
        if !cached.text.trim().is_empty() {
            return limit_text(&cached.text);
        }
    }

    let worktree = git_ok(["--no-pager", "diff", "--color=never", "--", &file.path]);
    if !worktree.text.trim().is_empty() {
        return limit_text(&worktree.text);
    }

    let cached = git_ok([
        "--no-pager",
        "diff",
        "--cached",
        "--color=never",
        "--",
        &file.path,
    ]);
    if !cached.text.trim().is_empty() {
        return limit_text(&cached.text);
    }

    "No hay diff disponible para este archivo.".to_string()
}

pub(crate) fn preview_untracked(path: &str) -> String {
    let path_ref = Path::new(path);
    let Ok(bytes) = std::fs::read(path_ref) else {
        return format!("Archivo no rastreado: {path}\nNo se pudo abrir para vista previa.");
    };
    if bytes.iter().take(4096).any(|byte| *byte == 0) {
        return format!(
            "Archivo no rastreado: {path}\nAVISO: parece ser binario. Revisa antes de agregarlo con SPACE."
        );
    }
    let text = String::from_utf8_lossy(&bytes);
    limit_text(&format!(
        "Archivo no rastreado: {path}\nUsa SPACE para agregarlo al stage.\n\n{text}"
    ))
}

pub(crate) fn limit_text(text: &str) -> String {
    const MAX_CHARS: usize = 50_000;
    if text.chars().count() <= MAX_CHARS {
        text.to_string()
    } else {
        let clipped: String = text.chars().take(MAX_CHARS).collect();
        format!("{clipped}\n\n--- salida recortada ---")
    }
}

pub(crate) fn remote_label(remote: &str) -> String {
    if remote == "sin origin" {
        remote.to_string()
    } else {
        "origin".to_string()
    }
}

pub(crate) fn short_path(path: &str, max: usize) -> String {
    if path.chars().count() <= max || max < 12 {
        return path.to_string();
    }
    let tail = max.saturating_sub(3);
    let suffix: String = path
        .chars()
        .rev()
        .take(tail)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("...{suffix}")
}
