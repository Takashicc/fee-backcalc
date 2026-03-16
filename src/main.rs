#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod ui;

use anyhow::Result;
use fee_backcalc::{apply_saved_theme, ensure_bundled_themes};
use gpui::*;
use gpui_component::ThemeRegistry;
use gpui_component_assets::Assets;

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(|cx| {
        configure_app(cx);
        gpui_component::init(cx);
        if let Ok(themes_dir) = ensure_bundled_themes()
            && let Err(err) = ThemeRegistry::watch_dir(themes_dir, cx, |cx| {
                apply_saved_theme(cx);
            })
        {
            eprintln!("Failed to watch themes directory: {err}");
        }
        apply_saved_theme(cx);
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        cx.spawn(async move |cx| {
            ui::open_main_window(cx)?;
            Result::<()>::Ok(())
        })
        .detach();
    });
}

fn configure_app(cx: &mut App) {
    #[cfg(target_os = "macos")]
    configure_macos_app_icon();

    cx.activate(true);
}

#[cfg(target_os = "macos")]
fn configure_macos_app_icon() {
    use cocoa::{
        appkit::{NSApp, NSApplication, NSImage},
        base::{id, nil},
        foundation::{NSData, NSUInteger},
    };
    use std::ffi::c_void;

    static APP_ICON_PNG: &[u8] = include_bytes!("../assets/app-icon.png");

    unsafe {
        let app = NSApp();
        if app.is_null() {
            return;
        }

        let data: id = NSData::dataWithBytes_length_(
            nil,
            APP_ICON_PNG.as_ptr() as *const c_void,
            APP_ICON_PNG.len() as NSUInteger,
        );
        let image: id = NSImage::initWithData_(NSImage::alloc(nil), data);
        if !image.is_null() {
            app.setApplicationIconImage_(image);
        }
    }
}
