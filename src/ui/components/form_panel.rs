use fee_backcalc::{jp, trim_trailing_zero};
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, IconName, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, NumberInput},
    select::Select,
    v_flex,
};

use crate::ui::{app::AppState, components::shared::render_error};

pub(crate) fn render_left_panel(
    state: &AppState,
    base_error: Option<String>,
    tax_error: Option<String>,
    site_form_error: Option<String>,
    compact: bool,
    cx: &mut Context<AppState>,
) -> AnyElement {
    let site_rows: Vec<AnyElement> = state
        .model
        .sites()
        .iter()
        .enumerate()
        .map(|(index, site)| render_site_row(state, index, site, compact, cx))
        .collect();

    v_flex()
        .gap_4()
        .w_full()
        .when(!compact, |this| this.flex_1())
        .child(
            v_flex()
                .gap_3()
                .p_4()
                .rounded_xl()
                .border_1()
                .border_color(cx.theme().border)
                .child(div().font_semibold().child(jp("逆算条件")))
                .child(
                    v_flex()
                        .gap_2()
                        .child(div().text_sm().child(jp("受け取りたい金額")))
                        .child(NumberInput::new(&state.base_amount_input).w_full())
                        .when_some(base_error, |this, message| {
                            this.child(render_error(message, cx))
                        }),
                )
                .child(
                    v_flex()
                        .gap_2()
                        .child(div().text_sm().child(jp("受け取りたい金額の入力形式")))
                        .child(
                            Select::new(&state.input_tax_mode_select)
                                .placeholder(jp("入力形式を選択"))
                                .icon(IconName::ChevronsUpDown)
                                .w_full(),
                        ),
                )
                .child(
                    v_flex()
                        .gap_2()
                        .child(div().text_sm().child(jp("消費税率 (%)")))
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(jp(
                                    "手数料の逆算に加えて、請求時の消費税もあわせて計算します。",
                                )),
                        )
                        .child(NumberInput::new(&state.tax_rate_input).w_full())
                        .when_some(tax_error, |this, message| {
                            this.child(render_error(message, cx))
                        }),
                )
                .child(
                    v_flex()
                        .gap_2()
                        .child(div().text_sm().child(jp("端数処理")))
                        .child(
                            Select::new(&state.rounding_mode_select)
                                .placeholder(jp("端数処理を選択"))
                                .icon(IconName::ChevronsUpDown)
                                .w_full(),
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
                .child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .child(div().font_semibold().child(jp("手数料設定")))
                        .when_some(state.model.editing_site(), |this, site| {
                            this.child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().primary)
                                    .child(format!("{}を編集中", site.name)),
                            )
                        }),
                )
                .child(
                    v_flex()
                        .gap_2()
                        .child(div().text_sm().child(jp("依頼サイト名")))
                        .child(Input::new(&state.site_name_input).cleanable(true).w_full()),
                )
                .child(
                    v_flex()
                        .gap_2()
                        .child(div().text_sm().child(jp("手数料率 (%)")))
                        .child(NumberInput::new(&state.site_fee_input).w_full())
                        .when_some(site_form_error, |this, message| {
                            this.child(render_error(message, cx))
                        }),
                )
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .when(compact, |this| this.flex_col())
                        .when(!compact, |this| this.flex_row())
                        .child(
                            Button::new("save-site")
                                .label(if state.model.editing_index().is_some() {
                                    jp("設定を更新")
                                } else {
                                    jp("設定を追加")
                                })
                                .primary()
                                .on_click(cx.listener(AppState::on_add_or_update_site)),
                        )
                        .when(state.model.editing_index().is_some(), |this| {
                            this.child(
                                Button::new("cancel-edit")
                                    .label(jp("編集をキャンセル"))
                                    .warning()
                                    .on_click(cx.listener(AppState::on_cancel_edit)),
                            )
                        }),
                )
                .when(state.model.sites().is_empty(), |this| {
                    this.child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(jp("まだ手数料設定はありません。")),
                    )
                })
                .children(site_rows),
        )
        .into_any_element()
}

fn render_site_row(
    state: &AppState,
    index: usize,
    site: &fee_backcalc::SiteFee,
    compact: bool,
    cx: &mut Context<AppState>,
) -> AnyElement {
    let editing = state.model.editing_index() == Some(index);
    v_flex()
        .gap_2()
        .p_3()
        .rounded_lg()
        .border_1()
        .border_color(cx.theme().border)
        .child(
            div()
                .flex()
                .gap_3()
                .when(compact, |this| this.flex_col())
                .when(!compact, |this| {
                    this.flex_row().justify_between().items_center()
                })
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(div().font_semibold().child(site.name.clone()))
                                .when(editing, |this| {
                                    this.child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().primary)
                                            .child(jp("編集中")),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!(
                                    "{}: {}%",
                                    jp("手数料率"),
                                    trim_trailing_zero(site.fee_percent)
                                )),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .when(compact, |this| this.flex_col())
                        .when(!compact, |this| this.flex_row())
                        .child(
                            Button::new(("edit-site", index))
                                .label(jp("編集"))
                                .info()
                                .on_click(cx.listener(move |this, event, window, cx| {
                                    this.on_start_edit_site(index, event, window, cx);
                                })),
                        )
                        .child(
                            Button::new(("delete-site", index))
                                .label(jp("削除"))
                                .danger()
                                .on_click(cx.listener(move |this, event, window, cx| {
                                    this.on_remove_site(index, event, window, cx);
                                })),
                        ),
                ),
        )
        .into_any_element()
}
