# hgrep

A grep-like command line tool that searches HTML files as plain text — stripping all tags, scripts, and styles so you only search what's actually visible.

## Usage

```
hgrep [OPTIONS] <PATTERN> [FILES]...
```

Reads from stdin if no files are given. Accepts regex patterns by default.

## Examples

```bash
# Search a single file
hgrep "hello world" index.html

# Case-insensitive search with line numbers
hgrep -in "error" site/index.html

# Recurse into a directory
hgrep -rn "contact" ./site/

# List files containing a match
hgrep -l "TODO" **/*.html

# Count matches per file
hgrep -c "subscribe" *.html

# Show 2 lines of context around each match
hgrep -C2 "warning" docs/*.html

# Search using a literal string (no regex)
hgrep -F "price: $4.99" shop.html

# Pipe HTML from stdin
curl -s https://example.com | hgrep "welcome"
```

## Options

### Matching

| Flag | Long form | Description |
|------|-----------|-------------|
| `-i` | `--ignore-case` | Case-insensitive matching |
| `-v` | `--invert-match` | Select non-matching lines |
| `-w` | `--word-regexp` | Match whole words only |
| `-x` | `--line-regexp` | Match whole lines only |
| `-F` | `--fixed-strings` | Treat pattern as a literal string, not a regex |
| `-e PATTERN` | `--regexp PATTERN` | Explicit pattern flag (positional arg becomes a file) |

### Output

| Flag | Long form | Description |
|------|-----------|-------------|
| `-n` | `--line-number` | Print line numbers |
| `-c` | `--count` | Print count of matching lines per file |
| `-l` | `--files-with-matches` | Print only names of files with matches |
| `-L` | `--files-without-matches` | Print only names of files without matches |
| `-H` | `--with-filename` | Always print filename |
|      | `--no-filename` | Never print filename |
|      | `--color` | Highlight matches (auto-enabled when output is a TTY) |

### Context

| Flag | Long form | Description |
|------|-----------|-------------|
| `-A N` | `--after-context N` | Print N lines after each match |
| `-B N` | `--before-context N` | Print N lines before each match |
| `-C N` | `--context N` | Print N lines before and after each match |

### Search

| Flag | Long form | Description |
|------|-----------|-------------|
| `-r` | `--recursive` | Recurse into directories |

When recursing, hgrep searches files with extensions: `.html`, `.htm`, `.xhtml`, `.shtml`.

## How It Works

hgrep parses HTML using [`html5ever`](https://github.com/servo/html5ever) — a spec-compliant parser — and extracts only the visible text content before searching.

- **Block elements** (`<p>`, `<div>`, `<h1>`–`<h6>`, `<li>`, `<td>`, `<tr>`, etc.) become line boundaries in the extracted text, so each logical block of content appears on its own searchable line.
- **Invisible elements** (`<script>`, `<style>`, `<head>`, `<noscript>`, `<template>`) are skipped entirely — their content never appears in results.
- **Inline elements** (`<a>`, `<span>`, `<strong>`, etc.) are transparent; their text flows into the surrounding line.
- Whitespace within each line is normalized.

Line numbers in the output refer to lines in the **extracted plain text**, not the raw HTML source.

## Installation

```bash
git clone https://github.com/yourname/hgrep
cd hgrep
cargo install --path .
```

Requires Rust 1.70 or later. No runtime dependencies.

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | One or more matches found |
| `1` | No matches found |
| `2` | Error (invalid pattern, missing file, etc.) |
