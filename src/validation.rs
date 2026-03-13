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
