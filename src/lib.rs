use std::{fs, path::Path};

use zed_extension_api::{
    self as zed,
    serde_json::{json, Map, Value},
    settings::LspSettings,
    LanguageServerId, Result,
};

const LSP_ID: &str = "ctrmml-lsp";
const LSP_REPO: &str = "ulalume/language-server-ctrmml";
// Language-server release the extension is tested with. Bump together with
// the pin in `megamml`'s `wasm-lang-core/build.sh`.
const LSP_PINNED_TAG: &str = "v0.8.1";

struct CtrmmlExtension;

/// Initialization options for Zed, as `(snake_case, camelCase, value)`: lens
/// titles in plain text without Load and Save, play and export as code
/// actions (Zed has no status bar for them), every command run by the
/// server, and the one-step FM picker.
fn default_initialization_options() -> [(&'static str, &'static str, Value); 5] {
    [
        ("code_lens_icons", "codeLensIcons", json!("text")),
        (
            "code_lens_file_actions",
            "codeLensFileActions",
            json!(false),
        ),
        ("command_code_actions", "commandCodeActions", json!(true)),
        ("client_commands", "clientCommands", json!([])),
        ("fm_picker_hierarchy", "fmPickerHierarchy", json!(false)),
    ]
}

/// The user's options over the defaults; a key set in either spelling wins.
fn initialization_options(user: Option<Value>) -> Value {
    let mut options = match user {
        Some(Value::Object(options)) => options,
        _ => Map::new(),
    };
    for (snake_case, camel_case, value) in default_initialization_options() {
        if !options.contains_key(snake_case) && !options.contains_key(camel_case) {
            options.insert(snake_case.to_string(), value);
        }
    }
    Value::Object(options)
}

fn is_file(path: impl AsRef<Path>) -> bool {
    fs::metadata(path).is_ok_and(|meta| meta.is_file())
}

/// Download and extract the pinned release into a temporary directory,
/// then move it to `version_dir` in one rename.
fn install(
    asset_name: &str,
    file_type: zed::DownloadedFileType,
    version_dir: &str,
    bin_name: &str,
) -> Result<()> {
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

/// The server on `PATH`, else the pinned release, installed on first use.
fn binary_path(language_server_id: &LanguageServerId, worktree: &zed::Worktree) -> Result<String> {
    if let Some(path) = worktree.which(LSP_ID) {
        return Ok(path);
    }

    let (platform, arch) = zed::current_platform();
    let (os, extension, file_type, bin_name) = match platform {
        zed::Os::Mac => (
            "macos",
            "tar.gz",
            zed::DownloadedFileType::GzipTar,
            LSP_ID.to_string(),
        ),
        zed::Os::Linux => (
            "linux",
            "tar.gz",
            zed::DownloadedFileType::GzipTar,
            LSP_ID.to_string(),
        ),
        zed::Os::Windows => (
            "windows",
            "zip",
            zed::DownloadedFileType::Zip,
            format!("{LSP_ID}.exe"),
        ),
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
        let arch = match arch {
            zed::Architecture::Aarch64 => "arm64",
            zed::Architecture::X8664 => "x64",
            _ => return Err(format!("unsupported architecture {arch:?}")),
        };
        let version = LSP_PINNED_TAG.trim_start_matches('v');
        let asset_name = format!("{LSP_ID}-{version}-{os}-{arch}.{extension}");
        install(&asset_name, file_type, &version_dir, &bin_name)?;
        remove_other_versions(&version_dir);
    }

    if platform != zed::Os::Windows {
        zed::make_file_executable(&binary_path)
            .map_err(|e| format!("failed to make binary executable: {e}"))?;
    }
    Ok(binary_path)
}

impl zed::Extension for CtrmmlExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let (path, args, env) = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|settings| settings.binary)
            .map(|binary| (binary.path, binary.arguments, binary.env))
            .unwrap_or_default();
        let command = match path {
            Some(path) => path,
            None => binary_path(language_server_id, worktree)?,
        };
        Ok(zed::Command {
            command,
            args: args.unwrap_or_default(),
            env: env.unwrap_or_default().into_iter().collect(),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<Value>> {
        let options = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|settings| settings.initialization_options);
        Ok(Some(initialization_options(options)))
    }

    /// The user's `settings` under `ctrmml`, the section the server pulls.
    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<Value>> {
        let settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|settings| settings.settings)
            .unwrap_or_else(|| json!({}));
        Ok(Some(json!({ "ctrmml": settings })))
    }
}

zed::register_extension!(CtrmmlExtension);
