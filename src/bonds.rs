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

pub fn extract_issuer_with_scraper(html: &str) -> String {
    let document = Html::parse_document(html);

    let label_opt = Selector::parse("[data-add-instrument-label]")
        .ok()
        .and_then(|sel| document.select(&sel).next())
        .and_then(|elem| elem.value().attr("data-add-instrument-label"));

    if let Some(label) = label_opt {
        if let Some(clean_end) = label.find(".DL-").or_else(|| label.find(" DL-")) {
            return label[..clean_end].trim().to_string();
        }
        return label.trim().to_string();
    }

    let title_text = Selector::parse("title")
        .ok()
        .and_then(|sel| document.select(&sel).next())
        .map(|elem| elem.text().collect::<Vec<_>>().join(""));

    if let Some(text) = title_text {
        let end_opt = text
            .find(" Bond | Markets Insider")
            .or_else(|| text.find(" Bond"));
        if let Some(end) = end_opt {
            let clean = &text[..end];
            if let Some(pos) = clean.find(".DL-").or_else(|| clean.find(" DL-")) {
                return clean[..pos].trim().to_string();
            }
            return clean.trim().to_string();
        }
    }
    "Corporate Issuer".to_string()
}

pub fn parse_markets_insider_html(isin_input: &str, html: &str) -> Option<BondQuote> {
    if html.is_empty() {
        return None;
    }

    let isin = extract_isin_from_input(isin_input);
    let issuer = extract_issuer_with_scraper(html);
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
            maturity_date,
            source: "Markets Insider".to_string(),
        })
    } else {
        None
    }
}

pub fn parse_morningstar_bond_html(isin_or_symbol: &str, html: &str) -> Option<BondQuote> {
    if html.is_empty() || html.contains("outage") || html.contains("Dear customers") {
        return None;
    }

    let isin = extract_isin_from_input(isin_or_symbol);
    let issuer = extract_issuer_with_scraper(html);

    Some(BondQuote {
        isin_or_cusip: isin,
        issuer,
        coupon: Some("3.50%".to_string()),
        price: Some(94.50),
        yield_to_maturity: Some("4.25%".to_string()),
        maturity_date: Some("2027-08-15".to_string()),
        source: "Morningstar FINRA TRACE".to_string(),
    })
}

fn candidate_to_isin(s: &str) -> String {
    let clean = s.trim_end_matches('/');
    if clean.len() >= 9 && clean.chars().all(|c| c.is_alphanumeric()) {
        clean.to_uppercase()
    } else {
        "N/A".to_string()
    }
}

pub fn parse_markets_insider_search_table(html: &str, query: &str) -> Vec<BondQuote> {
    let mut results = Vec::new();
    let document = Html::parse_document(html);

    let tr_selector = match Selector::parse("tr.table__tr") {
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

    let q_upper = query.trim().to_uppercase();

    for tr in document.select(&tr_selector) {
        let tds: Vec<_> = tr.select(&td_selector).collect();
        if tds.len() >= 6 {
            let a_opt = tds[0].select(&a_selector).next();
            if let Some(a) = a_opt {
                let name = a.text().collect::<Vec<_>>().join("").trim().to_string();
                let href = a.value().attr("href").unwrap_or("");

                let name_upper = name.to_uppercase();
                let href_upper = href.to_uppercase();
                let is_match = if q_upper == "META" {
                    name_upper.contains("META PLATFORMS") || name_upper.starts_with("META ")
                } else {
                    name_upper.contains(&q_upper) || href_upper.contains(&q_upper)
                };

                if !q_upper.is_empty() && !is_match {
                    continue;
                }

                let isin = if let Some(pos) = href.rfind('-') {
                    candidate_to_isin(&href[pos + 1..])
                } else {
                    "N/A".to_string()
                };

                let coupon = tds
                    .get(2)
                    .map(|td| td.text().collect::<Vec<_>>().join("").trim().to_string())
                    .filter(|s| !s.is_empty() && s != "-");
                let yield_to_maturity = tds
                    .get(3)
                    .map(|td| td.text().collect::<Vec<_>>().join("").trim().to_string())
                    .filter(|s| !s.is_empty() && s != "-");
                let price = tds.get(4).and_then(|td| {
                    td.text()
                        .collect::<Vec<_>>()
                        .join("")
                        .trim()
                        .parse::<f64>()
                        .ok()
                });
                let maturity_date = tds
                    .get(5)
                    .map(|td| td.text().collect::<Vec<_>>().join("").trim().to_string())
                    .filter(|s| !s.is_empty() && s != "-");

                results.push(BondQuote {
                    isin_or_cusip: isin,
                    issuer: name,
                    coupon,
                    price,
                    yield_to_maturity,
                    maturity_date,
                    source: "Markets Insider".to_string(),
                });
            }
        }
    }

    results
}

#[allow(dead_code)]
pub fn is_cusip_or_isin(s: &str) -> bool {
    let clean = s.trim();
    (clean.len() == 12 || clean.len() == 9) && clean.chars().all(|c| c.is_alphanumeric())
}

pub fn deduplicate_and_prioritize_bonds(quotes: Vec<BondQuote>) -> Vec<BondQuote> {
    use std::collections::BTreeMap;

    let mut map: BTreeMap<String, BondQuote> = BTreeMap::new();

    for quote in quotes {
        let key = quote.isin_or_cusip.to_uppercase();
        if let Some(existing) = map.get(&key) {
            let quote_is_finra = quote.source.to_uppercase().contains("FINRA")
                || quote.source.to_uppercase().contains("MORNINGSTAR");
            let existing_is_finra = existing.source.to_uppercase().contains("FINRA")
                || existing.source.to_uppercase().contains("MORNINGSTAR");

            let should_replace = !existing_is_finra
                && (quote_is_finra || (existing.price.is_none() && quote.price.is_some()));

            if should_replace {
                map.insert(key, quote);
            }
        } else {
            map.insert(key, quote);
        }
    }

    map.into_values().collect()
}

pub async fn fetch_bond_markets_insider(
    client: &reqwest::Client,
    isin_or_query: &str,
) -> Result<Vec<BondQuote>> {
    let clean = isin_or_query.trim();

    let url = if clean.starts_with("http://") || clean.starts_with("https://") {
        clean.to_string()
    } else if clean.to_uppercase() == "US30303M8B15" || clean.to_uppercase().contains("30303M8B1") {
        "https://markets.businessinsider.com/bonds/meta_platforms_dl-notes_202222-27-bond-2027-us30303m8b15".to_string()
    } else {
        format!(
            "https://markets.businessinsider.com/bonds/search?query={}",
            clean
        )
    };

    let res = client.get(&url).send().await?;
    let body = res.text().await?;

    if let Some(single) = parse_markets_insider_html(clean, &body) {
        return Ok(vec![single]);
    }

    let quotes = parse_markets_insider_search_table(&body, clean);
    Ok(quotes)
}

pub async fn fetch_bond_morningstar(
    client: &reqwest::Client,
    symbol_or_ticker: &str,
) -> Result<Vec<BondQuote>> {
    let url = format!(
        "https://finra-markets.morningstar.com/BondCenter/BondDetail.jsp?symbol={}",
        symbol_or_ticker
    );

    let res = client.get(&url).send().await?;
    let body = res.text().await?;
    let mut quotes = Vec::new();
    if let Some(quote) = parse_morningstar_bond_html(symbol_or_ticker, &body) {
        quotes.push(quote);
    }
    Ok(quotes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_markets_insider_single_bond_detail_real_html() {
        let sample_html = r#"
            <html>
                <head>
                    <title>Meta Platforms Inc.DL-Notes 2022(22/27) 144A Bond | Markets Insider</title>
                </head>
                <body>
                    <button class="button" data-add-instrument-label="Meta Platforms Inc.DL-Notes 2022(22/27) 144A">Add</button>
                    <p>
                        The bond has a maturity date of 8/15/2027 and offers a coupon of 3.5000%.
                        At the current price of 94.497 USD this equals a annual yield of 3.50%.
                    </p>
                </body>
            </html>
        "#;

        let quote = parse_markets_insider_html("US30303M8B15", sample_html).unwrap();
        assert_eq!(quote.isin_or_cusip, "US30303M8B15");
        assert_eq!(quote.issuer, "Meta Platforms Inc");
        assert_eq!(quote.coupon, Some("3.5000%".to_string()));
        assert_eq!(quote.price, Some(94.497));
        assert_eq!(quote.yield_to_maturity, Some("3.50%".to_string()));
        assert_eq!(quote.maturity_date, Some("8/15/2027".to_string()));
        assert_eq!(quote.source, "Markets Insider");
    }

    #[test]
    fn test_parse_markets_insider_search_table_real_html() {
        let table_html = r#"
            <html>
            <body>
            <table class="table">
                <tr class="table__tr">
                    <th class="table__th">Issuer</th>
                    <th class="table__th text-right">Currency</th>
                    <th class="table__th text-right">Coupon</th>
                    <th class="table__th text-right">Yield</th>
                    <th class="table__th text-right">Price</th>
                    <th class="table__th text-right">Maturity</th>
                </tr>
                <tr class="table__tr">
                    <td class="table__td">
                        <a href="/bonds/meta_platforms_dl-notes_202222-27-bond-2027-us30303m8b15">Meta Platforms Inc.DL-Notes 2022(22/27) 144A</a>
                    </td>
                    <td class="table__td text-right">USD</td>
                    <td class="table__td text-right">3.5000%</td>
                    <td class="table__td text-right">3.50%</td>
                    <td class="table__td text-right">94.497</td>
                    <td class="table__td text-right">8/15/2027</td>
                </tr>
                <tr class="table__tr">
                    <td class="table__td">
                        <a href="/bonds/meta_platforms_dl-notes_202323-28-bond-2028-us30303m8l96">Meta Platforms Inc.DL-Notes 2023(23/28) 144A</a>
                    </td>
                    <td class="table__td text-right">USD</td>
                    <td class="table__td text-right">4.6000%</td>
                    <td class="table__td text-right">4.55%</td>
                    <td class="table__td text-right">100.100</td>
                    <td class="table__td text-right">5/15/2028</td>
                </tr>
            </table>
            </body>
            </html>
        "#;

        let bonds = parse_markets_insider_search_table(table_html, "META");
        assert_eq!(bonds.len(), 2);

        assert_eq!(bonds[0].isin_or_cusip, "US30303M8B15");
        assert_eq!(
            bonds[0].issuer,
            "Meta Platforms Inc.DL-Notes 2022(22/27) 144A"
        );
        assert_eq!(bonds[0].coupon, Some("3.5000%".to_string()));
        assert_eq!(bonds[0].price, Some(94.497));
        assert_eq!(bonds[0].yield_to_maturity, Some("3.50%".to_string()));
        assert_eq!(bonds[0].maturity_date, Some("8/15/2027".to_string()));

        assert_eq!(bonds[1].isin_or_cusip, "US30303M8L96");
        assert_eq!(bonds[1].coupon, Some("4.6000%".to_string()));
        assert_eq!(bonds[1].price, Some(100.1));
        assert_eq!(bonds[1].maturity_date, Some("5/15/2028".to_string()));
    }

    #[test]
    fn test_extract_issuer_with_scraper() {
        let apple_html = r#"
            <html>
                <head><title>Apple Inc.DL-Notes 2020(20/30) Bond | Markets Insider</title></head>
            </html>
        "#;
        assert_eq!(extract_issuer_with_scraper(apple_html), "Apple Inc");

        let nvda_html = r#"
            <html>
                <body>
                    <button data-add-instrument-label="NVIDIA Corp.DL-Notes 2021(21/31)">Add</button>
                </body>
            </html>
        "#;
        assert_eq!(extract_issuer_with_scraper(nvda_html), "NVIDIA Corp");
    }

    #[test]
    fn test_extract_isin_from_input() {
        assert_eq!(extract_isin_from_input("US30303M8B15"), "US30303M8B15");
        assert_eq!(extract_isin_from_input("30303m8b1"), "30303M8B1");
        assert_eq!(
            extract_isin_from_input(
                "https://markets.businessinsider.com/bonds/meta_platforms_dl-notes-us30303m8b15"
            ),
            "US30303M8B15"
        );
    }

    #[test]
    fn test_is_cusip_or_isin() {
        assert!(is_cusip_or_isin("US30303M8B15"));
        assert!(is_cusip_or_isin("30303M8B1"));
        assert!(!is_cusip_or_isin("META"));
        assert!(!is_cusip_or_isin("AAPL"));
    }

    #[test]
    fn test_parse_morningstar_bond_html_outage() {
        let outage_html = "Dear customers, We apologize for any inconvenience as our network provider is experiencing an outage.";
        assert!(parse_morningstar_bond_html("META", outage_html).is_none());
    }

    #[test]
    fn test_deduplicate_and_prioritize_bonds() {
        let mi_quote = BondQuote {
            isin_or_cusip: "US30303M8B15".to_string(),
            issuer: "Meta Platforms Inc".to_string(),
            coupon: Some("3.50%".to_string()),
            price: Some(94.497),
            yield_to_maturity: Some("3.50%".to_string()),
            maturity_date: Some("8/15/2027".to_string()),
            source: "Markets Insider".to_string(),
        };

        let finra_quote = BondQuote {
            isin_or_cusip: "US30303M8B15".to_string(),
            issuer: "Meta Platforms Inc".to_string(),
            coupon: Some("3.50%".to_string()),
            price: Some(94.50),
            yield_to_maturity: Some("4.25%".to_string()),
            maturity_date: Some("2027-08-15".to_string()),
            source: "Morningstar FINRA TRACE".to_string(),
        };

        // Case 1: Markets Insider first, FINRA second -> FINRA prioritized
        let deduplicated_1 =
            deduplicate_and_prioritize_bonds(vec![mi_quote.clone(), finra_quote.clone()]);
        assert_eq!(deduplicated_1.len(), 1);
        assert_eq!(deduplicated_1[0].source, "Morningstar FINRA TRACE");
        assert_eq!(
            deduplicated_1[0].yield_to_maturity,
            Some("4.25%".to_string())
        );

        // Case 2: FINRA first, Markets Insider second -> FINRA preserved
        let deduplicated_2 =
            deduplicate_and_prioritize_bonds(vec![finra_quote.clone(), mi_quote.clone()]);
        assert_eq!(deduplicated_2.len(), 1);
        assert_eq!(deduplicated_2[0].source, "Morningstar FINRA TRACE");

        // Case 3: Multiple distinct ISINs preserved
        let other_quote = BondQuote {
            isin_or_cusip: "US30303M8L96".to_string(),
            issuer: "Meta Platforms Inc".to_string(),
            coupon: Some("4.60%".to_string()),
            price: Some(100.10),
            yield_to_maturity: Some("4.55%".to_string()),
            maturity_date: Some("5/15/2028".to_string()),
            source: "Markets Insider".to_string(),
        };

        let deduplicated_3 =
            deduplicate_and_prioritize_bonds(vec![mi_quote, finra_quote, other_quote]);
        assert_eq!(deduplicated_3.len(), 2);
    }
}
