use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Rect;

use crate::{config::same_key, git::file_diff, model::*};

pub(crate) fn handle_events(app: &mut App, events: Vec<Event>) -> bool {
    if app.prompt.is_some() && events.len() > 1 {
        if let Some(text) = events_as_pasted_text(&events) {
            return handle_paste(app, &text);
        }
    }

    let mut changed = false;
    for event in events {
        changed |= match event {
            Event::Key(key) => handle_key(app, key),
            Event::Paste(text) => handle_paste(app, &text),
            Event::Mouse(mouse) => handle_mouse(app, mouse),
            Event::Resize(_, _) => true,
            _ => false,
        };
        if !app.running {
            break;
        }
    }
    changed
}

pub(crate) fn events_as_pasted_text(events: &[Event]) -> Option<String> {
    let mut text = String::new();
    for event in events {
        let Event::Key(key) = event else {
            return None;
        };
        if key.kind != KeyEventKind::Press {
            return None;
        }
        if is_paste_shortcut_key(*key) {
            continue;
        }
        let ch = key_as_paste_char(*key)?;
        text.push(ch);
    }
    if text.is_empty() { None } else { Some(text) }
}

pub(crate) fn key_as_paste_char(key: KeyEvent) -> Option<char> {
    match key.code {
        KeyCode::Char(ch) if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT => {
            Some(ch)
        }
        KeyCode::Enter => Some('\n'),
        KeyCode::Tab => Some('\t'),
        _ => None,
    }
}

pub(crate) fn is_paste_shortcut_key(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('v' | 'V')) && key.modifiers.contains(KeyModifiers::CONTROL)
}

pub(crate) fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    if key.kind != KeyEventKind::Press {
        return false;
    }

    if app.help_open {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') => app.help_open = false,
            _ => return false,
        }
        return true;
    }

    if app.confirm.is_some() {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => app.run_confirmed(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_confirm(),
            _ => return false,
        }
        return true;
    }

    if app.github_visibility.is_some() {
        match key.code {
            KeyCode::Esc => app.cancel_github_visibility(),
            KeyCode::Left | KeyCode::Up => {
                if let Some(choice) = app.github_visibility.as_mut() {
                    choice.public_selected = false;
                }
            }
            KeyCode::Right | KeyCode::Down => {
                if let Some(choice) = app.github_visibility.as_mut() {
                    choice.public_selected = true;
                }
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                if let Some(choice) = app.github_visibility.as_mut() {
                    choice.public_selected = false;
                }
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                if let Some(choice) = app.github_visibility.as_mut() {
                    choice.public_selected = true;
                }
            }
            KeyCode::Enter => app.confirm_github_visibility(),
            _ => return false,
        }
        return true;
    }

    if app.delete_repo_choice.is_some() {
        match key.code {
            KeyCode::Esc => app.cancel_delete_repo_choice(),
            KeyCode::Left | KeyCode::Up => {
                if let Some(choice) = app.delete_repo_choice.as_mut() {
                    choice.target = choice.target.previous();
                }
            }
            KeyCode::Right | KeyCode::Down => {
                if let Some(choice) = app.delete_repo_choice.as_mut() {
                    choice.target = choice.target.next();
                }
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                if let Some(choice) = app.delete_repo_choice.as_mut() {
                    choice.target = DeleteTarget::Local;
                }
            }
            KeyCode::Char('g') | KeyCode::Char('G') => {
                if let Some(choice) = app.delete_repo_choice.as_mut() {
                    choice.target = DeleteTarget::Github;
                }
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                if let Some(choice) = app.delete_repo_choice.as_mut() {
                    choice.target = DeleteTarget::Both;
                }
            }
            KeyCode::Enter => app.confirm_delete_repo_choice(),
            _ => return false,
        }
        return true;
    }

    if app.prompt.is_some() && is_paste_shortcut_key(key) {
        app.arm_prompt_paste();
        return true;
    }

    let prompt_paste_active = app.prompt_paste_active();
    let mut extend_prompt_paste = false;
    let mut clear_prompt_paste = false;
    if let Some(prompt) = app.prompt.as_mut() {
        match key.code {
            KeyCode::Esc => {
                clear_prompt_paste = true;
                let mode = prompt.mode;
                app.prompt = None;
                app.message = match mode {
                    PromptMode::Commit => "Commit cancelado.".to_string(),
                    PromptMode::GithubRepo => "GitHub cancelado.".to_string(),
                    PromptMode::NewLocalRepo => "Repo nuevo cancelado.".to_string(),
                    PromptMode::SwitchBranch => "Cambio de rama cancelado.".to_string(),
                    PromptMode::CreateBranch => "Crear rama cancelado.".to_string(),
                };
            }
            KeyCode::Enter
                if prompt.mode == PromptMode::Commit
                    && key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                prompt.editing_description = true;
                insert_prompt_char(
                    &mut prompt.description,
                    &mut prompt.description_cursor,
                    '\n',
                );
            }
            KeyCode::Enter if prompt.mode == PromptMode::Commit && prompt_paste_active => {
                prompt.editing_description = true;
                insert_prompt_char(
                    &mut prompt.description,
                    &mut prompt.description_cursor,
                    '\n',
                );
                extend_prompt_paste = true;
            }
            KeyCode::Enter => app.submit_prompt(),
            KeyCode::Backspace => {
                if prompt.mode == PromptMode::Commit && prompt.editing_description {
                    delete_prompt_char_before(
                        &mut prompt.description,
                        &mut prompt.description_cursor,
                    );
                } else {
                    delete_prompt_char_before(&mut prompt.value, &mut prompt.value_cursor);
                }
            }
            KeyCode::Delete => {
                if prompt.mode == PromptMode::Commit && prompt.editing_description {
                    delete_prompt_char_at(&mut prompt.description, prompt.description_cursor);
                } else {
                    delete_prompt_char_at(&mut prompt.value, prompt.value_cursor);
                }
            }
            KeyCode::Left => {
                if prompt.mode == PromptMode::Commit && prompt.editing_description {
                    move_prompt_cursor(&prompt.description, &mut prompt.description_cursor, -1);
                } else {
                    move_prompt_cursor(&prompt.value, &mut prompt.value_cursor, -1);
                }
            }
            KeyCode::Right => {
                if prompt.mode == PromptMode::Commit && prompt.editing_description {
                    move_prompt_cursor(&prompt.description, &mut prompt.description_cursor, 1);
                } else {
                    move_prompt_cursor(&prompt.value, &mut prompt.value_cursor, 1);
                }
            }
            KeyCode::Tab if prompt.mode == PromptMode::Commit => {
                prompt.editing_description = !prompt.editing_description;
            }
            KeyCode::Up if prompt.mode == PromptMode::Commit => {
                if prompt.editing_description {
                    move_prompt_cursor_vertical(
                        &prompt.description,
                        &mut prompt.description_cursor,
                        -1,
                    );
                }
            }
            KeyCode::Down if prompt.mode == PromptMode::Commit => {
                if prompt.editing_description {
                    move_prompt_cursor_vertical(
                        &prompt.description,
                        &mut prompt.description_cursor,
                        1,
                    );
                }
            }
            KeyCode::Char(ch) => {
                if !ch.is_control() {
                    if prompt.mode == PromptMode::Commit && prompt.editing_description {
                        insert_prompt_char(
                            &mut prompt.description,
                            &mut prompt.description_cursor,
                            ch,
                        );
                    } else {
                        insert_prompt_char(&mut prompt.value, &mut prompt.value_cursor, ch);
                    }
                    if prompt_paste_active {
                        extend_prompt_paste = true;
                    }
                }
            }
            _ => {}
        }
        if clear_prompt_paste {
            app.clear_prompt_paste();
        } else if extend_prompt_paste {
            app.arm_prompt_paste();
        }
        return true;
    }

    if app.browsing_diff {
        match key.code {
            KeyCode::Esc => {
                app.browsing_diff = false;
                app.message = match app.focus {
                    FocusPane::Files => "Archivos".to_string(),
                    FocusPane::Commits => "Commits".to_string(),
                };
            }
            KeyCode::Up => app.scroll_diff(-1),
            KeyCode::Down => app.scroll_diff(1),
            KeyCode::PageUp => app.scroll_diff(-8),
            KeyCode::PageDown => app.scroll_diff(8),
            KeyCode::Home => app.diff_scroll = 0,
            KeyCode::End => app.scroll_diff(app.diff_text.lines().count() as isize),
            _ => return false,
        }
        return true;
    }

    let keys = app.config.keys.clone();
    match key.code {
        KeyCode::Char(ch) if same_key(ch, keys.quit) => app.running = false,
        KeyCode::Char('?') => app.help_open = true,
        KeyCode::Tab => app.next_focus(),
        KeyCode::Up => match app.focus {
            FocusPane::Files => app.move_selection(-1),
            FocusPane::Commits => app.move_commit_selection(-1),
        },
        KeyCode::Down => match app.focus {
            FocusPane::Files => app.move_selection(1),
            FocusPane::Commits => app.move_commit_selection(1),
        },
        KeyCode::PageUp => match app.focus {
            FocusPane::Commits => app.move_commit_selection(-5),
            FocusPane::Files => app.move_selection(-5),
        },
        KeyCode::PageDown => match app.focus {
            FocusPane::Commits => app.move_commit_selection(5),
            FocusPane::Files => app.move_selection(5),
        },
        KeyCode::Enter => match app.focus {
            FocusPane::Files | FocusPane::Commits => app.open_preview_view(),
        },
        KeyCode::Delete => match app.focus {
            FocusPane::Files => app.message = "Archivos".to_string(),
            FocusPane::Commits => app.delete_selected_commit(),
        },
        KeyCode::Home => match app.focus {
            FocusPane::Files => app.move_selection(-(app.files.len() as isize)),
            FocusPane::Commits => app.move_commit_selection(-(app.commits.len() as isize)),
        },
        KeyCode::End => match app.focus {
            FocusPane::Files => app.move_selection(app.files.len() as isize),
            FocusPane::Commits => app.move_commit_selection(app.commits.len() as isize),
        },
        KeyCode::Char(' ') => match app.focus {
            FocusPane::Files => app.toggle_stage(),
            FocusPane::Commits => app.message = "Commits".to_string(),
        },
        KeyCode::Char(ch) if same_key(ch, keys.stage_all) => app.stage_all(),
        KeyCode::Char(ch) if same_key(ch, keys.commit) => {
            let staged = app.files.iter().filter(|file| file.is_staged()).count();
            if staged == 0 {
                app.message = "No hay cambios en stage. Usa SPACE o A primero.".to_string();
            } else {
                app.open_prompt(PromptMode::Commit);
            }
        }
        KeyCode::Char(ch) if same_key(ch, keys.push) => app.push(),
        KeyCode::Char(ch) if same_key(ch, keys.pull) => app.pull(),
        KeyCode::Char('l') | KeyCode::Char('L') => app.align_branch(),
        KeyCode::Char(ch) if same_key(ch, keys.github) => app.open_prompt(PromptMode::GithubRepo),
        KeyCode::Char(ch) if same_key(ch, keys.new_repo) => {
            app.open_prompt(PromptMode::NewLocalRepo)
        }
        KeyCode::Char(ch) if same_key(ch, keys.delete_repo) => app.delete_repo(),
        KeyCode::Char(ch) if same_key(ch, keys.switch_branch) => {
            app.open_prompt(PromptMode::SwitchBranch)
        }
        KeyCode::Char(ch) if same_key(ch, keys.create_branch) => {
            app.open_prompt(PromptMode::CreateBranch)
        }
        KeyCode::Char(ch) if same_key(ch, keys.refresh) => {
            app.refresh(Some("Informacion actualizada.".to_string()));
        }
        KeyCode::Char(ch) if same_key(ch, keys.init) => app.init_repo(),
        _ => return false,
    }
    true
}

pub(crate) fn handle_paste(app: &mut App, text: &str) -> bool {
    let mut arm_paste = false;
    let Some(prompt) = app.prompt.as_mut() else {
        return false;
    };

    if prompt.mode == PromptMode::Commit {
        paste_into_commit_prompt(prompt, text);
        arm_paste = true;
    } else {
        let text = text.replace(['\r', '\n'], " ");
        insert_prompt_text(&mut prompt.value, &mut prompt.value_cursor, &text);
    }
    if arm_paste {
        app.arm_prompt_paste();
    }
    true
}

pub(crate) fn paste_into_commit_prompt(prompt: &mut Prompt, text: &str) {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    if prompt.editing_description {
        insert_prompt_text(
            &mut prompt.description,
            &mut prompt.description_cursor,
            &text,
        );
        return;
    }

    if let Some((title, description)) = text.split_once('\n') {
        insert_prompt_text(&mut prompt.value, &mut prompt.value_cursor, title);
        let description = description.trim_start_matches('\n');
        if !description.is_empty() {
            if !prompt.description.is_empty() && prompt.description_cursor > 0 {
                insert_prompt_char(
                    &mut prompt.description,
                    &mut prompt.description_cursor,
                    '\n',
                );
            }
            insert_prompt_text(
                &mut prompt.description,
                &mut prompt.description_cursor,
                description,
            );
        }
    } else {
        insert_prompt_text(&mut prompt.value, &mut prompt.value_cursor, &text);
    }
}

pub(crate) fn handle_mouse(app: &mut App, mouse: MouseEvent) -> bool {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let x = mouse.column;
            let y = mouse.row;
            let files = app.files_rect;
            if x > files.x
                && x < files.x.saturating_add(files.width).saturating_sub(1)
                && y > files.y
                && y < files.y.saturating_add(files.height).saturating_sub(1)
            {
                let index = app.file_scroll + usize::from(y - files.y - 1);
                if index < app.files.len() && index != app.selected {
                    app.focus = FocusPane::Files;
                    app.selected = index;
                    app.diff_scroll = 0;
                    app.diff_text = file_diff(&app.files[app.selected]);
                    return true;
                }
            }
            false
        }
        MouseEventKind::ScrollUp => {
            if app.browsing_diff && point_in_rect(mouse.column, mouse.row, app.diff_rect) {
                app.scroll_diff(-3);
            } else {
                app.focus = FocusPane::Files;
                app.move_selection(-1);
            }
            true
        }
        MouseEventKind::ScrollDown => {
            if app.browsing_diff && point_in_rect(mouse.column, mouse.row, app.diff_rect) {
                app.scroll_diff(3);
            } else {
                app.focus = FocusPane::Files;
                app.move_selection(1);
            }
            true
        }
        MouseEventKind::Moved | MouseEventKind::Drag(_) | MouseEventKind::Up(_) => false,
        _ => false,
    }
}

pub(crate) fn point_in_rect(x: u16, y: u16, rect: Rect) -> bool {
    x >= rect.x
        && x < rect.x.saturating_add(rect.width)
        && y >= rect.y
        && y < rect.y.saturating_add(rect.height)
}

pub(crate) fn char_to_byte_index(value: &str, char_index: usize) -> usize {
    value
        .char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(value.len())
}

pub(crate) fn insert_prompt_char(value: &mut String, cursor: &mut usize, ch: char) {
    let byte_index = char_to_byte_index(value, *cursor);
    value.insert(byte_index, ch);
    *cursor += 1;
}

pub(crate) fn insert_prompt_text(value: &mut String, cursor: &mut usize, text: &str) {
    let byte_index = char_to_byte_index(value, *cursor);
    value.insert_str(byte_index, text);
    *cursor += text.chars().count();
}

pub(crate) fn delete_prompt_char_before(value: &mut String, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let start = char_to_byte_index(value, (*cursor).saturating_sub(1));
    let end = char_to_byte_index(value, *cursor);
    value.replace_range(start..end, "");
    *cursor -= 1;
}

pub(crate) fn delete_prompt_char_at(value: &mut String, cursor: usize) {
    let len = value.chars().count();
    if cursor >= len {
        return;
    }
    let start = char_to_byte_index(value, cursor);
    let end = char_to_byte_index(value, cursor + 1);
    value.replace_range(start..end, "");
}

pub(crate) fn move_prompt_cursor(value: &str, cursor: &mut usize, delta: isize) {
    let max = value.chars().count();
    *cursor = cursor.saturating_add_signed(delta).min(max);
}

pub(crate) fn move_prompt_cursor_vertical(value: &str, cursor: &mut usize, delta: isize) {
    let (line, column) = cursor_line_column(value, *cursor);
    let lines: Vec<&str> = value.split('\n').collect();
    if lines.is_empty() {
        *cursor = 0;
        return;
    }
    let target_line = line.saturating_add_signed(delta).min(lines.len() - 1);
    *cursor = cursor_from_line_column(value, target_line, column);
}

pub(crate) fn cursor_line_column(value: &str, cursor: usize) -> (usize, usize) {
    let mut line = 0;
    let mut column = 0;
    for ch in value.chars().take(cursor) {
        if ch == '\n' {
            line += 1;
            column = 0;
        } else {
            column += 1;
        }
    }
    (line, column)
}

pub(crate) fn cursor_from_line_column(
    value: &str,
    target_line: usize,
    target_column: usize,
) -> usize {
    let mut line = 0;
    let mut column = 0;
    let mut cursor = 0;
    for ch in value.chars() {
        if line == target_line && column >= target_column {
            return cursor;
        }
        if ch == '\n' {
            if line == target_line {
                return cursor;
            }
            line += 1;
            column = 0;
        } else {
            column += 1;
        }
        cursor += 1;
    }
    cursor
}
