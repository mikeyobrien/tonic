use std::time::{SystemTime, UNIX_EPOCH};

pub fn utc_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{now}")
}

pub fn ratio(candidate: f64, reference: f64) -> Option<f64> {
    if reference <= 0.0 {
        None
    } else {
        Some(candidate / reference)
    }
}

pub fn score_ratio(ratio: Option<f64>, budget_ratio: f64) -> Option<f64> {
    let value = ratio?;
    if value <= 1.0 {
        return Some(1.0);
    }

    if budget_ratio <= 1.0 {
        return Some(0.0);
    }

    if value >= budget_ratio {
        return Some(0.0);
    }

    Some(1.0 - ((value - 1.0) / (budget_ratio - 1.0)))
}
