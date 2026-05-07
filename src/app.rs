use std::{
    env, fs,
    path::PathBuf,
    time::{Duration, Instant},
};

use ratatui::layout::Rect;

use crate::{config::*, git::*, model::*};

impl App {
    pub(crate) fn new() -> Self {
        let config = load_config();
        let mut app = Self {
            config,
            files: Vec::new(),
            selected: 0,
            file_scroll: 0,
            diff_scroll: 0,
            root: String::new(),
            branch: String::new(),
            remote: String::new(),
            branch_sync: BranchSync::default(),
            align_hint: false,
            message: "Listo. Usa flechas para seleccionar archivos.".to_string(),
            diff_text: String::new(),
            log_text: String::new(),
            commits: Vec::new(),
            selected_commit: 0,
            commit_scroll: 0,
            inside_repo: false,
            running: true,
            browsing_diff: false,
            viewing_commit: false,
            focus: FocusPane::Files,
            help_open: false,
            prompt: None,
            prompt_paste_deadline: None,
            confirm: None,
            github_visibility: None,
            delete_repo_choice: None,
            files_rect: Rect::default(),
            diff_rect: Rect::default(),
        };
        let msg = app.message.clone();
        app.refresh(Some(msg));
        app
    }

    pub(crate) fn refresh(&mut self, message: Option<String>) {
        if let Some(message) = message {
            self.message = message;
        }

        self.inside_repo = git_ok(["rev-parse", "--is-inside-work-tree"]).text.trim() == "true";

        if self.inside_repo {
            self.root = git_text(["rev-parse", "--show-toplevel"], ".");
            self.branch = git_text(["rev-parse", "--abbrev-ref", "HEAD"], "sin rama");
            self.remote = git_text(["remote", "get-url", "origin"], "sin origin");
            self.branch_sync = branch_sync_state();
            if self.branch_sync.needs_align() {
                self.align_hint = true;
            }
            self.files = status_files();
            self.commits = recent_commits();
        } else {
            self.root = env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".".to_string());
            self.branch = "sin repo".to_string();
            self.remote = "sin origin".to_string();
            self.branch_sync = BranchSync::default();
            self.align_hint = false;
            self.files.clear();
            self.commits.clear();
        }

        if self.files.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.files.len() {
            self.selected = self.files.len() - 1;
        }
        if self.commits.is_empty() {
            self.selected_commit = 0;
        } else if self.selected_commit >= self.commits.len() {
            self.selected_commit = self.commits.len() - 1;
        }
        self.log_text = git_ok([
            "--no-pager",
            "log",
            "--oneline",
            "--decorate",
            "--graph",
            "-12",
            "--color=never",
        ])
        .text;
        if self.log_text.trim().is_empty() {
            self.log_text = "Sin commits todavia.".to_string();
        }
        self.preview_for_focus();
    }

    pub(crate) fn next_focus(&mut self) {
        self.focus = match self.focus {
            FocusPane::Files => FocusPane::Commits,
            FocusPane::Commits => FocusPane::Files,
        };
        self.preview_for_focus();
        self.message = match self.focus {
            FocusPane::Files => "Archivos".to_string(),
            FocusPane::Commits => "Commits".to_string(),
        };
    }

    pub(crate) fn preview_for_focus(&mut self) {
        match self.focus {
            FocusPane::Files => self.preview_selected_file(),
            FocusPane::Commits => self.preview_selected_commit(),
        }
    }

    pub(crate) fn preview_selected_file(&mut self) {
        self.diff_scroll = 0;
        self.viewing_commit = false;
        self.diff_text = self
            .files
            .get(self.selected)
            .map(file_diff)
            .unwrap_or_else(|| "No hay archivo seleccionado.".to_string());
    }

    pub(crate) fn preview_selected_commit(&mut self) {
        self.diff_scroll = 0;
        let Some(commit) = self.commits.get(self.selected_commit) else {
            self.viewing_commit = true;
            self.diff_text = "Sin commits todavia.".to_string();
            return;
        };
        self.viewing_commit = true;
        self.diff_text = commit_detail(&commit.hash);
    }

    pub(crate) fn move_selection(&mut self, delta: isize) {
        if self.files.is_empty() {
            return;
        }
        let last = self.files.len() - 1;
        let next = self.selected.saturating_add_signed(delta).min(last);
        if next != self.selected {
            self.selected = next;
            self.preview_selected_file();
        }
    }

    pub(crate) fn move_commit_selection(&mut self, delta: isize) {
        if self.commits.is_empty() {
            return;
        }
        let last = self.commits.len() - 1;
        let next = self.selected_commit.saturating_add_signed(delta).min(last);
        if next != self.selected_commit {
            self.selected_commit = next;
            self.preview_selected_commit();
        }
    }

    pub(crate) fn scroll_diff(&mut self, delta: isize) {
        let max = self.diff_text.chars().count().saturating_sub(1);
        self.diff_scroll = self.diff_scroll.saturating_add_signed(delta).min(max);
    }

    pub(crate) fn open_preview_view(&mut self) {
        if self.viewing_commit {
            if self.commits.is_empty() {
                self.message = "No hay commits para navegar.".to_string();
                return;
            }
            self.browsing_diff = true;
            self.message = "Vista de commit: flechas navegan, Esc vuelve.".to_string();
            return;
        }

        if self.files.is_empty() {
            return;
        }
        self.browsing_diff = true;
        self.message = "Vista de archivo: flechas navegan, Esc vuelve.".to_string();
    }

    pub(crate) fn delete_selected_commit(&mut self) {
        let Some(commit) = self.commits.get(self.selected_commit).cloned() else {
            self.message = "No hay commits para borrar".to_string();
            return;
        };
        let head = git_text(["rev-parse", "--short", "HEAD"], "");
        if !is_selected_head_commit(&commit.hash, &head) {
            self.message = "Solo puedes borrar el commit mas reciente".to_string();
            return;
        }
        self.confirm(
            "Borrar commit",
            &format!(
                "Commit: {} {}\nEsto quitara el commit mas reciente y conservara sus cambios en stage.\nSi ya hiciste push, revisa antes de continuar.",
                commit.hash, commit.summary
            ),
            PendingAction::DeleteCommit { hash: commit.hash },
        );
    }

    pub(crate) fn delete_commit_now(&mut self, hash: String) {
        let head = git_text(["rev-parse", "--short", "HEAD"], "");
        if !is_selected_head_commit(&hash, &head) {
            self.refresh(Some(
                "Borrar commit: ERROR. El commit seleccionado ya no es HEAD".to_string(),
            ));
            return;
        }

        let parent = git_ok(["rev-parse", "--verify", "HEAD~1"]);
        let result = if parent.ok {
            git_ok(["reset", "--soft", "HEAD~1"])
        } else {
            git_ok(["update-ref", "-d", "HEAD"])
        };
        self.refresh(Some(format_action(
            "Borrar commit",
            if result.ok {
                CmdResult {
                    ok: true,
                    text: "commit eliminado; cambios conservados en stage".to_string(),
                }
            } else {
                result
            },
        )));
    }

    pub(crate) fn toggle_stage(&mut self) {
        let Some(file) = self.files.get(self.selected).cloned() else {
            self.refresh(Some("No hay archivos para stage/unstage.".to_string()));
            return;
        };
        let result = if file.is_staged() {
            unstage_file(&file.path)
        } else {
            git_ok(["add", "--", &file.path])
        };
        self.refresh(Some(format_action(
            if file.is_staged() { "Unstage" } else { "Stage" },
            result,
        )));
    }

    pub(crate) fn stage_all(&mut self) {
        self.confirm(
            "Stage de todos los cambios",
            "Esto agregara todos los archivos modificados y nuevos al proximo commit.",
            PendingAction::StageAll,
        );
    }

    pub(crate) fn stage_all_now(&mut self) {
        let result = git_ok(["add", "-A"]);
        self.refresh(Some(format_action("Stage de todos los cambios", result)));
    }

    pub(crate) fn commit(&mut self, message: String) {
        let result = git_ok(["commit", "-m", &message]);
        self.refresh(Some(format_action("Commit", result)));
    }

    pub(crate) fn push(&mut self) {
        let pending = pending_push_count();
        self.confirm(
            "Push",
            &format!(
                "Rama: {}\nRemote: {}\nCommits pendientes: {pending}\nGitSimple-Terminal configurara upstream si hace falta.",
                self.branch,
                remote_label(&self.remote)
            ),
            PendingAction::Push,
        );
    }

    pub(crate) fn push_now(&mut self) {
        let upstream = git_ok(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"]);
        let result = if upstream.ok {
            git_ok(["push"])
        } else if self.remote != "sin origin" && !self.branch.is_empty() && self.branch != "HEAD" {
            git_ok(["push", "-u", "origin", &self.branch])
        } else {
            CmdResult {
                ok: false,
                text: "No hay remote origin. Usa H para crear un repo privado con GitHub CLI."
                    .to_string(),
            }
        };
        let needs_align = is_push_alignment_error(&result.text);
        let ok = result.ok;
        let message = if needs_align {
            "Repositorio desalineado con GitHub".to_string()
        } else {
            format_action("Push", result)
        };
        self.refresh(Some(message));
        if needs_align {
            self.align_hint = true;
        } else if ok {
            self.align_hint = false;
        }
    }

    pub(crate) fn pull(&mut self) {
        self.confirm(
            "Pull",
            &format!(
                "Esto traera cambios desde {}.\nRama actual: {}\nPuede mezclar archivos locales si hay diferencias.",
                remote_label(&self.remote),
                self.branch
            ),
            PendingAction::Pull,
        );
    }

    pub(crate) fn pull_now(&mut self) {
        let result = git_ok(["pull"]);
        let ok = result.ok;
        self.refresh(Some(format_action("Pull", result)));
        if ok && !self.branch_sync.needs_align() {
            self.align_hint = false;
        }
    }

    pub(crate) fn align_branch(&mut self) {
        if !self.inside_repo {
            self.message = "Alinear: no estas dentro de un repo".to_string();
            return;
        }
        if self.remote == "sin origin" {
            self.message = "Alinear: no hay remote origin".to_string();
            return;
        }
        self.confirm(
            "Alinear con GitHub",
            &format!(
                "Esto conectara la rama con GitHub si hace falta, traera cambios con rebase y autostash, luego hara push.\nRama: {}\nRemote: {}\nNo usa force push.",
                self.branch,
                remote_label(&self.remote)
            ),
            PendingAction::AlignBranch,
        );
    }

    pub(crate) fn align_branch_now(&mut self) {
        if rebase_in_progress() {
            let continued = continue_rebase();
            if !continued.ok {
                self.refresh(Some(format_action("Alinear rebase", continued)));
                self.align_hint = true;
                return;
            }
        }

        let Some(branch) = current_local_branch() else {
            self.refresh(Some(
                "Alinear upstream: ERROR. no se pudo detectar una rama local para conectar con GitHub".to_string(),
            ));
            self.align_hint = true;
            return;
        };

        if self.branch == "HEAD" {
            let switched = git_ok(["switch", &branch]);
            if !switched.ok {
                self.refresh(Some(format_action("Alinear cambio de rama", switched)));
                self.align_hint = true;
                return;
            }
        }

        let upstream = git_ok(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"]);
        if !upstream.ok {
            let setup = setup_upstream_for_branch(&branch);
            if !setup.ok {
                self.refresh(Some(format_action("Alinear upstream", setup)));
                self.align_hint = true;
                return;
            }
        }

        let pull = git_ok(["pull", "--rebase", "--autostash"]);
        if !pull.ok {
            self.refresh(Some(format_action("Alinear pull --rebase", pull)));
            self.align_hint = true;
            return;
        }

        let push = git_ok(["push"]);
        let push_ok = push.ok;
        let message = if push.ok {
            if pull.text.is_empty() && push.text.is_empty() {
                "Alinear: OK. Rama local y GitHub alineados".to_string()
            } else {
                format!("Alinear: OK. {} {}", pull.text.trim(), push.text.trim())
                    .trim()
                    .to_string()
            }
        } else {
            format_action("Alinear push", push)
        };
        self.refresh(Some(message));
        if push_ok && !self.branch_sync.needs_align() {
            self.align_hint = false;
        }
    }

    pub(crate) fn init_repo(&mut self) {
        self.confirm(
            "Git init",
            "Esto inicializara un repositorio Git en la carpeta actual.",
            PendingAction::InitRepo,
        );
    }

    pub(crate) fn init_repo_now(&mut self) {
        let result = git_ok(["init"]);
        self.refresh(Some(format_action("Git init", result)));
    }

    pub(crate) fn github_repo(&mut self, raw: String) {
        let (public, name) = parse_github_repo_prompt(&raw);
        self.github_visibility = Some(GithubVisibility {
            name,
            public_selected: public,
        });
    }

    pub(crate) fn confirm_github_visibility(&mut self) {
        let Some(choice) = self.github_visibility.take() else {
            return;
        };
        let public = choice.public_selected;
        self.confirm(
            "Crear repo en GitHub",
            if public {
                "Esto creara un repositorio publico en GitHub, conectara origin y hara push."
            } else {
                "Esto creara un repositorio privado en GitHub, conectara origin y hara push."
            },
            PendingAction::GithubRepo {
                name: choice.name,
                public,
            },
        );
    }

    pub(crate) fn cancel_github_visibility(&mut self) {
        self.github_visibility = None;
        self.message = "GitHub cancelado.".to_string();
    }

    pub(crate) fn github_repo_now(&mut self, name: String, public: bool) {
        let visibility = if public { "--public" } else { "--private" };
        let result = run_cmd(
            "gh",
            [
                "repo",
                "create",
                &name,
                visibility,
                "--source=.",
                "--remote=origin",
                "--push",
            ],
        );
        self.refresh(Some(format_action("GitHub", result)));
    }

    pub(crate) fn delete_repo(&mut self) {
        if !self.inside_repo {
            self.message = "Borrar repo: no estas dentro de un repositorio".to_string();
            return;
        }
        self.delete_repo_choice = Some(DeleteRepoChoice {
            target: DeleteTarget::Local,
        });
    }

    pub(crate) fn confirm_delete_repo_choice(&mut self) {
        let Some(choice) = self.delete_repo_choice.take() else {
            return;
        };
        let target = choice.target;
        let repo = github_repo_slug(&self.remote);
        if target.includes_github() && repo.is_none() {
            self.message = "Borrar GitHub: no se detecto un repo de GitHub en origin".to_string();
            return;
        }

        let body = delete_repo_confirmation_body(target, repo.as_deref(), &self.root);
        self.confirm(
            "Eliminar repositorio",
            &body,
            PendingAction::DeleteRepo {
                target,
                repo,
                root: self.root.clone(),
            },
        );
    }

    pub(crate) fn cancel_delete_repo_choice(&mut self) {
        self.delete_repo_choice = None;
        self.message = "Borrar repo cancelado".to_string();
    }

    pub(crate) fn delete_repo_now(
        &mut self,
        target: DeleteTarget,
        repo: Option<String>,
        root: String,
    ) {
        let mut done = Vec::new();

        if target.includes_github() {
            let Some(repo) = repo else {
                self.refresh(Some(
                    "Borrar GitHub: ERROR. no se detecto repo remoto".to_string(),
                ));
                return;
            };
            let result = delete_github_repo(&repo);
            if !result.ok {
                self.refresh(Some(format_action("Borrar GitHub", result)));
                return;
            }
            done.push(format!("GitHub {repo} eliminado"));
        }

        if target.includes_local() {
            let result = delete_local_repo(&root);
            if !result.ok {
                self.refresh(Some(format_action("Borrar local", result)));
                return;
            }
            done.push("repo local eliminado; archivos conservados".to_string());
        }

        self.refresh(Some(format!("Borrar repo: OK. {}", done.join(" | "))));
    }

    pub(crate) fn new_local_repo(&mut self, path: String) {
        self.confirm(
            "Crear repositorio nuevo",
            "Esto creara una carpeta, ejecutara git init y cambiara GitSimple-Terminal a ese repo.",
            PendingAction::NewLocalRepo { path },
        );
    }

    pub(crate) fn new_local_repo_now(&mut self, path: String) {
        let target = PathBuf::from(path.trim());
        if target.as_os_str().is_empty() {
            self.message = "Repo nuevo cancelado: ruta vacia.".to_string();
            return;
        }
        if let Err(error) = fs::create_dir_all(&target) {
            self.message = format!("Repo nuevo: ERROR. {error}");
            return;
        }
        if let Err(error) = env::set_current_dir(&target) {
            self.message = format!("Repo nuevo: ERROR. {error}");
            return;
        }
        let result = git_ok(["init"]);
        self.refresh(Some(format_action("Repo nuevo", result)));
    }

    pub(crate) fn switch_branch(&mut self, name: String) {
        if self.has_changes() {
            self.confirm(
                "Cambiar rama",
                "Hay cambios locales. Cambiar de rama puede fallar o mover el contexto de trabajo.",
                PendingAction::SwitchBranch { name },
            );
        } else {
            self.switch_branch_now(name);
        }
    }

    pub(crate) fn switch_branch_now(&mut self, name: String) {
        let result = git_ok(["switch", &name]);
        self.refresh(Some(format_action("Cambiar rama", result)));
    }

    pub(crate) fn create_branch(&mut self, name: String) {
        self.confirm(
            "Crear rama",
            "Esto creara una rama nueva y cambiara tu working tree a ella.",
            PendingAction::CreateBranch { name },
        );
    }

    pub(crate) fn create_branch_now(&mut self, name: String) {
        let result = git_ok(["switch", "-c", &name]);
        self.refresh(Some(format_action("Crear rama", result)));
    }

    pub(crate) fn has_changes(&self) -> bool {
        !self.files.is_empty()
    }

    pub(crate) fn confirm(&mut self, title: &str, body: &str, action: PendingAction) {
        self.confirm = Some(Confirm {
            title: title.to_string(),
            body: body.to_string(),
            action,
        });
    }

    pub(crate) fn arm_prompt_paste(&mut self) {
        self.prompt_paste_deadline = Some(Instant::now() + Duration::from_millis(350));
    }

    pub(crate) fn prompt_paste_active(&self) -> bool {
        self.prompt_paste_deadline
            .is_some_and(|deadline| Instant::now() <= deadline)
    }

    pub(crate) fn clear_prompt_paste(&mut self) {
        self.prompt_paste_deadline = None;
    }

    pub(crate) fn cancel_confirm(&mut self) {
        self.confirm = None;
        self.message = "Accion cancelada.".to_string();
    }

    pub(crate) fn run_confirmed(&mut self) {
        let Some(confirm) = self.confirm.take() else {
            return;
        };
        match confirm.action {
            PendingAction::StageAll => self.stage_all_now(),
            PendingAction::Push => self.push_now(),
            PendingAction::Pull => self.pull_now(),
            PendingAction::AlignBranch => self.align_branch_now(),
            PendingAction::InitRepo => self.init_repo_now(),
            PendingAction::GithubRepo { name, public } => self.github_repo_now(name, public),
            PendingAction::NewLocalRepo { path } => self.new_local_repo_now(path),
            PendingAction::DeleteRepo { target, repo, root } => {
                self.delete_repo_now(target, repo, root)
            }
            PendingAction::SwitchBranch { name } => self.switch_branch_now(name),
            PendingAction::CreateBranch { name } => self.create_branch_now(name),
            PendingAction::DeleteCommit { hash } => self.delete_commit_now(hash),
        }
    }

    pub(crate) fn open_prompt(&mut self, mode: PromptMode) {
        let title = match mode {
            PromptMode::Commit => "Nuevo commit",
            PromptMode::GithubRepo => "Nombre del repo en GitHub",
            PromptMode::NewLocalRepo => "Ruta/carpeta del repo nuevo",
            PromptMode::SwitchBranch => "Nombre de rama existente",
            PromptMode::CreateBranch => "Nombre de rama nueva",
        };
        self.clear_prompt_paste();
        self.prompt = Some(Prompt {
            mode,
            title: title.to_string(),
            value: String::new(),
            description: String::new(),
            value_cursor: 0,
            description_cursor: 0,
            editing_description: false,
        });
    }

    pub(crate) fn submit_prompt(&mut self) {
        self.clear_prompt_paste();
        let Some(prompt) = self.prompt.take() else {
            return;
        };
        let value = prompt.value.trim().to_string();
        if value.is_empty() {
            self.message = match prompt.mode {
                PromptMode::Commit => "Commit cancelado: mensaje vacio.".to_string(),
                PromptMode::GithubRepo => "GitHub cancelado: nombre vacio.".to_string(),
                PromptMode::NewLocalRepo => "Repo nuevo cancelado: ruta vacia.".to_string(),
                PromptMode::SwitchBranch => "Cambio de rama cancelado: nombre vacio.".to_string(),
                PromptMode::CreateBranch => "Crear rama cancelado: nombre vacio.".to_string(),
            };
            return;
        }
        match prompt.mode {
            PromptMode::Commit => {
                let description = prompt.description.trim();
                let message = if description.is_empty() {
                    value
                } else {
                    format!("{value}\n\n{description}")
                };
                self.commit(message);
            }
            PromptMode::GithubRepo => self.github_repo(value),
            PromptMode::NewLocalRepo => self.new_local_repo(value),
            PromptMode::SwitchBranch => self.switch_branch(value),
            PromptMode::CreateBranch => self.create_branch(value),
        }
    }
}
