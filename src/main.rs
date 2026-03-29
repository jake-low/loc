use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, IsTerminal, Seek, SeekFrom, Write};
use std::ops::AddAssign;
use std::path::Path;
use std::sync::mpsc;

use clap::Parser;
use num_format::{Locale, SystemLocale, ToFormattedString};
use rayon::prelude::*;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

mod count;
mod languages;

use count::{Counts, count_lines_from_reader};
use languages::{LanguageId, LanguageRegistry};

#[derive(Parser)]
#[command(version, about, author, styles = clap::builder::Styles::plain())]
#[command(long_about = r#"
loc counts lines of code in files, reporting blank, comment, and code line
counts by language. It reads file paths from stdin, so you control which files
are counted by composing it with tools like find or git ls-files."#)]
#[command(after_help = "\
Examples:
  git ls-files | loc
  find src/ -name '*.rs' | loc")]
#[command(help_template = "{about}\n\n{all-args}{after-help}\n\nAuthor: {author}")]
struct CliArgs {
    /// Print supported languages and their file patterns, then exit
    #[arg(long)]
    languages: bool,
}

#[derive(Debug, Default, Clone, Copy)]
struct Stats {
    files: usize,
    bytes: u64,
    counts: Counts,
}

impl AddAssign<&Stats> for Stats {
    fn add_assign(&mut self, other: &Stats) {
        self.files += other.files;
        self.bytes += other.bytes;
        self.counts += other.counts;
    }
}

fn format_bytes(bytes: u64, decimal_sep: &str) -> String {
    if bytes < 1024 {
        format!("{}", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0).replace('.', decimal_sep)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0)).replace('.', decimal_sep)
    }
}

fn process_file(path: &str, registry: &LanguageRegistry) -> Option<(LanguageId, Stats)> {
    let (lang_id, syntax, file) = if let Some((lang_id, syntax)) = registry.get_language(path) {
        let file = fs::File::open(path).ok()?;
        (lang_id, syntax, file)
    } else if Path::new(path).extension().is_none() {
        let mut file = fs::File::open(path).ok()?;
        let (lang_id, syntax) = registry.sniff_language_from_shebang(&mut file)?;
        file.seek(SeekFrom::Start(0)).ok()?;
        (lang_id, syntax, file)
    } else {
        return None;
    };

    let (counts, bytes) = count_lines_from_reader(file, syntax);

    Some((
        lang_id,
        Stats {
            files: 1,
            bytes,
            counts,
        },
    ))
}

fn main() {
    let args = CliArgs::parse();

    let registry = LanguageRegistry::new();

    if args.languages {
        let width = registry
            .all_languages_with_patterns()
            .map(|(_, name, _)| name.len())
            .max()
            .unwrap();
        for (_, name, patterns) in registry.all_languages_with_patterns() {
            println!("{:<width$}  {}", name, patterns.join("  "));
        }
        return;
    }

    let (tx, rx) = mpsc::sync_channel::<String>(1024);
    std::thread::spawn(move || {
        for line in io::stdin().lock().lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    let results: Vec<(LanguageId, Stats)> = rx
        .into_iter()
        .par_bridge()
        .filter_map(|path| process_file(&path, &registry))
        .collect();

    let mut lang_stats: HashMap<LanguageId, Stats> = HashMap::new();
    for (lang_id, stats) in results {
        *lang_stats.entry(lang_id).or_default() += &stats;
    }

    let mut sorted_langs: Vec<_> = lang_stats.iter().collect();
    sorted_langs.sort_by_key(|(id, stats)| (std::cmp::Reverse(stats.counts.code), **id));

    let sys_locale = SystemLocale::default().ok();
    let decimal_sep = sys_locale.as_ref().map(|l| l.decimal()).unwrap_or(".");
    let fmt = |n: usize| -> String {
        match &sys_locale {
            Some(loc) => n.to_formatted_string(loc),
            None => n.to_formatted_string(&Locale::en),
        }
    };

    let color_choice = if std::io::stdout().is_terminal() {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    };
    let mut stdout = StandardStream::stdout(color_choice);

    stdout
        .set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))
        .unwrap();
    writeln!(
        &mut stdout,
        "{:<14} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
        "Language", "Files", "Bytes", "Lines", "Blanks", "Comments", "Code"
    )
    .unwrap();
    stdout.reset().unwrap();

    let mut total = Stats::default();
    for (lang_id, stats) in sorted_langs {
        writeln!(
            &mut stdout,
            "{:<14} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
            registry.language_name(*lang_id),
            fmt(stats.files),
            format_bytes(stats.bytes, decimal_sep),
            fmt(stats.counts.lines),
            fmt(stats.counts.blanks),
            fmt(stats.counts.comments),
            fmt(stats.counts.code)
        )
        .unwrap();
        total += stats;
    }

    stdout
        .set_color(
            ColorSpec::new()
                .set_fg(Some(Color::White))
                .set_intense(true)
                .set_bold(true),
        )
        .unwrap();
    writeln!(
        &mut stdout,
        "{:<14} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
        "Total",
        fmt(total.files),
        format_bytes(total.bytes, decimal_sep),
        fmt(total.counts.lines),
        fmt(total.counts.blanks),
        fmt(total.counts.comments),
        fmt(total.counts.code)
    )
    .unwrap();
    stdout.reset().unwrap();
}
