use anyhow::{Context, Result, bail};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ParsedSecContent {
    pub financial_tables: Vec<String>,
    pub mda_text: Option<String>,
    pub full_markdown: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct SecFiling {
    pub ticker: String,
    pub sec_cik: String,
    pub requested_form: String,
    pub form: String,
    pub filing_date: String,
    pub report_date: String,
    pub accession_number: String,
    pub document_description: String,
    pub document_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parsed_content: Option<ParsedSecContent>,
}

pub fn clean_text(input: &str) -> String {
    let unescaped = input
        .replace("&nbsp;", " ")
        .replace("\u{a0}", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace('’', "'");

    let mut result = String::with_capacity(unescaped.len());
    let mut in_ws = false;

    for c in unescaped.chars() {
        if c.is_whitespace() {
            if !in_ws {
                result.push(' ');
                in_ws = true;
            }
        } else {
            result.push(c);
            in_ws = false;
        }
    }

    result.trim().to_string()
}

fn is_financial_table(md: &str, row_count: usize) -> bool {
    if row_count < 3 {
        return false;
    }
    let lower = md.to_lowercase();
    lower.contains("revenue")
        || lower.contains("operating income")
        || lower.contains("net income")
        || lower.contains("total assets")
        || lower.contains("cash flows")
        || lower.contains("gross profit")
        || lower.contains("three months ended")
        || lower.contains("nine months ended")
        || lower.contains("twelve months ended")
}

fn is_toc_snippet(snippet: &str) -> bool {
    let lower = snippet.to_lowercase();
    let check_len = lower.len().min(120);
    let prefix = &lower[..check_len];
    prefix.contains("item 3.")
        || prefix.contains("item 4.")
        || prefix.contains("item 1a.")
        || prefix.contains("item 8.")
}

fn extract_mda_section(body: &str) -> Option<String> {
    let mda_headings = [
        "item 2. management's discussion and analysis",
        "item 2. management’s discussion and analysis",
        "item 7. management's discussion and analysis",
        "item 7. management’s discussion and analysis",
    ];

    let lower_body = body.to_lowercase();

    for heading in mda_headings {
        let mut search_offset = 0;
        while let Some(pos) = lower_body[search_offset..].find(heading) {
            let abs_pos = search_offset + pos;
            let end_check = (abs_pos + 300).min(body.len());
            let snippet = &body[abs_pos..end_check];

            if is_toc_snippet(snippet) {
                search_offset = abs_pos + heading.len();
                continue;
            }

            let remaining = &body[abs_pos..];
            let end_headings = ["item 3.", "item 4.", "item 7a.", "item 8.", "part ii"];

            let lower_remaining = remaining.to_lowercase();
            let mut end_pos = remaining.len().min(40000);

            for end_h in end_headings {
                if let Some(ep) = lower_remaining[100..].find(end_h) {
                    let calc_end = ep + 100;
                    if calc_end < end_pos {
                        end_pos = calc_end;
                    }
                }
            }

            let mda = remaining[..end_pos].trim().to_string();
            if mda.len() > 100 {
                return Some(mda);
            }

            search_offset = abs_pos + heading.len();
        }
    }

    None
}

pub fn remove_empty_columns(matrix: Vec<Vec<String>>) -> Vec<Vec<String>> {
    if matrix.is_empty() {
        return Vec::new();
    }

    let max_cols = matrix.iter().map(|r| r.len()).max().unwrap_or(0);
    if max_cols == 0 {
        return Vec::new();
    }

    let mut padded_matrix = matrix;
    for row in &mut padded_matrix {
        while row.len() < max_cols {
            row.push(String::new());
        }
    }

    let empty_cols: Vec<bool> = (0..max_cols)
        .map(|col_idx| padded_matrix.iter().all(|r| r[col_idx].trim().is_empty()))
        .collect();

    padded_matrix
        .into_iter()
        .map(|row| {
            row.into_iter()
                .enumerate()
                .filter(|(col_idx, _)| !empty_cols[*col_idx])
                .map(|(_, val)| val)
                .collect()
        })
        .collect()
}

pub fn merge_currency_symbols(row: Vec<String>) -> Vec<String> {
    let mut new_row = row;
    let len = new_row.len();
    let mut i = 0;

    while i + 1 < len {
        if new_row[i] == "$" && !new_row[i + 1].is_empty() {
            let val = new_row[i + 1].clone();
            new_row[i] = String::new();
            new_row[i + 1] = format!("${}", val);
            i += 2;
        } else {
            i += 1;
        }
    }

    new_row
}

pub fn clean_table_matrix(raw_matrix: Vec<Vec<String>>) -> Option<String> {
    if raw_matrix.is_empty() {
        return None;
    }

    let step1 = remove_empty_columns(raw_matrix);
    if step1.is_empty() {
        return None;
    }

    let step2: Vec<Vec<String>> = step1.into_iter().map(merge_currency_symbols).collect();

    let step3 = remove_empty_columns(step2);
    if step3.is_empty() {
        return None;
    }

    let max_cols = step3.iter().map(|r| r.len()).max().unwrap_or(0);
    if max_cols < 2 {
        return None;
    }

    let mut md = String::new();
    for (r_idx, row) in step3.iter().enumerate() {
        let mut line = String::from("| ");
        for c_idx in 0..max_cols {
            let val = row.get(c_idx).map(|s| s.as_str()).unwrap_or("");
            line.push_str(val);
            line.push_str(" | ");
        }
        md.push_str(line.trim_end());
        md.push('\n');

        if r_idx == 0 {
            let mut sep = String::from("| ");
            for _ in 0..max_cols {
                sep.push_str("--- | ");
            }
            md.push_str(sep.trim_end());
            md.push('\n');
        }
    }

    Some(md)
}

pub fn parse_sec_html(html: &str) -> ParsedSecContent {
    let document = Html::parse_document(html);
    let table_sel = Selector::parse("table").unwrap();
    let tr_sel = Selector::parse("tr").unwrap();
    let cell_sel = Selector::parse("td, th").unwrap();

    let mut financial_tables = Vec::new();

    for table in document.select(&table_sel) {
        let mut rows = Vec::new();
        for tr in table.select(&tr_sel) {
            let mut cell_texts = Vec::new();
            for cell in tr.select(&cell_sel) {
                let text = cell.text().collect::<Vec<_>>().join(" ");
                let cleaned = clean_text(&text);
                cell_texts.push(cleaned);
            }

            if cell_texts.iter().any(|c| !c.is_empty()) {
                rows.push(cell_texts);
            }
        }

        let row_count = rows.len();
        if let Some(md) = clean_table_matrix(rows)
            && is_financial_table(&md, row_count)
        {
            financial_tables.push(md);
        }
    }

    let body_sel = Selector::parse("body").unwrap();
    let body_text = match document.select(&body_sel).next() {
        Some(b) => b.text().collect::<Vec<_>>().join(" "),
        None => document.root_element().text().collect::<Vec<_>>().join(" "),
    };
    let clean_body = clean_text(&body_text);

    let mda_text = extract_mda_section(&clean_body);

    let mut full_md = String::new();
    if let Some(ref mda) = mda_text {
        full_md.push_str("## MANAGEMENT'S DISCUSSION AND ANALYSIS (MD&A)\n\n");
        full_md.push_str(mda);
        full_md.push_str("\n\n");
    }

    if !financial_tables.is_empty() {
        full_md.push_str("## FINANCIAL TABLES\n\n");
        for (i, table_md) in financial_tables.iter().enumerate() {
            full_md.push_str(&format!("### Table {}\n\n", i + 1));
            full_md.push_str(table_md);
            full_md.push_str("\n\n");
        }
    }

    ParsedSecContent {
        financial_tables,
        mda_text,
        full_markdown: full_md,
    }
}

pub fn parse_sec_submissions_json(
    ticker: &str,
    cik: &str,
    requested_form: &str,
    submissions_json: &Value,
) -> Option<SecFiling> {
    let recent = submissions_json.get("filings")?.get("recent")?;

    let forms = recent.get("form")?.as_array()?;
    let filing_dates = recent.get("filingDate")?.as_array()?;
    let report_dates = recent.get("reportDate")?.as_array()?;
    let accession_numbers = recent.get("accessionNumber")?.as_array()?;
    let primary_docs = recent.get("primaryDocument")?.as_array()?;
    let primary_descriptions = recent.get("primaryDocDescription")?.as_array()?;

    let req_form_upper = requested_form.trim().to_uppercase();

    for (i, form_val) in forms.iter().enumerate() {
        let form_str = form_val.as_str().unwrap_or("");
        if form_str.to_uppercase() == req_form_upper {
            let filing_date = filing_dates
                .get(i)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let report_date = report_dates
                .get(i)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let raw_acc = accession_numbers
                .get(i)
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let clean_acc = raw_acc.replace('-', "");
            let doc_name = primary_docs.get(i).and_then(|v| v.as_str()).unwrap_or("");
            let doc_desc = primary_descriptions
                .get(i)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let cik_num_path = cik.trim_start_matches('0');

            let document_url = format!(
                "https://www.sec.gov/Archives/edgar/data/{}/{}/{}",
                cik_num_path, clean_acc, doc_name
            );

            return Some(SecFiling {
                ticker: ticker.to_uppercase(),
                sec_cik: cik.to_string(),
                requested_form: requested_form.to_string(),
                form: form_str.to_string(),
                filing_date,
                report_date,
                accession_number: raw_acc.to_string(),
                document_description: doc_desc,
                document_url,
                parsed_content: None,
            });
        }
    }

    None
}

pub async fn fetch_sec_filing(
    client: &reqwest::Client,
    ticker: &str,
    requested_form: &str,
    parse_content: bool,
) -> Result<Option<SecFiling>> {
    let cik = match crate::cik::get_static_cik(ticker) {
        Some(c) => c.to_string(),
        None => match crate::cik::resolve_cik(client, ticker).await? {
            Some(c) => c,
            None => bail!("Could not resolve SEC CIK for ticker: {}", ticker),
        },
    };

    let url = format!("https://data.sec.gov/submissions/CIK{}.json", cik);
    let res = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, "Stonk/1.0 (admin@stonk.dev)")
        .send()
        .await
        .with_context(|| format!("Failed to fetch SEC filings for CIK {}", cik))?;

    let json_data: Value = res.json().await?;
    let mut filing = match parse_sec_submissions_json(ticker, &cik, requested_form, &json_data) {
        Some(f) => f,
        None => return Ok(None),
    };

    if parse_content {
        let html_res = client
            .get(&filing.document_url)
            .header(reqwest::header::USER_AGENT, "Stonk/1.0 (admin@stonk.dev)")
            .send()
            .await;

        if let Ok(html_response) = html_res
            && let Ok(html_text) = html_response.text().await
        {
            filing.parsed_content = Some(parse_sec_html(&html_text));
        }
    }

    Ok(Some(filing))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_sec_submissions_json_real_nvda_payload() {
        let nvda_submissions = json!({
            "filings": {
                "recent": {
                    "accessionNumber": ["0001045810-26-000060", "0001045810-26-000045"],
                    "filingDate": ["2026-07-02", "2026-05-28"],
                    "reportDate": ["2026-06-28", "2026-04-28"],
                    "form": ["8-K", "10-Q"],
                    "primaryDocument": ["nvda-20260628.htm", "nvda-20260428.htm"],
                    "primaryDocDescription": ["8-K EARNINGS RELEASE", "FORM 10-Q QUARTERLY REPORT"]
                }
            }
        });

        let filing_10q =
            parse_sec_submissions_json("NVDA", "0001045810", "10-Q", &nvda_submissions).unwrap();
        assert_eq!(filing_10q.ticker, "NVDA");
        assert_eq!(filing_10q.sec_cik, "0001045810");
        assert_eq!(filing_10q.form, "10-Q");
        assert_eq!(filing_10q.filing_date, "2026-05-28");
        assert_eq!(filing_10q.report_date, "2026-04-28");
        assert_eq!(filing_10q.accession_number, "0001045810-26-000045");
        assert_eq!(
            filing_10q.document_url,
            "https://www.sec.gov/Archives/edgar/data/1045810/000104581026000045/nvda-20260428.htm"
        );

        let filing_8k =
            parse_sec_submissions_json("NVDA", "0001045810", "8-K", &nvda_submissions).unwrap();
        assert_eq!(filing_8k.form, "8-K");
        assert_eq!(filing_8k.filing_date, "2026-07-02");
        assert_eq!(
            filing_8k.document_url,
            "https://www.sec.gov/Archives/edgar/data/1045810/000104581026000060/nvda-20260628.htm"
        );
    }

    #[test]
    fn test_parse_sec_submissions_json_not_found() {
        let empty_submissions = json!({
            "filings": {
                "recent": {
                    "accessionNumber": [],
                    "filingDate": [],
                    "reportDate": [],
                    "form": [],
                    "primaryDocument": [],
                    "primaryDocDescription": []
                }
            }
        });

        assert!(
            parse_sec_submissions_json("AAPL", "0000320193", "10-K", &empty_submissions).is_none()
        );
    }

    #[test]
    fn test_remove_empty_columns() {
        let matrix = vec![
            vec![
                "Revenue".to_string(),
                "".to_string(),
                "100".to_string(),
                "".to_string(),
            ],
            vec![
                "Net Income".to_string(),
                "".to_string(),
                "50".to_string(),
                "".to_string(),
            ],
        ];
        let cleaned = remove_empty_columns(matrix);
        assert_eq!(cleaned.len(), 2);
        assert_eq!(cleaned[0], vec!["Revenue".to_string(), "100".to_string()]);
        assert_eq!(cleaned[1], vec!["Net Income".to_string(), "50".to_string()]);
    }

    #[test]
    fn test_merge_currency_symbols() {
        let row = vec![
            "Revenue".to_string(),
            "$".to_string(),
            "81,615".to_string(),
            "$".to_string(),
            "44,062".to_string(),
        ];
        let merged = merge_currency_symbols(row);
        assert_eq!(
            merged,
            vec![
                "Revenue".to_string(),
                "".to_string(),
                "$81,615".to_string(),
                "".to_string(),
                "$44,062".to_string()
            ]
        );
    }

    #[test]
    fn test_clean_table_matrix() {
        let raw = vec![
            vec![
                "Revenue".to_string(),
                "".to_string(),
                "$".to_string(),
                "81,615".to_string(),
                "".to_string(),
            ],
            vec![
                "Cost of revenue".to_string(),
                "".to_string(),
                "".to_string(),
                "20,458".to_string(),
                "".to_string(),
            ],
        ];
        let md = clean_table_matrix(raw).unwrap();
        assert!(md.contains("| Revenue | $81,615 |"));
        assert!(md.contains("| Cost of revenue | 20,458 |"));
    }

    #[test]
    fn test_clean_text() {
        let raw = "  Revenue&nbsp;grew &quot;substantially&quot; &amp; &lt;fast&gt;’s  \n\t ";
        let cleaned = clean_text(raw);
        assert_eq!(cleaned, "Revenue grew \"substantially\" & <fast>'s");
    }

    #[test]
    fn test_is_financial_table() {
        let fin_md =
            "| Metric | Amount |\n| --- | --- |\n| Revenue | $81,615 |\n| Net income | $58,321 |";
        assert!(is_financial_table(fin_md, 4));

        let non_fin_md = "| Name | Role |\n| --- | --- |\n| John Doe | CEO |\n| Jane Smith | CFO |";
        assert!(!is_financial_table(non_fin_md, 4));

        let short_md = "| Revenue | $100 |\n| --- | --- |";
        assert!(!is_financial_table(short_md, 2));
    }

    #[test]
    fn test_is_toc_snippet() {
        let toc_str = "Item 2. Management's Discussion and Analysis 23 Item 3. Quantitative Disclosures 30 Item 4. Controls";
        assert!(is_toc_snippet(toc_str));

        let body_str = "Item 2. Management's Discussion and Analysis of Financial Condition and Results of Operations. The following discussion should be read in conjunction with our condensed consolidated financial statements.";
        assert!(!is_toc_snippet(body_str));
    }

    #[test]
    fn test_extract_mda_section_10k() {
        let html_body = r#"
            Table of Contents
            Item 7. Management's Discussion and Analysis 45 Item 8. Financial Statements 60
            ...
            Item 7. Management's Discussion and Analysis of Financial Condition and Results of Operations
            In fiscal 2026, total net sales reached $120 billion driven by cloud expansion.
            Item 7A. Quantitative and Qualitative Disclosures About Market Risk
        "#;
        let cleaned = clean_text(html_body);
        let mda = extract_mda_section(&cleaned);
        assert!(mda.is_some());
        let mda_text = mda.unwrap();
        assert!(mda_text.contains("total net sales reached $120 billion"));
    }

    #[test]
    fn test_parse_sec_html_table_and_mda() {
        let sample_html = r#"
            <html>
            <body>
            <table>
                <tr><th>Metric</th><th>Q1 2026</th><th>Q1 2025</th></tr>
                <tr><td>Revenue</td><td>$81,615</td><td>$44,062</td></tr>
                <tr><td>Net income</td><td>$58,321</td><td>$18,775</td></tr>
            </table>
            <div>
                Item 2. Management's Discussion and Analysis of Financial Condition and Results of Operations
                Revenue grew significantly due to Data Center demand.
                Item 3. Quantitative Disclosures
            </div>
            </body>
            </html>
        "#;

        let parsed = parse_sec_html(sample_html);
        assert!(!parsed.financial_tables.is_empty());
        assert!(parsed.financial_tables[0].contains("Revenue"));
        assert!(parsed.financial_tables[0].contains("81,615"));

        assert!(parsed.mda_text.is_some());
        let mda = parsed.mda_text.unwrap();
        assert!(mda.contains("Revenue grew significantly"));
    }

    #[test]
    fn test_parse_real_sec_10q_sample_file_if_present() {
        if let Ok(content) = std::fs::read_to_string("/tmp/sec_10q_sample.htm") {
            let parsed = parse_sec_html(&content);
            assert!(!parsed.financial_tables.is_empty());
            assert!(parsed.mda_text.is_some());
            let mda = parsed.mda_text.unwrap();
            assert!(mda.to_lowercase().contains("forward-looking statements"));
            assert!(
                parsed
                    .full_markdown
                    .contains("MANAGEMENT'S DISCUSSION AND ANALYSIS")
            );
        }
    }
}
