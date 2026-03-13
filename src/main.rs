use anyhow::Result;
use fee_backcalc::{
    AppConfig, CalculatedRow, InputTaxMode, RoundingMode, SiteFee, calculate_row, format_yen, jp,
    load_config, parse_non_negative_number, save_config, trim_trailing_zero, validate_fee_percent,
};
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, IconName, IndexPath, Root, StyledExt as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
    clipboard::Clipboard,
    h_flex,
    input::{
        Input, InputEvent, InputState, MaskPattern, NumberInput, NumberInputEvent, StepAction,
    },
    scroll::ScrollableElement as _,
    select::{Select, SelectEvent, SelectItem, SelectState},
    v_flex,
};
use gpui_component_assets::Assets;

#[derive(Clone)]
struct EnumSelectItem<T: Clone> {
    label: SharedString,
    value: T,
}

impl<T: Clone> EnumSelectItem<T> {
    fn new(label: impl Into<SharedString>, value: T) -> Self {
        Self {
            label: label.into(),
            value,
        }
    }
}

impl<T: Clone + 'static> SelectItem for EnumSelectItem<T> {
    type Value = T;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }
}

struct AppState {
    base_amount_input: Entity<InputState>,
    tax_rate_input: Entity<InputState>,
    site_name_input: Entity<InputState>,
    site_fee_input: Entity<InputState>,
    input_tax_mode_select: Entity<SelectState<Vec<EnumSelectItem<InputTaxMode>>>>,
    rounding_mode_select: Entity<SelectState<Vec<EnumSelectItem<RoundingMode>>>>,
    result_rows: Vec<CalculatedRow>,
    base_amount: String,
    tax_rate: String,
    site_name: String,
    site_fee: String,
    input_tax_mode: InputTaxMode,
    rounding_mode: RoundingMode,
    sites: Vec<SiteFee>,
    editing_index: Option<usize>,
    result_error: Option<String>,
    save_error: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl AppState {
    const COMPACT_LAYOUT_BREAKPOINT: f32 = 1100.0;
    const BASE_AMOUNT_STEP: f64 = 1000.0;
    const RATE_STEP: f64 = 0.1;
    const SITE_FEE_MAX: f64 = 99.9;

    fn mode_options() -> Vec<EnumSelectItem<InputTaxMode>> {
        vec![
            EnumSelectItem::new(
                InputTaxMode::TaxExclusive.label(),
                InputTaxMode::TaxExclusive,
            ),
            EnumSelectItem::new(
                InputTaxMode::TaxInclusive.label(),
                InputTaxMode::TaxInclusive,
            ),
        ]
    }

    fn rounding_options() -> Vec<EnumSelectItem<RoundingMode>> {
        vec![
            EnumSelectItem::new(RoundingMode::Round.label(), RoundingMode::Round),
            EnumSelectItem::new(RoundingMode::Ceil.label(), RoundingMode::Ceil),
            EnumSelectItem::new(RoundingMode::Floor.label(), RoundingMode::Floor),
        ]
    }

    fn selected_index<T: PartialEq + Clone + 'static>(
        items: &[EnumSelectItem<T>],
        selected: &T,
    ) -> Option<IndexPath> {
        items
            .iter()
            .position(|item| item.value() == selected)
            .map(IndexPath::new)
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let config = load_config();
        let input_tax_mode_options = Self::mode_options();
        let rounding_mode_options = Self::rounding_options();

        let base_amount_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("例: 100000")
                .mask_pattern(MaskPattern::Number {
                    separator: Some(','),
                    fraction: None,
                })
                .default_value(config.base_amount.clone())
        });
        let tax_rate_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("例: 10")
                .default_value(config.tax_rate.clone())
        });
        let site_name_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("例: CrowdWorks"));
        let site_fee_input = cx.new(|cx| InputState::new(window, cx).placeholder("例: 20"));

        base_amount_input.update(cx, |input, cx| {
            input.set_value(config.base_amount.clone(), window, cx);
        });

        let input_tax_mode_select = cx.new(|cx| {
            SelectState::new(
                input_tax_mode_options,
                Self::selected_index(&Self::mode_options(), &config.input_tax_mode),
                window,
                cx,
            )
        });
        let rounding_mode_select = cx.new(|cx| {
            SelectState::new(
                rounding_mode_options,
                Self::selected_index(&Self::rounding_options(), &config.rounding_mode),
                window,
                cx,
            )
        });

        let subscriptions = vec![
            cx.subscribe(
                &base_amount_input,
                |this: &mut Self, input, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.base_amount = Self::number_input_value(&input, cx);
                        this.recalculate_results(cx);
                        cx.notify();
                    }
                },
            ),
            cx.subscribe_in(
                &base_amount_input,
                window,
                |_: &mut Self, input, event: &NumberInputEvent, window, cx| {
                    let NumberInputEvent::Step(action) = event;
                    Self::step_number_input(
                        input,
                        *action,
                        Self::BASE_AMOUNT_STEP,
                        0.0,
                        None,
                        Self::format_decimal_number,
                        window,
                        cx,
                    );
                },
            ),
            cx.subscribe(
                &tax_rate_input,
                |this: &mut Self, input, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.tax_rate = Self::number_input_value(&input, cx);
                        this.recalculate_results(cx);
                        this.persist();
                        cx.notify();
                    }
                },
            ),
            cx.subscribe_in(
                &tax_rate_input,
                window,
                |_: &mut Self, input, event: &NumberInputEvent, window, cx| {
                    let NumberInputEvent::Step(action) = event;
                    Self::step_number_input(
                        input,
                        *action,
                        Self::RATE_STEP,
                        0.0,
                        None,
                        Self::format_rate_number,
                        window,
                        cx,
                    );
                },
            ),
            cx.subscribe(
                &input_tax_mode_select,
                |this: &mut Self, _, event: &SelectEvent<Vec<EnumSelectItem<InputTaxMode>>>, cx| {
                    let SelectEvent::Confirm(Some(mode)) = event else {
                        return;
                    };
                    this.update_input_tax_mode(*mode, cx);
                },
            ),
            cx.subscribe(
                &rounding_mode_select,
                |this: &mut Self, _, event: &SelectEvent<Vec<EnumSelectItem<RoundingMode>>>, cx| {
                    let SelectEvent::Confirm(Some(mode)) = event else {
                        return;
                    };
                    this.update_rounding_mode(*mode, cx);
                },
            ),
            cx.subscribe(
                &site_name_input,
                |this: &mut Self, input, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.site_name = input.read(cx).value().to_string();
                        cx.notify();
                    }
                },
            ),
            cx.subscribe(
                &site_fee_input,
                |this: &mut Self, input, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.site_fee = Self::number_input_value(&input, cx);
                        cx.notify();
                    }
                },
            ),
            cx.subscribe_in(
                &site_fee_input,
                window,
                |_: &mut Self, input, event: &NumberInputEvent, window, cx| {
                    let NumberInputEvent::Step(action) = event;
                    Self::step_number_input(
                        input,
                        *action,
                        Self::RATE_STEP,
                        0.0,
                        Some(Self::SITE_FEE_MAX),
                        Self::format_rate_number,
                        window,
                        cx,
                    );
                },
            ),
        ];

        Self {
            base_amount_input,
            tax_rate_input,
            site_name_input,
            site_fee_input,
            input_tax_mode_select,
            rounding_mode_select,
            result_rows: Vec::new(),
            base_amount: config.base_amount,
            tax_rate: config.tax_rate,
            site_name: String::new(),
            site_fee: String::new(),
            input_tax_mode: config.input_tax_mode,
            rounding_mode: config.rounding_mode,
            sites: config.sites,
            editing_index: None,
            result_error: None,
            save_error: None,
            _subscriptions: subscriptions,
        }
        .with_recalculated_results(cx)
    }

    fn config(&self) -> AppConfig {
        AppConfig {
            base_amount: self.base_amount.clone(),
            input_tax_mode: self.input_tax_mode,
            tax_rate: self.tax_rate.clone(),
            rounding_mode: self.rounding_mode,
            sites: self.sites.clone(),
        }
    }

    fn persist(&mut self) {
        self.save_error = save_config(&self.config())
            .err()
            .map(|err| format!("設定の保存に失敗しました: {err}"));
    }

    fn number_input_value(input: &Entity<InputState>, cx: &App) -> String {
        input.read(cx).unmask_value().to_string()
    }

    fn round_to_tenths(value: f64) -> f64 {
        (value * 10.0).round() / 10.0
    }

    fn format_decimal_number(value: f64) -> String {
        trim_trailing_zero(value)
    }

    fn format_rate_number(value: f64) -> String {
        trim_trailing_zero(Self::round_to_tenths(value))
    }

    fn step_number_input(
        input: &Entity<InputState>,
        action: StepAction,
        step: f64,
        min: f64,
        max: Option<f64>,
        formatter: fn(f64) -> String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let raw_value = Self::number_input_value(input, cx);
        let current_value = raw_value.trim().parse::<f64>().unwrap_or(0.0);
        let delta = match action {
            StepAction::Increment => step,
            StepAction::Decrement => -step,
        };

        let mut next_value = current_value + delta;
        if next_value < min {
            next_value = min;
        }
        if let Some(max) = max {
            if next_value > max {
                next_value = max;
            }
        }

        input.update(cx, |state, cx| {
            state.set_value(formatter(next_value), window, cx);
        });
    }

    fn update_input_tax_mode(&mut self, mode: InputTaxMode, cx: &mut Context<Self>) {
        if self.input_tax_mode == mode {
            return;
        }
        self.input_tax_mode = mode;
        self.recalculate_results(cx);
        self.persist();
        cx.notify();
    }

    fn update_rounding_mode(&mut self, mode: RoundingMode, cx: &mut Context<Self>) {
        if self.rounding_mode == mode {
            return;
        }
        self.rounding_mode = mode;
        self.recalculate_results(cx);
        self.persist();
        cx.notify();
    }

    fn add_or_update_site(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.site_name.trim();
        let fee_text = self.site_fee.trim();
        if name.is_empty() || validate_fee_percent(fee_text).is_err() {
            cx.notify();
            return;
        }

        let site = SiteFee {
            name: name.to_string(),
            fee_percent: validate_fee_percent(fee_text).unwrap_or_default(),
        };

        if let Some(index) = self.editing_index {
            if let Some(target) = self.sites.get_mut(index) {
                *target = site;
            }
        } else {
            self.sites.push(site);
        }

        self.clear_site_form(window, cx);
        self.recalculate_results(cx);
        self.persist();
        cx.notify();
    }

    fn start_edit_site(
        &mut self,
        index: usize,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(site) = self.sites.get(index).cloned() {
            self.editing_index = Some(index);
            self.site_name = site.name.clone();
            self.site_fee = trim_trailing_zero(site.fee_percent);
            let site_name = self.site_name.clone();
            let site_fee = self.site_fee.clone();
            self.site_name_input.update(cx, |input, cx| {
                input.set_value(SharedString::from(site_name.clone()), window, cx);
            });
            self.site_fee_input.update(cx, |input, cx| {
                input.set_value(SharedString::from(site_fee.clone()), window, cx);
            });
            cx.notify();
        }
    }

    fn remove_site(
        &mut self,
        index: usize,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if index < self.sites.len() {
            self.sites.remove(index);
            if self.editing_index == Some(index) {
                self.clear_site_form(window, cx);
            }
            self.recalculate_results(cx);
            self.persist();
            cx.notify();
        }
    }

    fn cancel_edit(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.clear_site_form(window, cx);
        cx.notify();
    }

    fn clear_site_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editing_index = None;
        self.site_name.clear();
        self.site_fee.clear();
        self.site_name_input.update(cx, |input, cx| {
            input.set_value(SharedString::default(), window, cx)
        });
        self.site_fee_input.update(cx, |input, cx| {
            input.set_value(SharedString::default(), window, cx)
        });
    }

    fn base_amount_error(&self) -> Option<String> {
        if self.base_amount.trim().is_empty() {
            None
        } else {
            parse_non_negative_number("受け取りたい金額", self.base_amount.trim()).err()
        }
    }

    fn tax_rate_error(&self) -> Option<String> {
        parse_non_negative_number("消費税率", self.tax_rate.trim()).err()
    }

    fn site_form_error(&self) -> Option<String> {
        let site_name = self.site_name.trim();
        let site_fee = self.site_fee.trim();

        if site_name.is_empty() && site_fee.is_empty() {
            None
        } else if site_name.is_empty() {
            Some(jp("依頼サイト名を入力してください"))
        } else {
            validate_fee_percent(site_fee).err()
        }
    }

    fn calculated_rows(&self) -> Result<Vec<CalculatedRow>, String> {
        if self.base_amount.trim().is_empty() || self.tax_rate.trim().is_empty() {
            return Ok(Vec::new());
        }

        let base_amount = parse_non_negative_number("受け取りたい金額", self.base_amount.trim())?;
        let tax_rate_percent = parse_non_negative_number("消費税率", self.tax_rate.trim())?;
        let tax_rate = tax_rate_percent / 100.0;

        Ok(self
            .sites
            .iter()
            .map(|site| {
                calculate_row(
                    site,
                    base_amount,
                    self.input_tax_mode,
                    tax_rate,
                    self.rounding_mode,
                )
            })
            .collect())
    }

    fn recalculate_results(&mut self, cx: &mut Context<Self>) {
        match self.calculated_rows() {
            Ok(rows) => {
                self.result_error = None;
                self.result_rows = rows;
            }
            Err(error) => {
                self.result_error = Some(error);
                self.result_rows.clear();
            }
        }
        cx.notify();
    }

    fn with_recalculated_results(mut self, cx: &mut Context<Self>) -> Self {
        self.recalculate_results(cx);
        self
    }

    fn render_error(message: String, cx: &mut Context<Self>) -> AnyElement {
        div()
            .text_sm()
            .text_color(cx.theme().danger)
            .child(message)
            .into_any_element()
    }

    fn render_result_head(
        &self,
        label: &str,
        width: Option<f32>,
        text_right: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let head = div()
            .px_3()
            .py_2()
            .text_sm()
            .font_semibold()
            .text_color(cx.theme().table_head_foreground)
            .child(jp(label));
        let head = match width {
            Some(width) => head.w(px(width)).flex_none(),
            None => head.flex_1(),
        };

        if text_right { head.text_right() } else { head }
    }

    fn render_result_cell(&self, value: String, width: Option<f32>, text_right: bool) -> Div {
        let cell = div().px_3().py_3().text_sm().child(value);
        let cell = match width {
            Some(width) => cell.w(px(width)).flex_none(),
            None => cell.flex_1(),
        };

        if text_right { cell.text_right() } else { cell }
    }

    fn render_copyable_result_cell(
        &self,
        clipboard_id: impl Into<ElementId>,
        value: String,
        notification: String,
        width: Option<f32>,
    ) -> AnyElement {
        let displayed_value = value.clone();
        let copied_value = value.clone();

        let cell = h_flex()
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

    fn render_results_table(&self, cx: &mut Context<Self>) -> AnyElement {
        let body_rows: Vec<AnyElement> = if self.result_rows.is_empty() {
            vec![
                h_flex()
                    .w_full()
                    .justify_center()
                    .items_center()
                    .px_3()
                    .py_8()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(jp("表示できる請求額がありません。"))
                    .into_any_element(),
            ]
        } else {
            self.result_rows
                .iter()
                .enumerate()
                .map(|(index, row)| {
                    let inclusive_total = format_yen(row.inclusive_total);
                    h_flex()
                        .w_full()
                        .border_b_1()
                        .border_color(cx.theme().table_row_border)
                        .when(index % 2 == 1, |this| this.bg(cx.theme().table_even))
                        .child(self.render_result_cell(row.site_name.clone(), None, false))
                        .child(self.render_result_cell(
                            format!("{}%", trim_trailing_zero(row.fee_percent)),
                            Some(88.0),
                            true,
                        ))
                        .child(self.render_result_cell(
                            format_yen(row.fee_amount),
                            Some(132.0),
                            true,
                        ))
                        .child(self.render_result_cell(
                            format_yen(row.exclusive_total),
                            Some(120.0),
                            true,
                        ))
                        .child(self.render_result_cell(
                            format_yen(row.tax_amount),
                            Some(108.0),
                            true,
                        ))
                        .child(self.render_copyable_result_cell(
                            ("copy-inclusive-total", index),
                            inclusive_total,
                            format!("{} の税込請求額をコピーしました", row.site_name),
                            Some(128.0),
                        ))
                        .into_any_element()
                })
                .collect()
        };

        v_flex()
            .w_full()
            .min_w(px(680.0))
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().table)
            .child(
                h_flex()
                    .w_full()
                    .bg(cx.theme().table_head)
                    .border_b_1()
                    .border_color(cx.theme().table_row_border)
                    .child(self.render_result_head("依頼サイト", None, false, cx))
                    .child(self.render_result_head("手数料率", Some(88.0), true, cx))
                    .child(self.render_result_head("差し引かれる手数料", Some(132.0), true, cx))
                    .child(self.render_result_head("請求額(税抜)", Some(120.0), true, cx))
                    .child(self.render_result_head("消費税額", Some(108.0), true, cx))
                    .child(self.render_result_head("税込請求額", Some(128.0), true, cx)),
            )
            .child(v_flex().w_full().children(body_rows))
            .child(
                div()
                    .px_3()
                    .py_2()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .border_t_1()
                    .border_color(cx.theme().table_row_border)
                    .child(format!("{} {}", self.result_rows.len(), jp("件の結果"))),
            )
            .into_any_element()
    }

    fn is_compact_layout(window: &Window) -> bool {
        window.viewport_size().width <= px(Self::COMPACT_LAYOUT_BREAKPOINT)
    }

    fn render_site_row(
        &self,
        index: usize,
        site: &SiteFee,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let editing = self.editing_index == Some(index);
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
                                    .ghost()
                                    .on_click(cx.listener(move |this, event, window, cx| {
                                        this.start_edit_site(index, event, window, cx);
                                    })),
                            )
                            .child(
                                Button::new(("delete-site", index))
                                    .label(jp("削除"))
                                    .ghost()
                                    .on_click(cx.listener(move |this, event, window, cx| {
                                        this.remove_site(index, event, window, cx);
                                    })),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_left_panel(
        &self,
        base_error: Option<String>,
        tax_error: Option<String>,
        site_form_error: Option<String>,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let site_rows: Vec<AnyElement> = self
            .sites
            .iter()
            .enumerate()
            .map(|(index, site)| self.render_site_row(index, site, compact, cx))
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
                            .child(NumberInput::new(&self.base_amount_input).w_full())
                            .when_some(base_error, |this, message| {
                                this.child(Self::render_error(message, cx))
                            }),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_sm().child(jp("受け取りたい金額の入力形式")))
                            .child(
                                Select::new(&self.input_tax_mode_select)
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
                                    .child(jp("手数料の逆算に加えて、請求時の消費税もあわせて計算します。")),
                            )
                            .child(NumberInput::new(&self.tax_rate_input).w_full())
                            .when_some(tax_error, |this, message| {
                                this.child(Self::render_error(message, cx))
                            }),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_sm().child(jp("端数処理")))
                            .child(
                                Select::new(&self.rounding_mode_select)
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
                            .when_some(
                                self.editing_index.and_then(|index| self.sites.get(index)),
                                |this, site| {
                                    this.child(
                                        div().text_sm().text_color(cx.theme().primary).child(
                                            format!("{}を編集中", site.name),
                                        ),
                                    )
                                },
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_sm().child(jp("依頼サイト名")))
                            .child(Input::new(&self.site_name_input).cleanable(true).w_full()),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_sm().child(jp("手数料率 (%)")))
                            .child(NumberInput::new(&self.site_fee_input).w_full())
                            .when_some(site_form_error, |this, message| {
                                this.child(Self::render_error(message, cx))
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
                                    .label(if self.editing_index.is_some() {
                                        jp("設定を更新")
                                    } else {
                                        jp("設定を追加")
                                    })
                                    .primary()
                                    .on_click(cx.listener(Self::add_or_update_site)),
                            )
                            .when(self.editing_index.is_some(), |this| {
                                this.child(
                                    Button::new("cancel-edit")
                                        .label(jp("編集をキャンセル"))
                                        .ghost()
                                        .on_click(cx.listener(Self::cancel_edit)),
                                )
                            }),
                    )
                    .when(self.sites.is_empty(), |this| {
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

    fn render_right_panel(&self, compact: bool, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .gap_3()
            .w_full()
            .when(!compact, |this| this.flex_1())
            .p_4()
            .rounded_xl()
            .border_1()
            .border_color(cx.theme().border)
            .child(div().font_semibold().child(jp("逆算結果")))
            .when(self.sites.is_empty(), |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(jp(
                            "手数料設定を追加すると、サイトごとの請求額がここに表示されます。",
                        )),
                )
            })
            .when_some(self.result_error.clone(), |this, message| {
                this.child(Self::render_error(message, cx))
            })
            .child(
                div()
                    .w_full()
                    .overflow_x_scrollbar()
                    .child(self.render_results_table(cx)),
            )
            .into_any_element()
    }
}

impl Render for AppState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let base_error = self.base_amount_error();
        let tax_error = self.tax_rate_error();
        let site_form_error = self.site_form_error();
        let compact = Self::is_compact_layout(window);
        let notification_layer = Root::render_notification_layer(window, cx);

        div()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                div()
                    .size_full()
                    .overflow_y_scrollbar()
                    .child(
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
                                this.child(div().p_3().rounded_lg().bg(cx.theme().danger.opacity(0.1)).child(Self::render_error(message, cx)))
                            })
                            .child(
                                div()
                                    .flex()
                                    .items_start()
                                    .gap_6()
                                    .w_full()
                                    .when(compact, |this| this.flex_col())
                                    .when(!compact, |this| this.flex_row())
                                    .child(self.render_left_panel(
                                        base_error,
                                        tax_error,
                                        site_form_error,
                                        compact,
                                        cx,
                                    ))
                                    .child(self.render_right_panel(compact, cx)),
                            ),
                    ),
            )
            .children(notification_layer)
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(|cx| {
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|cx| AppState::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })?;

            Result::<()>::Ok(())
        })
        .detach();
    });
}
