use std::borrow::{Borrow, BorrowMut};
use std::fs;
use std::path::PathBuf;

use gpui::{App, BorrowAppContext, SharedString};
use gpui_component::{Theme, ThemeRegistry};

use crate::{config_dir, load_config};

const BUNDLED_THEMES: &[(&str, &str)] = &[
    ("adventure.json", include_str!("../themes/adventure.json")),
    ("alduin.json", include_str!("../themes/alduin.json")),
    ("asciinema.json", include_str!("../themes/asciinema.json")),
    ("ayu.json", include_str!("../themes/ayu.json")),
    ("catppuccin.json", include_str!("../themes/catppuccin.json")),
    ("everforest.json", include_str!("../themes/everforest.json")),
    ("fahrenheit.json", include_str!("../themes/fahrenheit.json")),
    ("flexoki.json", include_str!("../themes/flexoki.json")),
    ("gruvbox.json", include_str!("../themes/gruvbox.json")),
    ("harper.json", include_str!("../themes/harper.json")),
    ("hybrid.json", include_str!("../themes/hybrid.json")),
    ("jellybeans.json", include_str!("../themes/jellybeans.json")),
    ("kibble.json", include_str!("../themes/kibble.json")),
    (
        "macos-classic.json",
        include_str!("../themes/macos-classic.json"),
    ),
    ("matrix.json", include_str!("../themes/matrix.json")),
    (
        "mellifluous.json",
        include_str!("../themes/mellifluous.json"),
    ),
    ("molokai.json", include_str!("../themes/molokai.json")),
    ("solarized.json", include_str!("../themes/solarized.json")),
    ("spaceduck.json", include_str!("../themes/spaceduck.json")),
    ("tokyonight.json", include_str!("../themes/tokyonight.json")),
    ("twilight.json", include_str!("../themes/twilight.json")),
];

pub fn apply_saved_theme(cx: &mut App) -> bool {
    let config = load_config();
    let Some(theme_name) = config.theme_name.as_deref() else {
        return false;
    };

    apply_theme_by_name(theme_name, cx)
}

pub fn apply_theme_by_name<C>(theme_name: &str, cx: &mut C) -> bool
where
    C: Borrow<App> + BorrowMut<App> + BorrowAppContext,
{
    let theme_name = SharedString::from(theme_name.to_string());
    let app: &App = Borrow::borrow(&*cx);
    let Some(theme) = ThemeRegistry::global(app)
        .themes()
        .get(&theme_name)
        .cloned()
    else {
        return false;
    };

    cx.update_global::<Theme, _>(|active_theme, cx| {
        active_theme.apply_config(&theme);
        cx.borrow_mut().refresh_windows();
    });

    true
}

pub fn ensure_bundled_themes() -> anyhow::Result<PathBuf> {
    let themes_dir = config_dir().join("themes");
    fs::create_dir_all(&themes_dir)?;

    for (file_name, contents) in BUNDLED_THEMES {
        let path = themes_dir.join(file_name);
        fs::write(path, contents)?;
    }

    Ok(themes_dir)
}
