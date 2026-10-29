use crate::error::{EudkitError, Result};
use crate::session::CompilationConfig;
use std::fs;
use std::path::{Path, PathBuf};

/// The project file discovered when a build request has no explicit config.
pub const PROJECT_FILE: &str = "eudkit.toml";

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
                | ("build", "preserve_classic_triggers")
                | ("project", "preserve_classic_triggers") => {
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

/// The effective project configuration plus the project file that supplied it.
///
/// `config_path` is `None` when the request resolved entirely from flags and
/// defaults. Callers use it for provenance and for watching the project file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedBuild {
    pub config: ProjectConfig,
    pub config_path: Option<PathBuf>,
}

/// One parsed build request.
///
/// The request owns the shared flag vocabulary for `build`, `watch`, and
/// `dump-ir`, and `resolve` owns project discovery, section aliases, and the
/// precedence rule "explicit flag, then project file, then default". Frontends
/// and CLIs construct the same request instead of re-parsing `eudkit.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildRequest {
    /// Explicit `--config` path. When absent, [`PROJECT_FILE`] is discovered.
    pub config: Option<PathBuf>,
    pub input_map: Option<PathBuf>,
    pub output_map: Option<PathBuf>,
    pub entry: Option<PathBuf>,
    pub dts: Option<PathBuf>,
    pub obscurate: Option<bool>,
    pub obscurate_seed: Option<u64>,
    pub preserve_classic_triggers: Option<bool>,
    /// `--watch` / `-w`: run a watch loop after the initial build.
    pub watch: bool,
    /// `--once`: stop after the initial watch-loop build.
    pub once: bool,
    /// `--hir-only`: dump High-Level IR only.
    pub hir_only: bool,
    /// `--lir-only`: dump Low-Level IR only.
    pub lir_only: bool,
    /// `--help` / `-h`: print command usage without resolving anything.
    pub help: bool,
}

impl BuildRequest {
    /// Parses the flags shared by `build`, `watch`, and `dump-ir`.
    ///
    /// A single bare argument is accepted as the input map, matching
    /// `dump-ir <map.scx>`.
    pub fn from_args(args: &[String]) -> Result<Self> {
        let mut request = Self::default();
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--config" | "-c" => {
                    request.config = Some(take_path(args, &mut index, "--config")?);
                }
                "--input" | "-i" | "--map" | "-m" => {
                    request.input_map = Some(take_path(args, &mut index, "--input")?);
                }
                "--output" | "-o" => {
                    request.output_map = Some(take_path(args, &mut index, "--output")?);
                }
                "--entry" | "-e" => {
                    request.entry = Some(take_path(args, &mut index, "--entry")?);
                }
                "--dts" => {
                    request.dts = Some(take_path(args, &mut index, "--dts")?);
                }
                "--obscurate" => request.obscurate = Some(true),
                "--seed" => {
                    let value = take_value(args, &mut index, "--seed")?;
                    request.obscurate_seed = Some(value.parse::<u64>().map_err(|_| {
                        EudkitError::Config(format!("invalid --seed value: {value}"))
                    })?);
                }
                "--watch" | "-w" => request.watch = true,
                "--once" => request.once = true,
                "--hir-only" => request.hir_only = true,
                "--lir-only" => request.lir_only = true,
                "--help" | "-h" => request.help = true,
                other if !other.starts_with('-') && request.input_map.is_none() => {
                    request.input_map = Some(PathBuf::from(other));
                }
                other => {
                    return Err(EudkitError::Config(format!(
                        "unknown argument for build: {other}"
                    )));
                }
            }
            index += 1;
        }

        Ok(request)
    }

    /// Resolves against the current working directory.
    pub fn resolve(&self) -> Result<ResolvedBuild> {
        self.resolve_in(Path::new("."))
    }

    /// Resolves flags and the project file rooted at `base`.
    ///
    /// An explicit `--config` always wins over discovery. Flags override every
    /// field the project file supplies; omitted flags keep the file value.
    pub fn resolve_in(&self, base: &Path) -> Result<ResolvedBuild> {
        let config_path = match &self.config {
            Some(path) => Some(if path.is_absolute() {
                path.clone()
            } else {
                base.join(path)
            }),
            None => {
                let candidate = base.join(PROJECT_FILE);
                candidate.is_file().then_some(candidate)
            }
        };

        let mut config = match &config_path {
            Some(path) => ProjectConfig::from_file(path)?,
            None => ProjectConfig::default(),
        };

        if let Some(input_map) = &self.input_map {
            config.input_map = input_map.clone();
        }
        if let Some(output_map) = &self.output_map {
            config.output_map = output_map.clone();
        }
        if let Some(entry) = &self.entry {
            config.entry = Some(entry.clone());
        }
        if let Some(dts) = &self.dts {
            config.dts = Some(dts.clone());
        }
        if let Some(obscurate) = self.obscurate {
            config.obscurate = obscurate;
        }
        if let Some(seed) = self.obscurate_seed {
            config.obscurate_seed = Some(seed);
        }
        if let Some(preserve) = self.preserve_classic_triggers {
            config.preserve_classic_triggers = preserve;
        }

        Ok(ResolvedBuild {
            config,
            config_path,
        })
    }
}

fn take_value<'a>(args: &'a [String], index: &mut usize, flag: &str) -> Result<&'a str> {
    *index += 1;
    args.get(*index)
        .map(String::as_str)
        .ok_or_else(|| EudkitError::Config(format!("missing value for {flag}")))
}

fn take_path(args: &[String], index: &mut usize, flag: &str) -> Result<PathBuf> {
    take_value(args, index, flag).map(PathBuf::from)
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
    if let Some(hex) = clean
        .strip_prefix("0x")
        .or_else(|| clean.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16)
    } else {
        clean.parse::<u64>()
    }
    .map_err(|_| EudkitError::Config(format!("expected integer on line {line_no}, got {val}")))
}
