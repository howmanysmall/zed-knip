pub mod cache;
pub mod config_detection;
pub mod errors;
pub mod logging;
pub mod managed_install;
pub mod package_manager;
pub mod reports;
pub mod resolver;
pub mod settings;

use crate::{
	cache::WorktreeCache,
	config_detection::detect_config,
	errors::KnipError,
	resolver::{build_language_server_command, resolve_knip, ManagedInstallDisabled},
	settings::{KnipSettings, LogLevel},
};
use std::path::PathBuf;
use zed_extension_api as zed;

const KNIP_LANGUAGE_SERVER_ID: &str = "knip";

pub struct ZedKnipExtension;

impl zed::Extension for ZedKnipExtension {
	fn new() -> Self {
		Self
	}

	fn language_server_command(
		&mut self,
		language_server_id: &zed::LanguageServerId,
		worktree: &zed::Worktree,
	) -> zed::Result<zed::Command> {
		language_server_command_for_worktree(
			language_server_id.as_ref(),
			&ZedWorktreeAdapter { worktree },
			&ProductionResolver,
		)
	}
}

trait WorktreeAdapter {
	fn root_path(&self) -> PathBuf;
	fn lsp_settings(&self, language_server_id: &str) -> zed::Result<Option<ZedKnipSettings>>;
}

trait CommandResolver {
	fn resolve_command(
		&self,
		settings: &KnipSettings,
		workspace_root: &std::path::Path,
	) -> Result<zed::Command, KnipError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct ZedKnipSettings {
	server_path: Option<String>,
	package_manager: Option<String>,
	auto_install: Option<bool>,
	log_level: Option<LogLevel>,
	extra_args: Option<Vec<String>>,
	config_path: Option<String>,
	require_config: Option<bool>,
}

struct ZedWorktreeAdapter<'a> {
	worktree: &'a zed::Worktree,
}

impl WorktreeAdapter for ZedWorktreeAdapter<'_> {
	fn root_path(&self) -> PathBuf {
		PathBuf::from(self.worktree.root_path())
	}

	fn lsp_settings(&self, language_server_id: &str) -> zed::Result<Option<ZedKnipSettings>> {
		let settings = zed::settings::LspSettings::for_worktree(language_server_id, self.worktree)?;
		Ok(Some(ZedKnipSettings::from_lsp_settings(settings)?))
	}
}

struct ProductionResolver;

impl CommandResolver for ProductionResolver {
	fn resolve_command(
		&self,
		settings: &KnipSettings,
		workspace_root: &std::path::Path,
	) -> Result<zed::Command, KnipError> {
		let cache = WorktreeCache::new(workspace_root.to_path_buf());
		let resolved = resolve_knip(settings, &cache, &ManagedInstallDisabled)?;
		let command = build_language_server_command(&resolved, settings, workspace_root);

		Ok(command.command)
	}
}

fn language_server_command_for_worktree(
	language_server_id: &str,
	worktree: &impl WorktreeAdapter,
	resolver: &impl CommandResolver,
) -> zed::Result<zed::Command> {
	if language_server_id != KNIP_LANGUAGE_SERVER_ID {
		return Err(unsupported_language_server_error(language_server_id));
	}

	let workspace_root = worktree.root_path();
	let settings = settings_for_workspace(&workspace_root, worktree.lsp_settings(language_server_id)?)?;

	resolver
		.resolve_command(&settings, &workspace_root)
		.map_err(zed_visible_error)
}

fn settings_for_workspace(
	workspace_root: &std::path::Path,
	overrides: Option<ZedKnipSettings>,
) -> zed::Result<KnipSettings> {
	let mut settings = KnipSettings::default();
	if let Some(overrides) = overrides {
		settings = apply_zed_settings(settings, overrides);
	}

	if settings.config_path.is_none() {
		settings.config_path = detect_config(workspace_root).map(|path| path.display().to_string());
	}

	settings.validate().map_err(|error| error.to_string())?;
	Ok(settings)
}

fn apply_zed_settings(mut settings: KnipSettings, overrides: ZedKnipSettings) -> KnipSettings {
	settings.server_path = overrides.server_path.or(settings.server_path);
	settings.package_manager = overrides.package_manager.or(settings.package_manager);
	if let Some(auto_install) = overrides.auto_install {
		settings.auto_install = auto_install;
	}
	if let Some(log_level) = overrides.log_level {
		settings.log_level = log_level;
	}
	if let Some(extra_args) = overrides.extra_args {
		settings.extra_args = extra_args;
	}
	settings.config_path = overrides.config_path.or(settings.config_path);
	if let Some(require_config) = overrides.require_config {
		settings.require_config = require_config;
	}
	settings
}

impl ZedKnipSettings {
	fn from_lsp_settings(settings: zed::settings::LspSettings) -> zed::Result<Self> {
		let mut overrides = Self::default();

		if let Some(binary) = settings.binary {
			overrides.server_path = binary.path;
			overrides.extra_args = binary.arguments;
			if let Some(env) = binary.env {
				overrides.package_manager = env.get("KNIP_PACKAGE_MANAGER").cloned();
				if let Some(log_level) = env.get("KNIP_LOG_LEVEL") {
					overrides.log_level = Some(log_level.parse::<LogLevel>().map_err(|error| error.to_string())?);
				}
			}
		}

		if let Some(custom_settings) = settings.settings {
			apply_custom_lsp_settings(&mut overrides, &custom_settings)?;
		}

		Ok(overrides)
	}
}

fn apply_custom_lsp_settings(overrides: &mut ZedKnipSettings, settings: &zed::serde_json::Value) -> zed::Result<()> {
	if let Some(server_path) = settings.get("server_path").and_then(zed::serde_json::Value::as_str) {
		overrides.server_path = Some(server_path.to_string());
	}
	if let Some(package_manager) = settings.get("package_manager").and_then(zed::serde_json::Value::as_str) {
		overrides.package_manager = Some(package_manager.to_string());
	}
	if let Some(auto_install) = settings.get("auto_install").and_then(zed::serde_json::Value::as_bool) {
		overrides.auto_install = Some(auto_install);
	}
	if let Some(log_level) = settings.get("log_level").and_then(zed::serde_json::Value::as_str) {
		overrides.log_level = Some(log_level.parse::<LogLevel>().map_err(|error| error.to_string())?);
	}
	if let Some(config_path) = settings.get("config_path").and_then(zed::serde_json::Value::as_str) {
		overrides.config_path = Some(config_path.to_string());
	}
	if let Some(require_config) = settings.get("require_config").and_then(zed::serde_json::Value::as_bool) {
		overrides.require_config = Some(require_config);
	}
	if let Some(extra_args) = settings.get("extra_args").and_then(zed::serde_json::Value::as_array) {
		overrides.extra_args = Some(
			extra_args
				.iter()
				.filter_map(zed::serde_json::Value::as_str)
				.map(str::to_string)
				.collect(),
		);
	}

	Ok(())
}

fn unsupported_language_server_error(language_server_id: &str) -> String {
	format!(
		"Unsupported language server '{language_server_id}'. This extension only starts the '{KNIP_LANGUAGE_SERVER_ID}' language server registered in extension.toml."
	)
}

fn zed_visible_error(error: KnipError) -> String {
	let message = error.to_string();
	eprintln!("zed-knip: {message}");
	message
}

zed::register_extension!(ZedKnipExtension);

#[cfg(test)]
const MODULE_COUNT: usize = 9;

#[cfg(test)]
mod tests {
	use super::*;
	use std::{
		fs,
		path::Path,
		time::{SystemTime, UNIX_EPOCH},
	};

	#[derive(Debug)]
	struct TestWorktree {
		root: PathBuf,
	}

	impl TestWorktree {
		fn new(name: &str) -> Self {
			let nanos = SystemTime::now()
				.duration_since(UNIX_EPOCH)
				.unwrap_or_default()
				.as_nanos();
			let root = std::env::temp_dir().join(format!("zed-knip-lib-{name}-{nanos}"));
			fs::create_dir_all(&root).unwrap_or_else(|error| panic!("failed to create {}: {error}", root.display()));
			Self { root }
		}

		fn write(&self, relative_path: &str, contents: &str) -> PathBuf {
			let path = self.root.join(relative_path);
			if let Some(parent) = path.parent() {
				fs::create_dir_all(parent)
					.unwrap_or_else(|error| panic!("failed to create {}: {error}", parent.display()));
			}
			fs::write(&path, contents).unwrap_or_else(|error| panic!("failed to write {}: {error}", path.display()));
			path
		}

		fn executable(&self, relative_path: &str) -> PathBuf {
			let path = self.write(relative_path, "#!/usr/bin/env node\n");
			make_executable(&path).unwrap_or_else(|error| panic!("failed to chmod {}: {error}", path.display()));
			path
		}
	}

	impl WorktreeAdapter for TestWorktree {
		fn root_path(&self) -> PathBuf {
			self.root.clone()
		}

		fn lsp_settings(&self, _language_server_id: &str) -> zed::Result<Option<ZedKnipSettings>> {
			Ok(None)
		}
	}

	#[derive(Debug, Clone)]
	struct MockResolver {
		result: Result<zed::Command, KnipError>,
	}

	impl CommandResolver for MockResolver {
		fn resolve_command(
			&self,
			_settings: &KnipSettings,
			_workspace_root: &std::path::Path,
		) -> Result<zed::Command, KnipError> {
			self.result.clone()
		}
	}

	impl Drop for TestWorktree {
		fn drop(&mut self) {
			let _ = fs::remove_dir_all(&self.root);
		}
	}

	fn make_executable(path: &Path) -> std::io::Result<()> {
		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;

			let mut permissions = fs::metadata(path)?.permissions();
			permissions.set_mode(0o755);
			fs::set_permissions(path, permissions)
		}

		#[cfg(not(unix))]
		{
			let _ = path;
			Ok(())
		}
	}

	#[test]
	fn scaffold_smoke_test() {
		assert_eq!(super::MODULE_COUNT, 9);
	}

	#[test]
	fn language_server_command_rejects_unsupported_language_server_id() {
		let worktree = TestWorktree::new("unsupported-id");

		let resolver = MockResolver {
			result: Ok(zed::Command {
				command: "unused".to_string(),
				args: Vec::new(),
				env: Vec::new(),
			}),
		};

		let error =
			language_server_command_for_worktree("typescript-language-server", &worktree, &resolver).unwrap_err();

		assert!(error.contains("Unsupported language server 'typescript-language-server'"));
	}

	#[test]
	fn language_server_command_uses_worktree_root_and_resolver_command() {
		let worktree = TestWorktree::new("command");
		worktree.write("package.json", "{\"packageManager\":\"pnpm@9.0.0\"}\n");
		let executable = worktree.executable("node_modules/.bin/knip-language-server");
		let config = worktree.write("knip.json", "{}\n");

		let command =
			language_server_command_for_worktree(KNIP_LANGUAGE_SERVER_ID, &worktree, &ProductionResolver).unwrap();

		assert_eq!(command.command, executable.display().to_string());
		assert_eq!(command.args[0], "--stdio");
		assert!(command.args.contains(&"--cwd".to_string()));
		assert!(command.args.contains(&worktree.root.display().to_string()));
		assert!(command.args.contains(&"--config".to_string()));
		assert!(command.args.contains(&config.display().to_string()));
		assert!(command
			.env
			.contains(&("KNIP_PACKAGE_MANAGER".to_string(), "pnpm".to_string())));
	}

	#[test]
	fn language_server_command_surfaces_resolver_errors_to_zed() {
		let worktree = TestWorktree::new("resolver-error");
		worktree.write("package.json", "{\"packageManager\":\"npm@10.0.0\"}\n");

		let error =
			language_server_command_for_worktree(KNIP_LANGUAGE_SERVER_ID, &worktree, &ProductionResolver).unwrap_err();

		assert!(error.contains("Managed Knip install failed"));
	}

	#[test]
	fn language_server_command_maps_mocked_resolver_error_to_zed_message() {
		let worktree = TestWorktree::new("mock-error");
		let resolver = MockResolver {
			result: Err(KnipError::UnsupportedWorkspace {
				reason: "missing package.json".to_string(),
			}),
		};

		let error = language_server_command_for_worktree(KNIP_LANGUAGE_SERVER_ID, &worktree, &resolver).unwrap_err();

		assert!(error.contains("This workspace is not supported: missing package.json"));
	}
}
