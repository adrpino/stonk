use anyhow::{Result, bail};
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, HeaderMap, HeaderValue, USER_AGENT};
use serde_json::Value;

pub fn create_client() -> Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36",
        ),
    );
    headers.insert(
        ACCEPT,
        HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8",
        ),
    );
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));

    let client = reqwest::Client::builder()
        .cookie_store(true)
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::limited(10))
        .default_headers(headers)
        .build()?;

    Ok(client)
}

pub async fn fetch_yahoo_summary(client: &reqwest::Client, ticker: &str) -> Result<Value> {
    // 1. Initial request to obtain session cookie
    let _ = client.get("https://fc.yahoo.com").send().await;

    // 2. Obtain crumb string
    let crumb_res = client
        .get("https://query1.finance.yahoo.com/v1/test/getcrumb")
        .send()
        .await?;

    let crumb = crumb_res.text().await?;
    let crumb = crumb.trim();

    // 3. Request quoteSummary using crumb and cookie
    let modules = "summaryDetail,financialData,defaultKeyStatistics,earningsHistory,calendarEvents";
    let url = if !crumb.is_empty() && !crumb.contains('<') {
        format!(
            "https://query1.finance.yahoo.com/v10/finance/quoteSummary/{}?modules={}&crumb={}",
            ticker.to_uppercase(),
            modules,
            crumb
        )
    } else {
        format!(
            "https://query2.finance.yahoo.com/v10/finance/quoteSummary/{}?modules={}",
            ticker.to_uppercase(),
            modules
        )
    };

    let res = client.get(&url).send().await?.json::<Value>().await?;

    let result = &res["quoteSummary"]["result"][0];
    if result.is_null() {
        bail!("No data found for ticker: {}", ticker);
    }

    Ok(result.clone())
}

pub async fn fetch_yahoo_chart(
    client: &reqwest::Client,
    ticker: &str,
    range: &str,
) -> Result<Value> {
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?range={}&interval=1mo",
        ticker.to_uppercase(),
        range
    );

    let res = client.get(&url).send().await?.json::<Value>().await?;

    let result = &res["chart"]["result"][0];
    if result.is_null() {
        bail!("No chart data found for ticker: {}", ticker);
    }

    Ok(result.clone())
}
