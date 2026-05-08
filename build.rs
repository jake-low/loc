use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use std::path::Path;

use serde::Deserialize;

// Generate language data

#[derive(Deserialize)]
struct LanguageDef {
    #[serde(skip)]
    name: String,

    /// Glob-style filename patterns for this language. Supports:
    /// - `*.ext` matches any file with the given extension
    /// - `Name` matches the exact filename
    /// - `Prefix*` matches any filename starting with `Prefix`
    #[serde(default)]
    patterns: Vec<String>,

    #[serde(default)]
    interpreters: Vec<String>,
    line_comment: Option<String>,
    block_comment_start: Option<String>,
    block_comment_end: Option<String>,

    #[serde(default)]
    single_line_strings: Vec<StringDelimiterDef>,

    #[serde(default)]
    multiline_strings: Vec<StringDelimiterDef>,

    #[serde(default)]
    docstring_delimiters: Vec<StringDelimiterDef>,
}

#[derive(Deserialize)]
struct StringDelimiterDef {
    open: String,
    close: String,
    backslash_escapes: bool,
}

/// Emit a Rust byte string literal, e.g. `b"//"`.
fn byte_lit(s: &str) -> String {
    let mut out = String::from("b\"");
    for b in s.bytes() {
        match b {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            0x20..=0x7e => out.push(b as char),
            _ => write!(out, "\\x{:02x}", b).unwrap(),
        }
    }
    out.push('"');
    out
}

fn emit_delimiter_array(out: &mut String, name: &str, delims: &[StringDelimiterDef]) {
    write!(out, "static {name}: &[StringDelimiter] = &[").unwrap();
    for d in delims {
        write!(
            out,
            "StringDelimiter {{ open: {}, close: {}, backslash_escapes: {} }},",
            byte_lit(&d.open),
            byte_lit(&d.close),
            d.backslash_escapes,
        )
        .unwrap();
    }
    writeln!(out, "];").unwrap();
}

#[derive(Default)]
struct LookupTables {
    extensions: BTreeMap<String, usize>,
    filenames: BTreeMap<String, usize>,
    prefixes: BTreeMap<String, usize>,
    interpreters: BTreeMap<String, usize>,
}

fn build_lookup_tables(langs: &[LanguageDef]) -> LookupTables {
    fn insert(
        map: &mut BTreeMap<String, usize>,
        kind: &str,
        key: String,
        id: usize,
        langs: &[LanguageDef],
    ) {
        if let Some(&prev) = map.get(&key) {
            if prev != id {
                panic!(
                    "languages.toml: {} {:?} is claimed by both {:?} and {:?}",
                    kind, key, langs[prev].name, langs[id].name
                );
            }
        }
        map.insert(key, id);
    }

    let mut tables = LookupTables::default();
    for (id, lang) in langs.iter().enumerate() {
        for pattern in &lang.patterns {
            let star_count = pattern.bytes().filter(|&b| b == b'*').count();
            match star_count {
                0 => insert(
                    &mut tables.filenames,
                    "filename",
                    pattern.clone(),
                    id,
                    &langs,
                ),
                1 => {
                    if let Some(ext) = pattern.strip_prefix("*.") {
                        insert(
                            &mut tables.extensions,
                            "extension",
                            ext.to_string(),
                            id,
                            &langs,
                        );
                    } else if let Some(prefix) = pattern.strip_suffix('*') {
                        insert(
                            &mut tables.prefixes,
                            "filename prefix",
                            prefix.to_string(),
                            id,
                            &langs,
                        );
                    } else {
                        panic!(
                            "Unsupported pattern {:?} in language {:?}",
                            pattern, lang.name
                        );
                    }
                }
                _ => panic!(
                    "Unsupported pattern {:?} in language {:?}",
                    pattern, lang.name
                ),
            }
        }
        for interp in &lang.interpreters {
            insert(
                &mut tables.interpreters,
                "interpreter",
                interp.to_lowercase(),
                id,
                &langs,
            );
        }
    }
    tables
}

fn gen_lang_data(out_dir: &str) {
    println!("cargo:rerun-if-changed=languages/");

    let mut entries: Vec<_> = fs::read_dir("languages")
        .expect("failed to read languages/ directory")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().and_then(|s| s.to_str()) == Some("toml") && e.path().is_file()
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut languages = Vec::new();
    for entry in &entries {
        let path = entry.path();
        println!("cargo:rerun-if-changed={}", path.display());
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("invalid language filename")
            .to_string();
        let src = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));
        let mut lang: LanguageDef = toml::from_str(&src)
            .unwrap_or_else(|e| panic!("failed to parse {}: {}", path.display(), e));
        lang.name = name;
        languages.push(lang);
    }

    let tables = build_lookup_tables(&languages);

    let dest = Path::new(out_dir).join("lang_data.rs");
    let mut out = String::new();

    writeln!(
        out,
        "// Auto-generated by build.rs from languages.toml. Do not edit."
    )
    .unwrap();
    writeln!(out).unwrap();

    for (i, lang) in languages.iter().enumerate() {
        write!(out, "static LANG_{i}_PATTERNS: &[&str] = &[").unwrap();
        for p in &lang.patterns {
            write!(out, "{:?},", p).unwrap();
        }
        writeln!(out, "];").unwrap();

        emit_delimiter_array(&mut out, &format!("LANG_{i}_SL"), &lang.single_line_strings);
        emit_delimiter_array(&mut out, &format!("LANG_{i}_ML"), &lang.multiline_strings);
        emit_delimiter_array(
            &mut out,
            &format!("LANG_{i}_DS"),
            &lang.docstring_delimiters,
        );
    }

    writeln!(out).unwrap();
    writeln!(
        out,
        "fn build_registry() -> (
    Vec<(&'static str, LangSyntax, &'static [&'static str])>,
    HashMap<&'static str, LanguageId>,
    HashMap<&'static str, LanguageId>,
    Vec<(&'static str, LanguageId)>,
    HashMap<&'static str, LanguageId>,
) {{"
    )
    .unwrap();
    writeln!(
        out,
        "    let mut languages: Vec<(&'static str, LangSyntax, &'static [&'static str])> = Vec::new();"
    )
    .unwrap();
    writeln!(
        out,
        "    let mut ext_map: HashMap<&'static str, LanguageId> = HashMap::new();"
    )
    .unwrap();
    writeln!(
        out,
        "    let mut filename_map: HashMap<&'static str, LanguageId> = HashMap::new();"
    )
    .unwrap();
    writeln!(
        out,
        "    let mut filename_prefix_list: Vec<(&'static str, LanguageId)> = Vec::new();"
    )
    .unwrap();
    writeln!(
        out,
        "    let mut interpreter_map: HashMap<&'static str, LanguageId> = HashMap::new();"
    )
    .unwrap();

    for (i, lang) in languages.iter().enumerate() {
        writeln!(out).unwrap();
        writeln!(out, "    // {}", lang.name).unwrap();

        let line_comment = lang
            .line_comment
            .as_deref()
            .map(|s| format!("Some({})", byte_lit(s)))
            .unwrap_or_else(|| "None".into());
        let block_comment = match (&lang.block_comment_start, &lang.block_comment_end) {
            (Some(start), Some(end)) => {
                format!("Some(({}, {}))", byte_lit(start), byte_lit(end))
            }
            _ => "None".into(),
        };

        writeln!(out, "    languages.push(({:?}, LangSyntax {{", lang.name).unwrap();
        writeln!(out, "        line_comment: {line_comment},").unwrap();
        writeln!(out, "        block_comment: {block_comment},").unwrap();
        writeln!(out, "        single_line_strings: LANG_{i}_SL,").unwrap();
        writeln!(out, "        multiline_strings: LANG_{i}_ML,").unwrap();
        writeln!(out, "        docstring_delimiters: LANG_{i}_DS,").unwrap();
        writeln!(out, "    }}, LANG_{i}_PATTERNS));").unwrap();
    }

    writeln!(out).unwrap();
    for (ext, &id) in &tables.extensions {
        writeln!(out, "    ext_map.insert({:?}, LanguageId({}));", ext, id).unwrap();
    }
    for (name, &id) in &tables.filenames {
        writeln!(
            out,
            "    filename_map.insert({:?}, LanguageId({}));",
            name, id
        )
        .unwrap();
    }
    for (prefix, &id) in &tables.prefixes {
        writeln!(
            out,
            "    filename_prefix_list.push(({:?}, LanguageId({})));",
            prefix, id
        )
        .unwrap();
    }
    for (interp, &id) in &tables.interpreters {
        writeln!(
            out,
            "    interpreter_map.insert({:?}, LanguageId({}));",
            interp, id
        )
        .unwrap();
    }

    writeln!(out).unwrap();
    writeln!(
        out,
        "    (languages, ext_map, filename_map, filename_prefix_list, interpreter_map)\n}}"
    )
    .unwrap();

    fs::write(&dest, out).expect("failed to write lang_data.rs");
}

// Generate tests from case files

fn parse_front_matter(content: &str, filename: &str) -> (usize, usize, usize, usize) {
    let parts: Vec<&str> = content.splitn(4, '\n').collect();
    assert!(
        parts.len() >= 3,
        "{}: file has fewer than 3 lines",
        filename
    );
    assert_eq!(
        parts[2], "---",
        "{}: expected '---' separator on line 3, got {:?}",
        filename, parts[2]
    );

    let mut lines: usize = 0;
    let mut code: usize = 0;
    let mut comments: usize = 0;
    let mut blank: usize = 0;

    for pair in parts[1].split_whitespace() {
        let (k, v) = pair
            .split_once('=')
            .unwrap_or_else(|| panic!("{}: bad key=value pair: {:?}", filename, pair));
        match k {
            "lines" => lines = v.parse().unwrap(),
            "code" => code = v.parse().unwrap(),
            "comments" => comments = v.parse().unwrap(),
            "blank" => blank = v.parse().unwrap(),
            _ => panic!("{}: unknown front matter key: {:?}", filename, k),
        }
    }

    (lines, blank, comments, code)
}

fn fn_name(filename: &str) -> String {
    filename
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

fn gen_test_cases(out_dir: &str) {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let tests_dir = Path::new(&manifest_dir).join("tests/");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=tests/");

    let out_path = Path::new(out_dir).join("count_tests.rs");

    let mut entries: Vec<_> = fs::read_dir(&tests_dir)
        .expect("tests directory not found")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut output = String::new();
    for entry in &entries {
        let path = entry.path();
        let filename = path.file_name().unwrap().to_str().unwrap();
        let abs_path = fs::canonicalize(&path).unwrap();
        println!("cargo:rerun-if-changed={}", path.display());

        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", filename, e));
        let (lines, blank, comments, code) = parse_front_matter(&content, filename);

        output.push_str(&format!(
            "test_case!({}, {:?}, {:?}, {}, {}, {}, {});\n",
            fn_name(filename),
            filename,
            abs_path.to_str().unwrap(),
            lines,
            blank,
            comments,
            code,
        ));
    }

    fs::write(&out_path, &output).unwrap();
}

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    gen_lang_data(&out_dir);
    gen_test_cases(&out_dir);
}
