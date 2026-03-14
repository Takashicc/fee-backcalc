use crate::{
    AppConfig, CalculatedRow, InputTaxMode, RoundingMode, SiteFee, calculate_row,
    parse_non_negative_number, trim_trailing_zero, validate_fee_percent,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ModelUpdate {
    pub should_notify: bool,
    pub should_persist: bool,
    pub should_sync_site_form: bool,
}

impl ModelUpdate {
    pub const fn none() -> Self {
        Self {
            should_notify: false,
            should_persist: false,
            should_sync_site_form: false,
        }
    }

    pub const fn notify() -> Self {
        Self {
            should_notify: true,
            ..Self::none()
        }
    }

    pub const fn persist() -> Self {
        Self {
            should_notify: true,
            should_persist: true,
            should_sync_site_form: false,
        }
    }

    pub const fn sync_site_form() -> Self {
        Self {
            should_notify: true,
            should_persist: false,
            should_sync_site_form: true,
        }
    }

    pub const fn persist_and_sync_site_form() -> Self {
        Self {
            should_notify: true,
            should_persist: true,
            should_sync_site_form: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppModel {
    base_amount: String,
    tax_rate: String,
    site_name: String,
    site_fee: String,
    input_tax_mode: InputTaxMode,
    rounding_mode: RoundingMode,
    sites: Vec<SiteFee>,
    editing_index: Option<usize>,
    result_rows: Vec<CalculatedRow>,
    result_error: Option<String>,
}

impl AppModel {
    pub fn from_config(config: AppConfig) -> Self {
        let mut model = Self {
            base_amount: config.base_amount,
            tax_rate: config.tax_rate,
            site_name: String::new(),
            site_fee: String::new(),
            input_tax_mode: config.input_tax_mode,
            rounding_mode: config.rounding_mode,
            sites: config.sites,
            editing_index: None,
            result_rows: Vec::new(),
            result_error: None,
        };
        model.recalculate();
        model
    }

    pub fn to_config(&self) -> AppConfig {
        AppConfig {
            base_amount: self.base_amount.clone(),
            input_tax_mode: self.input_tax_mode,
            tax_rate: self.tax_rate.clone(),
            rounding_mode: self.rounding_mode,
            sites: self.sites.clone(),
        }
    }

    pub fn base_amount(&self) -> &str {
        &self.base_amount
    }

    pub fn tax_rate(&self) -> &str {
        &self.tax_rate
    }

    pub fn site_name(&self) -> &str {
        &self.site_name
    }

    pub fn site_fee(&self) -> &str {
        &self.site_fee
    }

    pub fn input_tax_mode(&self) -> InputTaxMode {
        self.input_tax_mode
    }

    pub fn rounding_mode(&self) -> RoundingMode {
        self.rounding_mode
    }

    pub fn sites(&self) -> &[SiteFee] {
        &self.sites
    }

    pub fn editing_index(&self) -> Option<usize> {
        self.editing_index
    }

    pub fn editing_site(&self) -> Option<&SiteFee> {
        self.editing_index.and_then(|index| self.sites.get(index))
    }

    pub fn result_rows(&self) -> &[CalculatedRow] {
        &self.result_rows
    }

    pub fn result_error(&self) -> Option<&str> {
        self.result_error.as_deref()
    }

    pub fn set_base_amount(&mut self, value: impl Into<String>) -> ModelUpdate {
        let value = value.into();
        if self.base_amount == value {
            return ModelUpdate::none();
        }

        self.base_amount = value;
        self.recalculate();
        ModelUpdate::notify()
    }

    pub fn set_tax_rate(&mut self, value: impl Into<String>) -> ModelUpdate {
        let value = value.into();
        if self.tax_rate == value {
            return ModelUpdate::none();
        }

        self.tax_rate = value;
        self.recalculate();
        ModelUpdate::persist()
    }

    pub fn set_site_name(&mut self, value: impl Into<String>) -> ModelUpdate {
        let value = value.into();
        if self.site_name == value {
            return ModelUpdate::none();
        }

        self.site_name = value;
        ModelUpdate::notify()
    }

    pub fn set_site_fee(&mut self, value: impl Into<String>) -> ModelUpdate {
        let value = value.into();
        if self.site_fee == value {
            return ModelUpdate::none();
        }

        self.site_fee = value;
        ModelUpdate::notify()
    }

    pub fn update_input_tax_mode(&mut self, mode: InputTaxMode) -> ModelUpdate {
        if self.input_tax_mode == mode {
            return ModelUpdate::none();
        }

        self.input_tax_mode = mode;
        self.recalculate();
        ModelUpdate::persist()
    }

    pub fn update_rounding_mode(&mut self, mode: RoundingMode) -> ModelUpdate {
        if self.rounding_mode == mode {
            return ModelUpdate::none();
        }

        self.rounding_mode = mode;
        self.recalculate();
        ModelUpdate::persist()
    }

    pub fn add_or_update_site(&mut self) -> ModelUpdate {
        let name = self.site_name.trim();
        let fee_text = self.site_fee.trim();
        if name.is_empty() || validate_fee_percent(fee_text).is_err() {
            return ModelUpdate::notify();
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

        self.clear_site_form_state();
        self.recalculate();
        ModelUpdate::persist_and_sync_site_form()
    }

    pub fn start_edit_site(&mut self, index: usize) -> ModelUpdate {
        let Some(site) = self.sites.get(index).cloned() else {
            return ModelUpdate::none();
        };

        self.editing_index = Some(index);
        self.site_name = site.name;
        self.site_fee = trim_trailing_zero(site.fee_percent);
        ModelUpdate::sync_site_form()
    }

    pub fn remove_site(&mut self, index: usize) -> ModelUpdate {
        if index >= self.sites.len() {
            return ModelUpdate::none();
        }

        self.sites.remove(index);
        let mut update = ModelUpdate::persist();
        if self.editing_index == Some(index) {
            self.clear_site_form_state();
            update.should_sync_site_form = true;
        }
        self.recalculate();
        update
    }

    pub fn cancel_edit(&mut self) -> ModelUpdate {
        if self.editing_index.is_none() && self.site_name.is_empty() && self.site_fee.is_empty() {
            return ModelUpdate::none();
        }

        self.clear_site_form_state();
        ModelUpdate::sync_site_form()
    }

    pub fn base_amount_error(&self) -> Option<String> {
        if self.base_amount.trim().is_empty() {
            None
        } else {
            parse_non_negative_number("受け取りたい金額", self.base_amount.trim()).err()
        }
    }

    pub fn tax_rate_error(&self) -> Option<String> {
        parse_non_negative_number("消費税率", self.tax_rate.trim()).err()
    }

    pub fn site_form_error(&self) -> Option<String> {
        let site_name = self.site_name.trim();
        let site_fee = self.site_fee.trim();

        if site_name.is_empty() && site_fee.is_empty() {
            None
        } else if site_name.is_empty() {
            Some("依頼サイト名を入力してください".to_string())
        } else {
            validate_fee_percent(site_fee).err()
        }
    }

    pub fn recalculate(&mut self) {
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

    fn clear_site_form_state(&mut self) {
        self.editing_index = None;
        self.site_name.clear();
        self.site_fee.clear();
    }
}
