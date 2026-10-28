//! Zed extension that runs the shared `eudkit-lsp` language server for Python.
//!
//! The extension deliberately contains no completion logic. It only resolves a
//! binary and lets Zed speak LSP to it, so Zed and VS Code see the same
//! capabilities and the same canonical unit table.

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use zed_extension_api::{
    self as zed, Architecture, DownloadedFileType, LanguageServerId,
    LanguageServerInstallationStatus, Os, Result, Worktree,
};

const LANGUAGE_SERVER_ID: &str = "eudkit-lsp";
const BINARY_NAME: &str = "eudkit-lsp";
const ENV_OVERRIDE: &str = "EUDKIT_LSP_PATH";
const RELEASE_REPOSITORY: &str = "Tb1281/eudplib";
const RELEASE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Release assets published for `eudkit-lsp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReleasePlatform {
    WindowsX86_64,
    LinuxX86_64,
    MacosAarch64,
    MacosX86_64,
}

impl ReleasePlatform {
    fn for_target(os: Os, arch: Architecture) -> Result<Self> {
        match (os, arch) {
            (Os::Windows, Architecture::X8664) => Ok(Self::WindowsX86_64),
            (Os::Linux, Architecture::X8664) => Ok(Self::LinuxX86_64),
            (Os::Mac, Architecture::Aarch64) => Ok(Self::MacosAarch64),
            (Os::Mac, Architecture::X8664) => Ok(Self::MacosX86_64),
            _ => Err(format!(
                "unsupported platform {os:?}/{arch:?}: eudkit-lsp release assets exist only for \
                 Windows x86_64, Linux x86_64, macOS arm64, and macOS x86_64. Set {ENV_OVERRIDE} \
                 to a locally built eudkit-lsp or put eudkit-lsp on PATH."
            )),
        }
    }

    fn asset_name(self) -> &'static str {
        match self {
            Self::WindowsX86_64 => "eudkit-lsp-windows-x86_64.exe",
            Self::LinuxX86_64 => "eudkit-lsp-linux-x86_64",
            Self::MacosAarch64 => "eudkit-lsp-macos-aarch64",
            Self::MacosX86_64 => "eudkit-lsp-macos-x86_64",
        }
    }
}

struct EudkitExtension {
    cached_binary_path: Option<String>,
}

impl EudkitExtension {
    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<String> {
        if let Some(path) = env_override(&worktree.shell_env()) {
            let path = path.trim();
            if path.is_empty() {
                return Err(format!(
                    "{ENV_OVERRIDE} is set but empty; point it at a locally built eudkit-lsp \
                     executable or unset it"
                ));
            }
            println!("eudkit-lsp: using {ENV_OVERRIDE}={path}");
            return Ok(path.to_string());
        }

        if let Some(path) = worktree.which(BINARY_NAME) {
            println!("eudkit-lsp: using '{BINARY_NAME}' from PATH at {path}");
            return Ok(path);
        }

        if let Some(path) = &self.cached_binary_path {
            if is_file(Path::new(path)) {
                return Ok(path.clone());
            }
        }

        let path = self.download_binary(language_server_id)?;
        self.cached_binary_path = Some(path.clone());
        Ok(path)
    }

    fn download_binary(&mut self, language_server_id: &LanguageServerId) -> Result<String> {
        let result = self.download_binary_inner(language_server_id);
        match &result {
            Ok(_) => zed::set_language_server_installation_status(
                language_server_id,
                &LanguageServerInstallationStatus::None,
            ),
            Err(error) => zed::set_language_server_installation_status(
                language_server_id,
                &LanguageServerInstallationStatus::Failed(error.clone()),
            ),
        }
        result
    }

    fn download_binary_inner(&mut self, language_server_id: &LanguageServerId) -> Result<String> {
        let (os, arch) = zed::current_platform();
        let platform = ReleasePlatform::for_target(os, arch)?;
        let asset_name = platform.asset_name();
        let asset_url = release_asset_url(RELEASE_VERSION, asset_name);

        let version_dir = format!("eudkit-lsp-{RELEASE_VERSION}");
        let binary_relative = format!("{version_dir}/{asset_name}");
        let work_dir = extension_work_dir()?;
        let binary_path = work_dir.join(&binary_relative);

        if is_file(&binary_path) {
            make_executable(os, &binary_relative)?;
            return Ok(path_to_string(&binary_path));
        }

        fs::create_dir_all(work_dir.join(&version_dir)).map_err(|error| {
            format!("failed to create extension cache directory '{version_dir}': {error}")
        })?;

        zed::set_language_server_installation_status(
            language_server_id,
            &LanguageServerInstallationStatus::CheckingForUpdate,
        );
        println!("eudkit-lsp: downloading {asset_url}");

        zed::set_language_server_installation_status(
            language_server_id,
            &LanguageServerInstallationStatus::Downloading,
        );

        let temp_relative = format!("{binary_relative}.download");
        let temp_path = work_dir.join(&temp_relative);
        remove_file_if_exists(&temp_path);
        zed::download_file(&asset_url, &temp_relative, DownloadedFileType::Uncompressed).map_err(
            |error| {
                format!(
                    "failed to download {asset_name} from {asset_url}: {error}. Set \
                     {ENV_OVERRIDE} to a locally built eudkit-lsp or put eudkit-lsp on PATH."
                )
            },
        )?;

        let checksum_url = format!("{asset_url}.sha256");
        let checksum_relative = format!("{binary_relative}.sha256");
        let checksum_path = work_dir.join(&checksum_relative);
        match download_published_checksum(&checksum_url, &checksum_relative, &checksum_path)? {
            Some(published_checksum) => {
                let contents = fs::read(&temp_path).map_err(|error| {
                    format!(
                        "failed to read downloaded '{}': {error}",
                        temp_path.display()
                    )
                })?;
                verify_sha256(&contents, &published_checksum).map_err(|error| {
                    remove_file_if_exists(&temp_path);
                    format!(
                        "refusing to install {asset_name}: {error} (published at {checksum_url})"
                    )
                })?;
                println!("eudkit-lsp: verified SHA-256 checksum for {asset_name}");
            }
            None => println!(
                "eudkit-lsp: warning: no published checksum at {checksum_url}; \
                 continuing without checksum verification"
            ),
        }

        remove_file_if_exists(&binary_path);
        fs::rename(&temp_path, &binary_path).map_err(|error| {
            format!(
                "failed to move downloaded binary '{}' to '{}': {error}",
                temp_path.display(),
                binary_path.display()
            )
        })?;
        make_executable(os, &binary_relative)?;

        println!(
            "eudkit-lsp: installed {asset_name} at {}",
            binary_path.display()
        );
        Ok(path_to_string(&binary_path))
    }
}

impl zed::Extension for EudkitExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        if language_server_id.as_ref() != LANGUAGE_SERVER_ID {
            return Err(format!(
                "unknown language server: {language_server_id}; expected {LANGUAGE_SERVER_ID}"
            ));
        }

        Ok(zed::Command {
            command: self.language_server_binary_path(language_server_id, worktree)?,
            args: Vec::new(),
            env: Default::default(),
        })
    }
}

zed::register_extension!(EudkitExtension);

fn env_override(env: &[(String, String)]) -> Option<&str> {
    env.iter()
        .rev()
        .find_map(|(key, value)| (key == ENV_OVERRIDE).then_some(value.as_str()))
}

fn release_asset_url(version: &str, asset_name: &str) -> String {
    let tag = version.trim_start_matches('v');
    format!("https://github.com/{RELEASE_REPOSITORY}/releases/download/v{tag}/{asset_name}")
}

/// Zed runs extensions with the extension's work directory as the WASI current
/// directory. `zed_extension_api` 0.7.0 does not expose a dedicated
/// `extension_work_dir` helper, so resolve it here.
fn extension_work_dir() -> Result<PathBuf> {
    std::env::current_dir()
        .map_err(|error| format!("failed to resolve the extension work directory: {error}"))
}

fn is_file(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|metadata| metadata.is_file())
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn remove_file_if_exists(path: &Path) {
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => println!(
            "eudkit-lsp: warning: failed to remove stale '{}': {error}",
            path.display()
        ),
    }
}

fn make_executable(os: Os, relative_path: &str) -> Result<()> {
    match os {
        Os::Windows => Ok(()),
        Os::Linux | Os::Mac => zed::make_file_executable(relative_path)
            .map_err(|error| format!("failed to make '{relative_path}' executable: {error}")),
    }
}

fn download_published_checksum(
    checksum_url: &str,
    checksum_relative: &str,
    checksum_path: &Path,
) -> Result<Option<String>> {
    match zed::download_file(
        checksum_url,
        checksum_relative,
        DownloadedFileType::Uncompressed,
    ) {
        Ok(()) => fs::read_to_string(checksum_path)
            .map(Some)
            .map_err(|error| {
                format!(
                    "downloaded checksum '{}' but could not read it: {error}",
                    checksum_path.display()
                )
            }),
        Err(error) => {
            println!("eudkit-lsp: could not download checksum from {checksum_url}: {error}");
            Ok(None)
        }
    }
}

fn sha256_hex(contents: &[u8]) -> String {
    use std::fmt::Write as _;

    let digest = Sha256::digest(contents);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

/// Parses the first token of a SHA-256 checksum file.
///
/// GitHub release checksum companions commonly contain either a bare hash or
/// `sha256sum` output (`<hash>  <filename>` or `<hash> *<filename>`).
fn parse_sha256(checksum_file: &str) -> Result<String> {
    let token = checksum_file
        .split_whitespace()
        .next()
        .ok_or_else(|| "checksum file is empty".to_string())?
        .trim_start_matches('*');

    if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "checksum file does not contain a valid SHA-256 hash: {checksum_file:?}"
        ));
    }

    Ok(token.to_ascii_lowercase())
}

fn verify_sha256(contents: &[u8], checksum_file: &str) -> Result<()> {
    let expected = parse_sha256(checksum_file)?;
    let actual = sha256_hex(contents);
    if actual != expected {
        return Err(format!(
            "SHA-256 mismatch: expected {expected}, got {actual}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_windows_x86_64_asset() {
        assert_eq!(
            release_asset_name(Os::Windows, Architecture::X8664).unwrap(),
            "eudkit-lsp-windows-x86_64.exe"
        );
    }

    #[test]
    fn selects_linux_x86_64_asset() {
        assert_eq!(
            release_asset_name(Os::Linux, Architecture::X8664).unwrap(),
            "eudkit-lsp-linux-x86_64"
        );
    }

    #[test]
    fn selects_macos_aarch64_asset() {
        assert_eq!(
            release_asset_name(Os::Mac, Architecture::Aarch64).unwrap(),
            "eudkit-lsp-macos-aarch64"
        );
    }

    #[test]
    fn selects_macos_x86_64_asset() {
        assert_eq!(
            release_asset_name(Os::Mac, Architecture::X8664).unwrap(),
            "eudkit-lsp-macos-x86_64"
        );
    }

    #[test]
    fn rejects_unsupported_platforms_with_override_hint() {
        for (os, arch) in [
            (Os::Linux, Architecture::Aarch64),
            (Os::Linux, Architecture::X86),
            (Os::Windows, Architecture::Aarch64),
            (Os::Windows, Architecture::X86),
            (Os::Mac, Architecture::X86),
        ] {
            let error = release_asset_name(os, arch).unwrap_err();
            assert!(error.contains("unsupported platform"), "{error}");
            assert!(error.contains(ENV_OVERRIDE), "{error}");
        }
    }

    #[test]
    fn builds_release_asset_url_without_double_v_prefix() {
        assert_eq!(
            release_asset_url("0.1.0", "eudkit-lsp-linux-x86_64"),
            "https://github.com/Tb1281/eudplib/releases/download/v0.1.0/eudkit-lsp-linux-x86_64"
        );
        assert_eq!(
            release_asset_url("v0.1.0", "eudkit-lsp-macos-aarch64"),
            "https://github.com/Tb1281/eudplib/releases/download/v0.1.0/eudkit-lsp-macos-aarch64"
        );
    }

    #[test]
    fn finds_environment_override() {
        let env = vec![
            ("PATH".to_string(), "/usr/bin".to_string()),
            (ENV_OVERRIDE.to_string(), "/tmp/eudkit-lsp".to_string()),
        ];
        assert_eq!(env_override(&env), Some("/tmp/eudkit-lsp"));
        assert_eq!(env_override(&[]), None);
    }

    #[test]
    fn parses_sha256sum_output_and_bare_hashes() {
        let hash = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(parse_sha256(hash).unwrap(), hash);
        assert_eq!(
            parse_sha256(&format!("{hash}  eudkit-lsp-linux-x86_64")).unwrap(),
            hash
        );
        assert_eq!(
            parse_sha256(&format!("{hash} *eudkit-lsp-linux-x86_64")).unwrap(),
            hash
        );
        assert_eq!(parse_sha256(&hash.to_ascii_uppercase()).unwrap(), hash);
    }

    #[test]
    fn rejects_malformed_checksums() {
        assert!(parse_sha256("").is_err());
        assert!(parse_sha256("not-a-checksum").is_err());
        assert!(parse_sha256(&"a".repeat(63)).is_err());
        assert!(parse_sha256(&"g".repeat(64)).is_err());
    }

    #[test]
    fn verifies_known_sha256_vectors() {
        let empty = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq!(sha256_hex(b""), empty);
        verify_sha256(b"", empty).unwrap();

        let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        verify_sha256(b"abc", &format!("{abc}  asset")).unwrap();
    }

    #[test]
    fn reports_checksum_mismatch() {
        let wrong = "0000000000000000000000000000000000000000000000000000000000000000";
        let error = verify_sha256(b"abc", wrong).unwrap_err();
        assert!(error.contains("SHA-256 mismatch"), "{error}");
        assert!(error.contains(wrong), "{error}");
    }

    fn release_asset_name(os: Os, arch: Architecture) -> Result<&'static str> {
        ReleasePlatform::for_target(os, arch).map(ReleasePlatform::asset_name)
    }
}
