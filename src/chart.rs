use anyhow::{Result, bail};
use chrono::DateTime;
use serde_json::Value;

pub fn auto_interval_for_range(range: &str) -> &'static str {
    match range.trim().to_lowercase().as_str() {
        "1d" => "5m",
        "5d" => "15m",
        "1mo" | "3mo" => "1d",
        "6mo" | "1y" => "1wk",
        "2y" | "5y" | "10y" | "max" => "1mo",
        _ => "1d",
    }
}

pub fn format_timestamp(ts: i64, interval: &str) -> String {
    let dt_opt = DateTime::from_timestamp(ts, 0);
    let Some(dt) = dt_opt else {
        return String::from("N/A");
    };

    if interval.ends_with('m') || interval.ends_with('h') {
        dt.format("%m/%d %H:%M").to_string()
    } else if interval.ends_with("mo") {
        dt.format("%Y-%m").to_string()
    } else {
        dt.format("%Y-%m-%d").to_string()
    }
}

pub struct CandlePoint {
    pub timestamp: i64,
    pub close: f64,
}

pub fn extract_valid_candle_points(chart_json: &Value) -> Result<(String, f64, Vec<CandlePoint>)> {
    let meta = &chart_json["meta"];
    let currency = meta["currency"].as_str().unwrap_or("USD").to_string();
    let regular_market_price = meta["regularMarketPrice"]
        .as_f64()
        .or_else(|| meta["chartPreviousClose"].as_f64())
        .unwrap_or(0.0);

    let timestamps = chart_json["timestamp"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing chart timestamps"))?;

    let quote = &chart_json["indicators"]["quote"][0];
    let closes = quote["close"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing chart close prices"))?;

    let mut points = Vec::new();
    let len = timestamps.len().min(closes.len());

    for i in 0..len {
        if let (Some(ts), Some(c)) = (timestamps[i].as_i64(), closes[i].as_f64()) {
            points.push(CandlePoint {
                timestamp: ts,
                close: c,
            });
        }
    }

    if points.is_empty() {
        bail!("No valid price points found in chart data");
    }

    Ok((currency, regular_market_price, points))
}

pub fn render_terminal_chart(
    ticker: &str,
    range: &str,
    interval: &str,
    chart_json: &Value,
    width: usize,
    height: usize,
) -> Result<String> {
    let (currency, current_price_fallback, points) = extract_valid_candle_points(chart_json)?;

    let first_price = points.first().map(|p| p.close).unwrap_or(0.0);
    let last_price = points
        .last()
        .map(|p| p.close)
        .unwrap_or(current_price_fallback);
    let min_price = points.iter().map(|p| p.close).fold(f64::INFINITY, f64::min);
    let max_price = points
        .iter()
        .map(|p| p.close)
        .fold(f64::NEG_INFINITY, f64::max);

    let net_change = last_price - first_price;
    let pct_change = if first_price.abs() > f64::EPSILON {
        (net_change / first_price) * 100.0
    } else {
        0.0
    };

    let sign = if net_change >= 0.0 { "+" } else { "" };
    let trend_symbol = if net_change >= 0.0 { "▲" } else { "▼" };

    let mut out = String::new();

    // 1. Header
    out.push_str(&format!(
        "{} · {} {:.2} ({} {:.2} / {}{:.2}% {})\n",
        ticker.to_uppercase(),
        currency,
        last_price,
        trend_symbol,
        net_change,
        sign,
        pct_change,
        range.to_uppercase()
    ));
    out.push_str(&format!(
        "High: {:.2} | Low: {:.2} | Interval: {}\n\n",
        max_price, min_price, interval
    ));

    // Downsample or interpolate points to target chart width
    let chart_w = width.clamp(30, 100);
    let chart_h = height.clamp(5, 25);

    let price_range = if (max_price - min_price).abs() < 1e-4 {
        1.0
    } else {
        max_price - min_price
    };

    // Resample points to fit width
    let mut sampled_y = Vec::with_capacity(chart_w);
    let num_points = points.len();

    for col in 0..chart_w {
        let src_idx = (col * num_points) / chart_w;
        let actual_idx = src_idx.min(num_points - 1);
        let p = points[actual_idx].close;

        // Map price to row index (0 = top / max_price, chart_h - 1 = bottom / min_price)
        let normalized = (p - min_price) / price_range;
        let row_float = (1.0 - normalized) * (chart_h - 1) as f64;
        let row = row_float.round().clamp(0.0, (chart_h - 1) as f64) as usize;
        sampled_y.push(row);
    }

    // Render Canvas Matrix
    let mut grid = vec![vec![' '; chart_w]; chart_h];

    for col in 0..chart_w {
        let r = sampled_y[col];
        if col == 0 {
            grid[r][col] = '─';
        } else {
            let prev_r = sampled_y[col - 1];
            if r == prev_r {
                grid[r][col] = '─';
            } else if r < prev_r {
                // Going up on screen (higher price)
                grid[prev_r][col] = '╯';
                for row_cells in grid.iter_mut().take(prev_r).skip(r + 1) {
                    row_cells[col] = '│';
                }
                grid[r][col] = '╭';
            } else {
                // Going down on screen (lower price)
                grid[prev_r][col] = '╮';
                for row_cells in grid.iter_mut().take(r).skip(prev_r + 1) {
                    row_cells[col] = '│';
                }
                grid[r][col] = '╰';
            }
        }
    }

    // Format Y-axis labels with lines
    let y_label_width = 9;
    for (r, row_cells) in grid.iter().enumerate().take(chart_h) {
        let price_at_row = max_price - (r as f64 / (chart_h - 1) as f64) * price_range;
        let axis_char = if r == 0 {
            '┬'
        } else if r == chart_h - 1 {
            '┴'
        } else {
            '┤'
        };

        let label = format!(
            "{:>width$.2} {}",
            price_at_row,
            axis_char,
            width = y_label_width
        );
        out.push_str(&label);

        for ch in row_cells.iter().take(chart_w) {
            out.push(*ch);
        }
        out.push('\n');
    }

    // 3. X-axis date labels
    let start_date = points
        .first()
        .map(|p| format_timestamp(p.timestamp, interval))
        .unwrap_or_default();
    let mid_date = points
        .get(num_points / 2)
        .map(|p| format_timestamp(p.timestamp, interval))
        .unwrap_or_default();
    let end_date = points
        .last()
        .map(|p| format_timestamp(p.timestamp, interval))
        .unwrap_or_default();

    let pad_spaces = " ".repeat(y_label_width + 2);
    out.push_str(&pad_spaces);

    let space_between = if chart_w > (start_date.len() + mid_date.len() + end_date.len()) {
        (chart_w - start_date.len() - mid_date.len() - end_date.len()) / 2
    } else {
        2
    };

    out.push_str(&start_date);
    out.push_str(&" ".repeat(space_between));
    out.push_str(&mid_date);
    out.push_str(&" ".repeat(space_between));
    out.push_str(&end_date);
    out.push('\n');

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_auto_interval_resolution() {
        assert_eq!(auto_interval_for_range("1d"), "5m");
        assert_eq!(auto_interval_for_range("5d"), "15m");
        assert_eq!(auto_interval_for_range("1mo"), "1d");
        assert_eq!(auto_interval_for_range("3mo"), "1d");
        assert_eq!(auto_interval_for_range("1y"), "1wk");
        assert_eq!(auto_interval_for_range("5y"), "1mo");
        assert_eq!(auto_interval_for_range("max"), "1mo");
    }

    #[test]
    fn test_format_timestamp() {
        let ts = 1782739800; // 2026-06-30 UTC
        let d = format_timestamp(ts, "1d");
        assert!(d.contains("2026-"));
        let m = format_timestamp(ts, "1mo");
        assert_eq!(m, "2026-06");
    }

    #[test]
    fn test_render_terminal_chart_mock_data() {
        let sample_chart = json!({
            "meta": {
                "currency": "USD",
                "regularMarketPrice": 250.0,
                "chartPreviousClose": 200.0
            },
            "timestamp": [1782739800, 1782826200, 1782912600, 1782999000],
            "indicators": {
                "quote": [
                    {
                        "close": [200.0, 220.0, 210.0, 250.0]
                    }
                ]
            }
        });

        let rendered = render_terminal_chart("AAPL", "1mo", "1d", &sample_chart, 40, 8).unwrap();
        assert!(rendered.contains("AAPL · USD"));
        assert!(rendered.contains("High: 250.00"));
        assert!(rendered.contains("Low: 200.00"));
        assert!(rendered.contains("┬"));
        assert!(rendered.contains("┴"));
    }
}
