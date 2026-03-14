use std::{
    env, fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    AppConfig, AppModel, InputTaxMode, RoundingMode, SiteFee, apply_rounding, calculate_row,
    config_dir, config_path, format_number, format_yen, integer_string, load_config_from_path,
    save_config_to_path,
};

fn temp_config_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    env::temp_dir().join(format!("fee-backcalc-{name}-{unique}.json"))
}

#[test]
fn calculates_fee_from_tax_exclusive_base() {
    let site = SiteFee {
        name: "A".to_string(),
        fee_percent: 10.0,
    };

    let row = calculate_row(
        &site,
        1000.0,
        InputTaxMode::TaxExclusive,
        0.1,
        RoundingMode::Round,
    );

    assert_eq!(row.fee_amount, 122.0);
    assert_eq!(row.invoice_amount, 1222.0);
    assert_eq!(row.received_amount, 1100.0);
}

#[test]
fn calculates_fee_from_tax_inclusive_base() {
    let site = SiteFee {
        name: "A".to_string(),
        fee_percent: 10.0,
    };

    let row = calculate_row(
        &site,
        1100.0,
        InputTaxMode::TaxInclusive,
        0.1,
        RoundingMode::Round,
    );

    assert_eq!(row.fee_amount, 122.0);
    assert_eq!(row.invoice_amount, 1222.0);
    assert_eq!(row.received_amount, 1100.0);
}

#[test]
fn tax_rate_change_affects_tax_and_total() {
    let site = SiteFee {
        name: "A".to_string(),
        fee_percent: 5.0,
    };

    let low_tax = calculate_row(
        &site,
        1000.0,
        InputTaxMode::TaxExclusive,
        0.08,
        RoundingMode::Round,
    );
    let high_tax = calculate_row(
        &site,
        1000.0,
        InputTaxMode::TaxExclusive,
        0.10,
        RoundingMode::Round,
    );

    assert!(high_tax.fee_amount > low_tax.fee_amount);
    assert!(high_tax.invoice_amount > low_tax.invoice_amount);
    assert!(high_tax.received_amount > low_tax.received_amount);
}

#[test]
fn calculates_tax_exclusive_example_from_report() {
    let site = SiteFee {
        name: "A".to_string(),
        fee_percent: 22.0,
    };

    let row = calculate_row(
        &site,
        10000.0,
        InputTaxMode::TaxExclusive,
        0.1,
        RoundingMode::Round,
    );

    assert_eq!(row.fee_amount, 3103.0);
    assert_eq!(row.invoice_amount, 14103.0);
    assert_eq!(row.received_amount, 11000.0);
}

#[test]
fn tax_inclusive_input_does_not_add_tax_twice() {
    let site = SiteFee {
        name: "A".to_string(),
        fee_percent: 10.0,
    };

    let low_tax = calculate_row(
        &site,
        1100.0,
        InputTaxMode::TaxInclusive,
        0.08,
        RoundingMode::Round,
    );
    let high_tax = calculate_row(
        &site,
        1100.0,
        InputTaxMode::TaxInclusive,
        0.10,
        RoundingMode::Round,
    );

    assert_eq!(low_tax.fee_amount, high_tax.fee_amount);
    assert_eq!(low_tax.invoice_amount, high_tax.invoice_amount);
    assert_eq!(low_tax.received_amount, high_tax.received_amount);
}

#[test]
fn applies_rounding_modes() {
    assert_eq!(apply_rounding(10.4, RoundingMode::Round), 10.0);
    assert_eq!(apply_rounding(10.5, RoundingMode::Round), 11.0);
    assert_eq!(apply_rounding(10.1, RoundingMode::Ceil), 11.0);
    assert_eq!(apply_rounding(10.9, RoundingMode::Floor), 10.0);
}

#[test]
fn formats_numbers_with_commas() {
    assert_eq!(format_number(0.0), "0");
    assert_eq!(format_number(1234.0), "1,234");
    assert_eq!(format_number(123456789.0), "123,456,789");
}

#[test]
fn formats_plain_integer_without_commas() {
    assert_eq!(integer_string(0.0), "0");
    assert_eq!(integer_string(1234.0), "1234");
    assert_eq!(integer_string(123456789.0), "123456789");
}

#[test]
fn formats_yen_with_commas() {
    assert_eq!(format_yen(14103.0), "14,103円");
}

#[test]
fn handles_zero_boundary_values() {
    let site = SiteFee {
        name: "A".to_string(),
        fee_percent: 0.0,
    };

    let row = calculate_row(
        &site,
        0.0,
        InputTaxMode::TaxExclusive,
        0.1,
        RoundingMode::Round,
    );

    assert_eq!(row.fee_amount, 0.0);
    assert_eq!(row.invoice_amount, 0.0);
    assert_eq!(row.received_amount, 0.0);
}

#[test]
fn saves_and_loads_config() {
    let path = temp_config_path("roundtrip");
    let config = AppConfig {
        base_amount: "1234".to_string(),
        input_tax_mode: InputTaxMode::TaxInclusive,
        tax_rate: "8".to_string(),
        rounding_mode: RoundingMode::Ceil,
        sites: vec![SiteFee {
            name: "Shop".to_string(),
            fee_percent: 12.5,
        }],
    };

    save_config_to_path(&config, &path).expect("save");
    let loaded = load_config_from_path(&path).expect("load");

    assert_eq!(loaded, config);

    let _ = fs::remove_file(path);
}

#[test]
fn missing_config_returns_default() {
    let path = temp_config_path("missing");
    let loaded = load_config_from_path(&path).expect("load");

    assert_eq!(loaded, AppConfig::default());
}

#[test]
fn config_path_uses_fee_backcalc_directory() {
    let path = config_path();

    assert!(path.to_string_lossy().contains("fee-backcalc"));
}

#[test]
fn config_dir_uses_fee_backcalc_directory() {
    let path = config_dir();

    assert!(path.to_string_lossy().contains("fee-backcalc"));
}

#[test]
fn app_model_restores_config_and_roundtrips_it() {
    let config = AppConfig {
        base_amount: "5000".to_string(),
        input_tax_mode: InputTaxMode::TaxInclusive,
        tax_rate: "8".to_string(),
        rounding_mode: RoundingMode::Ceil,
        sites: vec![SiteFee {
            name: "Shop".to_string(),
            fee_percent: 12.5,
        }],
    };

    let model = AppModel::from_config(config.clone());

    assert_eq!(model.base_amount(), "5000");
    assert_eq!(model.tax_rate(), "8");
    assert_eq!(model.input_tax_mode(), InputTaxMode::TaxInclusive);
    assert_eq!(model.rounding_mode(), RoundingMode::Ceil);
    assert_eq!(model.sites(), config.sites.as_slice());
    assert_eq!(model.to_config(), config);
}

#[test]
fn app_model_adds_updates_and_removes_sites() {
    let mut model = AppModel::from_config(AppConfig::default());

    model.set_site_name("CrowdWorks");
    model.set_site_fee("20");
    let add_update = model.add_or_update_site();
    assert!(add_update.should_persist);
    assert!(add_update.should_sync_site_form);
    assert_eq!(model.sites().len(), 1);
    assert_eq!(model.sites()[0].name, "CrowdWorks");

    let edit_update = model.start_edit_site(0);
    assert!(edit_update.should_sync_site_form);
    assert_eq!(model.site_name(), "CrowdWorks");
    assert_eq!(model.site_fee(), "20");

    model.set_site_name("Lancers");
    model.set_site_fee("15");
    model.add_or_update_site();
    assert_eq!(model.sites()[0].name, "Lancers");
    assert_eq!(model.sites()[0].fee_percent, 15.0);

    model.start_edit_site(0);
    let remove_update = model.remove_site(0);
    assert!(remove_update.should_persist);
    assert!(remove_update.should_sync_site_form);
    assert!(model.sites().is_empty());
    assert_eq!(model.editing_index(), None);
}

#[test]
fn app_model_cancel_edit_clears_site_form_state() {
    let config = AppConfig {
        base_amount: String::new(),
        input_tax_mode: InputTaxMode::TaxExclusive,
        tax_rate: "10".to_string(),
        rounding_mode: RoundingMode::Round,
        sites: vec![SiteFee {
            name: "Shop".to_string(),
            fee_percent: 12.5,
        }],
    };
    let mut model = AppModel::from_config(config);

    model.start_edit_site(0);
    let update = model.cancel_edit();

    assert!(update.should_sync_site_form);
    assert_eq!(model.editing_index(), None);
    assert_eq!(model.site_name(), "");
    assert_eq!(model.site_fee(), "");
}

#[test]
fn app_model_clears_results_for_invalid_input_and_recovers() {
    let config = AppConfig {
        base_amount: "1000".to_string(),
        input_tax_mode: InputTaxMode::TaxExclusive,
        tax_rate: "10".to_string(),
        rounding_mode: RoundingMode::Round,
        sites: vec![SiteFee {
            name: "Shop".to_string(),
            fee_percent: 10.0,
        }],
    };
    let mut model = AppModel::from_config(config);

    assert_eq!(model.result_rows().len(), 1);
    assert_eq!(model.result_error(), None);

    model.set_tax_rate("-1");
    assert!(model.result_rows().is_empty());
    assert_eq!(
        model.result_error(),
        Some("消費税率は0以上で入力してください")
    );

    model.set_tax_rate("10");
    assert_eq!(model.result_error(), None);
    assert_eq!(model.result_rows().len(), 1);
}
