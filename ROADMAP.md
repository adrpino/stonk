# `stonk` - Feature Specification: Historical Pricing & Financials (`--history`)

## Overview
This document outlines the architectural specification for adding historical pricing data and multi-year financial trends to the `stonk` CLI.

The primary goal is to provide **token-efficient, multi-period pricing trends** for LLMs (AI analysis) without overwhelming prompt token budgets.

---

## 1. CLI Interface Additions

### New Flag: `--history [RANGE]` / `-H [RANGE]`
Allows fetching historical candle data aggregated monthly or weekly:

```bash
# Fetch 5 years of monthly historical data alongside valuation dossier
stonk TSM --history 5y

# Combine with AI prompt mode
stonk MU --history 5y --ai | opencode "Analyze this 5-year price evolution and valuation"

# Specify custom range (default: 1y if no range provided)
stonk META -h 1y
stonk NVDA -h 10y
```

### Supported Time Ranges
* `1y` (12 monthly data points)
* `2y` (24 monthly data points)
* `5y` (60 monthly data points)
* `10y` (120 monthly data points)
* `max` (Complete IPO history)

---

## 2. Yahoo Finance Endpoint Integration

### Chart & Historical Candles Endpoint
* **URL:** `https://query1.finance.yahoo.com/v8/finance/chart/{TICKER}?range={RANGE}&interval=1mo`
* **Response Payload:** Contains parallel arrays (`timestamp`, `open`, `high`, `low`, `close`, `volume`).

### Sample Yahoo Chart Response Structure
```json
{
  "chart": {
    "result": [
      {
        "meta": {
          "currency": "USD",
          "symbol": "TSM",
          "fiftyTwoWeekHigh": 479.0,
          "fiftyTwoWeekLow": 223.7
        },
        "timestamp": [1782739800, 1782826200],
        "indicators": {
          "quote": [
            {
              "open": [437.0, 455.6],
              "high": [456.1, 479.0],
              "low": [431.1, 453.0],
              "close": [455.1, 477.5],
              "volume": [14881400, 15074300]
            }
          ]
        }
      }
    ]
  }
}
```

---

## 3. Data Downsampling & Token Optimization for AI

To avoid wasting context tokens when sending historical data to LLMs:

1. **Monthly Aggregation:** By requesting `interval=1mo`, a 5-year history consumes only ~60 data points (< 500 tokens).
2. **Compact Record Formatting:**
   ```json
   "price_history_monthly": [
     { "date": "2021-08", "open": 118.2, "high": 122.8, "low": 108.6, "close": 118.2, "vol": "14.8M" },
     { "date": "2022-10", "open": 68.0,  "high": 73.7,  "low": 59.5,  "close": 61.5,  "vol": "22.1M" },
     { "date": "2026-07", "open": 388.0, "high": 391.2, "low": 373.9, "close": 377.7, "vol": "9.4M" }
   ]
   ```
3. **Calculated Performance Summary Header:**
   ```json
   "performance_summary": {
     "one_year_return": "+12.4%",
     "five_year_return": "+185.2%",
     "drawdown_from_52w_high": "-21.1%"
   }
   ```

---

## 4. Module Architecture in Rust

### `src/types.rs`
Update `Args` struct to include:
```rust
#[arg(short = 'h', long, value_name = "RANGE", num_args = 0..=1, default_missing_value = "1y")]
pub history: Option<String>,
```

### `src/api.rs`
Add `fetch_yahoo_chart(ticker: &str, range: &str)`:
```rust
pub async fn fetch_yahoo_chart(ticker: &str, range: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?range={}&interval=1mo",
        ticker.to_uppercase(),
        range
    );
    let client = reqwest::Client::new();
    let res = client.get(&url).send().await?.json::<Value>().await?;
    Ok(res)
}
```

### `src/dossier.rs`
Add `append_history_to_dossier(dossier: &mut Value, chart_data: &Value)` to attach downsampled monthly candles and summary metrics.

---

## 5. Next Steps for Implementation
1. Add `chrono` dependency or epoch converter for date formatting (`YYYY-MM`).
2. Add `--history` parsing to `Args`.
3. Integrate `fetch_yahoo_chart` into `main.rs`.
4. Add unit test for chart parsing and downsampling in `dossier.rs`.
