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

pub fn format_number(value: f64) -> String {
    let digits = integer_string(value);
    let (sign, digits) = match digits.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", digits.as_str()),
    };

    let len = digits.len();
    let mut formatted = String::with_capacity(len + (len.saturating_sub(1) / 3) + sign.len());
    formatted.push_str(sign);

    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (len - index) % 3 == 0 {
            formatted.push(',');
        }
        formatted.push(ch);
    }

    formatted
}

pub fn integer_string(value: f64) -> String {
    (value as i64).to_string()
}

pub fn format_yen(value: f64) -> String {
    format!("{}{}", format_number(value), jp("円"))
}

pub fn jp(input: &str) -> String {
    input.to_string()
}
