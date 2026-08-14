# `stonk` 📈

A high-performance Rust CLI utility that aggregates financial valuation metrics, multi-year historical price trends, SEC EDGAR filings, earnings call transcripts, and corporate bond intelligence—formatted specifically for AI LLM prompt pipelines and automated investment research.

---

## Features

- **Valuation & Fundamentals (`stonk quote` / `stonk <TICKER>`):** Trailing/Forward P/E, PEG, EV/EBITDA, Price/Sales, Price/Book, operating margins, free cash flow, return on equity, and upcoming earnings dates.
- **Historical Price Trends (`-H, --history`):** Token-optimized monthly candles (`1y`, `2y`, `5y`, `10y`, `max`) with calculated performance metrics (52-week drawdown, 1-year and 5-year returns) consuming `< 500` tokens for 5 years of data.
- **SEC EDGAR Filings (`stonk sec`):** Real-time lookup of `10-K`, `10-Q`, `8-K`, `4`, `S-1`, `S-3`, `6-K`, `20-F` filings with direct document URLs. Automatically parses filing HTML into clean markdown financial tables and MD&A commentary (`-p, --parse`).
- **Earnings Call Transcripts (`stonk transcript`):** Formatted quarterly earnings call transcripts with executive remarks and analyst Q&A sections (`-q Q3 -y 2026`).
- **Corporate Bond Intelligence (`stonk bond`):** Scrapes corporate debt data by stock ticker or ISIN/CUSIP (coupon rate, maturity date, current price, and Yield to Maturity).
- **AI-First Mode (`--ai`):** Wraps data in clean markdown prompt containers optimized for direct piping into LLM CLIs (`opencode`, `claude`, `gemini-cli`).
- **Resilient Network Layer:** Handles Yahoo session cookies and crumbs, browser Client Hints (`Sec-CH-UA`), SEC EDGAR User-Agent conventions, and broken pipe signals gracefully.

---

## Installation

```bash
cargo install --path .
```

---

## Commands & Usage

### 1. Stock Valuation & Price History (`quote`)

```bash
# Pretty-printed JSON dossier (shortcut)
stonk TSM

# Or using explicit subcommand
stonk quote TSM

# 5-year monthly candles formatted for AI prompt
stonk quote MU -H 5y --ai

# Compact single-line JSON
stonk quote NVDA --compact
```

### 2. SEC EDGAR Filings (`sec`)

```bash
# Retrieve latest 10-Q filing metadata and SEC archive link
stonk sec NVDA

# Specific form (e.g. 10-K, 8-K, Form 4, S-1)
stonk sec AAPL 10-K

# Parse filing HTML into clean markdown financial tables & MD&A sections
stonk sec AAPL 10-K -p --ai
```

### 3. Earnings Call Transcripts (`transcript`)

```bash
# Latest earnings call transcript (defaults to Q3 2026)
stonk transcript MSFT --ai

# Specific fiscal quarter and year
stonk transcript GOOGL -q Q2 -y 2026 --ai
```

### 4. Corporate Bonds & Debt Analysis (`bond`)

```bash
# List all active corporate bonds for a company
stonk bond META --ai

# Look up a specific bond by ISIN / CUSIP
stonk bond US30303M8B15 --ai
```

### 5. Piping to AI CLI Agents

```bash
# Comprehensive valuation & bull/bear thesis
stonk quote TSM -H 5y --ai | opencode "Analyze this 5-year valuation evolution and generate a bull vs bear thesis"

# Earnings call sentiment & forward guidance review
stonk transcript NVDA -q Q2 -y 2026 --ai | opencode "Summarize management guidance on data center revenue and margins"

# SEC 10-K statement review
stonk sec AAPL 10-K -p --ai | opencode "Extract all revenue breakdown items and segment margins"

# Debt & capital structure evaluation
stonk bond AAPL --ai | opencode "Evaluate Apple's corporate bond maturities and interest rate risk"
```

---

## Subcommands & Options Reference

### Global Flags
- `--ai`: Output formatted as a markdown prompt container for LLMs.
- `--compact`: Output single-line compact JSON.
- `-h, --help`: Print comprehensive command help.

### Command Reference

| Subcommand | Arguments & Flags | Description |
|---|---|---|
| **`quote`** | `<TICKER> [-H <RANGE>]` | Valuation fundamentals and historical monthly price candles (`1y`, `2y`, `5y`, `10y`, `max`). |
| **`sec`** | `<TICKER> [FORM] [-p, --parse]` | SEC filing metadata and HTML table/MD&A parsing (`10-K`, `10-Q`, `8-K`, `4`, `S-1`, `S-3`, `6-K`, `20-F`). |
| **`transcript`** | `<TICKER> [-q <QUARTER>] [-y <YEAR>]` | Quarterly earnings call transcripts (`Q1`, `Q2`, `Q3`, `Q4`, default year `2026`). |
| **`bond`** | `<QUERY>` | Corporate bond yield and maturity lookup by ticker (`META`) or ISIN/CUSIP (`US30303M8B15`). |

---

## Development & Testing

```bash
# Format code
cargo fmt --check

# Lint with clippy (zero warnings enforced)
cargo clippy --all-targets --all-features -- -D warnings

# Run offline unit test suite (runs in milliseconds)
cargo test

# Run live end-to-end network integration tests against SEC/Yahoo feeds
cargo test --test live_integration -- --ignored
```
