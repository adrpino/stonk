# AGENTS.md - Development & AI Agent Guidelines for `stonk`

This repository contains **`stonk`**, a high-performance Rust CLI utility that fetches Yahoo Finance metrics, valuation data, and downsampled historical price trends formatted specifically for AI LLMs and prompt pipelines.

---

## Core Mandates & Verification Commands

When making modifications or adding features to this codebase, always run and verify the following commands before completing tasks:

```bash
# 1. Format code according to Rust standards (ensure zero formatting diffs)
cargo fmt --check

# 2. Run linter and ensure ZERO warnings across all targets and features
cargo clippy --all-targets --all-features -- -D warnings

# 3. Execute unit test suite
cargo test

# 4. Install optimized release binary to cargo bin
cargo install --path .
```

---

## Testing Guidelines

- **OFFLINE UNIT TESTS**:
  - The automated test suite (`cargo test`) **MUST NEVER** rely on live Yahoo Finance network requests.
  - All unit tests in `src/dossier.rs` use mock JSON payloads to test quote extraction, monthly candle downsampling, volume formatting, and performance metric calculations offline in milliseconds.
  - Live network calls (`fetch_yahoo_summary`, `fetch_yahoo_chart`) are isolated in `src/api.rs`.

---

## Architecture & Module Overview

1. **CLI Interface (`src/types.rs`)**:
   - Built using `clap` (derive subcommands feature).
   - Implements dedicated subcommands: `quote`, `sec`, `transcript`, and `bond` with top-level shortcut resolution for ticker queries.
   - Handles global `--ai` and `--compact` output flags.

2. **Yahoo Finance API Integration (`src/api.rs`)**:
   - Built on `reqwest` and `serde_json`.
   - Obtains cookie/crumb authentication for `quoteSummary` endpoints.
   - Fetches historical chart candles via `query1.finance.yahoo.com/v8/finance/chart/{TICKER}?range={RANGE}&interval=1mo`.

3. **Data Downsampling & Formatting (`src/dossier.rs`)**:
   - `extract_ai_dossier`: Extracts valuation multiples, profitability margins, cashflow data, and key stats.
   - `append_history_to_dossier`: Downsamples candle data into monthly records (`YYYY-MM`) and calculates summary metrics (`drawdown_from_52w_high`, `one_year_return`, `five_year_return`).
   - Token optimization: Monthly candle aggregation keeps context window usage low (< 500 tokens for 5 years of data).

4. **CLI Entry Point (`src/main.rs`)**:
   - Orchestrates async data fetching and dossier construction.
   - Handles output formatting (pretty JSON, compact JSON, or markdown LLM prompt container).

---

## Engineering Guidelines & Rules

- **Surgical Editing**: Use targeted replacements to preserve existing tests and structure.
- **Zero Linter Warnings**: Always pass `cargo clippy` cleanly.
- **Robust Error Handling**: Safely filter out `null` or missing candle entries in quote arrays to prevent runtime panics.
- **Safe I/O & Pipe Handling**: Always use `safe_println` in `src/main.rs` instead of standard `println!` macros to gracefully handle `ErrorKind::BrokenPipe` when output is piped to external utilities like `head`, `tail`, or `jq` without panicking.
