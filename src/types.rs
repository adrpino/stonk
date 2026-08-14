use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "stonk",
    author,
    version,
    about = "CLI tool to fetch Yahoo Finance metrics for AI stock analysis"
)]
pub struct Args {
    /// Stock Ticker symbol (e.g. TSM, MU, META, NVDA)
    #[arg(required_unless_present_any = ["bond", "sec"])]
    pub ticker: Option<String>,

    /// Fetch corporate bond metrics for a ticker or CUSIP/ISIN (e.g. META, US30303M8B15)
    #[arg(short = 'b', long, value_name = "BOND")]
    pub bond: Option<String>,

    /// Fetch SEC filing metadata for a form type (e.g. 10-Q, 10-K, 8-K, 4)
    #[arg(short = 'S', long = "sec", value_name = "FORM")]
    pub sec: Option<String>,

    /// Parse SEC filing HTML document into clean markdown tables and MD&A commentary
    #[arg(short = 'p', long, default_value_t = false)]
    pub parse: bool,

    /// Fetch earnings call transcript for ticker
    #[arg(short = 't', long, default_value_t = false)]
    pub transcript: bool,

    /// Fiscal quarter for transcript (e.g. Q1, Q2, Q3, Q4)
    #[arg(short = 'q', long, value_name = "QUARTER")]
    pub quarter: Option<String>,

    /// Fiscal year for transcript (e.g. 2026)
    #[arg(short = 'y', long, value_name = "YEAR")]
    pub year: Option<i32>,

    /// Fetch historical price data (e.g. 1y, 2y, 5y, 10y, max)
    #[arg(short = 'H', long, value_name = "RANGE", num_args = 0..=1, default_missing_value = "1y")]
    pub history: Option<String>,

    /// Format output specifically as a prompt for AI LLMs
    #[arg(long, default_value_t = false)]
    pub ai: bool,

    /// Output compact single-line JSON
    #[arg(long, default_value_t = false)]
    pub compact: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_args_parsing() {
        let args = Args::try_parse_from(["stonk", "NVDA", "-H", "5y", "--ai"]).unwrap();
        assert_eq!(args.ticker, Some("NVDA".to_string()));
        assert_eq!(args.history, Some("5y".to_string()));
        assert!(args.ai);
        assert!(!args.compact);
    }

    #[test]
    fn test_cli_args_history_default_value() {
        let args = Args::try_parse_from(["stonk", "TSM", "-H"]).unwrap();
        assert_eq!(args.ticker, Some("TSM".to_string()));
        assert_eq!(args.history, Some("1y".to_string()));
    }

    #[test]
    fn test_cli_args_bond_flag() {
        let args = Args::try_parse_from(["stonk", "-b", "US30303M8B15", "--ai"]).unwrap();
        assert_eq!(args.bond, Some("US30303M8B15".to_string()));
        assert_eq!(args.ticker, None);
        assert!(args.ai);
    }

    #[test]
    fn test_cli_args_sec_flag() {
        let args = Args::try_parse_from(["stonk", "NVDA", "-S", "10-Q", "-p"]).unwrap();
        assert_eq!(args.ticker, Some("NVDA".to_string()));
        assert_eq!(args.sec, Some("10-Q".to_string()));
        assert!(args.parse);
    }

    #[test]
    fn test_cli_args_transcript_flags() {
        let args = Args::try_parse_from(["stonk", "AAPL", "-t", "-q", "Q3", "-y", "2026", "--ai"])
            .unwrap();
        assert_eq!(args.ticker, Some("AAPL".to_string()));
        assert!(args.transcript);
        assert_eq!(args.quarter, Some("Q3".to_string()));
        assert_eq!(args.year, Some(2026));
        assert!(args.ai);
    }
}
