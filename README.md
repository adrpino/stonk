# `stonk` 📈

A fast, lightweight Rust CLI tool that fetches Yahoo Finance metrics and generates dense, token-efficient dossiers formatted for AI analysis (LLMs / prompt context).

---

## Installation

```bash
cargo install --path .
```

---

## Features

* **Cookie & Crumb Yahoo Authentication:** Handles session cookies and crumbs automatically to bypass rate limits and blocks.
* **Modern HTTP Headers:** Uses modern Browser User-Agent and Client Hints (`Sec-CH-UA`).
* **AI-First Output (`--ai`):** Formats stock metrics into prompt markdown blocks ready to pipe to opencode or LLM CLIs.
* **Zero Clutter:** Strips away web UI noise, keeping payloads under 1,000 tokens.

---

## Usage

### Basic Lookup
```bash
stonk TSM
```

### Format for AI Analysis
```bash
stonk MU --ai
```

### Pipe directly to LLM CLI
```bash
stonk TSM --ai | opencode "Analyze this valuation and give me a bull vs bear case"
```

### Compact JSON Output
```bash
stonk NVDA --compact
```

---

## Roadmap

See [ROADMAP.md](./ROADMAP.md) for the specification on adding historical monthly price trends (`--history 5y`).
