use fee_backcalc::{format_yen, integer_string, trim_trailing_zero};
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, StyledExt as _, WindowExt as _, clipboard::Clipboard, h_flex,
    scroll::ScrollableElement as _, v_flex,
};

use crate::ui::{app::AppState, components::shared::render_error};

pub(crate) fn render_right_panel(
    state: &AppState,
    compact: bool,
    cx: &mut Context<AppState>,
) -> AnyElement {
    v_flex()
        .gap_3()
        .w_full()
        .when(!compact, |this| this.flex_1())
        .p_4()
        .rounded_xl()
        .border_1()
        .border_color(cx.theme().border)
        .child(div().font_semibold().child("逆算結果"))
        .when(state.model.sites().is_empty(), |this| {
            this.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("手数料設定を追加すると、サイトごとの請求額がここに表示されます。"),
            )
        })
        .when_some(state.model.result_error(), |this, message| {
            this.child(render_error(message.to_string(), cx))
        })
        .child(
            div()
                .w_full()
                .overflow_x_scrollbar()
                .child(render_results_table(state, cx)),
        )
        .into_any_element()
}

fn render_results_table(state: &AppState, cx: &mut Context<AppState>) -> AnyElement {
    let body_rows: Vec<AnyElement> = if state.model.result_rows().is_empty() {
        vec![
            h_flex()
                .w_full()
                .justify_center()
                .items_center()
                .px_3()
                .py_8()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child("表示できる請求額がありません。")
                .into_any_element(),
        ]
    } else {
        state
            .model
            .result_rows()
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let invoice_amount = format_yen(row.invoice_amount);
                let invoice_amount_to_copy = integer_string(row.invoice_amount);
                let received_amount = format_yen(row.received_amount);
                h_flex()
                    .w_full()
                    .border_b_1()
                    .border_color(cx.theme().table_row_border)
                    .when(index % 2 == 1, |this| this.bg(cx.theme().table_even))
                    .child(render_result_cell(
                        row.site_name.clone(),
                        Some(168.0),
                        false,
                        None,
                    ))
                    .child(render_result_cell(
                        format!("{}%", trim_trailing_zero(row.fee_percent)),
                        Some(88.0),
                        true,
                        None,
                    ))
                    .child(render_result_cell(
                        format_yen(row.fee_amount),
                        Some(160.0),
                        true,
                        None,
                    ))
                    .child(render_copyable_result_cell(
                        ("copy-invoice-amount", index),
                        invoice_amount,
                        invoice_amount_to_copy,
                        format!("{} の請求額をコピーしました", row.site_name),
                        Some(136.0),
                    ))
                    .child(render_result_cell(
                        received_amount,
                        Some(164.0),
                        true,
                        Some(verification_text_color()),
                    ))
                    .into_any_element()
            })
            .collect()
    };

    v_flex()
        .w_full()
        .min_w(px(716.0))
        .rounded_lg()
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().table)
        .child(
            h_flex()
                .bg(cx.theme().table_head)
                .border_b_1()
                .border_color(cx.theme().table_row_border)
                .child(render_result_head(
                    "依頼サイト",
                    Some(168.0),
                    false,
                    None,
                    cx,
                ))
                .child(render_result_head("手数料率", Some(88.0), true, None, cx))
                .child(render_result_head(
                    "差し引かれる手数料",
                    Some(160.0),
                    true,
                    None,
                    cx,
                ))
                .child(render_result_head("請求額", Some(136.0), true, None, cx))
                .child(render_result_head(
                    "受け取る金額",
                    Some(164.0),
                    true,
                    Some(verification_text_color()),
                    cx,
                )),
        )
        .child(v_flex().children(body_rows))
        .child(
            div()
                .px_3()
                .py_2()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .border_t_1()
                .border_color(cx.theme().table_row_border)
                .child(format!(
                    "{} {}",
                    state.model.result_rows().len(),
                    "件の結果"
                )),
        )
        .into_any_element()
}

fn render_result_head(
    label: &str,
    width: Option<f32>,
    text_right: bool,
    text_color: Option<Hsla>,
    cx: &mut Context<AppState>,
) -> Div {
    let head = div()
        .px_3()
        .py_2()
        .text_sm()
        .font_semibold()
        .text_color(text_color.unwrap_or(cx.theme().table_head_foreground))
        .child(label.to_string());
    let head = match width {
        Some(width) => head.w(px(width)).flex_none(),
        None => head.flex_1(),
    };

    if text_right { head.text_right() } else { head }
}

fn render_result_cell(
    value: String,
    width: Option<f32>,
    text_right: bool,
    text_color: Option<Hsla>,
) -> Div {
    let cell = div().px_3().py_3().text_sm().child(value);
    let cell = match text_color {
        Some(text_color) => cell.text_color(text_color),
        None => cell,
    };
    let cell = match width {
        Some(width) => cell.w(px(width)).flex_none(),
        None => cell.flex_1(),
    };

    if text_right { cell.text_right() } else { cell }
}

fn render_copyable_result_cell(
    clipboard_id: impl Into<ElementId>,
    displayed_value: String,
    copied_value: String,
    notification: String,
    width: Option<f32>,
) -> AnyElement {
    let cell =
        h_flex()
            .px_3()
            .py_3()
            .gap_2()
            .items_center()
            .justify_end()
            .text_sm()
            .child(div().text_right().child(displayed_value))
            .child(Clipboard::new(clipboard_id).value(copied_value).on_copied(
                move |_, window, cx| {
                    window.push_notification(notification.clone(), cx);
                },
            ));
    let cell = match width {
        Some(width) => cell.w(px(width)).flex_none(),
        None => cell.flex_1(),
    };

    cell.into_any_element()
}

fn verification_text_color() -> Hsla {
    green()
}
