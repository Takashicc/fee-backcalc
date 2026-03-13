use crate::{CalculatedRow, InputTaxMode, RoundingMode, SiteFee};

pub fn apply_rounding(value: f64, rounding_mode: RoundingMode) -> f64 {
    match rounding_mode {
        RoundingMode::Round => value.round(),
        RoundingMode::Ceil => value.ceil(),
        RoundingMode::Floor => value.floor(),
    }
}

pub fn calculate_row(
    site: &SiteFee,
    base_amount: f64,
    input_tax_mode: InputTaxMode,
    tax_rate: f64,
    rounding_mode: RoundingMode,
) -> CalculatedRow {
    let fee_rate = site.fee_percent / 100.0;
    let desired_received_amount = match input_tax_mode {
        InputTaxMode::TaxExclusive => base_amount * (1.0 + tax_rate),
        InputTaxMode::TaxInclusive => base_amount,
    };
    let invoice_amount = if fee_rate >= 1.0 {
        f64::INFINITY
    } else {
        desired_received_amount / (1.0 - fee_rate)
    };
    let fee_amount = apply_rounding(invoice_amount * fee_rate, rounding_mode);
    let invoice_amount = apply_rounding(invoice_amount, rounding_mode);

    CalculatedRow {
        site_name: site.name.clone(),
        fee_percent: site.fee_percent,
        fee_amount,
        invoice_amount,
        received_amount: invoice_amount - fee_amount,
    }
}
