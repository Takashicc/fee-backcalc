use gpui::*;
use gpui_component::{
    ActiveTheme, IconName, StyledExt as _,
    button::{Button, ButtonVariants as _},
    select::Select,
    v_flex,
};

use crate::ui::app::AppState;

pub(crate) fn render_settings_panel(
    state: &AppState,
    compact: bool,
    cx: &mut Context<AppState>,
) -> AnyElement {
    v_flex()
        .gap_4()
        .w_full()
        .max_w(px(if compact { 720.0 } else { 820.0 }))
        .child(
            v_flex()
                .gap_3()
                .p_4()
                .rounded_xl()
                .border_1()
                .border_color(cx.theme().border)
                .child(div().font_semibold().child("テーマ"))
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child("現在のテーマ"),
                        )
                        .child(
                            div()
                                .text_base()
                                .font_semibold()
                                .child(cx.theme().theme_name().to_string()),
                        ),
                )
                .child(
                    v_flex()
                        .gap_2()
                        .child(div().text_sm().child("アプリテーマ"))
                        .child(
                            Select::new(&state.theme_select)
                                .placeholder("テーマを選択")
                                .icon(IconName::Palette)
                                .w_full(),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(
                                    "選択するとすぐに画面へ反映され、次回起動時にも復元されます。",
                                ),
                        ),
                ),
        )
        .child(
            v_flex()
                .gap_3()
                .p_4()
                .rounded_xl()
                .border_1()
                .border_color(cx.theme().border)
                .child(div().font_semibold().child("設定ファイル"))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("保存済み設定を直接確認したいときは、設定フォルダを開けます。"),
                )
                .child(
                    Button::new("open-config-directory")
                        .label("設定ファイルの場所を開く")
                        .primary()
                        .on_click(cx.listener(AppState::on_open_config_directory)),
                ),
        )
        .into_any_element()
}
