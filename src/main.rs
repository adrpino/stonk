mod api;
mod bonds;
mod cik;
mod dossier;
mod sec;
mod transcripts;
mod types;

use std::io::{self, Write};

use anyhow::{Result, bail};
use api::{create_client, fetch_yahoo_chart, fetch_yahoo_summary};
use bonds::fetch_bonds;
use clap::Parser;
use dossier::{append_history_to_dossier, extract_ai_dossier};
use reqwest::Client;
use sec::fetch_sec_filing;
use transcripts::fetch_wsb_transcript;
use types::{BondArgs, Cli, Commands, QuoteArgs, SecArgs, TranscriptArgs};

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

async fn handle_quote(client: &Client, args: &QuoteArgs, ai: bool, compact: bool) -> Result<()> {
    let ticker = &args.ticker;
    let (raw_data, chart_res) = if let Some(range) = &args.history {
        let (s, c) = tokio::join!(
            fetch_yahoo_summary(client, ticker),
            fetch_yahoo_chart(client, ticker, range)
        );
        (s?, Some(c?))
    } else {
        (fetch_yahoo_summary(client, ticker).await?, None)
    };

    let mut dossier = extract_ai_dossier(ticker, &raw_data);
    if let Some(chart_data) = chart_res {
        append_history_to_dossier(&mut dossier, &chart_data);
    }

    if ai {
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
    } else if compact {
        safe_println(&serde_json::to_string(&dossier)?);
    } else {
        safe_println(&serde_json::to_string_pretty(&dossier)?);
    }

    Ok(())
}

async fn handle_sec(client: &Client, args: &SecArgs, ai: bool, compact: bool) -> Result<()> {
    let ticker = &args.ticker;
    let form = args.form;
    let should_parse = args.parse || ai;
    let sec_filing = fetch_sec_filing(client, ticker, form, should_parse).await?;

    if ai {
        safe_println(&format!(
            "### SEC FILING ({}) FOR {}\n",
            form.as_str(),
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
    } else if compact {
        safe_println(&serde_json::to_string(&sec_filing)?);
    } else {
        safe_println(&serde_json::to_string_pretty(&sec_filing)?);
    }

    Ok(())
}

async fn handle_transcript(
    client: &Client,
    args: &TranscriptArgs,
    ai: bool,
    compact: bool,
) -> Result<()> {
    let ticker = &args.ticker;
    let quarter = args.quarter;
    let year = args.year;

    let transcript = fetch_wsb_transcript(client, ticker, quarter.as_str(), year).await?;

    if ai {
        safe_println(&format!(
            "### EARNINGS CALL TRANSCRIPT FOR {} ({}, {})\n",
            ticker.to_uppercase(),
            quarter,
            year
        ));
        safe_println(&transcript.formatted_markdown);
    } else if compact {
        safe_println(&serde_json::to_string(&transcript)?);
    } else {
        safe_println(&serde_json::to_string_pretty(&transcript)?);
    }

    Ok(())
}

async fn handle_bond(client: &Client, args: &BondArgs, ai: bool, compact: bool) -> Result<()> {
    let bond_query = &args.query;
    let results = fetch_bonds(client, bond_query).await?;

    if ai {
        safe_println(&format!(
            "### CORPORATE BOND DOSSIER FOR {}\n",
            bond_query.to_uppercase()
        ));
        safe_println(&format!(
            "```json\n{}\n```",
            serde_json::to_string_pretty(&results)?
        ));
    } else if compact {
        safe_println(&serde_json::to_string(&results)?);
    } else {
        safe_println(&serde_json::to_string_pretty(&results)?);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = create_client()?;

    match &cli.command {
        Some(Commands::Quote(args)) => handle_quote(&client, args, cli.ai, cli.compact).await,
        Some(Commands::Sec(args)) => handle_sec(&client, args, cli.ai, cli.compact).await,
        Some(Commands::Transcript(args)) => {
            handle_transcript(&client, args, cli.ai, cli.compact).await
        }
        Some(Commands::Bond(args)) => handle_bond(&client, args, cli.ai, cli.compact).await,
        None => {
            if let Some(ticker) = cli.ticker {
                let quote_args = QuoteArgs {
                    ticker,
                    history: cli.history,
                };
                handle_quote(&client, &quote_args, cli.ai, cli.compact).await
            } else {
                bail!("No command or ticker specified. Run 'stonk --help' for usage instructions.");
            }
        }
    }
}
