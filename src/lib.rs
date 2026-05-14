use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result};

const LSP_ID: &str = "ctrmml-lsp";
const LSP_REPO: &str = "ulalume/language-server-ctrmml";
// Pin to a known-good language-server release. Bump in lockstep with
// `web-ctrmml/wasm-lang-core/build.sh` so the grammar contract stays
// in sync.
const LSP_PINNED_TAG: &str = "v0.6.1";
const UPDATE_CHECK_INTERVAL: Duration = Duration::from_secs(30 * 60);
const UPDATE_CHECK_FILENAME: &str = ".ctrmml-lsp-last-check";

struct CtrmmlExtension {
    cached_binary_path: Option<String>,
}

fn update_check_path() -> PathBuf {
    Path::new(UPDATE_CHECK_FILENAME).to_path_buf()
}

fn read_last_update_check() -> Option<SystemTime> {
    let contents = fs::read_to_string(update_check_path()).ok()?;
    let secs = contents.trim().parse::<u64>().ok()?;
    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(secs))
}

fn update_check_due() -> bool {
    match read_last_update_check() {
        Some(last) => last
            .elapsed()
            .map(|elapsed| elapsed >= UPDATE_CHECK_INTERVAL)
            .unwrap_or(true),
        None => true,
    }
}

fn record_update_check() {
    let secs = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => return,
    };
    let _ = fs::write(update_check_path(), secs.to_string());
}

fn find_cached_binary(platform: zed::Os) -> Option<String> {
    let bin_name = match platform {
        zed::Os::Windows => format!("{LSP_ID}.exe"),
        _ => LSP_ID.to_string(),
    };
    let mut best: Option<(SystemTime, PathBuf)> = None;
    let entries = fs::read_dir(".").ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        if !name.to_string_lossy().starts_with(LSP_ID) {
            continue;
        }
        let candidate = path.join(&bin_name);
        if candidate.is_file() {
            let modified = fs::metadata(&candidate)
                .and_then(|meta| meta.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let replace = match &best {
                Some((current, _)) => modified > *current,
                None => true,
            };
            if replace {
                best = Some((modified, candidate));
            }
        }
    }
    best.map(|(_, path)| path.to_string_lossy().to_string())
}

impl CtrmmlExtension {
    fn lsp_settings(
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Option<LspSettings> {
        LspSettings::for_worktree(language_server_id.as_ref(), worktree).ok()
    }

    fn lsp_settings_value<F>(
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
        extractor: F,
    ) -> Result<Option<zed::serde_json::Value>>
    where
        F: FnOnce(&LspSettings) -> Option<zed::serde_json::Value>,
    {
        let settings = Self::lsp_settings(language_server_id, worktree)
            .and_then(|lsp_settings| extractor(&lsp_settings))
            .unwrap_or_default();
        Ok(Some(settings))
    }

    fn resolve_language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let lsp_settings = Self::lsp_settings(language_server_id, worktree);
        let binary_settings = lsp_settings
            .as_ref()
            .and_then(|settings| settings.binary.as_ref());

        let args = binary_settings
            .and_then(|settings| settings.arguments.as_ref())
            .cloned()
            .unwrap_or_default();

        let env = binary_settings
            .and_then(|settings| settings.env.as_ref())
            .map(|env| {
                env.iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        if let Some(path) = binary_settings.and_then(|settings| settings.path.clone()) {
            return Ok(zed::Command {
                command: path,
                args,
                env,
            });
        }

        let command = self.language_server_binary_path(language_server_id, worktree)?;
        Ok(zed::Command { command, args, env })
    }

    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        if let Some(path) = worktree.which(LSP_ID) {
            return Ok(path);
        }

        if let Some(path) = &self.cached_binary_path {
            if matches!(fs::metadata(path), Ok(stat) if stat.is_file()) {
                return Ok(path.clone());
            }
        }

        let (platform, arch) = zed::current_platform();
        let cached_disk_path = find_cached_binary(platform);
        let check_due = cached_disk_path.is_none() || update_check_due();
        if let Some(path) = cached_disk_path.clone() {
            if !check_due {
                self.cached_binary_path = Some(path.clone());
                return Ok(path);
            }
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let release = match zed::github_release_by_tag_name(LSP_REPO, LSP_PINNED_TAG) {
            Ok(release) => {
                record_update_check();
                release
            }
            Err(err) => {
                if let Some(path) = cached_disk_path.as_ref() {
                    record_update_check();
                    self.cached_binary_path = Some(path.clone());
                    return Ok(path.clone());
                }
                return Err(format!("failed to fetch release: {err}"));
            }
        };

        #[allow(unreachable_patterns)]
        let os = match platform {
            zed::Os::Mac => "macos",
            zed::Os::Linux => "linux",
            zed::Os::Windows => "windows",
            _ => return Err(format!("unsupported platform {platform:?}")),
        };
        let arch = match arch {
            zed::Architecture::Aarch64 => "arm64",
            zed::Architecture::X8664 => "x64",
            _ => return Err(format!("unsupported architecture {arch:?}")),
        };
        let extension = match platform {
            zed::Os::Windows => "zip",
            _ => "tar.gz",
        };
        let version = release.version.trim_start_matches('v');
        let asset_name = format!(
            "{id}-{version}-{os}-{arch}.{extension}",
            id = LSP_ID,
            version = version,
            os = os,
            arch = arch,
            extension = extension,
        );
        let asset = match release.assets.iter().find(|asset| asset.name == asset_name) {
            Some(asset) => asset,
            None => {
                if let Some(path) = cached_disk_path.as_ref() {
                    self.cached_binary_path = Some(path.clone());
                    return Ok(path.clone());
                }
                return Err(format!("no asset found matching {asset_name}"));
            }
        };

        let version_dir = format!("{id}-{version}", id = LSP_ID, version = release.version);
        let bin_name = match platform {
            zed::Os::Windows => format!("{LSP_ID}.exe"),
            _ => LSP_ID.to_string(),
        };
        let binary_path = Path::new(&version_dir).join(&bin_name);

        if !matches!(fs::metadata(&binary_path), Ok(meta) if meta.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            if let Err(err) = zed::download_file(
                &asset.download_url,
                &version_dir,
                match platform {
                    zed::Os::Windows => zed::DownloadedFileType::Zip,
                    _ => zed::DownloadedFileType::GzipTar,
                },
            ) {
                if let Some(path) = cached_disk_path.as_ref() {
                    self.cached_binary_path = Some(path.clone());
                    return Ok(path.clone());
                }
                return Err(format!("failed to download language server: {err}"));
            }
        }

        let binary_path_str = binary_path.to_string_lossy().to_string();
        if platform != zed::Os::Windows {
            zed::make_file_executable(&binary_path_str)
                .map_err(|e| format!("failed to make binary executable: {e}"))?;
        }

        self.cached_binary_path = Some(binary_path_str.clone());
        Ok(binary_path_str)
    }
}

impl zed::Extension for CtrmmlExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        self.resolve_language_server_command(language_server_id, worktree)
    }

    fn language_server_initialization_options(
        &mut self,
        server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        Self::lsp_settings_value(server_id, worktree, |lsp_settings| {
            lsp_settings.initialization_options.clone()
        })
    }

    fn language_server_workspace_configuration(
        &mut self,
        server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        Self::lsp_settings_value(server_id, worktree, |lsp_settings| {
            lsp_settings.settings.clone()
        })
    }
}

zed::register_extension!(CtrmmlExtension);
