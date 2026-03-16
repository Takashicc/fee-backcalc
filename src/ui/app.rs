use anyhow::Result;
use fee_backcalc::{
    APP_ID, APP_TITLE, AppModel, ModelUpdate, apply_theme_by_name, load_config,
    open_config_directory, save_config,
};
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, Root, StyledExt as _, ThemeRegistry, WindowExt as _,
    button::{Button, ButtonVariant, ButtonVariants as _},
    dialog::DialogButtonProps,
    input::InputState,
    scroll::ScrollableElement as _,
    select::SelectState,
    v_flex,
};

use super::{
    bindings,
    components::{
        form_panel::render_left_panel, results_panel::render_right_panel,
        settings_panel::render_settings_panel, shared::render_error,
    },
    select_item::EnumSelectItem,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppScreen {
    Main,
    Settings,
}

pub(crate) struct AppState {
    pub(crate) base_amount_input: Entity<InputState>,
    pub(crate) tax_rate_input: Entity<InputState>,
    pub(crate) site_name_input: Entity<InputState>,
    pub(crate) site_fee_input: Entity<InputState>,
    pub(crate) input_tax_mode_select:
        Entity<SelectState<Vec<EnumSelectItem<fee_backcalc::InputTaxMode>>>>,
    pub(crate) rounding_mode_select:
        Entity<SelectState<Vec<EnumSelectItem<fee_backcalc::RoundingMode>>>>,
    pub(crate) theme_select: Entity<SelectState<Vec<EnumSelectItem<String>>>>,
    pub(crate) model: AppModel,
    screen: AppScreen,
    pub(crate) save_error: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl AppState {
    const COMPACT_LAYOUT_BREAKPOINT: f32 = 1100.0;
    const WINDOW_MIN_WIDTH: f32 = 820.0;
    const WINDOW_MIN_HEIGHT: f32 = 680.0;

    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let model = AppModel::from_config(load_config());
        let base_amount_input = bindings::create_base_amount_input(&model, window, cx);
        let tax_rate_input = bindings::create_tax_rate_input(&model, window, cx);
        let site_name_input = bindings::create_site_name_input(window, cx);
        let site_fee_input = bindings::create_site_fee_input(window, cx);
        let input_tax_mode_select = bindings::create_input_tax_mode_select(&model, window, cx);
        let rounding_mode_select = bindings::create_rounding_mode_select(&model, window, cx);
        let theme_select = bindings::create_theme_select(&model, window, cx);

        let mut state = Self {
            base_amount_input,
            tax_rate_input,
            site_name_input,
            site_fee_input,
            input_tax_mode_select,
            rounding_mode_select,
            theme_select,
            model,
            screen: AppScreen::Main,
            save_error: None,
            _subscriptions: Vec::new(),
        };
        state._subscriptions = bindings::build_subscriptions(&state, window, cx);
        state._subscriptions.push(cx.observe_global_in::<ThemeRegistry>(
            window,
            |this, window, cx| {
                this.sync_theme_select(window, cx);
            },
        ));
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
        self.confirm_remove_site(index, window, cx);
    }

    fn confirm_remove_site(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(site) = self.model.sites().get(index) else {
            return;
        };

        let site_name = site.name.clone();
        let view = cx.entity();

        window.open_dialog(cx, move |alert, _, _| {
            let view = view.clone();
            alert
                .confirm()
                .title(format!("サイト「{}」を削除しますか？", site_name))
                .child(
                    v_flex()
                        .gap_2()
                        .child(div().text_xs().child("この操作は取り消せません。")),
                )
                .button_props(
                    DialogButtonProps::default()
                        .ok_text("削除")
                        .cancel_text("キャンセル")
                        .ok_variant(ButtonVariant::Danger),
                )
                .on_ok({
                    let site_name = site_name.clone();
                    move |_, window, cx| {
                        view.update(cx, |this, cx| {
                            let update = this.model.remove_site(index);
                            window.push_notification(
                                format!("サイト「{}」を削除しました", site_name),
                                cx,
                            );
                            this.apply_site_form_update(update, window, cx);
                        });
                        true
                    }
                })
        });
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

    pub(crate) fn on_open_config_directory(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match open_config_directory() {
            Ok(_) => {
                self.save_error = None;
                window.push_notification("設定フォルダを開きました", cx);
            }
            Err(err) => {
                self.save_error = Some(format!("設定フォルダを開けませんでした: {err}"));
            }
        }
        cx.notify();
    }

    pub(crate) fn on_show_settings(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.screen = AppScreen::Settings;
        cx.notify();
    }

    pub(crate) fn on_show_main(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.screen = AppScreen::Main;
        cx.notify();
    }

    pub(crate) fn on_select_theme(&mut self, theme_name: &str, cx: &mut Context<Self>) {
        if !apply_theme_by_name(theme_name, cx) {
            self.save_error = Some(format!("テーマを適用できませんでした: {theme_name}"));
            cx.notify();
            return;
        }

        self.save_error = None;
        let update = self.model.update_theme_name(theme_name.to_string());
        self.apply_update(update, cx);
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

    fn sync_theme_select(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let current_theme = self
            .model
            .theme_name()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| cx.theme().theme_name().to_string());
        let items = bindings::theme_options(cx);
        self.theme_select.update(cx, |select, cx| {
            select.set_items(items, window, cx);
            select.set_selected_value(&current_theme, window, cx);
        });
    }

    fn header_copy(&self) -> (&'static str, &'static str) {
        match self.screen {
            AppScreen::Main => (
                APP_TITLE,
                "受け取りたい金額から逆算して、サイト手数料を差し引かれても希望額が残る請求金額を求めます。消費税もあわせて確認できます。",
            ),
            AppScreen::Settings => ("設定", "アプリテーマや設定ファイルの保存先を確認できます。"),
        }
    }
}

impl Render for AppState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let base_error = self.model.base_amount_error();
        let tax_error = self.model.tax_rate_error();
        let site_form_error = self.model.site_form_error();
        let compact = Self::is_compact_layout(window);
        let notification_layer = Root::render_notification_layer(window, cx);
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let (header_title, header_description) = self.header_copy();
        let content = match self.screen {
            AppScreen::Main => div()
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
                .child(render_right_panel(self, compact, cx))
                .into_any_element(),
            AppScreen::Settings => render_settings_panel(self, compact, cx),
        };

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
                            div()
                                .flex()
                                .items_center()
                                .gap_4()
                                .justify_between()
                                .child(
                                    v_flex()
                                        .gap_2()
                                        .flex_1()
                                        .min_w_0()
                                        .child(
                                            div()
                                                .text_2xl()
                                                .font_semibold()
                                                .whitespace_nowrap()
                                                .child(header_title),
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(header_description),
                                        ),
                                )
                                .child(match self.screen {
                                    AppScreen::Main => Button::new("open-settings")
                                        .label("設定")
                                        .info()
                                        .on_click(cx.listener(AppState::on_show_settings))
                                        .into_any_element(),
                                    AppScreen::Settings => Button::new("back-to-main")
                                        .label("戻る")
                                        .primary()
                                        .on_click(cx.listener(AppState::on_show_main))
                                        .into_any_element(),
                                }),
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
                        .child(content),
                ),
            )
            .children(dialog_layer)
            .children(notification_layer)
    }
}

pub(crate) fn open_main_window(cx: &mut AsyncApp) -> Result<()> {
    cx.open_window(
        WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some(APP_TITLE.into()),
                ..Default::default()
            }),
            app_id: Some(APP_ID.to_string()),
            window_min_size: Some(size(
                px(AppState::WINDOW_MIN_WIDTH),
                px(AppState::WINDOW_MIN_HEIGHT),
            )),
            ..Default::default()
        },
        |window, cx| {
            let view = cx.new(|cx| AppState::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        },
    )?;

    Ok(())
}
