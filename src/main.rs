use anyhow::Result;
use fee_tax_calculator::{
    AppConfig, CalculatedRow, InputTaxMode, RoundingMode, SiteFee, calculate_row, format_yen, jp,
    load_config, parse_non_negative_number, save_config, trim_trailing_zero, validate_fee_percent,
};
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, Root, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    table::{Column, Table, TableDelegate, TableState},
    v_flex,
};

struct ResultTableDelegate {
    columns: Vec<Column>,
    rows: Vec<CalculatedRow>,
}

impl ResultTableDelegate {
    fn new() -> Self {
        Self {
            columns: vec![
                Column::new("site_name", jp("サイト名")).width(px(170.0)),
                Column::new("fee_percent", jp("手数料率"))
                    .width(px(90.0))
                    .text_right(),
                Column::new("fee_amount", jp("手数料額"))
                    .width(px(110.0))
                    .text_right(),
                Column::new("tax_amount", jp("消費税額"))
                    .width(px(110.0))
                    .text_right(),
                Column::new("inclusive_total", jp("税込合計"))
                    .width(px(120.0))
                    .text_right(),
            ],
            rows: Vec::new(),
        }
    }

    fn set_rows(&mut self, rows: Vec<CalculatedRow>) {
        self.rows = rows;
    }
}

impl TableDelegate for ResultTableDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> &Column {
        &self.columns[col_ix]
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let Some(row) = self.rows.get(row_ix) else {
            return div();
        };

        let value = match col_ix {
            0 => row.site_name.clone(),
            1 => format!("{}%", trim_trailing_zero(row.fee_percent)),
            2 => format_yen(row.fee_amount),
            3 => format_yen(row.tax_amount),
            4 => format_yen(row.inclusive_total),
            _ => String::new(),
        };

        div()
            .size_full()
            .when(col_ix != 0, |this| this.text_right())
            .child(value)
    }

    fn render_empty(
        &mut self,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        h_flex()
            .size_full()
            .justify_center()
            .items_center()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child(jp("表示できる計算結果がありません。"))
    }
}

struct AppState {
    base_amount_input: Entity<InputState>,
    tax_rate_input: Entity<InputState>,
    site_name_input: Entity<InputState>,
    site_fee_input: Entity<InputState>,
    result_table: Entity<TableState<ResultTableDelegate>>,
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
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let config = load_config();

        let base_amount_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("ex: 1000")
                .default_value(config.base_amount.clone())
        });
        let tax_rate_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("ex: 10")
                .default_value(config.tax_rate.clone())
        });
        let site_name_input = cx.new(|cx| InputState::new(window, cx).placeholder("site name"));
        let site_fee_input = cx.new(|cx| InputState::new(window, cx).placeholder("fee percent"));
        let result_table = cx.new(|cx| {
            TableState::new(ResultTableDelegate::new(), window, cx)
                .col_movable(false)
                .col_resizable(false)
                .col_selectable(false)
                .row_selectable(false)
                .sortable(false)
        });

        let subscriptions = vec![
            cx.subscribe(
                &base_amount_input,
                |this: &mut Self, input, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.base_amount = input.read(cx).value().to_string();
                        this.recalculate_results(cx);
                        cx.notify();
                    }
                },
            ),
            cx.subscribe(
                &tax_rate_input,
                |this: &mut Self, input, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.tax_rate = input.read(cx).value().to_string();
                        this.recalculate_results(cx);
                        this.persist();
                        cx.notify();
                    }
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
                        this.site_fee = input.read(cx).value().to_string();
                        cx.notify();
                    }
                },
            ),
        ];

        Self {
            base_amount_input,
            tax_rate_input,
            site_name_input,
            site_fee_input,
            result_table,
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

    fn set_input_tax_mode(
        &mut self,
        mode: InputTaxMode,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.input_tax_mode = mode;
        self.recalculate_results(cx);
        self.persist();
        cx.notify();
    }

    fn set_rounding_mode(
        &mut self,
        mode: RoundingMode,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
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
            parse_non_negative_number("ベース金額", self.base_amount.trim()).err()
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
            Some(jp("サイト名を入力してください"))
        } else {
            validate_fee_percent(site_fee).err()
        }
    }

    fn calculated_rows(&self) -> Result<Vec<CalculatedRow>, String> {
        if self.base_amount.trim().is_empty() || self.tax_rate.trim().is_empty() {
            return Ok(Vec::new());
        }

        let base_amount = parse_non_negative_number("ベース金額", self.base_amount.trim())?;
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
                self.result_table.update(cx, |table, cx| {
                    table.delegate_mut().set_rows(rows);
                    table.refresh(cx);
                });
            }
            Err(error) => {
                self.result_error = Some(error);
                self.result_table.update(cx, |table, cx| {
                    table.delegate_mut().set_rows(Vec::new());
                    table.refresh(cx);
                });
            }
        }
    }

    fn with_recalculated_results(mut self, cx: &mut Context<Self>) -> Self {
        self.recalculate_results(cx);
        self
    }

    fn render_mode_button(
        &self,
        id: &'static str,
        label: &'static str,
        selected: bool,
        on_click: impl Fn(&mut Self, &ClickEvent, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> Button {
        let button = Button::new(id).label(label);
        if selected {
            button.primary().on_click(cx.listener(on_click))
        } else {
            button.ghost().on_click(cx.listener(on_click))
        }
    }

    fn render_error(message: String, cx: &mut Context<Self>) -> AnyElement {
        div()
            .text_sm()
            .text_color(cx.theme().danger)
            .child(message)
            .into_any_element()
    }

    fn render_site_row(&self, index: usize, site: &SiteFee, cx: &mut Context<Self>) -> AnyElement {
        let editing = self.editing_index == Some(index);
        v_flex()
            .gap_2()
            .p_3()
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
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
                        h_flex()
                            .gap_2()
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
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let site_rows: Vec<AnyElement> = self
            .sites
            .iter()
            .enumerate()
            .map(|(index, site)| self.render_site_row(index, site, cx))
            .collect();

        v_flex()
            .gap_4()
            .flex_1()
            .child(
                v_flex()
                    .gap_3()
                    .p_4()
                    .rounded_xl()
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(div().font_semibold().child(jp("入力")))
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_sm().child(jp("ベース金額")))
                            .child(Input::new(&self.base_amount_input).cleanable(true).w_full())
                            .when_some(base_error, |this, message| {
                                this.child(Self::render_error(message, cx))
                            }),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_sm().child(jp("入力金額の扱い")))
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(self.render_mode_button(
                                        "mode-exclusive",
                                        InputTaxMode::TaxExclusive.label(),
                                        self.input_tax_mode == InputTaxMode::TaxExclusive,
                                        |this, event, window, cx| {
                                            this.set_input_tax_mode(
                                                InputTaxMode::TaxExclusive,
                                                event,
                                                window,
                                                cx,
                                            )
                                        },
                                        cx,
                                    ))
                                    .child(self.render_mode_button(
                                        "mode-inclusive",
                                        InputTaxMode::TaxInclusive.label(),
                                        self.input_tax_mode == InputTaxMode::TaxInclusive,
                                        |this, event, window, cx| {
                                            this.set_input_tax_mode(
                                                InputTaxMode::TaxInclusive,
                                                event,
                                                window,
                                                cx,
                                            )
                                        },
                                        cx,
                                    )),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_sm().child(jp("消費税率 (%)")))
                            .child(Input::new(&self.tax_rate_input).w_full())
                            .when_some(tax_error, |this, message| {
                                this.child(Self::render_error(message, cx))
                            }),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_sm().child(jp("端数処理")))
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(self.render_mode_button(
                                        "rounding-round",
                                        RoundingMode::Round.label(),
                                        self.rounding_mode == RoundingMode::Round,
                                        |this, event, window, cx| {
                                            this.set_rounding_mode(
                                                RoundingMode::Round,
                                                event,
                                                window,
                                                cx,
                                            )
                                        },
                                        cx,
                                    ))
                                    .child(self.render_mode_button(
                                        "rounding-ceil",
                                        RoundingMode::Ceil.label(),
                                        self.rounding_mode == RoundingMode::Ceil,
                                        |this, event, window, cx| {
                                            this.set_rounding_mode(
                                                RoundingMode::Ceil,
                                                event,
                                                window,
                                                cx,
                                            )
                                        },
                                        cx,
                                    ))
                                    .child(self.render_mode_button(
                                        "rounding-floor",
                                        RoundingMode::Floor.label(),
                                        self.rounding_mode == RoundingMode::Floor,
                                        |this, event, window, cx| {
                                            this.set_rounding_mode(
                                                RoundingMode::Floor,
                                                event,
                                                window,
                                                cx,
                                            )
                                        },
                                        cx,
                                    )),
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
                            .child(div().font_semibold().child(jp("サイト管理")))
                            .when_some(
                                self.editing_index.and_then(|index| self.sites.get(index)),
                                |this, site| {
                                    this.child(
                                        div().text_sm().text_color(cx.theme().primary).child(
                                            format!("{}", jp(&format!("{}を編集中", site.name))),
                                        ),
                                    )
                                },
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_sm().child(jp("サイト名")))
                            .child(Input::new(&self.site_name_input).cleanable(true).w_full()),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_sm().child(jp("手数料率 (%)")))
                            .child(Input::new(&self.site_fee_input).cleanable(true).w_full())
                            .when_some(site_form_error, |this, message| {
                                this.child(Self::render_error(message, cx))
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("save-site")
                                    .label(if self.editing_index.is_some() {
                                        jp("サイトを更新")
                                    } else {
                                        jp("サイトを追加")
                                    })
                                    .primary()
                                    .on_click(cx.listener(Self::add_or_update_site)),
                            )
                            .when(self.editing_index.is_some(), |this| {
                                this.child(
                                    Button::new("cancel-edit")
                                        .label(jp("編集をやめる"))
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
                                .child(jp("まだサイトは登録されていません。")),
                        )
                    })
                    .children(site_rows),
            )
            .into_any_element()
    }

    fn render_right_panel(
        &self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        v_flex()
            .gap_3()
            .flex_1()
            .p_4()
            .rounded_xl()
            .border_1()
            .border_color(cx.theme().border)
            .child(div().font_semibold().child(jp("計算結果")))
            .when(self.sites.is_empty(), |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(jp("サイトを登録すると、ここに結果が表示されます。")),
                )
            })
            .when_some(self.result_error.clone(), |this, message| {
                this.child(Self::render_error(message, cx))
            })
            .child(
                div()
                    .h(px(320.0))
                    .child(Table::new(&self.result_table).stripe(true).bordered(true)),
            )
            .into_any_element()
    }
}

impl Render for AppState {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let base_error = self.base_amount_error();
        let tax_error = self.tax_rate_error();
        let site_form_error = self.site_form_error();

        div()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .p_6()
            .child(
                v_flex()
                    .gap_6()
                    .size_full()
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_2xl().font_semibold().child(jp("手数料・消費税計算")))
                .child(div().text_sm().text_color(cx.theme().muted_foreground).child(jp("ベース金額から、合計額基準の手数料額と消費税額、税込み金額をサイト別に一覧表示します。"))),
                    )
                    .when_some(self.save_error.clone(), |this, message| {
                        this.child(div().p_3().rounded_lg().bg(cx.theme().danger.opacity(0.1)).child(Self::render_error(message, cx)))
                    })
                    .child(
                        h_flex()
                            .items_start()
                            .gap_6()
                            .w_full()
                            .child(self.render_left_panel(base_error, tax_error, site_form_error, cx))
                            .child(self.render_right_panel(cx)),
                    ),
            )
    }
}

fn main() {
    let app = Application::new();

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
