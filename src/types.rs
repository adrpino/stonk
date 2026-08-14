use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "stonk",
    author,
    version,
    about = "Dense, token-efficient financial CLI: valuation metrics, price history, SEC EDGAR filings, transcripts, and corporate bonds for AI agents & LLMs",
    after_help = "Examples:\n  stonk quote NVDA -H 5y --ai              # 5-year price history and valuation for LLM\n  stonk sec AAPL 10-K --parse --ai         # Parse SEC 10-K tables & MD&A\n  stonk transcript META -q Q2 -y 2026 --ai # Earnings call transcript\n  stonk bond US30303M8B15 --ai             # Corporate bond quotes & yields\n  stonk TSM                                # Shortcut for 'stonk quote TSM'"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Stock ticker symbol shortcut (e.g. 'stonk NVDA' is equivalent to 'stonk quote NVDA')
    #[arg(value_name = "TICKER", global = false)]
    pub ticker: Option<String>,

    /// Fetch historical monthly price candles (shortcut when ticker is passed at top-level)
    #[arg(short = 'H', long, value_name = "RANGE", num_args = 0..=1, default_missing_value = "1y")]
    pub history: Option<String>,

    /// Format output specifically as a prompt container for AI LLMs
    #[arg(long, default_value_t = false, global = true)]
    pub ai: bool,

    /// Output compact single-line JSON
    #[arg(long, default_value_t = false, global = true)]
    pub compact: bool,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum Commands {
    /// Fetch stock valuation metrics, fundamentals, and historical price candles
    #[command(
        name = "quote",
        about = "Fetch stock valuation metrics, fundamentals, and historical price candles",
        after_help = "Examples:\n  stonk quote NVDA\n  stonk quote TSM -H 5y --ai\n  stonk quote AMZN -H max --compact"
    )]
    Quote(QuoteArgs),

    /// Fetch SEC EDGAR filings and parse financial statements / MD&A sections
    #[command(
        name = "sec",
        about = "Fetch SEC EDGAR filings and parse financial statements / MD&A sections",
        after_help = "Examples:\n  stonk sec NVDA 10-Q\n  stonk sec AAPL 10-K --parse --ai\n  stonk sec TSLA 8-K --compact"
    )]
    Sec(SecArgs),

    /// Fetch quarterly earnings call transcripts with executive remarks and analyst Q&A
    #[command(
        name = "transcript",
        about = "Fetch quarterly earnings call transcripts with executive remarks and analyst Q&A",
        after_help = "Examples:\n  stonk transcript AAPL --ai\n  stonk transcript META -q Q2 -y 2026 --ai\n  stonk transcript MSFT --compact"
    )]
    Transcript(TranscriptArgs),

    /// Fetch corporate bond intelligence (yields, coupon, maturity) by ticker or ISIN/CUSIP
    #[command(
        name = "bond",
        about = "Fetch corporate bond intelligence (yields, coupon, maturity) by ticker or ISIN/CUSIP",
        after_help = "Examples:\n  stonk bond META --ai\n  stonk bond US30303M8B15 --ai\n  stonk bond AAPL --compact"
    )]
    Bond(BondArgs),
}

#[derive(clap::Args, Debug, PartialEq)]
pub struct QuoteArgs {
    /// Stock ticker symbol (e.g. NVDA, TSM, AAPL, META)
    pub ticker: String,

    /// Fetch historical monthly price candles (e.g. 1y, 2y, 5y, 10y, max) [default: 1y]
    #[arg(short = 'H', long, value_name = "RANGE", num_args = 0..=1, default_missing_value = "1y")]
    pub history: Option<String>,
}

#[derive(clap::Args, Debug, PartialEq)]
pub struct SecArgs {
    /// Stock ticker symbol (e.g. NVDA, AAPL, TSLA)
    pub ticker: String,

    /// SEC filing form type (e.g. 10-K, 10-Q, 8-K, 4)
    #[arg(value_name = "FORM", default_value = "10-Q")]
    pub form: String,

    /// Parse SEC filing HTML document into clean markdown tables and MD&A commentary
    #[arg(short = 'p', long = "parse", default_value_t = false)]
    pub parse: bool,
}

#[derive(clap::Args, Debug, PartialEq)]
pub struct TranscriptArgs {
    /// Stock ticker symbol (e.g. AAPL, META, MSFT, NVDA)
    pub ticker: String,

    /// Fiscal quarter for transcript (e.g. Q1, Q2, Q3, Q4)
    #[arg(short = 'q', long, value_name = "QUARTER", default_value = "Q3")]
    pub quarter: String,

    /// Fiscal year for transcript (e.g. 2026)
    #[arg(short = 'y', long, value_name = "YEAR", default_value_t = 2026)]
    pub year: i32,
}

#[derive(clap::Args, Debug, PartialEq)]
pub struct BondArgs {
    /// Stock ticker (e.g. META) or bond CUSIP/ISIN (e.g. US30303M8B15)
    pub query: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_top_level_ticker_shortcut() {
        let cli = Cli::try_parse_from(["stonk", "NVDA", "-H", "5y", "--ai"]).unwrap();
        assert_eq!(cli.ticker, Some("NVDA".to_string()));
        assert_eq!(cli.history, Some("5y".to_string()));
        assert!(cli.ai);
        assert!(!cli.compact);
        assert_eq!(cli.command, None);
    }

    #[test]
    fn test_top_level_ticker_shortcut_default_history() {
        let cli = Cli::try_parse_from(["stonk", "TSM", "-H"]).unwrap();
        assert_eq!(cli.ticker, Some("TSM".to_string()));
        assert_eq!(cli.history, Some("1y".to_string()));
    }

    #[test]
    fn test_subcommand_quote() {
        let cli =
            Cli::try_parse_from(["stonk", "quote", "AAPL", "-H", "10y", "--compact"]).unwrap();
        assert!(cli.compact);
        assert_eq!(
            cli.command,
            Some(Commands::Quote(QuoteArgs {
                ticker: "AAPL".to_string(),
                history: Some("10y".to_string()),
            }))
        );
    }

    #[test]
    fn test_subcommand_sec_default_form() {
        let cli = Cli::try_parse_from(["stonk", "sec", "NVDA"]).unwrap();
        assert_eq!(
            cli.command,
            Some(Commands::Sec(SecArgs {
                ticker: "NVDA".to_string(),
                form: "10-Q".to_string(),
                parse: false,
            }))
        );
    }

    #[test]
    fn test_subcommand_sec_with_form_and_parse() {
        let cli = Cli::try_parse_from(["stonk", "sec", "NVDA", "10-K", "-p", "--ai"]).unwrap();
        assert!(cli.ai);
        assert_eq!(
            cli.command,
            Some(Commands::Sec(SecArgs {
                ticker: "NVDA".to_string(),
                form: "10-K".to_string(),
                parse: true,
            }))
        );
    }

    #[test]
    fn test_subcommand_transcript_defaults() {
        let cli = Cli::try_parse_from(["stonk", "transcript", "GOOGL"]).unwrap();
        assert_eq!(
            cli.command,
            Some(Commands::Transcript(TranscriptArgs {
                ticker: "GOOGL".to_string(),
                quarter: "Q3".to_string(),
                year: 2026,
            }))
        );
    }

    #[test]
    fn test_subcommand_transcript_custom_quarter_and_year() {
        let cli = Cli::try_parse_from([
            "stonk",
            "transcript",
            "AAPL",
            "-q",
            "Q1",
            "-y",
            "2025",
            "--ai",
        ])
        .unwrap();
        assert!(cli.ai);
        assert_eq!(
            cli.command,
            Some(Commands::Transcript(TranscriptArgs {
                ticker: "AAPL".to_string(),
                quarter: "Q1".to_string(),
                year: 2025,
            }))
        );
    }

    #[test]
    fn test_subcommand_bond_ticker() {
        let cli = Cli::try_parse_from(["stonk", "bond", "META", "--ai"]).unwrap();
        assert!(cli.ai);
        assert_eq!(
            cli.command,
            Some(Commands::Bond(BondArgs {
                query: "META".to_string(),
            }))
        );
    }

    #[test]
    fn test_subcommand_bond_isin() {
        let cli = Cli::try_parse_from(["stonk", "bond", "US30303M8B15", "--compact"]).unwrap();
        assert!(cli.compact);
        assert_eq!(
            cli.command,
            Some(Commands::Bond(BondArgs {
                query: "US30303M8B15".to_string(),
            }))
        );
    }
}
