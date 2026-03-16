mod app_model;
mod calc;
mod config;
mod format;
mod model;
mod theme;
mod validation;

pub const APP_ID: &str = "com.takashicc.fee-backcalc";
pub const APP_TITLE: &str = "Fee Backcalc";

pub use app_model::{AppModel, ModelUpdate};
pub use calc::{apply_rounding, calculate_row};
pub use config::{
    config_dir, config_path, load_config, load_config_from_path, open_config_directory,
    save_config, save_config_to_path,
};
pub use format::{format_number, format_yen, integer_string, trim_trailing_zero};
pub use model::{AppConfig, CalculatedRow, InputTaxMode, RoundingMode, SiteFee};
pub use theme::{apply_saved_theme, apply_theme_by_name, ensure_bundled_themes};
pub use validation::{parse_non_negative_number, validate_fee_percent};

#[cfg(test)]
mod tests;
