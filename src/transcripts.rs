use anyhow::{Context, Result, bail};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct TranscriptChunk {
    pub id: Option<String>,
    pub speaker: Option<String>,
    pub content: String,
    pub timestamp: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct TranscriptResponse {
    pub ticker: String,
    pub quarter: String,
    pub fiscal_year: i32,
    pub chunks: Vec<TranscriptChunk>,
    pub formatted_markdown: String,
}

const DEFAULT_TOKEN_V2: &str = "eyJhbGciOiJSUzI1NiIsImtpZCI6IlNIQTI1NjpzS3dsMnlsV0VtMjVmcXhwTU40cWY4MXE2OWFFdWFyMnpLMUdhVGxjdWNZIiwidHlwIjoiSldUIn0.eyJzdWIiOiJsb2lkIiwiZXhwIjoxNzg1NTc0ODE5LjY0Mzg4NywiaWF0IjoxNzg1NDg4NDE5LjY0Mzg4NywianRpIjoidVNXem5oRzN0WVpuQTVNZVFybURPUElQSVJkQmR3IiwiY2lkIjoiMFItV0FNaHVvby1NeVEiLCJsaWQiOiJ0Ml8yZzc3cHpqaWZvIiwibGNhIjoxNzgxMDg1MzY4OTY0LCJzY3AiOiJlSnhra2RHT3REQUloZC1GYTVfZ2Y1VV9tMDF0Y1lhc0xRYW9rM243RFZvY2s3MDdjRDRwSFA5REtvcUZEQ1pYZ3FuQUJGZ1RyVERCUnVUOW5MbTNnMmlOZTh0WXNabkNCRm13RkRya21MR3NpUVFtZUpJYXl4c21vSUxOeUZ5dXRHTk5MVDBRSnFoY01yZUZIcGMyb2JrYmk1NmRHRlc1ckR5b3NWZmwwdGpHRkxZbnhqY2JxdzJwdUM2bk1rbkxRdmtzWHZUak45VzM5dm16X1NhMEo4T0txdW1CM2hsSkNHNHNmcGltM2Q5VGs1NnRDeGExOTNxUTJ1ZDYzSzU5MWl3ME83ZWY2X2xySXhtWFkyaC1KdnQzMXktaEE0ODhMelBxQUVhczRVY1pkbVFkX2xVSFVMbWdKR01KNHRNSTVNcmwyMzhKdG12VHY4YnRFejk4TS1LbU5feldETlJ6Q2VMUXBfSDFHd0FBX184UTFlVFIiLCJmbG8iOjF9.lXlJF_kBnpbYwKgSPFHRc-fbQJhWpRWzP1o8to4VerfC_1RvVfBEJTv4uDHNq2SllNmVCDd2PdqI5bUjThqaFiHrmbElTxsYh1oCXgDci3OwBa_GkO6B9qJQC9W35QKFKHrfxoF6t_D86DtnYKYwFVfnC4Bh1UsQOwbC3xsoS9FHpcas0XQ_0KIDM_P4PGhFYcNUUm35Cp840CUAbl8DbB4znb7ydHOFnMjJAt9iWVsk6vAs31K6u4rVoEByvCTmbc2MnrXq25mEjQ41VnoB9ffBxKnFvUhPnVj2HiRbboSNp06hiqwXCS32KPb3xoicT5rdaPxDCsU6ciqEEl3ovQ";

pub fn parse_signed_request_context_from_html(html: &str) -> Option<String> {
    if html.is_empty() {
        return None;
    }

    let document = Html::parse_document(html);
    let input_sel = Selector::parse("input[name=\"data\"]").ok()?;

    for element in document.select(&input_sel) {
        if let Some(val_attr) = element.value().attr("value") {
            let unescaped = val_attr.replace("&quot;", "\"").replace("&amp;", "&");
            let parsed_opt = serde_json::from_str::<Value>(&unescaped).ok();
            if let Some(parsed_json) = parsed_opt {
                let token_opt = parsed_json["pdpPostFragment"]["devvit"]["signedRequestContext"]
                    .as_str()
                    .or_else(|| parsed_json["pdpPostFragment"]["devvit"]["webbitToken"].as_str());
                if let Some(token) = token_opt {
                    return Some(token.to_string());
                }
            }
        }
    }

    if let Some(pos) = html.find("signedRequestContext&quot;:&quot;") {
        let rest = &html[pos + "signedRequestContext&quot;:&quot;".len()..];
        if let Some(end) = rest.find("&quot;") {
            return Some(rest[..end].to_string());
        }
    }

    if let Some(pos) = html.find("\"signedRequestContext\":\"") {
        let rest = &html[pos + "\"signedRequestContext\":\"".len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }

    if let Some(pos) = html.find("&quot;signedRequestContext&quot;:&quot;") {
        let rest = &html[pos + "&quot;signedRequestContext&quot;:&quot;".len()..];
        if let Some(end) = rest.find("&quot;") {
            return Some(rest[..end].to_string());
        }
    }

    None
}

pub fn parse_transcript_chunks(json_val: &Value) -> Vec<TranscriptChunk> {
    let mut chunks = Vec::new();

    let arr = match json_val.as_array() {
        Some(a) => a,
        None => match json_val["entries"].as_array() {
            Some(a) => a,
            None => match json_val["chunks"].as_array() {
                Some(a) => a,
                None => match json_val["data"].as_array() {
                    Some(a) => a,
                    None => return chunks,
                },
            },
        },
    };

    for item in arr {
        let content = match item["content"].as_str() {
            Some(c) if !c.trim().is_empty() => c.trim().to_string(),
            _ => continue,
        };

        let id = item["id"].as_str().map(|s| s.to_string());
        let speaker = item["speaker"].as_str().map(|s| s.to_string());
        let timestamp = item["timestamp"].as_str().map(|s| s.to_string());

        chunks.push(TranscriptChunk {
            id,
            speaker,
            content,
            timestamp,
        });
    }

    chunks
}

pub fn format_transcript_markdown(
    ticker: &str,
    quarter: &str,
    year: i32,
    chunks: &[TranscriptChunk],
) -> String {
    let mut md = String::new();
    md.push_str(&format!(
        "# EARNINGS CALL TRANSCRIPT FOR {} ({}, {})\n\n",
        ticker.to_uppercase(),
        quarter,
        year
    ));

    if chunks.is_empty() {
        md.push_str("No transcript chunks available.\n");
        return md;
    }

    for chunk in chunks {
        let speaker = chunk.speaker.as_deref().unwrap_or("Speaker");
        let ts_suffix = match &chunk.timestamp {
            Some(ts) => format!(" ({})", ts),
            None => String::new(),
        };

        md.push_str(&format!("### {}{}\n", speaker, ts_suffix));
        md.push_str(&chunk.content);
        md.push_str("\n\n");
    }

    md
}

pub fn obtain_fresh_token_v2_from_reddit(impersonate_bin: Option<&str>) -> Option<String> {
    if let Ok(env_token) = std::env::var("REDDIT_TOKEN_V2").or_else(|_| std::env::var("TOKEN_V2")) {
        let trimmed = env_token.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    if let Some(bin) = impersonate_bin {
        let output = std::process::Command::new(bin)
            .arg("-s")
            .arg("-i")
            .arg("https://www.reddit.com/svc/shreddit/graphql")
            .arg("-H")
            .arg("content-type: application/json")
            .arg("-H")
            .arg("accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .arg("-H")
            .arg("accept-language: es-ES,es;q=0.7")
            .arg("-H")
            .arg(
                "user-agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/150.0.0.0 Safari/537.36",
            )
            .arg("-d")
            .arg("{}")
            .output()
            .ok()?;

        if output.status.success() {
            let resp_str = String::from_utf8_lossy(&output.stdout);
            if let Some(pos) = resp_str.find("token_v2=") {
                let rest = &resp_str[pos + "token_v2=".len()..];
                let end = rest.find(';').unwrap_or(rest.len());
                let token = rest[..end].trim().to_string();
                if !token.is_empty() {
                    return Some(token);
                }
            }
        }
    }

    None
}

pub async fn fetch_devvit_token_from_reddit(
    client: &reqwest::Client,
    post_id_or_url: &str,
) -> Result<String> {
    let url = if post_id_or_url.starts_with("http://") || post_id_or_url.starts_with("https://") {
        post_id_or_url.to_string()
    } else if post_id_or_url == "1vb14d0" {
        "https://www.reddit.com/r/wallstreetbets/comments/1vb14d0/apple_q3_earnings_call_live_transcript/".to_string()
    } else {
        format!(
            "https://www.reddit.com/r/wallstreetbets/comments/{}/",
            post_id_or_url
        )
    };

    let impersonate_bin = [
        "/opt/homebrew/bin/curl_chrome131",
        "/opt/homebrew/bin/curl-impersonate",
        "curl_chrome131",
        "curl-impersonate",
    ]
    .iter()
    .find(|bin| {
        std::process::Command::new(bin)
            .arg("--version")
            .output()
            .is_ok()
    })
    .copied();

    let token_v2 = obtain_fresh_token_v2_from_reddit(impersonate_bin)
        .unwrap_or_else(|| DEFAULT_TOKEN_V2.to_string());

    let cookie_arg = format!("token_v2={}", token_v2);

    let html_text = if let Some(bin) = impersonate_bin {
        let output = std::process::Command::new(bin)
            .arg("-s")
            .arg(&url)
            .arg("-H").arg("accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8")
            .arg("-H").arg("accept-language: es-ES,es;q=0.7")
            .arg("-H").arg("available-dictionary: :hLsJrcZri+gdJnpY12N4UB3qJj6g06x1/LzqT5kGudQ=:")
            .arg("-H").arg("priority: u=0, i")
            .arg("-H").arg("sec-ch-ua: \"Not;A=Brand\";v=\"8\", \"Chromium\";v=\"150\", \"Google Chrome\";v=\"150\"")
            .arg("-H").arg("sec-ch-ua-mobile: ?0")
            .arg("-H").arg("sec-ch-ua-platform: \"macOS\"")
            .arg("-H").arg("sec-fetch-dest: document")
            .arg("-H").arg("sec-fetch-mode: navigate")
            .arg("-H").arg("sec-fetch-site: none")
            .arg("-H").arg("sec-fetch-user: ?1")
            .arg("-H").arg("sec-gpc: 1")
            .arg("-H").arg("upgrade-insecure-requests: 1")
            .arg("-H").arg("user-agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/150.0.0.0 Safari/537.36")
            .arg("-b").arg(&cookie_arg)
            .output();

        match output {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
            _ => String::new(),
        }
    } else {
        String::new()
    };

    let html_text = if html_text.is_empty() {
        let res = client
            .get(&url)
            .header(
                reqwest::header::USER_AGENT,
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/150.0.0.0 Safari/537.36",
            )
            .header(
                reqwest::header::ACCEPT,
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8",
            )
            .header(reqwest::header::ACCEPT_LANGUAGE, "es-ES,es;q=0.7")
            .header("available-dictionary", ":hLsJrcZri+gdJnpY12N4UB3qJj6g06x1/LzqT5kGudQ=:")
            .header("priority", "u=0, i")
            .header(
                "sec-ch-ua",
                "\"Not;A=Brand\";v=\"8\", \"Chromium\";v=\"150\", \"Google Chrome\";v=\"150\"",
            )
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", "\"macOS\"")
            .header("sec-fetch-dest", "document")
            .header("sec-fetch-mode", "navigate")
            .header("sec-fetch-site", "none")
            .header("sec-fetch-user", "?1")
            .header("sec-gpc", "1")
            .header("upgrade-insecure-requests", "1")
            .header(reqwest::header::COOKIE, &cookie_arg)
            .send()
            .await
            .with_context(|| format!("Failed to fetch Reddit post {}", post_id_or_url))?;

        res.text().await?
    } else {
        html_text
    };

    match parse_signed_request_context_from_html(&html_text) {
        Some(token) => Ok(token),
        None => bail!("Could not extract Devvit JWT token from Reddit post HTML"),
    }
}

pub async fn fetch_wsb_transcript(
    client: &reqwest::Client,
    ticker: &str,
    quarter: &str,
    year: i32,
) -> Result<TranscriptResponse> {
    let token = if let Ok(t) = std::env::var("WSB_TOKEN").or_else(|_| std::env::var("DEVVIT_TOKEN"))
    {
        t
    } else {
        fetch_devvit_token_from_reddit(client, "1vb14d0").await?
    };

    let endpoint_url = "https://wsb-earnings-2th52-0-0-113-webview.devvit.net/api/wsb/transcript";
    let body_json = serde_json::json!({
        "ticker": ticker.to_uppercase(),
        "quarter": quarter,
        "fiscalYear": year
    });

    let res = client
        .post(endpoint_url)
        .header("Authorization", format!("Bearer {}", token))
        .header(
            "Origin",
            "https://wsb-earnings-2th52-0-0-113-webview.devvit.net",
        )
        .header(
            "Referer",
            "https://wsb-earnings-2th52-0-0-113-webview.devvit.net/index.html",
        )
        .json(&body_json)
        .send()
        .await
        .with_context(|| format!("Failed to fetch WSB transcript for {}", ticker))?;

    let res_text = res.text().await?;
    let json_val: Value = serde_json::from_str(&res_text)
        .with_context(|| format!("Failed to parse transcript response JSON: {}", res_text))?;
    let chunks = parse_transcript_chunks(&json_val);
    let formatted_markdown = format_transcript_markdown(ticker, quarter, year, &chunks);

    Ok(TranscriptResponse {
        ticker: ticker.to_uppercase(),
        quarter: quarter.to_string(),
        fiscal_year: year,
        chunks,
        formatted_markdown,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_signed_request_context_from_html_real_payload() {
        let sample_html = r#"
            <html>
            <body>
            <input type="hidden" name="data" value="{&quot;pdpPostFragment&quot;:{&quot;devvit&quot;:{&quot;signedRequestContext&quot;:&quot;eyJhbGciOiJSUzI1NiJ9.sample_jwt_payload.sample_sig&quot;}}}" />
            </body>
            </html>
        "#;

        let token = parse_signed_request_context_from_html(sample_html).unwrap();
        assert_eq!(token, "eyJhbGciOiJSUzI1NiJ9.sample_jwt_payload.sample_sig");
    }

    #[test]
    fn test_parse_transcript_chunks_real_payload() {
        let quartr_json = json!([
            {
                "id": "quartr-AAPL-4",
                "speaker": "Tim Cook",
                "content": "Thanks Kevin. Revenue was $109.4 billion, a June quarter record.",
                "timestamp": "2026-07-31T05:11:36-04:00"
            },
            {
                "id": "quartr-AAPL-5",
                "speaker": "Kevin Maestri",
                "content": "Our installed base of active devices reached another all time high.",
                "timestamp": "2026-07-31T05:27:14-04:00"
            }
        ]);

        let chunks = parse_transcript_chunks(&quartr_json);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].id, Some("quartr-AAPL-4".to_string()));
        assert_eq!(chunks[0].speaker, Some("Tim Cook".to_string()));
        assert!(chunks[0].content.contains("Revenue was $109.4 billion"));

        assert_eq!(chunks[1].speaker, Some("Kevin Maestri".to_string()));
    }

    #[test]
    fn test_format_transcript_markdown() {
        let chunks = vec![TranscriptChunk {
            id: Some("1".to_string()),
            speaker: Some("Tim Cook".to_string()),
            content: "We had a great quarter.".to_string(),
            timestamp: Some("2026-07-31".to_string()),
        }];

        let md = format_transcript_markdown("AAPL", "Q3", 2026, &chunks);
        assert!(md.contains("# EARNINGS CALL TRANSCRIPT FOR AAPL (Q3, 2026)"));
        assert!(md.contains("### Tim Cook (2026-07-31)"));
        assert!(md.contains("We had a great quarter."));
    }
}
