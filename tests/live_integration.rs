use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

#[test]
#[ignore = "live network integration test"]
fn test_live_quote_apple_with_history() {
    let output = Command::cargo_bin("stonk")
        .unwrap()
        .args(["quote", "AAPL", "-H", "1y", "--compact"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command failed: {:?}", output);
    let v: Value = serde_json::from_slice(&output.stdout).expect("Valid JSON output");

    assert_eq!(v["ticker"], "AAPL");
    assert!(
        v["valuation"]["trailing_pe_ttm"].is_number()
            || v["valuation"]["forward_pe_next_1y"].is_number(),
        "Valuation P/E metrics must be numeric"
    );
    assert!(
        v["cashflow_ttm"]["operating_cashflow_ttm"].is_string(),
        "Operating cash flow must be present"
    );

    let history = v["price_history_monthly"]
        .as_array()
        .expect("price_history_monthly array");
    assert!(
        history.len() >= 12,
        "Expected at least 12 monthly candles for 1y, got {}",
        history.len()
    );

    let candle = &history[0];
    assert!(candle["open"].is_number(), "Open price must be numeric");
    assert!(candle["high"].is_number(), "High price must be numeric");
    assert!(candle["low"].is_number(), "Low price must be numeric");
    assert!(candle["close"].is_number(), "Close price must be numeric");
    assert!(
        candle["date"].is_string(),
        "Candle date must be a formatted string"
    );
    assert!(
        v["performance_summary"]["one_year_return"].is_string(),
        "1-year return must be present"
    );
}

#[test]
#[ignore = "live network integration test"]
fn test_live_quote_shortcut() {
    let output = Command::cargo_bin("stonk")
        .unwrap()
        .args(["TSM", "--compact"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command failed: {:?}", output);
    let v: Value = serde_json::from_slice(&output.stdout).expect("Valid JSON output");

    assert_eq!(v["ticker"], "TSM");
    assert!(
        v["valuation"]["market_cap"].is_string(),
        "Market cap must be a string"
    );
    assert!(
        v["profitability_margins_ttm"]["operating_margins_ttm"].is_string(),
        "Operating margins must be present"
    );
    assert!(
        v["balance_sheet_solvency_mrq"]["total_cash_mrq"].is_string(),
        "Total cash must be present"
    );
}

#[test]
#[ignore = "live network integration test"]
fn test_live_quote_ai_mode() {
    let output = Command::cargo_bin("stonk")
        .unwrap()
        .args(["quote", "NVDA", "--ai"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command failed: {:?}", output);
    let stdout_str = String::from_utf8_lossy(&output.stdout);

    assert!(stdout_str.contains("### FINANCIAL DOSSIER FOR NVDA"));
    assert!(stdout_str.contains("```json"));
    assert!(stdout_str.contains("\"ticker\": \"NVDA\""));
}

#[test]
#[ignore = "live network integration test"]
fn test_live_sec_edgar_nvidia_10q() {
    let output = Command::cargo_bin("stonk")
        .unwrap()
        .args(["sec", "NVDA", "10-Q", "--compact"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command failed: {:?}", output);
    let v: Value = serde_json::from_slice(&output.stdout).expect("Valid JSON output");

    assert_eq!(v["ticker"], "NVDA");
    assert_eq!(v["sec_cik"], "0001045810");
    assert_eq!(v["form"], "10-Q");
    assert_eq!(v["requested_form"], "10-Q");
    assert!(
        v["accession_number"]
            .as_str()
            .unwrap()
            .starts_with("0001045810-"),
        "Accession number must start with NVIDIA CIK prefix"
    );
    assert!(
        v["filing_date"].as_str().unwrap().len() == 10,
        "Filing date must be YYYY-MM-DD"
    );

    let doc_url = v["document_url"].as_str().unwrap();
    assert!(
        doc_url.starts_with("https://www.sec.gov/Archives/edgar/data/1045810/"),
        "Document URL must point to SEC archive: {}",
        doc_url
    );
    assert!(doc_url.ends_with(".htm"), "Document must be an HTML report");
}

#[test]
#[ignore = "live network integration test"]
fn test_live_sec_edgar_form8k() {
    let output = Command::cargo_bin("stonk")
        .unwrap()
        .args(["sec", "TSLA", "8-K", "--compact"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command failed: {:?}", output);
    let v: Value = serde_json::from_slice(&output.stdout).expect("Valid JSON output");

    assert_eq!(v["ticker"], "TSLA");
    assert_eq!(v["form"], "8-K");
    assert!(
        v["document_url"]
            .as_str()
            .unwrap()
            .starts_with("https://www.sec.gov/")
    );
}

#[test]
#[ignore = "live network integration test"]
fn test_live_sec_edgar_apple_10k_parse() {
    let output = Command::cargo_bin("stonk")
        .unwrap()
        .args(["sec", "AAPL", "10-K", "--parse", "--compact"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command failed: {:?}", output);
    let v: Value = serde_json::from_slice(&output.stdout).expect("Valid JSON output");

    assert_eq!(v["ticker"], "AAPL");
    assert_eq!(v["form"], "10-K");

    let parsed = &v["parsed_content"];
    assert!(
        parsed.is_object(),
        "parsed_content must be present when --parse is passed"
    );

    let tables = parsed["financial_tables"]
        .as_array()
        .expect("financial_tables array");
    assert!(
        !tables.is_empty(),
        "Must parse at least 1 financial table from Apple 10-K"
    );
    assert!(
        tables[0].as_str().unwrap().contains('|'),
        "Table must be formatted in markdown syntax"
    );
}

#[test]
#[ignore = "live network integration test"]
fn test_live_transcript_apple() {
    let output = Command::cargo_bin("stonk")
        .unwrap()
        .args(["transcript", "AAPL", "-q", "Q3", "-y", "2026", "--compact"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command failed: {:?}", output);
    let v: Value = serde_json::from_slice(&output.stdout).expect("Valid JSON output");

    assert_eq!(v["ticker"], "AAPL");
    assert_eq!(v["quarter"], "Q3");
    assert_eq!(v["fiscal_year"], 2026);

    let chunks = v["chunks"].as_array().expect("chunks array");
    assert!(!chunks.is_empty(), "Transcript must contain speech chunks");
    assert!(
        chunks[0]["content"].is_string(),
        "Chunk must contain text content"
    );
    assert!(
        v["formatted_markdown"]
            .as_str()
            .unwrap()
            .contains("# EARNINGS CALL TRANSCRIPT"),
        "Formatted markdown must contain header"
    );
}

#[test]
#[ignore = "live network integration test"]
fn test_live_chart_apple_1mo() {
    let output = Command::cargo_bin("stonk")
        .unwrap()
        .args(["chart", "AAPL"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command failed: {:?}", output);
    let stdout_str = String::from_utf8_lossy(&output.stdout);

    assert!(stdout_str.contains("AAPL · USD"));
    assert!(stdout_str.contains("High:"));
    assert!(stdout_str.contains("Low:"));
    assert!(stdout_str.contains("Interval: 1d"));
    assert!(stdout_str.contains('┤') || stdout_str.contains('─'));
}

#[test]
#[ignore = "live network integration test"]
fn test_live_chart_nvda_5d() {
    let output = Command::cargo_bin("stonk")
        .unwrap()
        .args(["chart", "NVDA", "5d", "-i", "15m"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command failed: {:?}", output);
    let stdout_str = String::from_utf8_lossy(&output.stdout);

    assert!(stdout_str.contains("NVDA · USD"));
    assert!(stdout_str.contains("Interval: 15m"));
    assert!(stdout_str.contains('┤') || stdout_str.contains('─'));
}

#[test]
#[ignore = "live network integration test"]
fn test_live_bond_company_name_apple() {
    let output = Command::cargo_bin("stonk")
        .unwrap()
        .args(["bond", "Apple", "--compact"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command failed: {:?}", output);
    let bonds: Vec<Value> = serde_json::from_slice(&output.stdout).expect("Array of bond quotes");

    assert!(
        bonds.len() >= 10,
        "Expected at least 10 active bonds for Apple, got {}",
        bonds.len()
    );

    for bond in &bonds {
        let isin = bond["isin_or_cusip"].as_str().expect("ISIN string");
        assert!(isin.len() >= 9, "ISIN must be valid: {}", isin);

        let issuer = bond["issuer"].as_str().expect("Issuer string");
        assert!(
            issuer.to_lowercase().contains("apple"),
            "Issuer must belong to Apple: {}",
            issuer
        );

        assert!(
            bond["coupon"].is_string() || bond["maturity_date"].is_string(),
            "Bond must have either coupon or maturity date: {:?}",
            bond
        );
        assert_eq!(bond["source"], "Markets Insider");
    }
}

#[test]
#[ignore = "live network integration test"]
fn test_live_bond_ticker_meta() {
    let output = Command::cargo_bin("stonk")
        .unwrap()
        .args(["bond", "META", "--compact"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command failed: {:?}", output);
    let bonds: Vec<Value> = serde_json::from_slice(&output.stdout).expect("Array of bond quotes");

    assert!(
        bonds.len() >= 10,
        "Expected at least 10 active bonds for Meta Platforms, got {}",
        bonds.len()
    );
    assert_eq!(bonds[0]["issuer"], "Meta Platforms Inc.");
}

#[test]
#[ignore = "live network integration test"]
fn test_live_bond_direct_url_lookup() {
    let output = Command::cargo_bin("stonk")
        .unwrap()
        .args([
            "bond",
            "https://markets.businessinsider.com/bonds/apple_incdl-notes_201717-27-bond-2027-us037833db33",
            "--compact",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command failed: {:?}", output);
    let bonds: Vec<Value> = serde_json::from_slice(&output.stdout).expect("Array of bond quotes");

    assert_eq!(bonds.len(), 1, "Direct URL must return exactly 1 bond");
    assert_eq!(bonds[0]["isin_or_cusip"], "US037833DB33");
    assert_eq!(bonds[0]["coupon"], "2.9000%");
    assert_eq!(bonds[0]["yield_to_maturity"], "4.32%");
    assert_eq!(bonds[0]["maturity_date"], "9/12/2027");
    assert!(
        bonds[0]["price"].as_f64().expect("numeric price") > 90.0,
        "Price must be parsed as a float"
    );
    assert_eq!(bonds[0]["source"], "Markets Insider");
}

#[test]
#[ignore = "live network integration test"]
fn test_live_invalid_ticker_returns_clean_error() {
    let mut cmd = Command::cargo_bin("stonk").unwrap();
    cmd.args(["sec", "NONEXISTENT_TICKER_XYZ999"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Could not resolve SEC CIK for ticker",
        ));
}
