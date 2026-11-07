use std::{fs, path::Path};

use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result};

const LSP_ID: &str = "ctrmml-lsp";
const LSP_REPO: &str = "ulalume/language-server-ctrmml";
// Pin to a known-good language-server release. Bump in lockstep with
// `megamml/wasm-lang-core/build.sh` so the grammar contract stays
// in sync.
const LSP_PINNED_TAG: &str = "v0.7.3";

struct CtrmmlExtension {
    cached_binary_path: Option<String>,
}

fn is_file(path: impl AsRef<Path>) -> bool {
    fs::metadata(path).is_ok_and(|meta| meta.is_file())
}

/// Download and extract the pinned release into a temporary directory,
/// then move it to `version_dir` in one rename.
fn install(
    platform: zed::Os,
    arch: zed::Architecture,
    version_dir: &str,
    bin_name: &str,
) -> Result<()> {
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
    let (extension, file_type) = match platform {
        zed::Os::Windows => ("zip", zed::DownloadedFileType::Zip),
        _ => ("tar.gz", zed::DownloadedFileType::GzipTar),
    };
    let version = LSP_PINNED_TAG.trim_start_matches('v');
    let asset_name = format!("{LSP_ID}-{version}-{os}-{arch}.{extension}");
    let url =
        format!("https://github.com/{LSP_REPO}/releases/download/{LSP_PINNED_TAG}/{asset_name}");

    let temp_dir = format!(".download-{version_dir}");
    let _ = fs::remove_dir_all(&temp_dir);
    let result = zed::download_file(&url, &temp_dir, file_type)
        .map_err(|err| format!("failed to download {url}: {err}"))
        .and_then(|()| {
            if is_file(Path::new(&temp_dir).join(bin_name)) {
                Ok(())
            } else {
                Err(format!("{bin_name} not found in {asset_name}"))
            }
        })
        .and_then(|()| move_into_place(&temp_dir, version_dir, bin_name));
    let _ = fs::remove_dir_all(&temp_dir);
    result
}

fn move_into_place(temp_dir: &str, version_dir: &str, bin_name: &str) -> Result<()> {
    if fs::rename(temp_dir, version_dir).is_ok() {
        return Ok(());
    }
    if is_file(Path::new(version_dir).join(bin_name)) {
        // Installed by someone else in the meantime.
        return Ok(());
    }
    // A directory without the binary is not a usable install; replace it.
    let _ = fs::remove_dir_all(version_dir);
    fs::rename(temp_dir, version_dir)
        .map_err(|err| format!("failed to install language server: {err}"))
}

/// Remove server directories of other versions.
fn remove_other_versions(keep: &str) {
    let Ok(entries) = fs::read_dir(".") else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name != keep && name.starts_with(&format!("{LSP_ID}-")) {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
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
            if is_file(path) {
                return Ok(path.clone());
            }
        }

        let (platform, arch) = zed::current_platform();
        let bin_name = match platform {
            zed::Os::Windows => format!("{LSP_ID}.exe"),
            _ => LSP_ID.to_string(),
        };
        let version_dir = format!("{LSP_ID}-{LSP_PINNED_TAG}");
        let binary_path = Path::new(&version_dir)
            .join(&bin_name)
            .to_string_lossy()
            .to_string();

        if !is_file(&binary_path) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            install(platform, arch, &version_dir, &bin_name)?;
            remove_other_versions(&version_dir);
        }

        if platform != zed::Os::Windows {
            zed::make_file_executable(&binary_path)
                .map_err(|e| format!("failed to make binary executable: {e}"))?;
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
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
