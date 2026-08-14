use chrono::DateTime;
use serde_json::{Value, json};

pub fn round_2_dec(val: f64) -> f64 {
    (val * 100.0).round() / 100.0
}

pub fn format_volume(vol: f64) -> String {
    if vol >= 1_000_000_000.0 {
        format!("{:.1}B", vol / 1_000_000_000.0)
    } else if vol >= 1_000_000.0 {
        format!("{:.1}M", vol / 1_000_000.0)
    } else if vol >= 1_000.0 {
        format!("{:.1}K", vol / 1_000.0)
    } else {
        format!("{:.0}", vol)
    }
}

pub fn timestamp_to_yyyymm(ts: i64) -> String {
    DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m").to_string())
        .unwrap_or_else(|| "N/A".to_string())
}

pub fn get_fmt(obj: &Value, key: &str) -> Value {
    let field = &obj[key];
    if !field["fmt"].is_null() {
        field["fmt"].clone()
    } else if field.is_string() || field.is_number() || field.is_boolean() {
        field.clone()
    } else {
        Value::Null
    }
}

pub fn get_raw(obj: &Value, key: &str) -> Value {
    let field = &obj[key];
    if !field["raw"].is_null() {
        field["raw"].clone()
    } else if field.is_string() || field.is_number() || field.is_boolean() {
        field.clone()
    } else {
        Value::Null
    }
}

pub fn extract_upcoming_earnings(raw: &Value) -> Value {
    let calendar = &raw["calendarEvents"]["earnings"];

    let next_earnings_date = calendar["earningsDate"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|v| v["fmt"].as_str().or_else(|| v["raw"].as_str()))
        .map(Value::from)
        .unwrap_or(Value::Null);

    let is_date_estimate = calendar["isEarningsDateEstimate"]
        .as_bool()
        .map(Value::from)
        .unwrap_or(Value::Null);

    json!({
        "next_earnings_date": next_earnings_date,
        "is_date_estimate": is_date_estimate,
        "eps_estimates": {
            "average": get_raw(calendar, "earningsAverage"),
            "low": get_raw(calendar, "earningsLow"),
            "high": get_raw(calendar, "earningsHigh")
        },
        "revenue_estimates": {
            "average": get_fmt(calendar, "revenueAverage"),
            "low": get_fmt(calendar, "revenueLow"),
            "high": get_fmt(calendar, "revenueHigh")
        }
    })
}

pub fn extract_ai_dossier(ticker: &str, raw: &Value) -> Value {
    let detail = &raw["summaryDetail"];
    let fin = &raw["financialData"];
    let stats = &raw["defaultKeyStatistics"];
    let calendar = &raw["calendarEvents"];

    json!({
        "ticker": ticker.to_uppercase(),
        "sec_cik": crate::cik::get_static_cik(ticker),
        "valuation": {
            "trailing_pe_ttm": get_raw(detail, "trailingPE"),
            "forward_pe_next_1y": get_raw(detail, "forwardPE"),
            "peg_ratio_expected_3to5y": get_raw(stats, "pegRatio"),
            "price_to_book_mrq": get_raw(stats, "priceToBook"),
            "price_to_sales_ttm": get_raw(stats, "priceToSalesTrailing12Months"),
            "ev_to_ebitda_ttm": get_raw(stats, "enterpriseToEbitda"),
            "ev_to_revenue_ttm": get_raw(stats, "enterpriseToRevenue"),
            "trailing_eps_ttm": get_raw(stats, "trailingEps"),
            "forward_eps_next_1y": get_raw(stats, "forwardEps"),
            "market_cap": get_fmt(detail, "marketCap"),
            "enterprise_value": get_fmt(stats, "enterpriseValue"),
            "beta": get_raw(stats, "beta")
        },
        "profitability_margins_ttm": {
            "gross_margins_ttm": get_fmt(fin, "grossMargins"),
            "operating_margins_ttm": get_fmt(fin, "operatingMargins"),
            "profit_margins_ttm": get_fmt(fin, "profitMargins"),
            "revenue_growth_yoy_quarterly": get_fmt(fin, "revenueGrowth"),
            "earnings_growth_yoy_quarterly": get_fmt(fin, "earningsGrowth"),
            "return_on_equity_ttm": get_fmt(fin, "returnOnEquity"),
            "return_on_assets_ttm": get_fmt(fin, "returnOnAssets")
        },
        "balance_sheet_solvency_mrq": {
            "total_cash_mrq": get_fmt(fin, "totalCash"),
            "total_debt_mrq": get_fmt(fin, "totalDebt"),
            "total_revenue_ttm": get_fmt(fin, "totalRevenue"),
            "ebitda_ttm": get_fmt(fin, "ebitda"),
            "current_ratio_mrq": get_raw(fin, "currentRatio"),
            "quick_ratio_mrq": get_raw(fin, "quickRatio"),
            "debt_to_equity_mrq": get_raw(fin, "debtToEquity")
        },
        "cashflow_ttm": {
            "free_cash_flow_ttm": get_fmt(fin, "freeCashflow"),
            "operating_cashflow_ttm": get_fmt(fin, "operatingCashflow")
        },
        "analyst_estimates_and_sentiment": {
            "target_mean_price": get_raw(fin, "targetMeanPrice"),
            "target_high_price": get_raw(fin, "targetHighPrice"),
            "target_low_price": get_raw(fin, "targetLowPrice"),
            "recommendation": get_fmt(fin, "recommendationKey"),
            "number_of_analysts": get_raw(fin, "numberOfAnalystOpinions"),
            "short_percent_of_float": get_fmt(stats, "shortPercentOfFloat"),
            "short_ratio": get_raw(stats, "shortRatio")
        },
        "upcoming_earnings": extract_upcoming_earnings(raw),
        "key_market_data": {
            "fifty_two_week_high": get_raw(detail, "fiftyTwoWeekHigh"),
            "fifty_two_week_low": get_raw(detail, "fiftyTwoWeekLow"),
            "fifty_day_average": get_raw(detail, "fiftyDayAverage"),
            "two_hundred_day_average": get_raw(detail, "twoHundredDayAverage"),
            "dividend_yield": get_fmt(detail, "dividendYield"),
            "ex_dividend_date": get_fmt(calendar, "exDividendDate")
        }
    })
}

pub fn append_history_to_dossier(dossier: &mut Value, chart_data: &Value) {
    let mut history_candles = Vec::new();

    if let Some(timestamps) = chart_data["timestamp"].as_array() {
        let quote = &chart_data["indicators"]["quote"][0];
        let opens = quote["open"].as_array();
        let highs = quote["high"].as_array();
        let lows = quote["low"].as_array();
        let closes = quote["close"].as_array();
        let volumes = quote["volume"].as_array();

        for (i, ts_val) in timestamps.iter().enumerate() {
            let ts = ts_val.as_i64();
            let open = opens.and_then(|a| a.get(i)).and_then(|v| v.as_f64());
            let high = highs.and_then(|a| a.get(i)).and_then(|v| v.as_f64());
            let low = lows.and_then(|a| a.get(i)).and_then(|v| v.as_f64());
            let close = closes.and_then(|a| a.get(i)).and_then(|v| v.as_f64());
            let vol = volumes
                .and_then(|a| a.get(i))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            if let (Some(ts), Some(open), Some(high), Some(low), Some(close)) =
                (ts, open, high, low, close)
            {
                history_candles.push(json!({
                    "date": timestamp_to_yyyymm(ts),
                    "open": round_2_dec(open),
                    "high": round_2_dec(high),
                    "low": round_2_dec(low),
                    "close": round_2_dec(close),
                    "vol": format_volume(vol)
                }));
            }
        }
    }

    let mut perf_summary = json!({});

    if !history_candles.is_empty() {
        let n = history_candles.len();
        let latest_close = history_candles[n - 1]["close"].as_f64().unwrap_or(0.0);

        let fifty_two_week_high = chart_data["meta"]["fiftyTwoWeekHigh"].as_f64().or_else(|| {
            history_candles
                .iter()
                .filter_map(|c| c["high"].as_f64())
                .fold(None, |max, v| match max {
                    None => Some(v),
                    Some(m) => Some(f64::max(m, v)),
                })
        });

        if let Some(high52) = fifty_two_week_high.filter(|&h| h > 0.0) {
            let drawdown = ((latest_close - high52) / high52) * 100.0;
            perf_summary["drawdown_from_52w_high"] = json!(format!("{:.1}%", drawdown));
        }

        if n >= 13 {
            let close_1y = history_candles[n - 13]["close"].as_f64().unwrap_or(0.0);
            if close_1y > 0.0 {
                let ret_1y = ((latest_close - close_1y) / close_1y) * 100.0;
                let formatted = if ret_1y >= 0.0 {
                    format!("+{:.1}%", ret_1y)
                } else {
                    format!("{:.1}%", ret_1y)
                };
                perf_summary["one_year_return"] = json!(formatted);
            }
        } else if n >= 2 {
            let close_start = history_candles[0]["close"].as_f64().unwrap_or(0.0);
            if close_start > 0.0 {
                let ret = ((latest_close - close_start) / close_start) * 100.0;
                let formatted = if ret >= 0.0 {
                    format!("+{:.1}%", ret)
                } else {
                    format!("{:.1}%", ret)
                };
                perf_summary["one_year_return"] = json!(formatted);
            }
        }

        if n >= 61 {
            let close_5y = history_candles[n - 61]["close"].as_f64().unwrap_or(0.0);
            if close_5y > 0.0 {
                let ret_5y = ((latest_close - close_5y) / close_5y) * 100.0;
                let formatted = if ret_5y >= 0.0 {
                    format!("+{:.1}%", ret_5y)
                } else {
                    format!("{:.1}%", ret_5y)
                };
                perf_summary["five_year_return"] = json!(formatted);
            }
        }
    }

    if let Some(obj) = dossier.as_object_mut() {
        obj.insert("performance_summary".to_string(), perf_summary);
        obj.insert(
            "price_history_monthly".to_string(),
            Value::Array(history_candles),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_ai_dossier() {
        let sample_raw = json!({
            "summaryDetail": {
                "trailingPE": { "raw": 27.16 },
                "forwardPE": { "raw": 22.1 },
                "marketCap": { "fmt": "1.82T" },
                "fiftyTwoWeekHigh": { "raw": 479.0 },
                "fiftyTwoWeekLow": { "raw": 223.7 },
                "fiftyDayAverage": { "raw": 390.5 },
                "twoHundredDayAverage": { "raw": 310.2 },
                "dividendYield": { "fmt": "0.74%" }
            },
            "financialData": {
                "grossMargins": { "fmt": "53.2%" },
                "operatingMargins": { "fmt": "42.1%" },
                "profitMargins": { "fmt": "38.5%" },
                "revenueGrowth": { "fmt": "32.8%" },
                "earningsGrowth": { "fmt": "25.0%" },
                "returnOnEquity": { "fmt": "30.1%" },
                "returnOnAssets": { "fmt": "18.2%" },
                "freeCashflow": { "fmt": "25.4B" },
                "totalCash": { "fmt": "40.0B" },
                "totalDebt": { "fmt": "28.1B" },
                "totalRevenue": { "fmt": "80.0B" },
                "ebitda": { "fmt": "45.0B" },
                "currentRatio": { "raw": 2.3 },
                "quickRatio": { "raw": 1.9 },
                "debtToEquity": { "raw": 0.4 },
                "operatingCashflow": { "fmt": "42.0B" },
                "targetMeanPrice": { "raw": 450.0 },
                "recommendationKey": "buy",
                "numberOfAnalystOpinions": { "raw": 35 }
            },
            "defaultKeyStatistics": {
                "pegRatio": { "raw": 0.85 },
                "priceToBook": { "raw": 6.8 },
                "priceToSalesTrailing12Months": { "raw": 12.5 },
                "enterpriseToEbitda": { "raw": 18.0 },
                "enterpriseToRevenue": { "raw": 10.0 },
                "trailingEps": { "raw": 12.4 },
                "forwardEps": { "raw": 15.2 },
                "enterpriseValue": { "fmt": "1.80T" },
                "beta": { "raw": 1.1 },
                "shortPercentOfFloat": { "fmt": "1.2%" },
                "shortRatio": { "raw": 2.1 }
            },
            "calendarEvents": {
                "exDividendDate": { "fmt": "2026-09-16" }
            }
        });

        let dossier = extract_ai_dossier("TSM", &sample_raw);

        assert_eq!(dossier["ticker"], "TSM");
        assert_eq!(dossier["valuation"]["trailing_pe_ttm"], 27.16);
        assert_eq!(dossier["valuation"]["price_to_sales_ttm"], 12.5);
        assert_eq!(
            dossier["profitability_margins_ttm"]["gross_margins_ttm"],
            "53.2%"
        );
        assert_eq!(
            dossier["balance_sheet_solvency_mrq"]["current_ratio_mrq"],
            2.3
        );
        assert_eq!(
            dossier["analyst_estimates_and_sentiment"]["recommendation"],
            "buy"
        );
    }

    #[test]
    fn test_extract_upcoming_earnings() {
        let sample_raw = json!({
            "calendarEvents": {
                "earnings": {
                    "earningsDate": [
                        { "fmt": "2026-08-26", "raw": 1787774400 }
                    ],
                    "isEarningsDateEstimate": false,
                    "earningsAverage": { "fmt": "2.08", "raw": 2.08 },
                    "earningsLow": { "fmt": "2.03", "raw": 2.03 },
                    "earningsHigh": { "fmt": "2.20", "raw": 2.20 },
                    "revenueAverage": { "fmt": "91.82B", "raw": 91821907760.0 },
                    "revenueLow": { "fmt": "90.3B", "raw": 90302000000.0 },
                    "revenueHigh": { "fmt": "96.66B", "raw": 96655000000.0 }
                }
            }
        });

        let earnings = extract_upcoming_earnings(&sample_raw);

        assert_eq!(earnings["next_earnings_date"], "2026-08-26");
        assert_eq!(earnings["is_date_estimate"], false);
        assert_eq!(earnings["eps_estimates"]["average"], 2.08);
        assert_eq!(earnings["eps_estimates"]["low"], 2.03);
        assert_eq!(earnings["eps_estimates"]["high"], 2.20);
        assert_eq!(earnings["revenue_estimates"]["average"], "91.82B");
        assert_eq!(earnings["revenue_estimates"]["low"], "90.3B");
        assert_eq!(earnings["revenue_estimates"]["high"], "96.66B");
    }

    #[test]
    fn test_extract_upcoming_earnings_null_fallback() {
        let empty_raw = json!({});
        let earnings = extract_upcoming_earnings(&empty_raw);

        assert!(earnings["next_earnings_date"].is_null());
        assert!(earnings["is_date_estimate"].is_null());
        assert!(earnings["eps_estimates"]["average"].is_null());
        assert!(earnings["revenue_estimates"]["average"].is_null());
    }

    #[test]
    fn test_extract_upcoming_earnings_real_nvda_payload() {
        let nvda_raw = json!({
            "calendarEvents": {
                "dividendDate": {
                    "fmt": "2026-06-26",
                    "raw": 1782432000
                },
                "earnings": {
                    "earningsAverage": {
                        "fmt": "2.08",
                        "raw": 2.08225
                    },
                    "earningsCallDate": [
                        {
                            "fmt": "2026-08-26",
                            "raw": 1787778000
                        }
                    ],
                    "earningsDate": [
                        {
                            "fmt": "2026-08-26",
                            "raw": 1787774400
                        }
                    ],
                    "earningsHigh": {
                        "fmt": "2.20",
                        "raw": 2.2
                    },
                    "earningsLow": {
                        "fmt": "2.03",
                        "raw": 2.03128
                    },
                    "isEarningsDateEstimate": false,
                    "revenueAverage": {
                        "fmt": "91.82B",
                        "longFmt": "91,821,907,760",
                        "raw": 91821907760.0
                    },
                    "revenueHigh": {
                        "fmt": "96.66B",
                        "longFmt": "96,655,000,000",
                        "raw": 96655000000.0
                    },
                    "revenueLow": {
                        "fmt": "90.3B",
                        "longFmt": "90,302,000,000",
                        "raw": 90302000000.0
                    }
                },
                "exDividendDate": {
                    "fmt": "2026-06-04",
                    "raw": 1780531200
                },
                "maxAge": 1
            }
        });

        let earnings = extract_upcoming_earnings(&nvda_raw);

        assert_eq!(earnings["next_earnings_date"], "2026-08-26");
        assert_eq!(earnings["is_date_estimate"], false);
        assert_eq!(earnings["eps_estimates"]["average"], 2.08225);
        assert_eq!(earnings["eps_estimates"]["low"], 2.03128);
        assert_eq!(earnings["eps_estimates"]["high"], 2.2);
        assert_eq!(earnings["revenue_estimates"]["average"], "91.82B");
        assert_eq!(earnings["revenue_estimates"]["low"], "90.3B");
        assert_eq!(earnings["revenue_estimates"]["high"], "96.66B");
    }

    #[test]
    fn test_format_volume() {
        assert_eq!(format_volume(14_881_400.0), "14.9M");
        assert_eq!(format_volume(1_500_000_000.0), "1.5B");
        assert_eq!(format_volume(500_000.0), "500.0K");
        assert_eq!(format_volume(250.0), "250");
    }

    #[test]
    fn test_timestamp_to_yyyymm() {
        assert_eq!(timestamp_to_yyyymm(1627776000), "2021-08");
    }

    #[test]
    fn test_append_history_to_dossier() {
        let mut dossier = json!({
            "ticker": "TSM",
            "valuation": {}
        });

        let chart_data = json!({
            "meta": {
                "fiftyTwoWeekHigh": 500.0
            },
            "timestamp": [1627776000, 1659312000, 1782826200],
            "indicators": {
                "quote": [
                    {
                        "open": [100.0, 150.0, 380.0],
                        "high": [110.0, 160.0, 400.0],
                        "low": [95.0, 140.0, 370.0],
                        "close": [105.0, 155.0, 395.0],
                        "volume": [10000000.0, 15000000.0, 20000000.0]
                    }
                ]
            }
        });

        append_history_to_dossier(&mut dossier, &chart_data);

        assert!(dossier.get("price_history_monthly").is_some());
        let history = dossier["price_history_monthly"].as_array().unwrap();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0]["date"], "2021-08");
        assert_eq!(history[0]["close"], 105.0);
        assert_eq!(history[0]["vol"], "10.0M");

        let perf = &dossier["performance_summary"];
        assert_eq!(perf["drawdown_from_52w_high"], "-21.0%");
        // Returns compare last element (395.0) to first (100.0 if n<13) => +276.2%
        assert_eq!(perf["one_year_return"], "+276.2%");
    }

    #[test]
    fn test_append_history_to_dossier_with_nulls() {
        let mut dossier = json!({"ticker": "NVDA"});

        let chart_data = json!({
            "meta": {
                "fiftyTwoWeekHigh": 200.0
            },
            "timestamp": [1627776000, 1659312000],
            "indicators": {
                "quote": [
                    {
                        "open": [100.0, null],
                        "high": [110.0, null],
                        "low": [90.0, null],
                        "close": [105.0, null],
                        "volume": [500000.0, null]
                    }
                ]
            }
        });

        append_history_to_dossier(&mut dossier, &chart_data);

        let history = dossier["price_history_monthly"].as_array().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0]["close"], 105.0);
    }
}
