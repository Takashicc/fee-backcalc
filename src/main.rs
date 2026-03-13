mod ui;

use anyhow::Result;
use gpui::*;
use gpui_component_assets::Assets;

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(|cx| {
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            ui::open_main_window(cx)?;
            Result::<()>::Ok(())
        })
        .detach();
    });
}
