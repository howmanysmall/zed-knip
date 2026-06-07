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
	config_detection::known_config_file_names,
	errors::KnipError,
	managed_install::ZedNpmManagedInstall,
	package_manager::PackageManagerError,
	resolver::{build_language_server_command, resolve_knip},
	settings::{KnipSettings, LogLevel},
};
use std::path::PathBuf;
use zed_extension_api as zed;

const KNIP_LANGUAGE_SERVER_ID: &str = "knip";

pub struct ZedKnipExtension;

pub(crate) fn is_executable(path: &std::path::Path) -> bool {
	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;

		std::fs::metadata(path)
			.map(|metadata| metadata.permissions().mode() & 0o111 != 0)
			.unwrap_or(false)
	}

	#[cfg(not(unix))]
	{
		path.is_file()
	}
}

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

	fn language_server_initialization_options(
		&mut self,
		language_server_id: &zed::LanguageServerId,
		worktree: &zed::Worktree,
	) -> zed::Result<Option<zed::serde_json::Value>> {
		language_server_initialization_options_for_worktree(
			language_server_id.as_ref(),
			&ZedWorktreeAdapter { worktree },
		)
	}

	fn language_server_workspace_configuration(
		&mut self,
		language_server_id: &zed::LanguageServerId,
		worktree: &zed::Worktree,
	) -> zed::Result<Option<zed::serde_json::Value>> {
		language_server_workspace_configuration_for_worktree(
			language_server_id.as_ref(),
			&ZedWorktreeAdapter { worktree },
		)
	}
}

trait WorktreeAdapter {
	fn root_path(&self) -> PathBuf;
	fn lsp_settings(&self, language_server_id: &str) -> zed::Result<Option<KnipSettings>>;
	fn read_text_file(&self, relative_path: &str) -> Option<String>;
}

trait CommandResolver {
	fn resolve_command(
		&self,
		settings: &KnipSettings,
		workspace_root: &std::path::Path,
	) -> Result<zed::Command, KnipError>;
}

struct ZedWorktreeAdapter<'a> {
	worktree: &'a zed::Worktree,
}

impl WorktreeAdapter for ZedWorktreeAdapter<'_> {
	fn root_path(&self) -> PathBuf {
		PathBuf::from(self.worktree.root_path())
	}

	fn lsp_settings(&self, language_server_id: &str) -> zed::Result<Option<KnipSettings>> {
		let settings = zed::settings::LspSettings::for_worktree(language_server_id, self.worktree)?;
		Ok(Some(settings_from_lsp_settings(settings)?))
	}

	fn read_text_file(&self, relative_path: &str) -> Option<String> {
		self.worktree.read_text_file(relative_path).ok()
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
		let resolved = resolve_knip(settings, &cache, &ZedNpmManagedInstall)?;
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

	let settings = settings_for_worktree(worktree, worktree.lsp_settings(language_server_id)?)?;
	let workspace_root = worktree.root_path();

	resolver
		.resolve_command(&settings, &workspace_root)
		.map_err(zed_visible_error)
}

fn language_server_initialization_options_for_worktree(
	language_server_id: &str,
	worktree: &impl WorktreeAdapter,
) -> zed::Result<Option<zed::serde_json::Value>> {
	if language_server_id != KNIP_LANGUAGE_SERVER_ID {
		return Err(unsupported_language_server_error(language_server_id));
	}

	let settings = settings_for_worktree(worktree, worktree.lsp_settings(language_server_id)?)?;

	Ok(Some(knip_initialization_options(&settings)))
}

fn language_server_workspace_configuration_for_worktree(
	language_server_id: &str,
	worktree: &impl WorktreeAdapter,
) -> zed::Result<Option<zed::serde_json::Value>> {
	if language_server_id != KNIP_LANGUAGE_SERVER_ID {
		return Err(unsupported_language_server_error(language_server_id));
	}

	let settings = settings_for_worktree(worktree, worktree.lsp_settings(language_server_id)?)?;

	Ok(Some(knip_workspace_configuration(&settings)))
}

fn knip_initialization_options(settings: &KnipSettings) -> zed::serde_json::Value {
	zed::serde_json::json!({ "config": knip_workspace_configuration(settings) })
}

fn knip_workspace_configuration(settings: &KnipSettings) -> zed::serde_json::Value {
	let mut config = zed::serde_json::json!({
		"deferSession": false,
		"editor": {
			"exports": {
				"codelens": { "enabled": true },
				"hover": {
					"enabled": true,
					"includeImportLocationSnippet": false,
					"maxSnippets": 10,
					"timeout": 300
				},
				"quickfix": { "enabled": true },
				"highlight": {
					"dimExports": false,
					"dimTypes": false,
					"dimEnumMembers": false,
					"dimClassMembers": false,
					"dimDuplicates": false
				}
			}
		},
		"imports": { "enabled": true },
		"exports": { "enabled": true, "contention": { "enabled": true } }
	});

	if let Some(config_path) = settings.config_path.as_deref() {
		config["configFilePath"] = zed::serde_json::Value::String(config_path.to_string());
	}

	config
}

fn settings_for_worktree(
	worktree: &impl WorktreeAdapter,
	overrides: Option<KnipSettings>,
) -> zed::Result<KnipSettings> {
	let mut settings = KnipSettings::default();
	if let Some(overrides) = overrides {
		settings = apply_zed_settings(settings, overrides);
	}

	let workspace_root = worktree.root_path();
	if settings.config_path.is_none() {
		settings.config_path = detect_config_for_worktree(worktree)
			.map(|relative_path| workspace_root.join(relative_path).display().to_string());
	}

	if settings.package_manager.is_none() {
		settings.package_manager = Some(detect_package_manager_for_worktree(worktree)?.to_string());
	}

	settings.validate().map_err(|error| error.to_string())?;
	Ok(settings)
}

fn detect_config_for_worktree(worktree: &impl WorktreeAdapter) -> Option<&'static str> {
	known_config_file_names()
		.iter()
		.copied()
		.find(|file_name| worktree.read_text_file(file_name).is_some())
}

fn detect_package_manager_for_worktree(
	worktree: &impl WorktreeAdapter,
) -> zed::Result<package_manager::PackageManager> {
	let package_json = worktree.read_text_file("package.json");
	package_manager::detect_from_workspace_files(package_json.as_deref(), |lockfile| {
		worktree.read_text_file(lockfile).is_some()
	})
	.map_err(package_manager_error_to_zed_error)
}

fn package_manager_error_to_zed_error(error: PackageManagerError) -> String {
	match error {
		PackageManagerError::NotFound => {
			"No supported package manager lockfile or packageManager field was found.".to_string()
		}
		PackageManagerError::Ambiguous { found } => format!(
			"Multiple package managers were detected ({}). Remove the extra lockfile(s) or set the package manager explicitly in settings.",
			found.join(", ")
		),
		PackageManagerError::UnsupportedPackageManager { found } => {
			format!("Unsupported package manager {found}. Use npm, pnpm, yarn, or bun.")
		}
	}
}

fn apply_zed_settings(settings: KnipSettings, overrides: KnipSettings) -> KnipSettings {
	settings.merge(overrides)
}

fn settings_from_lsp_settings(settings: zed::settings::LspSettings) -> zed::Result<KnipSettings> {
	let mut overrides = KnipSettings::default();

	if let Some(binary) = settings.binary {
		overrides.server_path = binary.path;
		if let Some(arguments) = binary.arguments {
			overrides.extra_args = arguments;
		}
		if let Some(env) = binary.env {
			overrides.package_manager = env.get("KNIP_PACKAGE_MANAGER").cloned();
			if let Some(log_level) = env.get("KNIP_LOG_LEVEL") {
				overrides.log_level = log_level.parse::<LogLevel>().map_err(|error| error.to_string())?;
			}
		}
	}

	if let Some(custom_settings) = settings.settings {
		apply_custom_lsp_settings(&mut overrides, &custom_settings)?;
	}

	Ok(overrides)
}

fn apply_custom_lsp_settings(overrides: &mut KnipSettings, settings: &zed::serde_json::Value) -> zed::Result<()> {
	if let Some(server_path) = settings
		.get("server_path")
		.cloned()
		.and_then(|value| zed::serde_json::from_value::<String>(value).ok())
	{
		overrides.server_path = Some(server_path);
	}
	if let Some(package_manager) = settings
		.get("package_manager")
		.cloned()
		.and_then(|value| zed::serde_json::from_value::<String>(value).ok())
	{
		overrides.package_manager = Some(package_manager);
	}
	if let Some(auto_install) = settings
		.get("auto_install")
		.cloned()
		.and_then(|value| zed::serde_json::from_value::<bool>(value).ok())
	{
		overrides.auto_install = auto_install;
	}
	if let Some(log_level) = settings
		.get("log_level")
		.cloned()
		.and_then(|value| zed::serde_json::from_value::<String>(value).ok())
	{
		overrides.log_level = log_level.parse::<LogLevel>().map_err(|error| error.to_string())?;
	}
	if let Some(config_path) = settings
		.get("config_path")
		.cloned()
		.and_then(|value| zed::serde_json::from_value::<String>(value).ok())
	{
		overrides.config_path = Some(config_path);
	}
	if let Some(require_config) = settings
		.get("require_config")
		.cloned()
		.and_then(|value| zed::serde_json::from_value::<bool>(value).ok())
	{
		overrides.require_config = require_config;
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
		collections::HashMap,
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

		fn lsp_settings(&self, _language_server_id: &str) -> zed::Result<Option<KnipSettings>> {
			Ok(None)
		}

		fn read_text_file(&self, relative_path: &str) -> Option<String> {
			fs::read_to_string(self.root.join(relative_path)).ok()
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
	fn lsp_settings_use_knip_settings_directly_for_binary_and_custom_overrides() {
		let settings = settings_from_lsp_settings(zed::settings::LspSettings {
			binary: Some(zed::settings::CommandSettings {
				path: Some("tools/knip-language-server".to_string()),
				arguments: Some(vec!["--from-binary".to_string()]),
				env: Some(HashMap::from([
					("KNIP_PACKAGE_MANAGER".to_string(), "npm".to_string()),
					("KNIP_LOG_LEVEL".to_string(), "debug".to_string()),
				])),
			}),
			initialization_options: None,
			settings: Some(zed::serde_json::json!({
				"package_manager": "pnpm",
				"auto_install": false,
				"log_level": "warn",
				"config_path": "knip.ts",
				"require_config": true
			})),
		})
		.unwrap();

		assert_eq!(
			settings,
			KnipSettings {
				server_path: Some("tools/knip-language-server".to_string()),
				package_manager: Some("pnpm".to_string()),
				auto_install: false,
				log_level: LogLevel::Warn,
				extra_args: vec!["--from-binary".to_string()],
				config_path: Some("knip.ts".to_string()),
				require_config: true,
			}
		);
	}

	#[test]
	fn lsp_settings_keep_binary_arguments_when_custom_extra_args_are_missing() {
		let settings = settings_from_lsp_settings(zed::settings::LspSettings {
			binary: Some(zed::settings::CommandSettings {
				path: None,
				arguments: Some(vec!["--binary-only".to_string()]),
				env: None,
			}),
			initialization_options: None,
			settings: Some(zed::serde_json::json!({
				"config_path": "knip.ts"
			})),
		})
		.unwrap();

		assert_eq!(settings.extra_args, vec!["--binary-only".to_string()]);
		assert_eq!(settings.config_path.as_deref(), Some("knip.ts"));
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
	fn language_server_command_detects_bun_lock_through_worktree_api() {
		let worktree = TestWorktree::new("bun-lock");
		worktree.write("bun.lock", "");
		let executable = worktree.executable("node_modules/.bin/knip-language-server");

		let command =
			language_server_command_for_worktree(KNIP_LANGUAGE_SERVER_ID, &worktree, &ProductionResolver).unwrap();

		assert_eq!(command.command, executable.display().to_string());
		assert!(command
			.env
			.contains(&("KNIP_PACKAGE_MANAGER".to_string(), "bun".to_string())));
	}

	#[test]
	fn language_server_command_package_manager_field_wins_over_lockfile_through_worktree_api() {
		let worktree = TestWorktree::new("package-manager-precedence");
		worktree.write("package.json", "{\"packageManager\":\"aube@1.15.0\"}\n");
		worktree.write("bun.lock", "");
		let executable = worktree.executable("node_modules/.bin/knip-language-server");

		let command =
			language_server_command_for_worktree(KNIP_LANGUAGE_SERVER_ID, &worktree, &ProductionResolver).unwrap();

		assert_eq!(command.command, executable.display().to_string());
		assert!(command
			.env
			.contains(&("KNIP_PACKAGE_MANAGER".to_string(), "aube".to_string())));
	}

	#[test]
	fn language_server_command_surfaces_resolver_errors_to_zed() {
		let worktree = TestWorktree::new("resolver-error");
		worktree.write("package.json", "{\"packageManager\":\"npm@10.0.0\"}\n");
		let resolver = MockResolver {
			result: Err(KnipError::FailedManagedInstall {
				reason: "host API unavailable in native tests".to_string(),
			}),
		};

		let error = language_server_command_for_worktree(KNIP_LANGUAGE_SERVER_ID, &worktree, &resolver).unwrap_err();

		assert!(error.contains("Managed Knip install failed"));
	}

	#[test]
	fn language_server_initialization_options_include_detected_config_path() {
		let worktree = TestWorktree::new("init-options-config");
		worktree.write("bun.lock", "");
		let config = worktree.write("knip.ts", "export default {};\n");

		let options = language_server_initialization_options_for_worktree(KNIP_LANGUAGE_SERVER_ID, &worktree)
			.unwrap()
			.unwrap();
		let expected_config_path = config.display().to_string();

		assert_eq!(
			options["config"]["configFilePath"].as_str(),
			Some(expected_config_path.as_str())
		);
		assert_eq!(options["config"]["editor"]["exports"]["quickfix"]["enabled"], true);
	}

	#[test]
	fn language_server_workspace_configuration_includes_detected_config_path() {
		let worktree = TestWorktree::new("workspace-config");
		worktree.write("bun.lock", "");
		let config = worktree.write("knip.ts", "export default {};\n");

		let configuration = language_server_workspace_configuration_for_worktree(KNIP_LANGUAGE_SERVER_ID, &worktree)
			.unwrap()
			.unwrap();
		let expected_config_path = config.display().to_string();

		assert_eq!(
			configuration["configFilePath"].as_str(),
			Some(expected_config_path.as_str())
		);
		assert_eq!(configuration["editor"]["exports"]["quickfix"]["enabled"], true);
	}

	#[test]
	fn language_server_command_maps_mocked_resolver_error_to_zed_message() {
		let worktree = TestWorktree::new("mock-error");
		worktree.write("bun.lock", "");
		let resolver = MockResolver {
			result: Err(KnipError::UnsupportedWorkspace {
				reason: "missing package.json".to_string(),
			}),
		};

		let error = language_server_command_for_worktree(KNIP_LANGUAGE_SERVER_ID, &worktree, &resolver).unwrap_err();

		assert!(error.contains("This workspace is not supported: missing package.json"));
	}
}
