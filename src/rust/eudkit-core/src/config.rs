use crate::error::{EudkitError, Result};
use crate::session::CompilationConfig;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectConfig {
    pub name: String,
    pub entry: Option<PathBuf>,
    pub input_map: PathBuf,
    pub output_map: PathBuf,
    pub dts: Option<PathBuf>,
    pub obscurate: bool,
    pub obscurate_seed: Option<u64>,
    pub preserve_classic_triggers: bool,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: "eudkit_project".to_string(),
            entry: None,
            input_map: PathBuf::new(),
            output_map: PathBuf::new(),
            dts: None,
            obscurate: false,
            obscurate_seed: None,
            preserve_classic_triggers: true,
        }
    }
}

impl ProjectConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())?;
        let mut cfg = Self::parse_str(&content)?;
        if let Some(parent) = path.as_ref().parent() {
            if !parent.as_os_str().is_empty() {
                if !cfg.input_map.as_os_str().is_empty() && cfg.input_map.is_relative() {
                    let candidate = parent.join(&cfg.input_map);
                    if candidate.exists() {
                        cfg.input_map = candidate;
                    }
                }
                if !cfg.output_map.as_os_str().is_empty() && cfg.output_map.is_relative() {
                    cfg.output_map = parent.join(&cfg.output_map);
                }
                if let Some(entry) = &cfg.entry {
                    if entry.is_relative() {
                        let candidate = parent.join(entry);
                        if candidate.exists() {
                            cfg.entry = Some(candidate);
                        }
                    }
                }
                if let Some(dts) = &cfg.dts {
                    if dts.is_relative() {
                        cfg.dts = Some(parent.join(dts));
                    }
                }
            }
        }
        Ok(cfg)
    }

    pub fn parse_str(input: &str) -> Result<Self> {
        let mut cfg = Self::default();
        let mut current_section = String::new();

        for (line_no, raw_line) in input.lines().enumerate() {
            let line = strip_toml_comment(raw_line).trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                current_section = line[1..line.len() - 1].trim().to_string();
                continue;
            }
            let Some((raw_key, raw_val)) = line.split_once('=') else {
                return Err(EudkitError::Config(format!(
                    "invalid key=value on line {}: {}",
                    line_no + 1,
                    raw_line
                )));
            };
            let key = raw_key.trim();
            let val = raw_val.trim();

            match (current_section.as_str(), key) {
                ("", "name") | ("project", "name") => {
                    cfg.name = parse_toml_string(val, line_no + 1)?;
                }
                ("", "entry") | ("project", "entry") | ("build", "entry") => {
                    cfg.entry = Some(PathBuf::from(parse_toml_string(val, line_no + 1)?));
                }
                ("", "dts") | ("project", "dts") | ("build", "dts") => {
                    cfg.dts = Some(PathBuf::from(parse_toml_string(val, line_no + 1)?));
                }
                ("", "input_map")
                | ("", "input")
                | ("project", "input_map")
                | ("build", "input_map")
                | ("build", "input") => {
                    cfg.input_map = PathBuf::from(parse_toml_string(val, line_no + 1)?);
                }
                ("", "output_map")
                | ("", "output")
                | ("project", "output_map")
                | ("build", "output_map")
                | ("build", "output") => {
                    cfg.output_map = PathBuf::from(parse_toml_string(val, line_no + 1)?);
                }
                ("", "obscurate") | ("build", "obscurate") | ("project", "obscurate") => {
                    cfg.obscurate = parse_toml_bool(val, line_no + 1)?;
                }
                ("", "obscurate_seed")
                | ("build", "obscurate_seed")
                | ("project", "obscurate_seed") => {
                    cfg.obscurate_seed = Some(parse_toml_u64(val, line_no + 1)?);
                }
                ("", "preserve_classic_triggers")
                | ("build", "preserve_classic_triggers") => {
                    cfg.preserve_classic_triggers = parse_toml_bool(val, line_no + 1)?;
                }
                _ => {}
            }
        }

        Ok(cfg)
    }

    pub fn to_compilation_config(&self) -> CompilationConfig {
        CompilationConfig {
            obscurate: self.obscurate,
            obscurate_seed: self.obscurate_seed,
            preserve_classic_triggers: self.preserve_classic_triggers,
        }
    }
}

fn strip_toml_comment(line: &str) -> &str {
    let mut in_quotes = false;
    let mut escaped = false;
    for (idx, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && in_quotes {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_quotes = !in_quotes;
            continue;
        }
        if ch == '#' && !in_quotes {
            return &line[..idx];
        }
    }
    line
}

fn parse_toml_string(val: &str, line_no: usize) -> Result<String> {
    if val.len() >= 2 && val.starts_with('"') && val.ends_with('"') {
        let inner = &val[1..val.len() - 1];
        let mut out = String::with_capacity(inner.len());
        let mut chars = inner.chars();
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some('\\') => out.push('\\'),
                    Some('"') => out.push('"'),
                    Some(other) => {
                        out.push('\\');
                        out.push(other);
                    }
                    None => out.push('\\'),
                }
            } else {
                out.push(ch);
            }
        }
        Ok(out)
    } else if val.len() >= 2 && val.starts_with('\'') && val.ends_with('\'') {
        Ok(val[1..val.len() - 1].to_string())
    } else {
        Err(EudkitError::Config(format!(
            "expected quoted string on line {line_no}, got {val}"
        )))
    }
}

fn parse_toml_bool(val: &str, line_no: usize) -> Result<bool> {
    match val {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(EudkitError::Config(format!(
            "expected boolean on line {line_no}, got {val}"
        ))),
    }
}

fn parse_toml_u64(val: &str, line_no: usize) -> Result<u64> {
    let clean = val.replace('_', "");
    if let Some(hex) = clean.strip_prefix("0x").or_else(|| clean.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16)
    } else {
        clean.parse::<u64>()
    }
    .map_err(|_| EudkitError::Config(format!("expected integer on line {line_no}, got {val}")))
}
