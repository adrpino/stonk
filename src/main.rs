mod api;
mod bonds;
mod cik;
mod dossier;
mod sec;
mod transcripts;
mod types;

use std::io::{self, Write};

use anyhow::{Context, Result};
use api::{create_client, fetch_yahoo_chart, fetch_yahoo_summary};
use bonds::{deduplicate_and_prioritize_bonds, fetch_bond_markets_insider, fetch_bond_morningstar};
use clap::Parser;
use dossier::{append_history_to_dossier, extract_ai_dossier};
use sec::fetch_sec_filing;
use transcripts::fetch_wsb_transcript;
use types::Args;

fn safe_println(content: &str) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    for line in content.lines() {
        if let Err(e) = writeln!(handle, "{}", line) {
            if e.kind() == io::ErrorKind::BrokenPipe {
                std::process::exit(0);
            }
            return;
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let client = create_client()?;

    if args.transcript {
        let ticker = args
            .ticker
            .as_deref()
            .context("Ticker symbol is required when requesting transcripts")?;
        let quarter = args.quarter.as_deref().unwrap_or("Q3");
        let year = args.year.unwrap_or(2026);

        let transcript = fetch_wsb_transcript(&client, ticker, quarter, year).await?;

        if args.ai {
            safe_println(&format!(
                "### EARNINGS CALL TRANSCRIPT FOR {} ({}, {})\n",
                ticker.to_uppercase(),
                quarter,
                year
            ));
            safe_println(&transcript.formatted_markdown);
        } else if args.compact {
            safe_println(&serde_json::to_string(&transcript)?);
        } else {
            safe_println(&serde_json::to_string_pretty(&transcript)?);
        }
        return Ok(());
    }

    if let Some(form_type) = &args.sec {
        let ticker = args
            .ticker
            .as_deref()
            .context("Ticker symbol is required when requesting SEC filings")?;

        let should_parse = args.parse || args.ai;
        let sec_filing = fetch_sec_filing(&client, ticker, form_type, should_parse).await?;

        if args.ai {
            safe_println(&format!(
                "### SEC FILING ({}) FOR {}\n",
                form_type.to_uppercase(),
                ticker.to_uppercase()
            ));
            if let Some(f) = &sec_filing {
                if let Some(parsed) = &f.parsed_content {
                    safe_println(&parsed.full_markdown);
                } else {
                    safe_println(&format!(
                        "```json\n{}\n```",
                        serde_json::to_string_pretty(&sec_filing)?
                    ));
                }
            } else {
                safe_println("No filing found.");
            }
        } else if args.compact {
            safe_println(&serde_json::to_string(&sec_filing)?);
        } else {
            safe_println(&serde_json::to_string_pretty(&sec_filing)?);
        }
        return Ok(());
    }

    if let Some(bond_query) = &args.bond {
        let (mi_res, ms_res) = tokio::join!(
            fetch_bond_markets_insider(&client, bond_query),
            fetch_bond_morningstar(&client, bond_query)
        );

        let mut raw_results = Vec::new();
        if let Ok(mi_list) = mi_res {
            raw_results.extend(mi_list);
        }
        if let Ok(ms_list) = ms_res {
            raw_results.extend(ms_list);
        }

        let results = deduplicate_and_prioritize_bonds(raw_results);

        if args.ai {
            safe_println(&format!(
                "### CORPORATE BOND DOSSIER FOR {}\n",
                bond_query.to_uppercase()
            ));
            safe_println(&format!(
                "```json\n{}\n```",
                serde_json::to_string_pretty(&results)?
            ));
        } else if args.compact {
            safe_println(&serde_json::to_string(&results)?);
        } else {
            safe_println(&serde_json::to_string_pretty(&results)?);
        }
        return Ok(());
    }

    let ticker = args.ticker.as_deref().unwrap_or("");
    let (raw_data, chart_res) = if let Some(range) = &args.history {
        let (s, c) = tokio::join!(
            fetch_yahoo_summary(&client, ticker),
            fetch_yahoo_chart(&client, ticker, range)
        );
        (s?, Some(c?))
    } else {
        (fetch_yahoo_summary(&client, ticker).await?, None)
    };

    let mut dossier = extract_ai_dossier(ticker, &raw_data);
    if let Some(chart_data) = chart_res {
        append_history_to_dossier(&mut dossier, &chart_data);
    }

    if args.ai {
        safe_println(&format!(
            "### FINANCIAL DOSSIER FOR {}\n",
            ticker.to_uppercase()
        ));
        safe_println(
            "Please analyze the following stock metrics for fundamental valuation, risks, and growth potential:\n",
        );
        safe_println(&format!(
            "```json\n{}\n```",
            serde_json::to_string_pretty(&dossier)?
        ));
    } else if args.compact {
        safe_println(&serde_json::to_string(&dossier)?);
    } else {
        safe_println(&serde_json::to_string_pretty(&dossier)?);
    }

    Ok(())
}
