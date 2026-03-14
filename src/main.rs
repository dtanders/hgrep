use clap::Parser;
use regex::{Regex, RegexBuilder};
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use walkdir::WalkDir;

mod html;

#[derive(Parser, Debug)]
#[command(
    name = "hgrep",
    about = "Search for text in HTML files as if they were plain text",
    after_help = "EXAMPLES:\n  hgrep \"hello world\" index.html\n  hgrep -i \"error\" *.html\n  hgrep -rn \"contact\" ./site/\n  hgrep -l \"TODO\" **/*.html"
)]
struct Args {
    /// Search pattern (regex by default)
    pattern: String,

    /// Files or directories to search (reads stdin if omitted)
    files: Vec<PathBuf>,

    /// Treat pattern as a literal string, not a regex
    #[arg(short = 'F', long = "fixed-strings")]
    fixed_strings: bool,

    /// Case-insensitive matching
    #[arg(short = 'i', long = "ignore-case")]
    ignore_case: bool,

    /// Invert match: select non-matching lines
    #[arg(short = 'v', long = "invert-match")]
    invert_match: bool,

    /// Match whole words only
    #[arg(short = 'w', long = "word-regexp")]
    word_regexp: bool,

    /// Match whole lines only
    #[arg(short = 'x', long = "line-regexp")]
    line_regexp: bool,

    /// Print line numbers
    #[arg(short = 'n', long = "line-number")]
    line_number: bool,

    /// Print count of matching lines per file
    #[arg(short = 'c', long = "count")]
    count: bool,

    /// Print only names of files with matches
    #[arg(short = 'l', long = "files-with-matches")]
    files_with_matches: bool,

    /// Print only names of files without matches
    #[arg(short = 'L', long = "files-without-matches")]
    files_without_matches: bool,

    /// Always print filename header
    #[arg(short = 'H', long = "with-filename")]
    with_filename: bool,

    /// Never print filename header
    #[arg(long = "no-filename")]
    no_filename: bool,

    /// Recurse into directories
    #[arg(short = 'r', long = "recursive")]
    recursive: bool,

    /// Print N lines of context after match
    #[arg(short = 'A', long = "after-context", value_name = "N")]
    after_context: Option<usize>,

    /// Print N lines of context before match
    #[arg(short = 'B', long = "before-context", value_name = "N")]
    before_context: Option<usize>,

    /// Print N lines of context before and after match
    #[arg(short = 'C', long = "context", value_name = "N")]
    context: Option<usize>,

    /// Highlight matches with color (auto-detected by default)
    #[arg(long = "color", alias = "colour")]
    color: bool,

    /// Use PATTERN as the search pattern (the positional pattern arg becomes a file path)
    #[arg(short = 'e', long = "regexp", value_name = "PATTERN")]
    extra_pattern: Option<String>,
}

fn main() {
    let mut args = Args::parse();

    // -e shifts pattern to files list
    if let Some(ep) = args.extra_pattern.take() {
        let old_pattern = std::mem::replace(&mut args.pattern, ep);
        args.files.insert(0, PathBuf::from(old_pattern));
    }

    let before_ctx = args.context.unwrap_or(0).max(args.before_context.unwrap_or(0));
    let after_ctx = args.context.unwrap_or(0).max(args.after_context.unwrap_or(0));

    let regex = build_regex(&args);

    let use_color = args.color
        || (io::stdout().is_terminal()
            && !args.count
            && !args.files_with_matches
            && !args.files_without_matches);

    let color_choice = if use_color {
        ColorChoice::Always
    } else {
        ColorChoice::Never
    };
    let stdout = StandardStream::stdout(color_choice);

    if args.files.is_empty() {
        let mut content = String::new();
        io::stdin().lock().read_to_string(&mut content).unwrap_or(0);
        let lines = html::extract_text(&content);
        let matched = search_lines(&lines, &regex, &args, before_ctx, after_ctx, None, false, &stdout);
        std::process::exit(if matched > 0 { 0 } else { 1 });
    }

    let html_exts = ["html", "htm", "xhtml", "shtml"];
    let files = collect_files(&args.files, args.recursive, &html_exts);

    if files.is_empty() {
        std::process::exit(1);
    }

    let multi = files.len() > 1;
    let mut total_matches: u64 = 0;

    for path in &files {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("hgrep: {}: {}", path.display(), e);
                continue;
            }
        };
        let lines = html::extract_text(&content);

        let show_filename = !args.count && !args.files_with_matches && !args.files_without_matches
            && (args.with_filename || (multi && !args.no_filename));

        let n = search_lines(
            &lines,
            &regex,
            &args,
            before_ctx,
            after_ctx,
            if show_filename { Some(path) } else { None },
            multi,
            &stdout,
        );

        if args.files_with_matches && n > 0 {
            println!("{}", path.display());
        } else if args.files_without_matches && n == 0 {
            println!("{}", path.display());
        } else if args.count {
            let prefix = if args.with_filename || (multi && !args.no_filename) {
                format!("{}:", path.display())
            } else {
                String::new()
            };
            println!("{}{}", prefix, n);
        }

        total_matches += n;
    }

    std::process::exit(if total_matches > 0 { 0 } else { 1 });
}

fn build_regex(args: &Args) -> Regex {
    let mut pat = args.pattern.clone();

    if args.fixed_strings {
        pat = regex::escape(&pat);
    }
    if args.word_regexp {
        pat = format!(r"\b{}\b", pat);
    }
    if args.line_regexp {
        pat = format!(r"^(?:{})$", pat);
    }

    match RegexBuilder::new(&pat)
        .case_insensitive(args.ignore_case)
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("hgrep: invalid pattern '{}': {}", pat, e);
            std::process::exit(2);
        }
    }
}

fn search_lines(
    lines: &[String],
    regex: &Regex,
    args: &Args,
    before_ctx: usize,
    after_ctx: usize,
    filename: Option<&Path>,
    _multi: bool,
    stdout: &StandardStream,
) -> u64 {
    let mut match_count: u64 = 0;

    let matched: Vec<bool> = lines
        .iter()
        .map(|line| {
            let m = regex.is_match(line);
            if args.invert_match { !m } else { m }
        })
        .collect();

    for &m in &matched {
        if m { match_count += 1; }
    }

    if args.count || args.files_with_matches || args.files_without_matches {
        return match_count;
    }

    // Determine which line indices to print, with context
    let mut to_print: Vec<(usize, bool)> = Vec::new();
    let mut prev_end: usize = 0;
    let mut need_sep = false;

    for (i, &is_matched) in matched.iter().enumerate() {
        if !is_matched { continue; }
        let start = i.saturating_sub(before_ctx);
        let end = (i + after_ctx + 1).min(lines.len());

        let has_context = before_ctx > 0 || after_ctx > 0;
        if has_context && need_sep && start > prev_end {
            to_print.push((usize::MAX, false)); // separator
        }
        for j in start..end {
            if j >= prev_end {
                to_print.push((j, matched[j]));
            }
        }
        if end > prev_end { prev_end = end; }
        need_sep = true;
    }

    let mut out = stdout.lock();
    for (idx, is_match_line) in to_print {
        if idx == usize::MAX {
            let _ = out.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)));
            let _ = writeln!(out, "--");
            let _ = out.reset();
            continue;
        }

        let line = &lines[idx];
        let lineno = idx + 1;
        let sep = if is_match_line { ':' } else { '-' };

        if let Some(path) = filename {
            let _ = out.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)));
            let _ = write!(out, "{}{}", path.display(), sep);
            let _ = out.reset();
        }
        if args.line_number {
            let _ = out.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
            let _ = write!(out, "{}{}", lineno, sep);
            let _ = out.reset();
        }

        if is_match_line && !args.invert_match {
            print_highlighted(&mut out, line, regex);
        } else {
            let _ = writeln!(out, "{}", line);
        }
    }

    match_count
}

fn print_highlighted(out: &mut termcolor::StandardStreamLock, line: &str, regex: &Regex) {
    let mut last = 0;
    for m in regex.find_iter(line) {
        let _ = write!(out, "{}", &line[last..m.start()]);
        let _ = out.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true));
        let _ = write!(out, "{}", m.as_str());
        let _ = out.reset();
        last = m.end();
    }
    let _ = writeln!(out, "{}", &line[last..]);
}

fn collect_files(paths: &[PathBuf], recursive: bool, extensions: &[&str]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_file() {
            files.push(path.clone());
        } else if path.is_dir() {
            if recursive {
                for entry in WalkDir::new(path)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    let p = entry.path();
                    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                        if extensions.iter().any(|&x| x.eq_ignore_ascii_case(ext)) {
                            files.push(p.to_path_buf());
                        }
                    }
                }
            } else {
                eprintln!("hgrep: {}: Is a directory", path.display());
            }
        } else {
            eprintln!("hgrep: {}: No such file or directory", path.display());
        }
    }
    files
}
