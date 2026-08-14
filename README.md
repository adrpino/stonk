# `stonk` 📈

A high-performance Rust CLI utility that aggregates financial valuation metrics, multi-year historical price trends, SEC EDGAR filings, earnings call transcripts, and corporate bond intelligence—formatted specifically for AI LLM prompt pipelines and automated investment research.

---

## Features

- **Valuation & Fundamentals Dossier:** Trailing/Forward P/E, PEG, EV/EBITDA, Price/Sales, Price/Book, operating margins, free cash flow, return on equity, and upcoming earnings dates.
- **Historical Price Trends (`-H, --history`):** Token-optimized monthly candles (`1y`, `2y`, `5y`, `10y`, `max`) with calculated performance metrics (52-week drawdown, 1-year and 5-year returns) consuming `< 500` tokens for 5 years of data.
- **SEC EDGAR Filings (`-S, --sec`, `-p, --parse`):** Real-time lookup of 10-K, 10-Q, 8-K, and Form 4 filings with direct document URLs. Automatically parses filing HTML into markdown financial tables and MD&A commentary.
- **Earnings Call Transcripts (`-t, --transcript`):** Formatted quarterly earnings call transcripts with executive remarks and analyst Q&A sections (`-q Q3 -y 2026`).
- **Corporate Bond Intelligence (`-b, --bond`):** Scrapes corporate debt data by stock ticker or ISIN/CUSIP (coupon rate, maturity date, current price, and Yield to Maturity).
- **AI-First Mode (`--ai`):** Wraps data in clean markdown prompt containers optimized for direct piping into LLM CLIs (`opencode`, `claude`, `gemini-cli`).
- **Resilient Network Layer:** Handles Yahoo session cookies and crumbs, browser Client Hints (`Sec-CH-UA`), SEC EDGAR User-Agent conventions, and broken pipe signals gracefully.

---

## Installation

```bash
cargo install --path .
```

---

## Usage

### 1. Fundamental Valuation & Metrics

```bash
# Pretty-printed JSON dossier
stonk TSM

# Compact single-line JSON
stonk NVDA --compact

# Formatted for AI context
stonk META --ai
```

### 2. Multi-Year Historical Price Trends

```bash
# 1-year monthly candles by default
stonk AAPL -H

# 5-year monthly candles formatted as AI prompt
stonk MU -H 5y --ai

# Full IPO history
stonk AMZN -H max
```

### 3. SEC EDGAR Filings & Financial Statement Parsing

```bash
# Retrieve latest 10-Q filing metadata and SEC archive link
stonk NVDA -S 10-Q

# Parse filing HTML into clean markdown financial tables & MD&A sections
stonk AAPL -S 10-K -p --ai
```

### 4. Earnings Call Transcripts

```bash
# Latest earnings call transcript
stonk MSFT -t --ai

# Specific fiscal quarter and year
stonk GOOGL -t -q Q2 -y 2026 --ai
```

### 5. Corporate Bonds & Debt Analysis

```bash
# List all active corporate bonds for a company
stonk -b META --ai

# Look up a specific bond by ISIN / CUSIP
stonk -b US30303M8B15 --ai
```

### 6. Piping to AI CLI Agents

```bash
# Comprehensive valuation & bull/bear thesis
stonk TSM -H 5y --ai | opencode "Analyze this 5-year valuation evolution and generate a bull vs bear thesis"

# Earnings call sentiment & forward guidance review
stonk NVDA -t -q Q2 -y 2026 --ai | opencode "Summarize management guidance on data center revenue and margins"

# Debt & capital structure evaluation
stonk -b AAPL --ai | opencode "Evaluate Apple's corporate bond maturities and interest rate risk"
```

---

## CLI Options

| Flag | Description |
|---|---|
| `<TICKER>` | Stock ticker symbol (e.g. `NVDA`, `TSM`, `AAPL`) |
| `-H, --history [<RANGE>]` | Historical monthly price candles (`1y`, `2y`, `5y`, `10y`, `max`, default: `1y`) |
| `-S, --sec <FORM>` | SEC filing form lookup (`10-K`, `10-Q`, `8-K`, `4`) |
| `-p, --parse` | Parse SEC filing HTML into markdown tables and MD&A commentary |
| `-t, --transcript` | Fetch earnings call transcript |
| `-q, --quarter <Q>` | Fiscal quarter for transcript (`Q1`, `Q2`, `Q3`, `Q4`) |
| `-y, --year <YEAR>` | Fiscal year for transcript (e.g. `2026`) |
| `-b, --bond <BOND>` | Corporate bond lookup by ticker or ISIN/CUSIP |
| `--ai` | Output formatted as a markdown prompt container for LLMs |
| `--compact` | Output single-line compact JSON |

---

## Development & Testing

```bash
# Format code
cargo fmt --check

# Lint with clippy (zero warnings enforced)
cargo clippy --all-targets --all-features -- -D warnings

# Run offline unit test suite
cargo test
```
