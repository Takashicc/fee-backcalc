use anyhow::Context as _;
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SiteFee {
    pub name: String,
    pub fee_percent: f64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum InputTaxMode {
    TaxExclusive,
    TaxInclusive,
}

impl InputTaxMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::TaxExclusive => "税抜き入力",
            Self::TaxInclusive => "税込み入力",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoundingMode {
    Round,
    Ceil,
    Floor,
}

impl RoundingMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Round => "四捨五入",
            Self::Ceil => "切り上げ",
            Self::Floor => "切り捨て",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    pub base_amount: String,
    pub input_tax_mode: InputTaxMode,
    pub tax_rate: String,
    pub rounding_mode: RoundingMode,
    pub sites: Vec<SiteFee>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            base_amount: String::new(),
            input_tax_mode: InputTaxMode::TaxExclusive,
            tax_rate: "10".to_string(),
            rounding_mode: RoundingMode::Round,
            sites: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalculatedRow {
    pub site_name: String,
    pub fee_percent: f64,
    pub fee_amount: f64,
    pub exclusive_total: f64,
    pub tax_amount: f64,
    pub inclusive_total: f64,
}

pub fn parse_non_negative_number(field_name: &str, text: &str) -> Result<f64, String> {
    if text.trim().is_empty() {
        return Err(format!("{field_name}を入力してください"));
    }

    let value = text
        .trim()
        .parse::<f64>()
        .map_err(|_| format!("{field_name}は数値で入力してください"))?;

    if value < 0.0 {
        return Err(format!("{field_name}は0以上で入力してください"));
    }

    Ok(value)
}

pub fn validate_fee_percent(text: &str) -> Result<f64, String> {
    let value = parse_non_negative_number("手数料率", text)?;
    if value >= 100.0 {
        return Err("手数料率は100未満で入力してください".to_string());
    }
    Ok(value)
}

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
    let base_exclusive = match input_tax_mode {
        InputTaxMode::TaxExclusive => base_amount,
        InputTaxMode::TaxInclusive => base_amount / (1.0 + tax_rate),
    };

    let fee_rate = site.fee_percent / 100.0;
    let exclusive_total = if fee_rate >= 1.0 {
        f64::INFINITY
    } else {
        base_exclusive / (1.0 - fee_rate)
    };
    let fee_amount = exclusive_total - base_exclusive;
    let tax_amount = exclusive_total * tax_rate;
    let inclusive_total = exclusive_total + tax_amount;

    CalculatedRow {
        site_name: site.name.clone(),
        fee_percent: site.fee_percent,
        fee_amount: apply_rounding(fee_amount, rounding_mode),
        exclusive_total: apply_rounding(exclusive_total, rounding_mode),
        tax_amount: apply_rounding(tax_amount, rounding_mode),
        inclusive_total: apply_rounding(inclusive_total, rounding_mode),
    }
}

pub fn trim_trailing_zero(value: f64) -> String {
    let mut text = format!("{value:.4}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

pub fn format_yen(value: f64) -> String {
    format!("{}{}", value as i64, jp("円"))
}

pub fn config_path() -> PathBuf {
    let base_dir = env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));

    base_dir.join("fee-tax-calculator").join("config.json")
}

pub fn load_config() -> AppConfig {
    load_config_from_path(&config_path()).unwrap_or_default()
}

pub fn load_config_from_path(path: &Path) -> anyhow::Result<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("設定ファイルの読み込みに失敗しました: {}", path.display()))?;
    let config = serde_json::from_str(&content)
        .with_context(|| format!("設定ファイルの解析に失敗しました: {}", path.display()))?;
    Ok(config)
}

pub fn save_config(config: &AppConfig) -> anyhow::Result<()> {
    save_config_to_path(config, &config_path())
}

pub fn save_config_to_path(config: &AppConfig, path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("設定ディレクトリの作成に失敗しました: {}", parent.display())
        })?;
    }

    let json = serde_json::to_string_pretty(config)?;
    fs::write(path, json)
        .with_context(|| format!("設定ファイルの保存に失敗しました: {}", path.display()))?;
    Ok(())
}

pub fn jp(input: &str) -> String {
    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_config_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        env::temp_dir().join(format!("fee-tax-calculator-{name}-{unique}.json"))
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

        assert_eq!(row.fee_amount, 111.0);
        assert_eq!(row.exclusive_total, 1111.0);
        assert_eq!(row.tax_amount, 111.0);
        assert_eq!(row.inclusive_total, 1222.0);
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

        assert_eq!(row.fee_amount, 111.0);
        assert_eq!(row.exclusive_total, 1111.0);
        assert_eq!(row.tax_amount, 111.0);
        assert_eq!(row.inclusive_total, 1222.0);
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

        assert!(high_tax.tax_amount > low_tax.tax_amount);
        assert!(high_tax.inclusive_total > low_tax.inclusive_total);
    }

    #[test]
    fn applies_rounding_modes() {
        assert_eq!(apply_rounding(10.4, RoundingMode::Round), 10.0);
        assert_eq!(apply_rounding(10.5, RoundingMode::Round), 11.0);
        assert_eq!(apply_rounding(10.1, RoundingMode::Ceil), 11.0);
        assert_eq!(apply_rounding(10.9, RoundingMode::Floor), 10.0);
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
        assert_eq!(row.exclusive_total, 0.0);
        assert_eq!(row.tax_amount, 0.0);
        assert_eq!(row.inclusive_total, 0.0);
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
}
