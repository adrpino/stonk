use anyhow::Result;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct BondQuote {
    pub isin_or_cusip: String,
    pub issuer: String,
    pub coupon: Option<String>,
    pub price: Option<f64>,
    pub yield_to_maturity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moodys_rating: Option<String>,
    pub maturity_date: Option<String>,
    pub source: String,
}

pub fn extract_isin_from_input(s: &str) -> String {
    if s.starts_with("http://") || s.starts_with("https://") {
        let trimmed = s.trim_end_matches('/');
        if let Some(pos) = trimmed.rfind('-') {
            let candidate = &trimmed[pos + 1..];
            if candidate.len() >= 9 && candidate.chars().all(|c| c.is_alphanumeric()) {
                return candidate.to_uppercase();
            }
        }
    }
    s.to_uppercase()
}

pub fn is_cusip_or_isin(s: &str) -> bool {
    let clean = s.trim();
    (clean.len() == 12 || clean.len() == 9) && clean.chars().all(|c| c.is_alphanumeric())
}

pub fn parse_borrower_options(html: &str) -> Vec<(String, String)> {
    let mut options = Vec::new();
    let document = Html::parse_document(html);

    let select_sel = match Selector::parse("select#bond-search-borrower option") {
        Ok(s) => s,
        Err(_) => return options,
    };

    for option in document.select(&select_sel) {
        if let Some(val) = option.value().attr("value") {
            let val_clean = val.trim();
            if !val_clean.is_empty() {
                let text = option
                    .text()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string();
                if !text.is_empty() && text != "All" {
                    options.push((val_clean.to_string(), text));
                }
            }
        }
    }

    options
}

pub fn match_borrower_id(options: &[(String, String)], query: &str) -> Option<String> {
    let clean_q = query.replace([',', '.'], "").trim().to_lowercase();
    if clean_q.is_empty() {
        return None;
    }

    // 1. Exact match (without punctuation)
    for (id, name) in options {
        let clean_n = name.replace([',', '.'], "").to_lowercase();
        if clean_n == clean_q {
            return Some(id.clone());
        }
    }

    // 2. Starts with query (e.g. "Apple" starts with "Apple Inc")
    for (id, name) in options {
        let clean_n = name.replace([',', '.'], "").to_lowercase();
        if clean_n.starts_with(&clean_q) || clean_q.starts_with(&clean_n) {
            return Some(id.clone());
        }
    }

    // 3. First significant word match (e.g. "Meta" in "Meta Platforms Inc")
    let first_word = clean_q.split_whitespace().next().unwrap_or("");
    if !first_word.is_empty() {
        for (id, name) in options {
            let clean_n = name.replace([',', '.'], "").to_lowercase();
            let first_word_n = clean_n.split_whitespace().next().unwrap_or("");
            if first_word_n == first_word {
                return Some(id.clone());
            }
        }
    }

    // 4. Substring match
    for (id, name) in options {
        let clean_n = name.replace([',', '.'], "").to_lowercase();
        if clean_n.contains(&clean_q) {
            return Some(id.clone());
        }
    }

    None
}

fn candidate_to_isin(s: &str) -> String {
    let clean = s.trim_end_matches('/');
    if clean.len() >= 9 && clean.chars().all(|c| c.is_alphanumeric()) {
        clean.to_uppercase()
    } else {
        "N/A".to_string()
    }
}

pub fn parse_markets_insider_finder_table(html: &str) -> Vec<BondQuote> {
    let mut results = Vec::new();
    let document = Html::parse_document(html);

    let tr_selector = match Selector::parse("tbody.table__tbody tr.table__tr") {
        Ok(s) => s,
        Err(_) => return results,
    };
    let td_selector = match Selector::parse("td.table__td") {
        Ok(s) => s,
        Err(_) => return results,
    };
    let a_selector = match Selector::parse("a") {
        Ok(s) => s,
        Err(_) => return results,
    };

    for tr in document.select(&tr_selector) {
        let tds: Vec<_> = tr.select(&td_selector).collect();
        if tds.len() >= 6 {
            let a_opt = tds[0].select(&a_selector).next();
            if let Some(a) = a_opt {
                let name = a.text().collect::<Vec<_>>().join(" ").trim().to_string();
                let href = a.value().attr("href").unwrap_or("");

                let isin = if let Some(pos) = href.rfind('-') {
                    candidate_to_isin(&href[pos + 1..])
                } else {
                    "N/A".to_string()
                };

                let coupon = tds
                    .get(2)
                    .map(|td| td.text().collect::<Vec<_>>().join(" ").trim().to_string())
                    .filter(|s| !s.is_empty() && s != "-");

                let yield_to_maturity = tds
                    .get(3)
                    .map(|td| td.text().collect::<Vec<_>>().join(" ").trim().to_string())
                    .filter(|s| !s.is_empty() && s != "-");

                let moodys_rating = tds
                    .get(4)
                    .map(|td| td.text().collect::<Vec<_>>().join(" ").trim().to_string())
                    .filter(|s| !s.is_empty() && s != "-");

                let maturity_date = tds
                    .get(5)
                    .map(|td| td.text().collect::<Vec<_>>().join(" ").trim().to_string())
                    .filter(|s| !s.is_empty() && s != "-");

                let price = tds.get(6).and_then(|td| {
                    td.text()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .trim()
                        .parse::<f64>()
                        .ok()
                });

                if isin != "N/A" {
                    results.push(BondQuote {
                        isin_or_cusip: isin,
                        issuer: name,
                        coupon,
                        price,
                        yield_to_maturity,
                        moodys_rating,
                        maturity_date,
                        source: "Markets Insider".to_string(),
                    });
                }
            }
        }
    }

    results
}

pub fn parse_markets_insider_single_html(isin_input: &str, html: &str) -> Option<BondQuote> {
    if html.is_empty() {
        return None;
    }

    let isin = extract_isin_from_input(isin_input);
    let document = Html::parse_document(html);

    let title_text = Selector::parse("title")
        .ok()
        .and_then(|sel| document.select(&sel).next())
        .map(|elem| elem.text().collect::<Vec<_>>().join(""));

    let issuer = if let Some(text) = title_text {
        let end_opt = text
            .find(" Bond | Markets Insider")
            .or_else(|| text.find(" Bond"));
        if let Some(end) = end_opt {
            text[..end].trim().to_string()
        } else {
            "Corporate Issuer".to_string()
        }
    } else {
        "Corporate Issuer".to_string()
    };

    let lower = html.to_lowercase();
    let mut coupon = None;
    let mut price = None;
    let mut yield_to_maturity = None;
    let mut maturity_date = None;

    if let Some(pos) = lower.find("offers a coupon of ") {
        let rest = &html[pos + "offers a coupon of ".len()..];
        if let Some(end) = rest.find('%') {
            coupon = Some(format!("{}%", &rest[..end].trim()));
        }
    }

    if let Some(pos) = lower.find("current price of ") {
        let rest = &html[pos + "current price of ".len()..];
        let rest_lower = &lower[pos + "current price of ".len()..];
        if let Some(end) = rest_lower.find(" usd") {
            let snippet = rest[..end].trim();
            if let Ok(p) = snippet.parse::<f64>() {
                price = Some(p);
            }
        }
    }

    if let Some(pos) = lower.find("annual yield of ") {
        let rest = &html[pos + "annual yield of ".len()..];
        if let Some(end) = rest.find('%') {
            yield_to_maturity = Some(format!("{}%", &rest[..end].trim()));
        }
    }

    if let Some(pos) = lower.find("maturity date of ") {
        let rest = &html[pos + "maturity date of ".len()..];
        let end = rest.find(' ').unwrap_or(rest.len());
        maturity_date = Some(rest[..end].trim().to_string());
    }

    if price.is_some() || coupon.is_some() || yield_to_maturity.is_some() {
        Some(BondQuote {
            isin_or_cusip: isin,
            issuer,
            coupon,
            price,
            yield_to_maturity,
            moodys_rating: None,
            maturity_date,
            source: "Markets Insider".to_string(),
        })
    } else {
        None
    }
}

pub fn deduplicate_and_sort_bonds(quotes: Vec<BondQuote>) -> Vec<BondQuote> {
    use std::collections::BTreeMap;

    let mut map: BTreeMap<String, BondQuote> = BTreeMap::new();

    for quote in quotes {
        let key = quote.isin_or_cusip.to_uppercase();
        if let Some(existing) = map.get(&key) {
            let should_replace = existing.price.is_none() && quote.price.is_some();
            if should_replace {
                map.insert(key, quote);
            }
        } else {
            map.insert(key, quote);
        }
    }

    map.into_values().collect()
}

pub async fn resolve_company_name(client: &reqwest::Client, query: &str) -> Option<String> {
    let url = format!(
        "https://query2.finance.yahoo.com/v1/finance/search?q={}&quotesCount=1",
        query
    );
    let res = client
        .get(&url)
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        )
        .send()
        .await
        .ok()?;

    let json_val: serde_json::Value = res.json().await.ok()?;
    let quotes = json_val.get("quotes")?.as_array()?;
    if let Some(first) = quotes.first() {
        if let Some(shortname) = first.get("shortname").and_then(|s| s.as_str()) {
            return Some(shortname.to_string());
        }
        if let Some(longname) = first.get("longname").and_then(|s| s.as_str()) {
            return Some(longname.to_string());
        }
    }
    None
}

pub async fn fetch_bonds(client: &reqwest::Client, raw_query: &str) -> Result<Vec<BondQuote>> {
    let clean = raw_query.trim();

    // 1. Direct bond URL lookup
    if clean.starts_with("http://") || clean.starts_with("https://") {
        let res = client
            .get(clean)
            .header(
                reqwest::header::USER_AGENT,
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
            )
            .send()
            .await?;
        let body = res.text().await?;
        if let Some(single) = parse_markets_insider_single_html(clean, &body) {
            return Ok(vec![single]);
        }
        let table_quotes = parse_markets_insider_finder_table(&body);
        if !table_quotes.is_empty() {
            return Ok(table_quotes);
        }
    }

    // 2. Direct ISIN / CUSIP lookup
    if is_cusip_or_isin(clean) {
        let search_url = format!(
            "https://markets.businessinsider.com/bonds/finder?search={}",
            clean
        );
        let res = client
            .get(&search_url)
            .header(
                reqwest::header::USER_AGENT,
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
            )
            .send()
            .await?;
        let body = res.text().await?;

        let table_quotes = parse_markets_insider_finder_table(&body);
        let matching: Vec<_> = table_quotes
            .into_iter()
            .filter(|b| b.isin_or_cusip.eq_ignore_ascii_case(clean))
            .collect();
        if !matching.is_empty() {
            return Ok(matching);
        }

        if let Some(single) = parse_markets_insider_single_html(clean, &body) {
            return Ok(vec![single]);
        }
    }

    // 3. Company Name / Ticker bond search
    let resolved_name = resolve_company_name(client, clean).await;
    let search_term = resolved_name.as_deref().unwrap_or(clean);

    let search_page_url = format!(
        "https://markets.businessinsider.com/bonds/finder?search={}",
        search_term
    );
    let res = client
        .get(&search_page_url)
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        )
        .send()
        .await?;
    let body = res.text().await?;

    let mut options = parse_borrower_options(&body);
    let mut borrower_id =
        match_borrower_id(&options, search_term).or_else(|| match_borrower_id(&options, clean));

    // If still not found, fetch the base finder page containing global borrowers
    if borrower_id.is_none()
        && let Ok(base_res) = client
            .get("https://markets.businessinsider.com/bonds/finder")
            .header(
                reqwest::header::USER_AGENT,
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
            )
            .send()
            .await
        && let Ok(base_body) = base_res.text().await
    {
        options = parse_borrower_options(&base_body);
        borrower_id =
            match_borrower_id(&options, search_term).or_else(|| match_borrower_id(&options, clean));
    }

    if let Some(id) = borrower_id {
        let mut all_bonds = Vec::new();

        // Fetch page 1
        let p1_url = format!(
            "https://markets.businessinsider.com/bonds/finder?borrower={}&p=1",
            id
        );
        if let Ok(p1_res) = client
            .get(&p1_url)
            .header(
                reqwest::header::USER_AGENT,
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
            )
            .send()
            .await
            && let Ok(p1_body) = p1_res.text().await
        {
            let p1_quotes = parse_markets_insider_finder_table(&p1_body);
            let p1_len = p1_quotes.len();
            all_bonds.extend(p1_quotes);

            // If page 1 was full (20 items), fetch page 2 as well
            if p1_len >= 20 {
                let p2_url = format!(
                    "https://markets.businessinsider.com/bonds/finder?borrower={}&p=2",
                    id
                );
                if let Ok(p2_res) = client
                    .get(&p2_url)
                    .header(
                        reqwest::header::USER_AGENT,
                        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
                    )
                    .send()
                    .await
                    && let Ok(p2_body) = p2_res.text().await
                {
                    let p2_quotes = parse_markets_insider_finder_table(&p2_body);
                    all_bonds.extend(p2_quotes);
                }
            }
        }

        if !all_bonds.is_empty() {
            return Ok(deduplicate_and_sort_bonds(all_bonds));
        }
    }

    // Fallback: parse direct table from initial search page
    let direct_quotes = parse_markets_insider_finder_table(&body);
    Ok(deduplicate_and_sort_bonds(direct_quotes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_borrower_options() {
        let sample_html = r#"
            <select id="bond-search-borrower" name="borrower">
                <option value="">All</option>
                <option value="20821">Apple Inc.</option>
                <option value="67415">Meta Platforms Inc.</option>
                <option value="41909">Microsoft Corp.</option>
            </select>
        "#;
        let options = parse_borrower_options(sample_html);
        assert_eq!(options.len(), 3);
        assert_eq!(options[0], ("20821".to_string(), "Apple Inc.".to_string()));
        assert_eq!(
            options[1],
            ("67415".to_string(), "Meta Platforms Inc.".to_string())
        );
    }

    #[test]
    fn test_match_borrower_id() {
        let options = vec![
            ("20821".to_string(), "Apple Inc.".to_string()),
            ("67415".to_string(), "Meta Platforms Inc.".to_string()),
            ("41909".to_string(), "Microsoft Corp.".to_string()),
            ("111142".to_string(), "NVIDIA Corp.".to_string()),
        ];

        assert_eq!(
            match_borrower_id(&options, "Apple"),
            Some("20821".to_string())
        );
        assert_eq!(
            match_borrower_id(&options, "apple inc."),
            Some("20821".to_string())
        );
        assert_eq!(
            match_borrower_id(&options, "meta"),
            Some("67415".to_string())
        );
        assert_eq!(
            match_borrower_id(&options, "Nvidia"),
            Some("111142".to_string())
        );
        assert_eq!(match_borrower_id(&options, "NonExistentCo"), None);
    }

    #[test]
    fn test_parse_markets_insider_finder_table() {
        let sample_table_html = r#"
            <table class="table">
                <tbody class="table__tbody">
                    <tr class="table__tr">
                        <td class="table__td">
                            <a href="/bonds/apple_incdl-notes_202121-31-bond-2031-us037833ej59">Apple Inc.</a>
                        </td>
                        <td class="table__td text-right">USD</td>
                        <td class="table__td text-right">1.7000%</td>
                        <td class="table__td text-right">4.60%</td>
                        <td class="table__td text-right">Aaa</td>
                        <td class="table__td text-right">8/5/2031</td>
                        <td class="table__td text-right">87.18</td>
                        <td class="table__td text-right">87.76</td>
                    </tr>
                    <tr class="table__tr">
                        <td class="table__td">
                            <a href="/bonds/apple_incdl-notes_201717-27-bond-2027-us037833db33">Apple Inc.</a>
                        </td>
                        <td class="table__td text-right">USD</td>
                        <td class="table__td text-right">2.9000%</td>
                        <td class="table__td text-right">4.32%</td>
                        <td class="table__td text-right">Aaa</td>
                        <td class="table__td text-right">9/12/2027</td>
                        <td class="table__td text-right">98.47</td>
                        <td class="table__td text-right">98.90</td>
                    </tr>
                </tbody>
            </table>
        "#;

        let bonds = parse_markets_insider_finder_table(sample_table_html);
        assert_eq!(bonds.len(), 2);

        assert_eq!(bonds[0].isin_or_cusip, "US037833EJ59");
        assert_eq!(bonds[0].issuer, "Apple Inc.");
        assert_eq!(bonds[0].coupon, Some("1.7000%".to_string()));
        assert_eq!(bonds[0].yield_to_maturity, Some("4.60%".to_string()));
        assert_eq!(bonds[0].moodys_rating, Some("Aaa".to_string()));
        assert_eq!(bonds[0].maturity_date, Some("8/5/2031".to_string()));
        assert_eq!(bonds[0].price, Some(87.18));

        assert_eq!(bonds[1].isin_or_cusip, "US037833DB33");
        assert_eq!(bonds[1].maturity_date, Some("9/12/2027".to_string()));
    }

    #[test]
    fn test_is_cusip_or_isin() {
        assert!(is_cusip_or_isin("US037833DB33"));
        assert!(is_cusip_or_isin("US30303M8B15"));
        assert!(is_cusip_or_isin("037833DB3"));
        assert!(!is_cusip_or_isin("Apple"));
        assert!(!is_cusip_or_isin("AAPL"));
    }
}
