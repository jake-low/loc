use crate::count::{LangSyntax, StringDelimiter};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LanguageId(usize);

pub struct LanguageRegistry {
    languages: Vec<(&'static str, LangSyntax, &'static [&'static str])>,
    ext_map: HashMap<&'static str, LanguageId>,
    filename_map: HashMap<&'static str, LanguageId>,
    filename_prefix_list: Vec<(&'static str, LanguageId)>,
    interpreter_map: HashMap<&'static str, LanguageId>,
}

include!(concat!(env!("OUT_DIR"), "/lang_data.rs"));

impl Default for LanguageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageRegistry {
    pub fn new() -> Self {
        let (languages, ext_map, filename_map, filename_prefix_list, interpreter_map) =
            build_registry();
        Self {
            languages,
            ext_map,
            filename_map,
            filename_prefix_list,
            interpreter_map,
        }
    }

    pub fn get_language(&self, path: &str) -> Option<(LanguageId, &LangSyntax)> {
        let path_obj = Path::new(path);

        // Fast path: extension lookup (case-sensitive).
        if let Some(ext) = path_obj.extension().and_then(|e| e.to_str()) {
            if let Some(id) = self.ext_map.get(ext) {
                return Some((*id, &self.languages[id.0].1));
            }
        }

        // Fallback: exact filename and prefix pattern checks (case-sensitive).
        if let Some(filename) = path_obj.file_name().and_then(|f| f.to_str()) {
            if let Some(id) = self.filename_map.get(filename) {
                return Some((*id, &self.languages[id.0].1));
            }
            for (prefix, id) in &self.filename_prefix_list {
                if filename.starts_with(prefix) {
                    return Some((*id, &self.languages[id.0].1));
                }
            }
        }

        None
    }

    /// Check the shebang line of a file to determine its language.
    ///
    /// Reads at most 128 bytes; the caller is responsible for seeking back if it
    /// wants to reuse the descriptor for counting.
    pub fn sniff_language_from_shebang(
        &self,
        file: &mut impl Read,
    ) -> Option<(LanguageId, &LangSyntax)> {
        let mut buf = [0u8; 128];
        let n = file.read(&mut buf).ok()?;
        let first_line = buf[..n]
            .split(|&b| b == b'\n')
            .next()
            .and_then(|b| std::str::from_utf8(b).ok())
            .unwrap_or("");
        let id = self.detect_shebang(first_line)?;
        Some((id, &self.languages[id.0].1))
    }

    /// Given a `#!` line, return the language id by looking up the interpreter
    /// basename (e.g. `python3` from `#!/usr/bin/python3`) and, for `env`-style
    /// shebangs, the next word (e.g. `ruby` from `#!/usr/bin/env ruby`).
    fn detect_shebang(&self, line: &str) -> Option<LanguageId> {
        if !line.starts_with("#!") {
            return None;
        }
        let mut words = line[2..].trim().split_ascii_whitespace();
        let exe = words.next()?;
        let basename = exe.split('/').next_back().unwrap_or("").to_lowercase();
        if let Some(id) = self.interpreter_map.get(basename.as_str()) {
            Some(*id)
        } else {
            let second = words.next()?.to_lowercase();
            self.interpreter_map.get(second.as_str()).copied()
        }
    }

    pub fn all_languages_with_patterns(
        &self,
    ) -> impl Iterator<Item = (LanguageId, &str, &'static [&'static str])> + '_ {
        self.languages
            .iter()
            .enumerate()
            .map(|(i, (name, _, patterns))| (LanguageId(i), *name, *patterns))
    }

    pub fn language_name(&self, id: LanguageId) -> &str {
        self.languages[id.0].0
    }
}
