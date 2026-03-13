use gpui::*;
use gpui_component::ActiveTheme;

use crate::ui::app::AppState;

pub(crate) fn render_error(message: String, cx: &mut Context<AppState>) -> AnyElement {
    div()
        .text_sm()
        .text_color(cx.theme().danger)
        .child(message)
        .into_any_element()
}
