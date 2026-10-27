use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigratedProject {
    pub name: String,
    pub input_map: PathBuf,
    pub output_map: PathBuf,
    pub entry_eps: Option<PathBuf>,
    pub entry_ts: PathBuf,
    pub plugins: Vec<String>,
}

impl MigratedProject {
    pub fn to_toml_string(&self) -> String {
        let input_map_str = self.input_map.to_string_lossy();
        let output_map_str = self.output_map.to_string_lossy();
        let entry_str = self.entry_ts.to_string_lossy();

        format!(
            r#"[project]
name = "{name}"
entry = "{entry}"

[build]
input_map = "{input_map}"
output_map = "{output_map}"
obscurate = false
"#,
            name = self.name,
            entry = entry_str,
            input_map = input_map_str,
            output_map = output_map_str,
        )
    }
}

pub fn parse_edd_or_eds(content: &str, file_path: &Path) -> Result<MigratedProject, String> {
    let mut current_section = String::new();
    let mut input_map = PathBuf::new();
    let mut output_map = PathBuf::new();
    let mut entry_eps: Option<PathBuf> = None;
    let mut plugins = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') || trimmed.starts_with("//") {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = trimmed[1..trimmed.len() - 1].trim();
            current_section = section.to_string();

            if section.ends_with(".eps") {
                if entry_eps.is_none() {
                    entry_eps = Some(PathBuf::from(section));
                }
            } else if !section.eq_ignore_ascii_case("main") && !section.eq_ignore_ascii_case("plugins") {
                plugins.push(section.to_string());
            }
            continue;
        }

        // Split by ':' or '='
        let split = if let Some(pos) = trimmed.find(':') {
            Some((&trimmed[..pos], &trimmed[pos + 1..]))
        } else {
            trimmed
                .find('=')
                .map(|pos| (&trimmed[..pos], &trimmed[pos + 1..]))
        };

        if let Some((raw_k, raw_v)) = split {
            let key = raw_k.trim().to_ascii_lowercase();
            let mut val = raw_v.trim();
            if ((val.starts_with('"') && val.ends_with('"'))
                || (val.starts_with('\'') && val.ends_with('\'')))
                && val.len() >= 2
            {
                val = &val[1..val.len() - 1];
            }

            match (current_section.to_ascii_lowercase().as_str(), key.as_str()) {
                ("main", "input") | ("main", "input_map") | ("main", "inputmap") => {
                    input_map = PathBuf::from(val);
                }
                ("main", "output") | ("main", "output_map") | ("main", "outputmap") => {
                    output_map = PathBuf::from(val);
                }
                ("main", "entry") | ("main", "main") => {
                    entry_eps = Some(PathBuf::from(val));
                }
                _ => {
                    if val.ends_with(".eps") && entry_eps.is_none() {
                        entry_eps = Some(PathBuf::from(val));
                    }
                }
            }
        }
    }

    let default_stem = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("migrated_project");

    let (name, entry_ts) = if let Some(ref eps) = entry_eps {
        let stem = eps
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(default_stem);
        let mut ts = eps.clone();
        ts.set_extension("ts");
        (stem.to_string(), ts)
    } else {
        (default_stem.to_string(), PathBuf::from("src/main.ts"))
    };

    Ok(MigratedProject {
        name,
        input_map,
        output_map,
        entry_eps,
        entry_ts,
        plugins,
    })
}
