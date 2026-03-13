use serde::{Deserialize, Serialize};

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
            Self::TaxExclusive => "税抜きで入力",
            Self::TaxInclusive => "税込みで入力",
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
    pub invoice_amount: f64,
    pub received_amount: f64,
}
