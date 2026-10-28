//! Discovery and caching of `eudkit.toml` projects for map-aware completion.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use eudkit_core::config::ProjectConfig;
use eudkit_core::map_context::MapContext;
use eudkit_core::session::{CompilationConfig, CompilationSession};

/// Project file searched for from each document's directory upwards.
pub const PROJECT_FILE: &str = "eudkit.toml";

/// Find the nearest `eudkit.toml` at or above `document_path`.
pub fn find_project_toml(document_path: &Path) -> Option<PathBuf> {
    let mut directory = if document_path.is_dir() {
        Some(document_path)
    } else {
        document_path.parent()
    };

    while let Some(current) = directory {
        let candidate = current.join(PROJECT_FILE);
        if candidate.is_file() {
            return Some(candidate);
        }
        directory = current.parent();
    }

    None
}

/// Map Contexts loaded from each discovered project, keyed by `eudkit.toml`.
#[derive(Debug, Default)]
pub struct ProjectCache {
    projects: HashMap<PathBuf, CachedProject>,
}

#[derive(Debug, Default)]
struct CachedProject {
    toml_modified: Option<SystemTime>,
    map_path: Option<PathBuf>,
    map_modified: Option<SystemTime>,
    context: Option<Arc<MapContext>>,
}

impl ProjectCache {
    /// Resolve the nearest project for `document_path` and return its Map
    /// Context, reloading it when the project file or input map changed.
    pub fn map_context_for_document(&mut self, document_path: &Path) -> Option<Arc<MapContext>> {
        let toml_path = find_project_toml(document_path)?;
        self.map_context_for_project(&toml_path)
    }

    fn map_context_for_project(&mut self, toml_path: &Path) -> Option<Arc<MapContext>> {
        let toml_path = if toml_path.is_absolute() {
            toml_path.to_path_buf()
        } else {
            fs::canonicalize(toml_path).unwrap_or_else(|_| toml_path.to_path_buf())
        };
        let toml_path = toml_path.as_path();
        let toml_modified = modified(toml_path);
        let needs_reload = match self.projects.get(toml_path) {
            None => true,
            Some(cached) => {
                cached.toml_modified != toml_modified
                    || cached
                        .map_path
                        .as_deref()
                        .is_some_and(|path| modified(path) != cached.map_modified)
            }
        };

        if needs_reload {
            let project = load_project(toml_path);
            self.projects.insert(toml_path.to_path_buf(), project);
        }

        self.projects
            .get(toml_path)
            .and_then(|cached| cached.context.clone())
    }
}

fn load_project(toml_path: &Path) -> CachedProject {
    let toml_modified = modified(toml_path);
    let config = match ProjectConfig::from_file(toml_path) {
        Ok(config) => config,
        Err(error) => {
            eprintln!(
                "eudkit-lsp: failed to load {}: {error}",
                toml_path.display()
            );
            return CachedProject {
                toml_modified,
                ..CachedProject::default()
            };
        }
    };

    if config.input_map.as_os_str().is_empty() {
        return CachedProject {
            toml_modified,
            ..CachedProject::default()
        };
    }

    let map_path = resolve_input_map(toml_path, &config);
    let map_modified = modified(&map_path);
    let context = load_map_context(&map_path, toml_path);

    CachedProject {
        toml_modified,
        map_path: Some(map_path),
        map_modified,
        context,
    }
}

fn load_map_context(map_path: &Path, toml_path: &Path) -> Option<Arc<MapContext>> {
    let loaded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        CompilationSession::from_map(map_path, CompilationConfig::default())
    }));

    match loaded {
        Ok(Ok(session)) => Some(Arc::new(session.map_context().clone())),
        Ok(Err(error)) => {
            eprintln!(
                "eudkit-lsp: failed to load input map {} for {}: {error}",
                map_path.display(),
                toml_path.display()
            );
            None
        }
        Err(_) => {
            eprintln!(
                "eudkit-lsp: panicked while loading input map {} for {}",
                map_path.display(),
                toml_path.display()
            );
            None
        }
    }
}

fn resolve_input_map(toml_path: &Path, config: &ProjectConfig) -> PathBuf {
    if config.input_map.is_absolute() {
        return config.input_map.clone();
    }

    match toml_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(&config.input_map),
        _ => config.input_map.clone(),
    }
}

fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

#[cfg(test)]
mod tests {
    use super::{find_project_toml, ProjectCache, PROJECT_FILE};
    use std::fs;

    #[test]
    fn finds_the_nearest_project_file() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let nested = root.join("workspace").join("project").join("src");
        fs::create_dir_all(&nested).unwrap();

        let outer = root.join(PROJECT_FILE);
        let inner = root.join("workspace").join("project").join(PROJECT_FILE);
        fs::write(&outer, "[project]\nname = \"outer\"\n").unwrap();
        fs::write(&inner, "[project]\nname = \"inner\"\n").unwrap();

        assert_eq!(
            find_project_toml(&nested.join("main.py")),
            Some(inner.clone())
        );
        assert_eq!(find_project_toml(&nested), Some(inner.clone()));
        assert_eq!(
            find_project_toml(&root.join("workspace").join("other.py")),
            Some(outer)
        );
    }

    #[test]
    fn returns_none_without_a_project_file() {
        let temp = tempfile::tempdir().unwrap();
        let document = temp.path().join("src").join("main.py");
        fs::create_dir_all(document.parent().unwrap()).unwrap();

        assert_eq!(find_project_toml(&document), None);
    }

    #[test]
    fn project_without_an_input_map_has_no_context() {
        let temp = tempfile::tempdir().unwrap();
        let toml = temp.path().join(PROJECT_FILE);
        let document = temp.path().join("main.py");
        fs::write(&toml, "[project]\nname = \"canonical\"\n").unwrap();

        let mut cache = ProjectCache::default();
        assert!(cache.map_context_for_document(&document).is_none());
    }
}
