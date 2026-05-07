use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Color;

use crate::{
    config::parse_color,
    events::{
        delete_prompt_char_before, events_as_pasted_text, insert_prompt_char, move_prompt_cursor,
        move_prompt_cursor_vertical,
    },
    git::{
        github_repo_slug, is_missing_head_error, is_push_alignment_error, is_selected_head_commit,
        parse_branch_sync_counts,
    },
    ui::wrap_preview_line,
};

#[test]
fn parses_extended_named_colors() {
    assert_eq!(parse_color("bright-red"), Color::LightRed);
    assert_eq!(parse_color("light blue"), Color::LightBlue);
    assert_eq!(parse_color("dark_gray"), Color::DarkGray);
    assert_eq!(parse_color("orange"), Color::Rgb(255, 165, 0));
}

#[test]
fn parses_custom_color_values() {
    assert_eq!(parse_color("#1a2b3c"), Color::Rgb(26, 43, 60));
    assert_eq!(parse_color("0xff8800"), Color::Rgb(255, 136, 0));
    assert_eq!(parse_color("rgb(12, 34, 56)"), Color::Rgb(12, 34, 56));
    assert_eq!(parse_color("ansi(208)"), Color::Indexed(208));
}

#[test]
fn wraps_long_preview_lines_inside_panel_width() {
    assert_eq!(
        wrap_preview_line("abcdefghijkl", 5),
        vec!["abcde", "fghij", "kl"]
    );
    assert_eq!(wrap_preview_line("a\tb", 3), vec!["a  ", "  b"]);
}

#[test]
fn detects_unborn_head_errors() {
    assert!(is_missing_head_error("fatal: could not resolve HEAD"));
    assert!(is_missing_head_error("fatal: could not resolve 'HEAD'"));
    assert!(is_missing_head_error("fatal: bad revision 'HEAD'"));
}

#[test]
fn detects_branch_alignment_state() {
    let sync = parse_branch_sync_counts("2\t1").unwrap();
    assert_eq!(sync.ahead, 2);
    assert_eq!(sync.behind, 1);
    assert!(sync.needs_align());
    assert!(
        !parse_branch_sync_counts("2")
            .unwrap_or_default()
            .needs_align()
    );
}

#[test]
fn detects_push_rejection_as_alignment_error() {
    assert!(is_push_alignment_error(
        "! [rejected] master -> master (fetch first)"
    ));
    assert!(is_push_alignment_error(
        "error: failed to push some refs to origin"
    ));
}

#[test]
fn parses_github_remote_slug() {
    assert_eq!(
        github_repo_slug("https://github.com/TabulateTech/gitsimple-terminal.git"),
        Some("TabulateTech/gitsimple-terminal".to_string())
    );
    assert_eq!(
        github_repo_slug("git@github.com:TabulateTech/gitsimple-terminal.git"),
        Some("TabulateTech/gitsimple-terminal".to_string())
    );
    assert_eq!(github_repo_slug("sin origin"), None);
}

#[test]
fn edits_prompt_text_at_cursor() {
    let mut value = "helo".to_string();
    let mut cursor = 2;
    insert_prompt_char(&mut value, &mut cursor, 'l');
    assert_eq!(value, "hello");
    assert_eq!(cursor, 3);

    move_prompt_cursor(&value, &mut cursor, -1);
    delete_prompt_char_before(&mut value, &mut cursor);
    assert_eq!(value, "hllo");
    assert_eq!(cursor, 1);
}

#[test]
fn only_allows_deleting_selected_head_commit() {
    assert!(is_selected_head_commit("abc1234", "abc1234"));
    assert!(!is_selected_head_commit("abc1234", "def5678"));
    assert!(!is_selected_head_commit("", "def5678"));
}

#[test]
fn moves_prompt_cursor_between_lines() {
    let value = "one\ntwo\nthree";
    let mut cursor = 5;
    move_prompt_cursor_vertical(value, &mut cursor, 1);
    assert_eq!(cursor, 9);
    move_prompt_cursor_vertical(value, &mut cursor, -1);
    assert_eq!(cursor, 5);
}

#[test]
fn treats_batched_enter_as_pasted_newline() {
    let events = vec![
        Event::Key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL)),
        Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE)),
    ];
    assert_eq!(events_as_pasted_text(&events), Some("a\nb".to_string()));
}
