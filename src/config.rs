use std::{env, fs, path::PathBuf};

use ratatui::style::Color;

use crate::model::{Config, RawConfig, Shortcuts, Theme};

pub(crate) fn load_config() -> Config {
    let path = config_path();
    if !path.exists() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, default_config_text());
    }

    let raw = fs::read_to_string(&path)
        .ok()
        .and_then(|text| toml::from_str::<RawConfig>(&text).ok())
        .unwrap_or_default();

    let mut theme = default_theme();
    if let Some(values) = raw.theme {
        for (key, value) in values {
            apply_theme_value(&mut theme, &key, &value);
        }
    }

    let mut keys = default_shortcuts();
    if let Some(values) = raw.keys {
        for (key, value) in values {
            apply_key_value(&mut keys, &key, &value);
        }
    }

    Config { theme, keys }
}

pub(crate) fn config_path() -> PathBuf {
    let home = env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".gitsimple").join("config.toml")
}

pub(crate) fn default_config_text() -> &'static str {
    r##"# GitSimple-Terminal config

[theme]
# Colores: black, red, green, yellow, blue, magenta, purple, cyan, gray, grey, white
# Tambien acepta bright_*/light_*, dark_gray, orange, pink, brown, #RRGGBB, rgb(r,g,b) y ansi(0-255)
border = "#6e7681"
title = "#d0d7de"
text = "#f0f6fc"
command_key = "#9da7b3"
muted = "#8b949e"
selected = "#d29922"
staged = "#7ee787"
unstaged = "#d29922"
untracked = "#ffa657"
error = "#ff7b72"
success = "#7ee787"
diff_add = "#7ee787"
diff_remove = "#ff7b72"
diff_meta = "#79c0ff"

[keys]
quit = "q"
refresh = "r"
stage_all = "a"
commit = "c"
push = "p"
pull = "u"
init = "i"
github = "h"
new_repo = "n"
delete_repo = "x"
switch_branch = "b"
create_branch = "m"
"##
}

pub(crate) fn default_theme() -> Theme {
    Theme {
        border: Color::Rgb(110, 118, 129),
        title: Color::Rgb(208, 215, 222),
        text: Color::Rgb(240, 246, 252),
        command_key: Color::Rgb(157, 167, 179),
        muted: Color::Rgb(139, 148, 158),
        selected: Color::Rgb(210, 153, 34),
        staged: Color::Rgb(126, 231, 135),
        unstaged: Color::Rgb(210, 153, 34),
        untracked: Color::Rgb(255, 166, 87),
        error: Color::Rgb(255, 123, 114),
        success: Color::Rgb(126, 231, 135),
        diff_add: Color::Rgb(126, 231, 135),
        diff_remove: Color::Rgb(255, 123, 114),
        diff_meta: Color::Rgb(121, 192, 255),
    }
}

pub(crate) fn default_shortcuts() -> Shortcuts {
    Shortcuts {
        quit: 'q',
        refresh: 'r',
        stage_all: 'a',
        commit: 'c',
        push: 'p',
        pull: 'u',
        init: 'i',
        github: 'h',
        new_repo: 'n',
        delete_repo: 'x',
        switch_branch: 'b',
        create_branch: 'm',
    }
}

pub(crate) fn apply_theme_value(theme: &mut Theme, key: &str, value: &str) {
    let color = parse_color(value);
    match key {
        "border" => theme.border = color,
        "title" => theme.title = color,
        "text" => theme.text = color,
        "command_key" | "command_keys" | "icons" => theme.command_key = color,
        "muted" => theme.muted = color,
        "selected" => theme.selected = color,
        "staged" => theme.staged = color,
        "unstaged" => theme.unstaged = color,
        "untracked" => theme.untracked = color,
        "error" => theme.error = color,
        "success" => theme.success = color,
        "diff_add" => theme.diff_add = color,
        "diff_remove" => theme.diff_remove = color,
        "diff_meta" => theme.diff_meta = color,
        _ => {}
    }
}

pub(crate) fn apply_key_value(keys: &mut Shortcuts, key: &str, value: &str) {
    let Some(ch) = value.chars().find(|ch| !ch.is_whitespace()) else {
        return;
    };
    match key {
        "quit" => keys.quit = ch,
        "refresh" => keys.refresh = ch,
        "stage_all" => keys.stage_all = ch,
        "commit" => keys.commit = ch,
        "push" => keys.push = ch,
        "pull" => keys.pull = ch,
        "init" => keys.init = ch,
        "github" => keys.github = ch,
        "new_repo" => keys.new_repo = ch,
        "delete_repo" => keys.delete_repo = ch,
        "switch_branch" => keys.switch_branch = ch,
        "create_branch" => keys.create_branch = ch,
        _ => {}
    }
}

pub(crate) fn parse_color(value: &str) -> Color {
    let color = value.trim().to_ascii_lowercase();
    if let Some(color) = parse_hex_color(&color) {
        return color;
    }
    if let Some(color) = parse_rgb_color(&color) {
        return color;
    }
    if let Some(color) = parse_ansi_color(&color) {
        return color;
    }

    match color.replace([' ', '-'], "_").as_str() {
        "reset" | "default" | "terminal" => Color::Reset,
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" | "purple" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" | "silver" => Color::Gray,
        "dark_gray" | "dark_grey" | "darkgray" | "darkgrey" => Color::DarkGray,
        "bright_red" | "light_red" | "lightred" => Color::LightRed,
        "bright_green" | "light_green" | "lightgreen" => Color::LightGreen,
        "bright_yellow" | "light_yellow" | "lightyellow" => Color::LightYellow,
        "bright_blue" | "light_blue" | "lightblue" => Color::LightBlue,
        "bright_magenta" | "light_magenta" | "lightmagenta" | "pink" => Color::LightMagenta,
        "bright_cyan" | "light_cyan" | "lightcyan" => Color::LightCyan,
        "white" => Color::White,
        "bright_black" | "light_black" | "lightblack" => Color::DarkGray,
        "bright_white" | "light_white" | "lightwhite" => Color::White,
        "orange" => Color::Rgb(255, 165, 0),
        "brown" => Color::Rgb(165, 42, 42),
        "lime" => Color::Rgb(0, 255, 0),
        "teal" => Color::Rgb(0, 128, 128),
        "navy" => Color::Rgb(0, 0, 128),
        "violet" => Color::Rgb(238, 130, 238),
        "indigo" => Color::Rgb(75, 0, 130),
        _ => Color::White,
    }
}

pub(crate) fn parse_hex_color(value: &str) -> Option<Color> {
    let hex = value
        .strip_prefix('#')
        .or_else(|| value.strip_prefix("0x"))?;
    if hex.len() != 6 || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(red, green, blue))
}

pub(crate) fn parse_rgb_color(value: &str) -> Option<Color> {
    let inner = value.strip_prefix("rgb(")?.strip_suffix(')')?;
    let mut parts = inner.split(',').map(str::trim);
    let red = parts.next()?.parse::<u8>().ok()?;
    let green = parts.next()?.parse::<u8>().ok()?;
    let blue = parts.next()?.parse::<u8>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(Color::Rgb(red, green, blue))
}

pub(crate) fn parse_ansi_color(value: &str) -> Option<Color> {
    let index = value
        .strip_prefix("ansi(")
        .and_then(|value| value.strip_suffix(')'))
        .or_else(|| {
            value
                .strip_prefix("color(")
                .and_then(|value| value.strip_suffix(')'))
        })
        .or_else(|| value.strip_prefix("ansi_"))
        .or_else(|| value.strip_prefix("color_"))?;
    Some(Color::Indexed(index.trim().parse::<u8>().ok()?))
}

pub(crate) fn same_key(input: char, configured: char) -> bool {
    input.eq_ignore_ascii_case(&configured)
}

pub(crate) fn parse_github_repo_prompt(raw: &str) -> (bool, String) {
    let trimmed = raw.trim();
    if let Some(name) = trimmed.strip_prefix("public:") {
        (true, name.trim().to_string())
    } else if let Some(name) = trimmed.strip_prefix("public ") {
        (true, name.trim().to_string())
    } else if let Some(name) = trimmed.strip_prefix("private:") {
        (false, name.trim().to_string())
    } else {
        (false, trimmed.to_string())
    }
}
