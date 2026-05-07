use std::{collections::HashMap, time::Instant};

use ratatui::{layout::Rect, style::Color};
use serde::Deserialize;

#[derive(Clone, Debug)]
pub(crate) struct FileStatus {
    pub(crate) xy: String,
    pub(crate) path: String,
    pub(crate) display: String,
}

impl FileStatus {
    pub(crate) fn is_untracked(&self) -> bool {
        self.xy == "??"
    }

    pub(crate) fn is_staged(&self) -> bool {
        !matches!(self.xy.as_bytes().first(), Some(b' ' | b'?') | None)
    }

    pub(crate) fn is_unstaged(&self) -> bool {
        self.xy.as_bytes().get(1).is_some_and(|c| *c != b' ') || self.is_untracked()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CommitEntry {
    pub(crate) hash: String,
    pub(crate) summary: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BranchSync {
    pub(crate) has_upstream: bool,
    pub(crate) ahead: usize,
    pub(crate) behind: usize,
}

impl BranchSync {
    pub(crate) fn needs_align(self) -> bool {
        self.has_upstream && self.behind > 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FocusPane {
    Files,
    Commits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PromptMode {
    Commit,
    GithubRepo,
    NewLocalRepo,
    SwitchBranch,
    CreateBranch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DeleteTarget {
    Local,
    Github,
    Both,
}

impl DeleteTarget {
    pub(crate) fn includes_local(self) -> bool {
        matches!(self, Self::Local | Self::Both)
    }

    pub(crate) fn includes_github(self) -> bool {
        matches!(self, Self::Github | Self::Both)
    }

    pub(crate) fn previous(self) -> Self {
        match self {
            Self::Local => Self::Both,
            Self::Github => Self::Local,
            Self::Both => Self::Github,
        }
    }

    pub(crate) fn next(self) -> Self {
        match self {
            Self::Local => Self::Github,
            Self::Github => Self::Both,
            Self::Both => Self::Local,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Prompt {
    pub(crate) mode: PromptMode,
    pub(crate) title: String,
    pub(crate) value: String,
    pub(crate) description: String,
    pub(crate) value_cursor: usize,
    pub(crate) description_cursor: usize,
    pub(crate) editing_description: bool,
}

#[derive(Clone, Debug)]
pub(crate) enum PendingAction {
    StageAll,
    Push,
    Pull,
    AlignBranch,
    InitRepo,
    GithubRepo {
        name: String,
        public: bool,
    },
    NewLocalRepo {
        path: String,
    },
    DeleteRepo {
        target: DeleteTarget,
        repo: Option<String>,
        root: String,
    },
    SwitchBranch {
        name: String,
    },
    CreateBranch {
        name: String,
    },
    DeleteCommit {
        hash: String,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct Confirm {
    pub(crate) title: String,
    pub(crate) body: String,
    pub(crate) action: PendingAction,
}

#[derive(Clone, Debug)]
pub(crate) struct GithubVisibility {
    pub(crate) name: String,
    pub(crate) public_selected: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct DeleteRepoChoice {
    pub(crate) target: DeleteTarget,
}

#[derive(Clone, Debug)]
pub(crate) struct Theme {
    pub(crate) border: Color,
    pub(crate) title: Color,
    pub(crate) text: Color,
    pub(crate) command_key: Color,
    pub(crate) muted: Color,
    pub(crate) selected: Color,
    pub(crate) staged: Color,
    pub(crate) unstaged: Color,
    pub(crate) untracked: Color,
    pub(crate) error: Color,
    pub(crate) success: Color,
    pub(crate) diff_add: Color,
    pub(crate) diff_remove: Color,
    pub(crate) diff_meta: Color,
}

#[derive(Clone, Debug)]
pub(crate) struct Shortcuts {
    pub(crate) quit: char,
    pub(crate) refresh: char,
    pub(crate) stage_all: char,
    pub(crate) commit: char,
    pub(crate) push: char,
    pub(crate) pull: char,
    pub(crate) init: char,
    pub(crate) github: char,
    pub(crate) new_repo: char,
    pub(crate) delete_repo: char,
    pub(crate) switch_branch: char,
    pub(crate) create_branch: char,
}

#[derive(Clone, Debug)]
pub(crate) struct Config {
    pub(crate) theme: Theme,
    pub(crate) keys: Shortcuts,
}

#[derive(Deserialize, Default)]
pub(crate) struct RawConfig {
    pub(crate) theme: Option<HashMap<String, String>>,
    pub(crate) keys: Option<HashMap<String, String>>,
}

#[derive(Debug)]
pub(crate) struct App {
    pub(crate) config: Config,
    pub(crate) files: Vec<FileStatus>,
    pub(crate) selected: usize,
    pub(crate) file_scroll: usize,
    pub(crate) diff_scroll: usize,
    pub(crate) root: String,
    pub(crate) branch: String,
    pub(crate) remote: String,
    pub(crate) branch_sync: BranchSync,
    pub(crate) align_hint: bool,
    pub(crate) message: String,
    pub(crate) diff_text: String,
    pub(crate) log_text: String,
    pub(crate) commits: Vec<CommitEntry>,
    pub(crate) selected_commit: usize,
    pub(crate) commit_scroll: usize,
    pub(crate) inside_repo: bool,
    pub(crate) running: bool,
    pub(crate) browsing_diff: bool,
    pub(crate) viewing_commit: bool,
    pub(crate) focus: FocusPane,
    pub(crate) help_open: bool,
    pub(crate) prompt: Option<Prompt>,
    pub(crate) prompt_paste_deadline: Option<Instant>,
    pub(crate) confirm: Option<Confirm>,
    pub(crate) github_visibility: Option<GithubVisibility>,
    pub(crate) delete_repo_choice: Option<DeleteRepoChoice>,
    pub(crate) files_rect: Rect,
    pub(crate) diff_rect: Rect,
}

#[derive(Debug)]
pub(crate) struct CmdResult {
    pub(crate) ok: bool,
    pub(crate) text: String,
}
