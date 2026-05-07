use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use crate::{git::*, model::*};

pub(crate) fn draw(frame: &mut Frame<'_>, app: &mut App) {
    let area = frame.area();
    if area.width < 36 || area.height < 10 {
        frame.render_widget(
            Paragraph::new(
                "GitSimple-Terminal\nAumenta un poco la ventana para ver archivos y diff.",
            )
            .style(Style::default().fg(app.config.theme.text))
            .block(panel(&app.config.theme)),
            area,
        );
        return;
    }

    let narrow = area.width < 92;
    let very_short = area.height < 22;
    let status_h = if narrow { 5 } else { 6 };
    let command_h = if narrow { 3 } else { 4 };
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(status_h),
            Constraint::Min(3),
            Constraint::Length(command_h),
        ])
        .split(area);

    draw_status(frame, app, root[0], narrow);

    if narrow {
        let body = if very_short {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
                .split(root[1])
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(34),
                    Constraint::Percentage(46),
                    Constraint::Percentage(20),
                ])
                .split(root[1])
        };
        app.files_rect = body[0];
        app.diff_rect = body[1];
        draw_files(frame, app, body[0]);
        draw_diff(frame, app, body[1]);
        if !very_short {
            draw_commits(frame, app, body[2]);
        }
    } else {
        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
            .split(root[1]);
        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(67), Constraint::Percentage(33)])
            .split(body[0]);
        app.files_rect = left[0];
        app.diff_rect = body[1];
        draw_files(frame, app, left[0]);
        draw_commits(frame, app, left[1]);
        draw_diff(frame, app, body[1]);
    }

    draw_commands(frame, app, root[2], narrow);
    if app.prompt.is_some() {
        draw_prompt(frame, app, area);
    }
    if app.github_visibility.is_some() {
        draw_github_visibility(frame, app, area);
    }
    if app.delete_repo_choice.is_some() {
        draw_delete_repo_choice(frame, app, area);
    }
    if app.confirm.is_some() {
        draw_confirm(frame, app, area);
    }
    if app.help_open {
        draw_help(frame, app, area);
    }
}

pub(crate) fn panel(theme: &Theme) -> Block<'static> {
    panel_with_border(theme.border)
}

pub(crate) fn active_panel(theme: &Theme, active: bool) -> Block<'static> {
    panel_with_border(if active {
        theme.command_key
    } else {
        theme.border
    })
}

pub(crate) fn panel_with_border(color: Color) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color))
        .style(Style::default())
}

pub(crate) fn draw_status(frame: &mut Frame<'_>, app: &App, area: Rect, narrow: bool) {
    let theme = &app.config.theme;
    clear_area(frame, area);
    let value_width = usize::from(area.width.saturating_sub(15));
    let text = if !app.inside_repo {
        vec![
            Line::from(Span::styled(
                "No estas dentro de un repositorio Git.",
                Style::default().fg(theme.error),
            )),
            status_line("Carpeta", short_path(&app.root, value_width), theme),
            status_line(
                "Accion",
                "Presiona I para ejecutar git init.".to_string(),
                theme,
            ),
        ]
    } else {
        let staged = app.files.iter().filter(|file| file.is_staged()).count();
        let unstaged = app.files.iter().filter(|file| file.is_unstaged()).count();
        let untracked = app.files.iter().filter(|file| file.is_untracked()).count();
        if narrow {
            vec![
                status_line("Repo", short_path(&app.root, value_width), theme),
                status_line("Rama", app.branch.clone(), theme),
                status_line(
                    "Remote",
                    short_path(&remote_label(&app.remote), value_width),
                    theme,
                ),
                status_line(
                    "Cambios",
                    format!(
                        "staged {staged}  unstaged {unstaged}  nuevos {untracked}  total {}",
                        app.files.len()
                    ),
                    theme,
                ),
            ]
        } else {
            vec![
                status_line("Repositorio", short_path(&app.root, value_width), theme),
                status_line("Rama", app.branch.clone(), theme),
                status_line("Remote", short_path(&app.remote, value_width), theme),
                status_line(
                    "Cambios",
                    format!(
                        "staged {staged}  unstaged {unstaged}  nuevos {untracked}  total {}",
                        app.files.len()
                    ),
                    theme,
                ),
            ]
        }
    };
    frame.render_widget(
        Paragraph::new(text)
            .block(panel(theme))
            .wrap(Wrap { trim: true }),
        area,
    );
}

pub(crate) fn status_line(label: &'static str, value: String, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label:<11}: "),
            Style::default()
                .fg(theme.muted)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(value, Style::default().fg(theme.text)),
    ])
}

pub(crate) fn clear_area(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(Clear, area);
}

pub(crate) fn panel_inner_width(area: Rect) -> usize {
    usize::from(area.width.saturating_sub(2))
}

pub(crate) fn truncate_text(value: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let count = value.chars().count();
    if count <= max {
        return value.to_string();
    }
    if max <= 3 {
        return value.chars().take(max).collect();
    }
    let mut text: String = value.chars().take(max - 3).collect();
    text.push_str("...");
    text
}

pub(crate) fn draw_files(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let theme = &app.config.theme;
    let active = app.focus == FocusPane::Files && !app.browsing_diff;
    clear_area(frame, area);
    let visible_files = usize::from(area.height.saturating_sub(3));
    let path_width = panel_inner_width(area).saturating_sub(5);
    if app.selected < app.file_scroll {
        app.file_scroll = app.selected;
    }
    if visible_files > 0 && app.selected >= app.file_scroll + visible_files {
        app.file_scroll = app.selected - visible_files + 1;
    }

    let mut items = Vec::new();
    items.push(ListItem::new(Line::from(Span::styled(
        if active { "Archivos *" } else { "Archivos" },
        Style::default()
            .fg(theme.title)
            .add_modifier(Modifier::BOLD),
    ))));

    if app.files.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "Sin cambios en el working tree.",
            Style::default().fg(theme.muted),
        ))));
    } else {
        for (index, file) in app
            .files
            .iter()
            .enumerate()
            .skip(app.file_scroll)
            .take(visible_files)
        {
            let marker = if index == app.selected { ">" } else { " " };
            let color = if index == app.selected {
                theme.selected
            } else if file.is_untracked() {
                theme.untracked
            } else if file.is_staged() {
                theme.staged
            } else {
                theme.unstaged
            };
            let style = if index == app.selected {
                Style::default().fg(color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            };
            let display = truncate_text(&file.display, path_width);
            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!("{marker} {} ", file.xy), style),
                Span::styled(display, style),
            ])));
        }
    }

    frame.render_widget(List::new(items).block(active_panel(theme, active)), area);
}

pub(crate) fn draw_commits(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let visible_commits = usize::from(area.height.saturating_sub(3));
    keep_commit_selection_visible(app, visible_commits);
    let theme = &app.config.theme;
    let active = app.focus == FocusPane::Commits && !app.browsing_diff;
    clear_area(frame, area);
    let line_width = panel_inner_width(area);
    let mut lines = vec![Line::from(Span::styled(
        if active {
            "Commits recientes *"
        } else {
            "Commits recientes"
        },
        Style::default()
            .fg(theme.title)
            .add_modifier(Modifier::BOLD),
    ))];
    if app.commits.is_empty() {
        lines.push(Line::from(Span::styled(
            "Sin commits todavia.",
            Style::default().fg(theme.muted),
        )));
    } else {
        for (index, commit) in app
            .commits
            .iter()
            .enumerate()
            .skip(app.commit_scroll)
            .take(visible_commits)
        {
            let selected = index == app.selected_commit;
            let style = if selected {
                Style::default()
                    .fg(theme.selected)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };
            let marker = if selected { ">" } else { " " };
            lines.push(Line::from(Span::styled(
                truncate_text(
                    &format!("{marker} {} {}", commit.hash, commit.summary),
                    line_width,
                ),
                style,
            )));
        }
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(active_panel(theme, active))
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub(crate) fn keep_commit_selection_visible(app: &mut App, visible_commits: usize) {
    if app.commits.is_empty() || visible_commits == 0 {
        app.commit_scroll = 0;
        return;
    }

    let last = app.commits.len() - 1;
    app.selected_commit = app.selected_commit.min(last);
    if app.selected_commit < app.commit_scroll {
        app.commit_scroll = app.selected_commit;
    } else if app.selected_commit >= app.commit_scroll.saturating_add(visible_commits) {
        app.commit_scroll = app
            .selected_commit
            .saturating_sub(visible_commits.saturating_sub(1));
    }

    let max_scroll = app.commits.len().saturating_sub(visible_commits);
    app.commit_scroll = app.commit_scroll.min(max_scroll);
}

pub(crate) fn draw_diff(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let theme = &app.config.theme;
    clear_area(frame, area);
    let line_width = panel_inner_width(area).saturating_sub(1).max(1);
    let visible_rows = usize::from(area.height.saturating_sub(3));
    let title = if app.browsing_diff {
        if app.viewing_commit {
            "Navegando commit"
        } else {
            "Navegando archivo"
        }
    } else if app.viewing_commit {
        "Commit / vista previa"
    } else {
        "Diff / vista previa"
    };
    let active = app.browsing_diff;
    let panel = active_panel(theme, active);
    let mut lines = vec![Line::from(Span::styled(
        title,
        Style::default()
            .fg(if app.browsing_diff {
                theme.command_key
            } else {
                theme.title
            })
            .add_modifier(Modifier::BOLD),
    ))];
    let visual_lines = diff_visual_lines(&app.diff_text, theme, line_width);
    if app.diff_scroll >= visual_lines.len() {
        app.diff_scroll = visual_lines.len().saturating_sub(1);
    }
    lines.extend(
        visual_lines
            .into_iter()
            .skip(app.diff_scroll)
            .take(visible_rows),
    );
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel)
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub(crate) fn draw_commands(frame: &mut Frame<'_>, app: &App, area: Rect, narrow: bool) {
    let theme = &app.config.theme;
    let keys = &app.config.keys;
    clear_area(frame, area);
    let message_width = panel_inner_width(area).saturating_sub(28);
    let message = command_message(app);
    let lines = vec![
        command_shortcuts_line(keys, theme, narrow, app.browsing_diff),
        Line::from(vec![
            Span::styled(
                truncate_text(&message, message_width),
                Style::default().fg(theme.muted),
            ),
            Span::styled("  |  ", Style::default().fg(theme.muted)),
            Span::styled(
                truncate_text(&next_step(app), panel_inner_width(area).saturating_sub(5)),
                Style::default().fg(theme.command_key),
            ),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel(theme))
            .wrap(Wrap { trim: true }),
        area,
    );
}

pub(crate) fn command_shortcuts_line(
    keys: &Shortcuts,
    theme: &Theme,
    narrow: bool,
    browsing_diff: bool,
) -> Line<'static> {
    let mut spans = Vec::new();
    if browsing_diff {
        command_item(&mut spans, "↑↓", "navegar", theme);
        command_item(&mut spans, "PgUp/PgDn", "rapido", theme);
        command_item(&mut spans, "Home/End", "inicio/fin", theme);
        command_item(&mut spans, "Esc", "volver", theme);
        return Line::from(spans);
    }

    command_item(&mut spans, "Space", "stage", theme);
    command_item(
        &mut spans,
        keys.stage_all.to_ascii_uppercase().to_string(),
        "todo",
        theme,
    );
    command_item(
        &mut spans,
        keys.commit.to_ascii_uppercase().to_string(),
        "commit",
        theme,
    );
    command_item(
        &mut spans,
        keys.push.to_ascii_uppercase().to_string(),
        "push",
        theme,
    );
    if !narrow {
        command_item(
            &mut spans,
            keys.pull.to_ascii_uppercase().to_string(),
            "pull",
            theme,
        );
    }
    command_item(
        &mut spans,
        keys.github.to_ascii_uppercase().to_string(),
        "GitHub",
        theme,
    );
    command_item(
        &mut spans,
        keys.new_repo.to_ascii_uppercase().to_string(),
        "repo",
        theme,
    );
    if !narrow {
        command_item(
            &mut spans,
            keys.switch_branch.to_ascii_uppercase().to_string(),
            "rama",
            theme,
        );
        command_item(
            &mut spans,
            keys.create_branch.to_ascii_uppercase().to_string(),
            "nueva-rama",
            theme,
        );
        command_item(
            &mut spans,
            keys.refresh.to_ascii_uppercase().to_string(),
            "refresh",
            theme,
        );
        command_item(
            &mut spans,
            keys.init.to_ascii_uppercase().to_string(),
            "init",
            theme,
        );
        command_item(
            &mut spans,
            keys.delete_repo.to_ascii_uppercase().to_string(),
            "borrar-repo",
            theme,
        );
    }
    command_item(
        &mut spans,
        keys.quit.to_ascii_uppercase().to_string(),
        "salir",
        theme,
    );
    Line::from(spans)
}

pub(crate) fn command_item(
    spans: &mut Vec<Span<'static>>,
    key: impl Into<String>,
    label: &'static str,
    theme: &Theme,
) {
    if !spans.is_empty() {
        spans.push(Span::raw("  "));
    }
    spans.push(Span::styled(
        key.into(),
        Style::default()
            .fg(theme.command_key)
            .add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::styled(
        format!(" {label}"),
        Style::default().fg(theme.text),
    ));
}

pub(crate) fn command_message(app: &App) -> String {
    if repo_needs_align(app) && should_show_align_message(&app.message) {
        return "Repositorio desalineado con GitHub".to_string();
    }
    app.message.clone()
}

pub(crate) fn should_show_align_message(message: &str) -> bool {
    let message = message.trim();
    message.is_empty()
        || message == "Archivos"
        || message == "Commits"
        || message.starts_with("Listo")
        || message.starts_with("Informacion actualizada")
        || is_push_alignment_error(message)
}

pub(crate) fn next_step(app: &App) -> String {
    if !app.inside_repo {
        return "I para inicializar repo".to_string();
    }
    if repo_needs_align(app) {
        return "L alinear repositorio".to_string();
    }
    let staged = app.files.iter().filter(|file| file.is_staged()).count();
    let unstaged = app.files.iter().filter(|file| file.is_unstaged()).count();
    if staged > 0 {
        return format!("C para commit ({staged} staged)");
    }
    if unstaged > 0 {
        return "Space para elegir archivos o A para todo".to_string();
    }
    if pending_push_count() > 0 {
        return "P para subir commits".to_string();
    }
    "Working tree limpio".to_string()
}

pub(crate) fn repo_needs_align(app: &App) -> bool {
    app.align_hint || app.branch_sync.needs_align()
}

pub(crate) fn draw_prompt(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let Some(prompt) = &app.prompt else {
        return;
    };
    if prompt.mode == PromptMode::Commit {
        draw_commit_prompt(frame, prompt, &app.config.theme, area);
        return;
    }

    let width = area.width.saturating_sub(4).min(78).max(36);
    let height = 6;
    let rect = Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    };
    frame.render_widget(Clear, rect);
    let theme = &app.config.theme;
    let lines = vec![
        Line::from(Span::styled(
            prompt.title.clone(),
            Style::default()
                .fg(theme.title)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        prompt_input_line(
            &prompt.value,
            prompt.value_cursor,
            true,
            panel_inner_width(rect).saturating_sub(2),
            theme,
        ),
        dialog_keys_line(theme, &[("Enter", "aceptar"), ("Esc", "cancelar")]),
    ];
    frame.render_widget(Paragraph::new(lines).block(panel(theme)), rect);
}

pub(crate) fn draw_commit_prompt(
    frame: &mut Frame<'_>,
    prompt: &Prompt,
    theme: &Theme,
    area: Rect,
) {
    let width = area.width.saturating_sub(4).min(82).max(42);
    let height = area.height.saturating_sub(4).min(18).max(14);
    let rect = Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    };
    frame.render_widget(Clear, rect);

    let title_rect = Rect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: 5,
    };
    let description_rect = Rect {
        x: rect.x,
        y: rect.y + 6,
        width: rect.width,
        height: rect.height.saturating_sub(6),
    };

    let mut title_lines = field_lines(
        "Titulo del commit",
        &prompt.value,
        prompt.value_cursor,
        !prompt.editing_description,
        1,
        usize::from(title_rect.width.saturating_sub(4)),
        theme,
    );
    title_lines.push(Line::from(Span::styled(
        staged_preview(),
        Style::default().fg(theme.muted),
    )));

    frame.render_widget(Paragraph::new(title_lines).block(panel(theme)), title_rect);

    frame.render_widget(panel(theme), description_rect);
    let description_inner = Rect {
        x: description_rect.x.saturating_add(1),
        y: description_rect.y.saturating_add(1),
        width: description_rect.width.saturating_sub(2),
        height: description_rect.height.saturating_sub(2),
    };
    let commands_rect = Rect {
        x: description_inner.x,
        y: description_inner
            .y
            .saturating_add(description_inner.height.saturating_sub(1)),
        width: description_inner.width,
        height: 1,
    };
    let description_text_rect = Rect {
        x: description_inner.x,
        y: description_inner.y,
        width: description_inner.width,
        height: description_inner.height.saturating_sub(1),
    };
    let description_content_lines =
        usize::from(description_text_rect.height.saturating_sub(1)).max(1);
    let description_lines = field_lines(
        "Descripcion del cambio",
        &prompt.description,
        prompt.description_cursor,
        prompt.editing_description,
        description_content_lines,
        usize::from(description_text_rect.width.saturating_sub(2)),
        theme,
    );

    frame.render_widget(
        Paragraph::new(description_lines).wrap(Wrap { trim: true }),
        description_text_rect,
    );
    frame.render_widget(
        Paragraph::new(dialog_keys_line(
            theme,
            &[("Tab", "campo"), ("Enter", "commit"), ("Esc", "cancelar")],
        )),
        commands_rect,
    );
}

pub(crate) fn field_lines(
    label: &'static str,
    value: &str,
    cursor: usize,
    active: bool,
    max_content_lines: usize,
    max_width: usize,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(Span::styled(
        label,
        Style::default()
            .fg(if active { theme.selected } else { theme.title })
            .add_modifier(Modifier::BOLD),
    ))];
    lines.extend(prompt_content_lines(
        value,
        cursor,
        active,
        max_content_lines,
        max_width,
        theme,
    ));
    lines
}

pub(crate) fn prompt_input_line(
    value: &str,
    cursor: usize,
    active: bool,
    max_width: usize,
    theme: &Theme,
) -> Line<'static> {
    prompt_content_lines(value, cursor, active, 1, max_width, theme)
        .into_iter()
        .next()
        .unwrap_or_else(|| Line::from(""))
}

pub(crate) fn prompt_content_lines(
    value: &str,
    cursor: usize,
    active: bool,
    max_lines: usize,
    max_width: usize,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let max_lines = max_lines.max(1);
    let (all_lines, cursor_line) = prompt_visual_lines(value, cursor, active, max_width);
    let start = cursor_line.saturating_sub(max_lines.saturating_sub(1));
    let style = Style::default().fg(if active { theme.selected } else { theme.text });

    all_lines
        .into_iter()
        .skip(start)
        .take(max_lines)
        .enumerate()
        .map(|(index, line)| {
            let marker = if active && index == 0 { "> " } else { "  " };
            Line::from(vec![
                Span::styled(marker, Style::default().fg(theme.command_key)),
                Span::styled(line, style),
            ])
        })
        .collect()
}

pub(crate) fn prompt_visual_lines(
    value: &str,
    cursor: usize,
    active: bool,
    max_width: usize,
) -> (Vec<String>, usize) {
    let max_width = max_width.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut width = 0;
    let mut cursor_line = 0;

    for (index, ch) in value.chars().enumerate() {
        if active && index == cursor {
            cursor_line = lines.len();
            push_prompt_visual_char(&mut lines, &mut current, &mut width, '_', max_width);
        }
        push_prompt_visual_char(&mut lines, &mut current, &mut width, ch, max_width);
    }
    if active && cursor >= value.chars().count() {
        cursor_line = lines.len();
        push_prompt_visual_char(&mut lines, &mut current, &mut width, '_', max_width);
    }
    lines.push(current);
    (lines, cursor_line)
}

pub(crate) fn push_prompt_visual_char(
    lines: &mut Vec<String>,
    current: &mut String,
    width: &mut usize,
    ch: char,
    max_width: usize,
) {
    if ch == '\n' {
        lines.push(std::mem::take(current));
        *width = 0;
        return;
    }
    if *width >= max_width {
        lines.push(std::mem::take(current));
        *width = 0;
    }
    current.push(ch);
    *width += 1;
}

pub(crate) fn draw_confirm(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let Some(confirm) = &app.confirm else {
        return;
    };
    let width = area.width.saturating_sub(4).min(82).max(36);
    let height = 8;
    let rect = Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    };
    frame.render_widget(Clear, rect);
    let theme = &app.config.theme;
    let lines = vec![
        Line::from(Span::styled(
            confirm.title.clone(),
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(confirm.body.clone()),
        Line::from(""),
        dialog_keys_line(
            theme,
            &[("Y / Enter", "confirmar"), ("N / Esc", "cancelar")],
        ),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel(theme))
            .wrap(Wrap { trim: true }),
        rect,
    );
}

pub(crate) fn draw_github_visibility(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let Some(choice) = &app.github_visibility else {
        return;
    };
    let theme = &app.config.theme;
    let width = area.width.saturating_sub(4).min(72).max(38);
    let height = 7;
    let rect = Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    };
    frame.render_widget(Clear, rect);
    let lines = vec![
        Line::from(Span::styled(
            "Visibilidad del repo",
            Style::default()
                .fg(theme.title)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("Repo: {}", choice.name)),
        Line::from(""),
        github_visibility_choice_line(choice.public_selected, theme),
        dialog_keys_line(
            theme,
            &[
                ("←/→", "elegir"),
                ("Enter", "continuar"),
                ("Esc", "cancelar"),
            ],
        ),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel(theme))
            .wrap(Wrap { trim: true }),
        rect,
    );
}

pub(crate) fn github_visibility_choice_line(public_selected: bool, theme: &Theme) -> Line<'static> {
    let selected = Style::default()
        .fg(theme.selected)
        .add_modifier(Modifier::BOLD);
    let normal = Style::default().fg(theme.text);
    Line::from(vec![
        Span::styled(
            if public_selected {
                "  Privado"
            } else {
                "> Privado"
            },
            if public_selected { normal } else { selected },
        ),
        Span::raw("      "),
        Span::styled(
            if public_selected {
                "> Publico"
            } else {
                "  Publico"
            },
            if public_selected { selected } else { normal },
        ),
    ])
}

pub(crate) fn draw_delete_repo_choice(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let Some(choice) = &app.delete_repo_choice else {
        return;
    };
    let theme = &app.config.theme;
    let width = area.width.saturating_sub(4).min(88).max(46);
    let height = 9;
    let rect = Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    };
    frame.render_widget(Clear, rect);
    let repo = github_repo_slug(&app.remote).unwrap_or_else(|| "sin repo GitHub".to_string());
    let lines = vec![
        Line::from(Span::styled(
            "Eliminar repositorio",
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("Local: {}", short_path(&app.root, 54))),
        Line::from(format!("GitHub: {repo}")),
        Line::from(""),
        delete_repo_choice_line(choice.target, theme),
        Line::from(Span::styled(
            "Local conserva tus archivos y solo elimina .git.",
            Style::default().fg(theme.muted),
        )),
        dialog_keys_line(
            theme,
            &[
                ("←/→", "elegir"),
                ("Enter", "continuar"),
                ("Esc", "cancelar"),
            ],
        ),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel(theme))
            .wrap(Wrap { trim: true }),
        rect,
    );
}

pub(crate) fn delete_repo_choice_line(target: DeleteTarget, theme: &Theme) -> Line<'static> {
    let selected = Style::default()
        .fg(theme.selected)
        .add_modifier(Modifier::BOLD);
    let normal = Style::default().fg(theme.text);
    let item = |label: &'static str, active: bool| {
        Span::styled(
            if active {
                format!("> {label}")
            } else {
                format!("  {label}")
            },
            if active { selected } else { normal },
        )
    };
    Line::from(vec![
        item("Local", target == DeleteTarget::Local),
        Span::raw("      "),
        item("GitHub", target == DeleteTarget::Github),
        Span::raw("      "),
        item("Ambos", target == DeleteTarget::Both),
    ])
}

pub(crate) fn draw_help(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let theme = &app.config.theme;
    let width = area.width.saturating_sub(4).min(72).max(40);
    let height = 15;
    let rect = Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    };
    frame.render_widget(Clear, rect);
    let lines = vec![
        Line::from(Span::styled(
            "Ayuda rapida",
            Style::default()
                .fg(theme.title)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        help_line("Tab", "cambiar foco entre archivos y commits", theme),
        help_line("Space", "stage / unstage del archivo seleccionado", theme),
        help_line("A", "stage de todos los cambios", theme),
        help_line("C", "crear commit", theme),
        help_line("P / U", "push / pull", theme),
        help_line("L", "alinear rama local con GitHub", theme),
        help_line("H", "crear repo GitHub y subir", theme),
        help_line("X", "borrar repo local o GitHub", theme),
        help_line("Enter", "navegar archivo o commit previsualizado", theme),
        help_line("Delete", "borrar el commit mas reciente", theme),
        help_line("Esc", "volver o cancelar", theme),
        help_line("?", "cerrar esta ayuda", theme),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel(theme))
            .wrap(Wrap { trim: true }),
        rect,
    );
}

pub(crate) fn help_line(key: &'static str, label: &'static str, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{key:<8}"),
            Style::default()
                .fg(theme.command_key)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(label, Style::default().fg(theme.text)),
    ])
}

pub(crate) fn dialog_keys_line(
    theme: &Theme,
    keys: &[(&'static str, &'static str)],
) -> Line<'static> {
    let mut spans = Vec::new();
    for (index, (key, label)) in keys.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("    "));
        }
        spans.push(Span::styled(
            *key,
            Style::default()
                .fg(theme.command_key)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {label}"),
            Style::default().fg(theme.text),
        ));
    }
    Line::from(spans)
}

pub(crate) fn diff_visual_lines(text: &str, theme: &Theme, max_width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for line in text.lines() {
        let color = diff_line_color(line, theme);
        for chunk in wrap_preview_line(line, max_width) {
            lines.push(Line::from(Span::styled(chunk, Style::default().fg(color))));
        }
    }
    if lines.is_empty() {
        lines.push(Line::from(""));
    }
    lines
}

pub(crate) fn wrap_preview_line(line: &str, max_width: usize) -> Vec<String> {
    let max_width = max_width.max(1);
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut width = 0;

    for ch in line.chars().flat_map(expand_preview_char) {
        if width >= max_width {
            chunks.push(current);
            current = String::new();
            width = 0;
        }
        current.push(ch);
        width += 1;
    }

    if current.is_empty() {
        chunks.push(String::new());
    } else {
        chunks.push(current);
    }
    chunks
}

pub(crate) fn expand_preview_char(ch: char) -> Vec<char> {
    match ch {
        '\t' => vec![' ', ' ', ' ', ' '],
        ch if ch.is_control() => vec![' '],
        ch => vec![ch],
    }
}

pub(crate) fn diff_line_color(line: &str, theme: &Theme) -> Color {
    let color = if line.starts_with('+') {
        theme.diff_add
    } else if line.starts_with('-') {
        theme.diff_remove
    } else if line.starts_with("@@") {
        theme.diff_meta
    } else if line.starts_with("diff ") {
        theme.unstaged
    } else {
        theme.text
    };
    color
}
