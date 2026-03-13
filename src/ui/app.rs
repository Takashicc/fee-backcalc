use anyhow::Result;
use fee_backcalc::{AppModel, ModelUpdate, jp, load_config, save_config};
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, Root, StyledExt as _, input::InputState, scroll::ScrollableElement as _,
    select::SelectState, v_flex,
};

use super::{
    bindings,
    components::{
        form_panel::render_left_panel, results_panel::render_right_panel, shared::render_error,
    },
    select_item::EnumSelectItem,
};

pub(crate) struct AppState {
    pub(crate) base_amount_input: Entity<InputState>,
    pub(crate) tax_rate_input: Entity<InputState>,
    pub(crate) site_name_input: Entity<InputState>,
    pub(crate) site_fee_input: Entity<InputState>,
    pub(crate) input_tax_mode_select:
        Entity<SelectState<Vec<EnumSelectItem<fee_backcalc::InputTaxMode>>>>,
    pub(crate) rounding_mode_select:
        Entity<SelectState<Vec<EnumSelectItem<fee_backcalc::RoundingMode>>>>,
    pub(crate) model: AppModel,
    pub(crate) save_error: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl AppState {
    const COMPACT_LAYOUT_BREAKPOINT: f32 = 1100.0;

    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let model = AppModel::from_config(load_config());
        let base_amount_input = bindings::create_base_amount_input(&model, window, cx);
        let tax_rate_input = bindings::create_tax_rate_input(&model, window, cx);
        let site_name_input = bindings::create_site_name_input(window, cx);
        let site_fee_input = bindings::create_site_fee_input(window, cx);
        let input_tax_mode_select = bindings::create_input_tax_mode_select(&model, window, cx);
        let rounding_mode_select = bindings::create_rounding_mode_select(&model, window, cx);

        let mut state = Self {
            base_amount_input,
            tax_rate_input,
            site_name_input,
            site_fee_input,
            input_tax_mode_select,
            rounding_mode_select,
            model,
            save_error: None,
            _subscriptions: Vec::new(),
        };
        state._subscriptions = bindings::build_subscriptions(&state, window, cx);
        state
    }

    pub(crate) fn on_add_or_update_site(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let update = self.model.add_or_update_site();
        self.apply_site_form_update(update, window, cx);
    }

    pub(crate) fn on_start_edit_site(
        &mut self,
        index: usize,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let update = self.model.start_edit_site(index);
        self.apply_site_form_update(update, window, cx);
    }

    pub(crate) fn on_remove_site(
        &mut self,
        index: usize,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let update = self.model.remove_site(index);
        self.apply_site_form_update(update, window, cx);
    }

    pub(crate) fn on_cancel_edit(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let update = self.model.cancel_edit();
        self.apply_site_form_update(update, window, cx);
    }

    pub(crate) fn apply_update(&mut self, update: ModelUpdate, cx: &mut Context<Self>) {
        if update.should_persist {
            self.persist();
        }
        if update.should_notify {
            cx.notify();
        }
    }

    fn apply_site_form_update(
        &mut self,
        update: ModelUpdate,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if update.should_sync_site_form {
            self.sync_site_form_inputs(window, cx);
        }
        self.apply_update(update, cx);
    }

    fn persist(&mut self) {
        self.save_error = save_config(&self.model.to_config())
            .err()
            .map(|err| format!("設定の保存に失敗しました: {err}"));
    }

    fn sync_site_form_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let site_name = self.model.site_name().to_string();
        let site_fee = self.model.site_fee().to_string();
        self.site_name_input.update(cx, |input, cx| {
            input.set_value(site_name.clone(), window, cx);
        });
        self.site_fee_input.update(cx, |input, cx| {
            input.set_value(site_fee.clone(), window, cx);
        });
    }

    fn is_compact_layout(window: &Window) -> bool {
        window.viewport_size().width <= px(Self::COMPACT_LAYOUT_BREAKPOINT)
    }
}

impl Render for AppState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let base_error = self.model.base_amount_error();
        let tax_error = self.model.tax_rate_error();
        let site_form_error = self.model.site_form_error();
        let compact = Self::is_compact_layout(window);
        let notification_layer = Root::render_notification_layer(window, cx);

        div()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                div().size_full().overflow_y_scrollbar().child(
                    v_flex()
                        .gap_6()
                        .w_full()
                        .when(compact, |this| this.p_4())
                        .when(!compact, |this| this.p_6())
                        .child(
                            v_flex()
                                .gap_2()
                                .child(div().text_2xl().font_semibold().child(jp("Fee Backcalc")))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(jp("受け取りたい金額から逆算して、サイト手数料を差し引かれても希望額が残る請求金額を求めます。消費税もあわせて確認できます。")),
                                ),
                        )
                        .when_some(self.save_error.clone(), |this, message| {
                            this.child(
                                div()
                                    .p_3()
                                    .rounded_lg()
                                    .bg(cx.theme().danger.opacity(0.1))
                                    .child(render_error(message, cx)),
                            )
                        })
                        .child(
                            div()
                                .flex()
                                .items_start()
                                .gap_6()
                                .w_full()
                                .when(compact, |this| this.flex_col())
                                .when(!compact, |this| this.flex_row())
                                .child(render_left_panel(
                                    self,
                                    base_error,
                                    tax_error,
                                    site_form_error,
                                    compact,
                                    cx,
                                ))
                                .child(render_right_panel(self, compact, cx)),
                        ),
                ),
            )
            .children(notification_layer)
    }
}

pub(crate) fn open_main_window(cx: &mut AsyncApp) -> Result<()> {
    cx.open_window(WindowOptions::default(), |window, cx| {
        let view = cx.new(|cx| AppState::new(window, cx));
        cx.new(|cx| Root::new(view, window, cx))
    })?;

    Ok(())
}
