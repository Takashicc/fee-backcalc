use fee_backcalc::{AppModel, InputTaxMode, RoundingMode, trim_trailing_zero};
use gpui::*;
use gpui_component::{
    IndexPath,
    input::{InputEvent, InputState, MaskPattern, NumberInputEvent, StepAction},
    select::{SelectEvent, SelectState},
};

use super::{app::AppState, select_item::EnumSelectItem};

const BASE_AMOUNT_STEP: f64 = 1000.0;
const RATE_STEP: f64 = 0.1;
const SITE_FEE_MAX: f64 = 99.9;

#[derive(Clone, Copy)]
struct StepNumberConfig {
    step: f64,
    min: f64,
    max: Option<f64>,
    formatter: fn(f64) -> String,
}

const BASE_AMOUNT_STEP_CONFIG: StepNumberConfig = StepNumberConfig {
    step: BASE_AMOUNT_STEP,
    min: 0.0,
    max: None,
    formatter: format_decimal_number,
};

const TAX_RATE_STEP_CONFIG: StepNumberConfig = StepNumberConfig {
    step: RATE_STEP,
    min: 0.0,
    max: None,
    formatter: format_rate_number,
};

const SITE_FEE_STEP_CONFIG: StepNumberConfig = StepNumberConfig {
    step: RATE_STEP,
    min: 0.0,
    max: Some(SITE_FEE_MAX),
    formatter: format_rate_number,
};

pub(super) fn create_base_amount_input(
    model: &AppModel,
    window: &mut Window,
    cx: &mut Context<AppState>,
) -> Entity<InputState> {
    let input = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("例: 100000")
            .mask_pattern(MaskPattern::Number {
                separator: Some(','),
                fraction: None,
            })
            .default_value(model.base_amount().to_string())
    });

    input.update(cx, |input, cx| {
        input.set_value(model.base_amount().to_string(), window, cx);
    });

    input
}

pub(super) fn create_tax_rate_input(
    model: &AppModel,
    window: &mut Window,
    cx: &mut Context<AppState>,
) -> Entity<InputState> {
    cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("例: 10")
            .default_value(model.tax_rate().to_string())
    })
}

pub(super) fn create_site_name_input(
    window: &mut Window,
    cx: &mut Context<AppState>,
) -> Entity<InputState> {
    cx.new(|cx| InputState::new(window, cx).placeholder("例: CrowdWorks"))
}

pub(super) fn create_site_fee_input(
    window: &mut Window,
    cx: &mut Context<AppState>,
) -> Entity<InputState> {
    cx.new(|cx| InputState::new(window, cx).placeholder("例: 20"))
}

pub(super) fn create_input_tax_mode_select(
    model: &AppModel,
    window: &mut Window,
    cx: &mut Context<AppState>,
) -> Entity<SelectState<Vec<EnumSelectItem<InputTaxMode>>>> {
    let options = mode_options();
    let selected = selected_index(&options, &model.input_tax_mode());
    cx.new(|cx| SelectState::new(options, selected, window, cx))
}

pub(super) fn create_rounding_mode_select(
    model: &AppModel,
    window: &mut Window,
    cx: &mut Context<AppState>,
) -> Entity<SelectState<Vec<EnumSelectItem<RoundingMode>>>> {
    let options = rounding_options();
    let selected = selected_index(&options, &model.rounding_mode());
    cx.new(|cx| SelectState::new(options, selected, window, cx))
}

pub(super) fn build_subscriptions(
    state: &AppState,
    window: &mut Window,
    cx: &mut Context<AppState>,
) -> Vec<Subscription> {
    vec![
        cx.subscribe(
            &state.base_amount_input,
            |this: &mut AppState, input, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    let update = this.model.set_base_amount(number_input_value(&input, cx));
                    this.apply_update(update, cx);
                }
            },
        ),
        cx.subscribe_in(
            &state.base_amount_input,
            window,
            |_: &mut AppState, input, event: &NumberInputEvent, window, cx| {
                let NumberInputEvent::Step(action) = event;
                step_number_input(input, *action, BASE_AMOUNT_STEP_CONFIG, window, cx);
            },
        ),
        cx.subscribe(
            &state.tax_rate_input,
            |this: &mut AppState, input, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    let update = this.model.set_tax_rate(number_input_value(&input, cx));
                    this.apply_update(update, cx);
                }
            },
        ),
        cx.subscribe_in(
            &state.tax_rate_input,
            window,
            |_: &mut AppState, input, event: &NumberInputEvent, window, cx| {
                let NumberInputEvent::Step(action) = event;
                step_number_input(input, *action, TAX_RATE_STEP_CONFIG, window, cx);
            },
        ),
        cx.subscribe(
            &state.input_tax_mode_select,
            |this: &mut AppState, _, event: &SelectEvent<Vec<EnumSelectItem<InputTaxMode>>>, cx| {
                let SelectEvent::Confirm(Some(mode)) = event else {
                    return;
                };
                let update = this.model.update_input_tax_mode(*mode);
                this.apply_update(update, cx);
            },
        ),
        cx.subscribe(
            &state.rounding_mode_select,
            |this: &mut AppState, _, event: &SelectEvent<Vec<EnumSelectItem<RoundingMode>>>, cx| {
                let SelectEvent::Confirm(Some(mode)) = event else {
                    return;
                };
                let update = this.model.update_rounding_mode(*mode);
                this.apply_update(update, cx);
            },
        ),
        cx.subscribe(
            &state.site_name_input,
            |this: &mut AppState, input, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    let update = this.model.set_site_name(input.read(cx).value().to_string());
                    this.apply_update(update, cx);
                }
            },
        ),
        cx.subscribe(
            &state.site_fee_input,
            |this: &mut AppState, input, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    let update = this.model.set_site_fee(number_input_value(&input, cx));
                    this.apply_update(update, cx);
                }
            },
        ),
        cx.subscribe_in(
            &state.site_fee_input,
            window,
            |_: &mut AppState, input, event: &NumberInputEvent, window, cx| {
                let NumberInputEvent::Step(action) = event;
                step_number_input(input, *action, SITE_FEE_STEP_CONFIG, window, cx);
            },
        ),
    ]
}

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
        .position(|item| item.value_ref() == selected)
        .map(IndexPath::new)
}

pub(super) fn number_input_value(input: &Entity<InputState>, cx: &App) -> String {
    input.read(cx).unmask_value().to_string()
}

fn round_to_tenths(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn format_decimal_number(value: f64) -> String {
    trim_trailing_zero(value)
}

fn format_rate_number(value: f64) -> String {
    trim_trailing_zero(round_to_tenths(value))
}

fn step_number_input(
    input: &Entity<InputState>,
    action: StepAction,
    config: StepNumberConfig,
    window: &mut Window,
    cx: &mut Context<AppState>,
) {
    let raw_value = number_input_value(input, cx);
    let current_value = raw_value.trim().parse::<f64>().unwrap_or(0.0);
    let delta = match action {
        StepAction::Increment => config.step,
        StepAction::Decrement => -config.step,
    };

    let mut next_value = current_value + delta;
    if next_value < config.min {
        next_value = config.min;
    }
    if let Some(max) = config.max
        && next_value > max
    {
        next_value = max;
    }

    input.update(cx, |state, cx| {
        state.set_value((config.formatter)(next_value), window, cx);
    });
}
