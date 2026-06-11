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
	settings::{DiagnosticSeverity, KnipDiagnosticsSettings, KnipSettings, KnipSettingsError},
};
use std::{
	collections::HashMap,
	path::{Component, PathBuf},
};
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

	let workspace_root = worktree.root_path();
	let settings = settings_for_worktree(worktree, worktree.lsp_settings(language_server_id)?)?;

	Ok(Some(knip_initialization_options(&settings, workspace_root.as_path())))
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

fn knip_initialization_options(settings: &KnipSettings, workspace_root: &std::path::Path) -> zed::serde_json::Value {
	zed::serde_json::json!({
		"cwd": workspace_root.display().to_string(),
		"config": knip_workspace_configuration(settings)
	})
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

	if settings.requires_managed_patch() {
		let mut zed_knip = zed::serde_json::json!({
			"diagnostics": build_diagnostics_json(&settings.diagnostics)
		});
		if let Some(ts_config_path) = settings.ts_config_path.as_deref() {
			zed_knip["tsConfigFilePath"] = zed::serde_json::Value::String(ts_config_path.to_string());
		}

		{
			let prep_array: Vec<zed::serde_json::Value> = settings
				.preprocessor
				.iter()
				.map(|s| zed::serde_json::Value::String(s.clone()))
				.collect();
			zed_knip["preprocessor"] = zed::serde_json::Value::Array(prep_array);

			let opts_obj = settings
				.preprocessor_options
				.as_ref()
				.map(|opts| {
					let map: std::collections::BTreeMap<String, zed::serde_json::Value> =
						opts.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
					zed::serde_json::to_value(map).unwrap_or(zed::serde_json::Value::Object(Default::default()))
				})
				.unwrap_or(zed::serde_json::Value::Object(Default::default()));
			zed_knip["preprocessorOptions"] = opts_obj;

			let prep_fingerprint = if settings.preprocessor.is_empty() {
				String::new()
			} else {
				let prepjoined = settings.preprocessor.join("|");
				let opts_fingerprint = settings
					.preprocessor_options
					.as_ref()
					.map(|opts| {
						let sorted_keys: std::collections::BTreeMap<String, ()> =
							opts.keys().map(|k| (k.clone(), ())).collect();
						zed::serde_json::to_string(&sorted_keys).unwrap_or_default()
					})
					.unwrap_or_default();
				format!("{}:{}", prepjoined, opts_fingerprint)
			};
			zed_knip["preprocessorFingerprint"] = zed::serde_json::Value::String(prep_fingerprint);
		}

		config["zedKnip"] = zed_knip;
	}

	config
}

fn build_diagnostics_json(diag: &KnipDiagnosticsSettings) -> zed::serde_json::Value {
	let severity_map: std::collections::BTreeMap<String, String> = diag
		.severity_by_issue_type
		.iter()
		.map(|(k, v)| (k.clone(), v.to_string()))
		.collect();

	zed::serde_json::json!({
		"includeIssueTypes": diag.include_issue_types,
		"excludeIssueTypes": diag.exclude_issue_types,
		"excludePathPrefixes": diag.exclude_path_prefixes,
		"severityByIssueType": severity_map
	})
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

	if settings.require_config && settings.config_path.is_none() {
		return Err(KnipError::RequireConfigMissing {
			workspace_root: workspace_root.clone(),
		}
		.to_string());
	}

	if let Some(config_path) = settings.config_path.as_deref() {
		let resolved = validate_config_path(config_path, &workspace_root).map_err(|error| error.to_string())?;
		settings.config_path = Some(resolved.display().to_string());
	}

	if let Some(ts_config_path) = settings.ts_config_path.as_deref() {
		validate_ts_config_path(ts_config_path, &workspace_root)?;
	}

	if settings.package_manager.is_none() {
		settings.package_manager = Some(detect_package_manager_for_worktree(worktree)?.to_string());
	}

	settings.validate().map_err(|error| error.to_string())?;
	Ok(settings)
}

fn validate_config_path(config_path: &str, workspace_root: &std::path::Path) -> Result<PathBuf, KnipError> {
	let path = std::path::Path::new(config_path);
	let resolved = if path.is_absolute() {
		path.to_path_buf()
	} else {
		workspace_root.join(path)
	};

	if !resolved.is_file() {
		return Err(KnipError::InvalidConfig {
			path: resolved.clone(),
			reason: format!(
				"lsp.knip.settings.config_path points at {} but no file exists at that path",
				resolved.display()
			),
		});
	}

	Ok(resolved)
}

fn validate_ts_config_path(ts_config_path: &str, workspace_root: &std::path::Path) -> zed::Result<()> {
	if ts_config_path.is_empty() {
		return Err(KnipError::InvalidTsConfigPath {
			path: PathBuf::from(""),
			reason: "path cannot be empty".to_string(),
		}
		.to_string());
	}

	let path = std::path::Path::new(ts_config_path);

	if path.is_absolute() {
		return Err(KnipError::InvalidTsConfigPath {
			path: path.to_path_buf(),
			reason: "path must be relative to the workspace root, not absolute".to_string(),
		}
		.to_string());
	}

	if path.components().any(|c| c == Component::ParentDir) {
		return Err(KnipError::InvalidTsConfigPath {
			path: path.to_path_buf(),
			reason: "path must not contain '..' parent directory traversal".to_string(),
		}
		.to_string());
	}

	let full_path = workspace_root.join(path);
	if !full_path.is_file() {
		return Err(KnipError::InvalidTsConfigPath {
			path: path.to_path_buf(),
			reason: format!("file not found at {}", full_path.display()),
		}
		.to_string());
	}

	Ok(())
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
		overrides.binary_path = binary.path;
		if let Some(arguments) = binary.arguments {
			reject_binary_arguments(&arguments)?;
		}
		if let Some(env) = binary.env.as_ref() {
			reject_binary_env_settings(env)?;
		}
	}

	if let Some(custom_settings) = settings.settings {
		apply_custom_lsp_settings(&mut overrides, &custom_settings)?;
	}

	Ok(overrides)
}

fn apply_custom_lsp_settings(overrides: &mut KnipSettings, settings: &zed::serde_json::Value) -> zed::Result<()> {
	// Reject removed settings before processing anything else.
	const REMOVED: &[(&str, &str)] = &[
		("server_path", "lsp.knip.binary.path"),
		("log_level", "the Knip config file (configure log verbosity there)"),
		("package_manager", "the packageManager field in package.json"),
	];
	for &(name, replacement) in REMOVED {
		if settings.get(name).is_some() {
			return Err(KnipSettingsError::RemovedSetting { name, replacement }.to_string());
		}
	}

	// Map supported settings.
	if let Some(binary_path) = settings
		.get("binary_path")
		.cloned()
		.and_then(|value| zed::serde_json::from_value::<String>(value).ok())
	{
		overrides.binary_path = Some(binary_path);
	}
	if let Some(auto_install) = settings
		.get("auto_install")
		.cloned()
		.and_then(|value| zed::serde_json::from_value::<bool>(value).ok())
	{
		overrides.auto_install = auto_install;
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
	if let Some(ts_config_path) = settings
		.get("ts_config_path")
		.cloned()
		.and_then(|value| zed::serde_json::from_value::<String>(value).ok())
	{
		overrides.ts_config_path = Some(ts_config_path);
	}
	if let Some(diag_value) = settings.get("diagnostics") {
		overrides.diagnostics = parse_diagnostics_settings(diag_value)?;
	}
	if let Some(prep_value) = settings.get("preprocessor") {
		let preprocessors: Vec<String> = zed::serde_json::from_value(prep_value.clone())
			.map_err(|e| format!("invalid preprocessor setting: {}", e))?;
		overrides.preprocessor = preprocessors;
	}
	if let Some(opts_value) = settings.get("preprocessor_options") {
		let opts: Option<std::collections::BTreeMap<String, zed::serde_json::Value>> =
			zed::serde_json::from_value(opts_value.clone())
				.map_err(|e| format!("invalid preprocessor_options setting: {}", e))?;
		overrides.preprocessor_options = opts;
	}

	Ok(())
}

fn parse_diagnostics_settings(value: &zed::serde_json::Value) -> zed::Result<KnipDiagnosticsSettings> {
	let mut diag = KnipDiagnosticsSettings::default();

	if let Some(obj) = value.as_object() {
		if let Some(include) = obj.get("include_issue_types").and_then(|v| v.as_array()) {
			diag.include_issue_types = include.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect();
		}
		if let Some(exclude) = obj.get("exclude_issue_types").and_then(|v| v.as_array()) {
			diag.exclude_issue_types = exclude.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect();
		}
		if let Some(prefixes) = obj.get("exclude_path_prefixes").and_then(|v| v.as_array()) {
			diag.exclude_path_prefixes = prefixes.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect();
		}
		if let Some(severity_map) = obj.get("severity_by_issue_type").and_then(|v| v.as_object()) {
			for (issue_type, severity_value) in severity_map {
				if let Some(severity_str) = severity_value.as_str() {
					let severity =
						DiagnosticSeverity::parse_with_type(issue_type, severity_str).map_err(|e| e.to_string())?;
					diag.severity_by_issue_type.insert(issue_type.clone(), severity);
				}
			}
		}
	}

	Ok(diag)
}

fn reject_binary_arguments(arguments: &[String]) -> zed::Result<()> {
	if arguments.is_empty() {
		return Ok(());
	}

	Err(format!(
		"`lsp.knip.binary.arguments` is not supported. `@knip/language-server` ignores launch arguments like {}. Move Knip analysis settings into your Knip config file and point the extension at it with `lsp.knip.settings.config_path`.",
		arguments
			.iter()
			.map(|argument| format!("`{argument}`"))
			.collect::<Vec<_>>()
			.join(", ")
	))
}

fn reject_binary_env_settings(env: &HashMap<String, String>) -> zed::Result<()> {
	const REMOVED_ENV_KEYS: &[(&str, &str)] = &[
		("KNIP_LOG_LEVEL", "lsp.knip.settings.log_level"),
		("KNIP_PACKAGE_MANAGER", "the packageManager field in package.json"),
	];

	for (name, replacement) in REMOVED_ENV_KEYS {
		if env.contains_key(*name) {
			return Err(KnipSettingsError::RemovedEnvSetting { name, replacement }.to_string());
		}
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
	use crate::{
		cache::InstallSource,
		managed_install::{ManagedInstall, ManagedInstallDisabled},
		package_manager::PackageManager,
	};
	use std::{
		collections::HashMap,
		fs,
		path::Path,
		time::{SystemTime, UNIX_EPOCH},
	};

	#[derive(Debug)]
	struct TestWorktree {
		root: PathBuf,
		settings_override: Option<KnipSettings>,
	}

	impl TestWorktree {
		fn new(name: &str) -> Self {
			let nanos = SystemTime::now()
				.duration_since(UNIX_EPOCH)
				.unwrap_or_default()
				.as_nanos();
			let root = std::env::temp_dir().join(format!("zed-knip-lib-{name}-{nanos}"));
			fs::create_dir_all(&root).unwrap_or_else(|error| panic!("failed to create {}: {error}", root.display()));
			Self {
				root,
				settings_override: None,
			}
		}

		fn with_settings_override(mut self, settings: KnipSettings) -> Self {
			self.settings_override = Some(settings);
			self
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
			Ok(self.settings_override.clone())
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
				arguments: None,
				env: Some(HashMap::from([(
					"UNRELATED_ENV_VAR".to_string(),
					"ignored".to_string(),
				)])),
			}),
			initialization_options: None,
			settings: Some(zed::serde_json::json!({
				"auto_install": false,
				"config_path": "knip.ts",
				"require_config": true,
				"ts_config_path": "tsconfig.app.json"
			})),
		})
		.unwrap();

		assert_eq!(
			settings,
			KnipSettings {
				binary_path: Some("tools/knip-language-server".to_string()),
				auto_install: false,
				config_path: Some("knip.ts".to_string()),
				require_config: true,
				ts_config_path: Some("tsconfig.app.json".to_string()),
				diagnostics: KnipDiagnosticsSettings::default(),
				..KnipSettings::default()
			}
		);
	}

	#[test]
	fn settings_reject_removed_env_settings() {
		let err = settings_from_lsp_settings(zed::settings::LspSettings {
			binary: Some(zed::settings::CommandSettings {
				path: None,
				arguments: None,
				env: Some(HashMap::from([("KNIP_LOG_LEVEL".to_string(), "info".to_string())])),
			}),
			initialization_options: None,
			settings: None,
		})
		.unwrap_err();
		assert!(
			err.contains("KNIP_LOG_LEVEL"),
			"error must name the removed env key 'KNIP_LOG_LEVEL', got: {err}"
		);
		assert!(
			err.contains("removed") || err.to_lowercase().contains("env"),
			"error must describe the env-based setting as removed, got: {err}"
		);

		let err = settings_from_lsp_settings(zed::settings::LspSettings {
			binary: Some(zed::settings::CommandSettings {
				path: None,
				arguments: None,
				env: Some(HashMap::from([(
					"KNIP_PACKAGE_MANAGER".to_string(),
					"pnpm".to_string(),
				)])),
			}),
			initialization_options: None,
			settings: None,
		})
		.unwrap_err();
		assert!(
			err.contains("KNIP_PACKAGE_MANAGER"),
			"error must name the removed env key 'KNIP_PACKAGE_MANAGER', got: {err}"
		);
	}

	#[test]
	fn lsp_settings_accept_unrelated_binary_env_entries() {
		let settings = settings_from_lsp_settings(zed::settings::LspSettings {
			binary: Some(zed::settings::CommandSettings {
				path: Some("tools/knip-language-server".to_string()),
				arguments: None,
				env: Some(HashMap::from([
					("PATH".to_string(), "/usr/bin".to_string()),
					("MY_TOOL_FLAG".to_string(), "true".to_string()),
				])),
			}),
			initialization_options: None,
			settings: None,
		})
		.unwrap();

		assert_eq!(settings.binary_path.as_deref(), Some("tools/knip-language-server"));
	}

	#[test]
	fn lsp_settings_reject_binary_arguments() {
		let error = settings_from_lsp_settings(zed::settings::LspSettings {
			binary: Some(zed::settings::CommandSettings {
				path: None,
				arguments: Some(vec!["--tsConfig".to_string(), "tsconfig.lib.json".to_string()]),
				env: None,
			}),
			initialization_options: None,
			settings: None,
		})
		.unwrap_err();

		assert!(error.contains("`lsp.knip.binary.arguments` is not supported"));
		assert!(error.contains("`--tsConfig`"));
		assert!(error.contains("`lsp.knip.settings.config_path`"));
	}

	#[test]
	fn lsp_settings_accept_empty_binary_arguments() {
		let settings = settings_from_lsp_settings(zed::settings::LspSettings {
			binary: Some(zed::settings::CommandSettings {
				path: None,
				arguments: Some(Vec::new()),
				env: None,
			}),
			initialization_options: None,
			settings: Some(zed::serde_json::json!({
				"config_path": "knip.ts"
			})),
		})
		.unwrap();

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
		worktree.write("knip.json", "{}\n");

		let command =
			language_server_command_for_worktree(KNIP_LANGUAGE_SERVER_ID, &worktree, &ProductionResolver).unwrap();

		assert_eq!(command.command, executable.display().to_string());
		assert_eq!(command.args, vec!["--stdio".to_string()]);
		assert!(command.env.is_empty());
	}

	#[test]
	fn language_server_command_detects_bun_lock_through_worktree_api() {
		let worktree = TestWorktree::new("bun-lock");
		worktree.write("bun.lock", "");
		let executable = worktree.executable("node_modules/.bin/knip-language-server");

		let command =
			language_server_command_for_worktree(KNIP_LANGUAGE_SERVER_ID, &worktree, &ProductionResolver).unwrap();

		assert_eq!(command.command, executable.display().to_string());
		assert_eq!(command.args, vec!["--stdio".to_string()]);
		assert!(command.env.is_empty());
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
		assert_eq!(command.args, vec!["--stdio".to_string()]);
		assert!(command.env.is_empty());
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
		let expected_cwd = worktree.root.display().to_string();

		assert_eq!(
			options["config"]["configFilePath"].as_str(),
			Some(expected_config_path.as_str())
		);
		assert_eq!(options["cwd"].as_str(), Some(expected_cwd.as_str()));
		assert_eq!(options["config"]["editor"]["exports"]["quickfix"]["enabled"], true);

		let config_file_path_str = options["config"]["configFilePath"].as_str().unwrap();
		assert!(
			Path::new(config_file_path_str).is_absolute(),
			"configFilePath must be an absolute path, got: {config_file_path_str}"
		);
		let cwd_str = options["cwd"].as_str().unwrap();
		assert!(
			Path::new(cwd_str).is_absolute(),
			"cwd must be an absolute path, got: {cwd_str}"
		);
		assert!(
			options["config"].get("zedKnip").is_none(),
			"config must not contain 'zedKnip' key for baseline settings"
		);
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

	#[test]
	fn language_server_command_uses_stdio_only() {
		let worktree = TestWorktree::new("stdio-only");
		worktree.write("package.json", "{\"packageManager\":\"npm@10.0.0\"}\n");
		worktree.write("knip.json", "{}\n");
		worktree.executable("node_modules/.bin/knip-language-server");

		let command =
			language_server_command_for_worktree(KNIP_LANGUAGE_SERVER_ID, &worktree, &ProductionResolver).unwrap();

		assert_eq!(
			command.args,
			vec!["--stdio".to_string()],
			"args must be exactly ['--stdio']; workspace and config paths are initialization options, not launch args"
		);
		assert!(
			command.env.is_empty(),
			"env must be empty; cwd/config/package-manager are initialization options, not env vars"
		);
	}

	#[test]
	fn language_server_command_rejects_non_transport_arguments() {
		let error = settings_from_lsp_settings(zed::settings::LspSettings {
			binary: Some(zed::settings::CommandSettings {
				path: None,
				arguments: Some(vec!["--tsConfig".to_string(), "tsconfig.app.json".to_string()]),
				env: None,
			}),
			initialization_options: None,
			settings: None,
		})
		.unwrap_err();

		assert!(
			error.contains("--tsConfig"),
			"rejection error must name the unsupported argument '--tsConfig', got: {error}"
		);
		assert!(
			error.contains("`lsp.knip.binary.arguments` is not supported"),
			"rejection error must name the unsupported config key, got: {error}"
		);
	}

	#[test]
	fn language_server_configuration_includes_zed_knip_advanced_settings() {
		let settings = settings_from_lsp_settings(zed::settings::LspSettings {
			binary: None,
			initialization_options: None,
			settings: Some(zed::serde_json::json!({
				"ts_config_path": "tsconfig.app.json",
				"diagnostics": { "include": ["exports"] }
			})),
		})
		.unwrap();

		let workspace_root = std::path::Path::new("/workspace");
		let config = knip_workspace_configuration(&settings);
		let init_options = knip_initialization_options(&settings, workspace_root);

		assert_eq!(
			config["zedKnip"]["tsConfigFilePath"].as_str(),
			Some("tsconfig.app.json"),
			"workspace configuration must include zedKnip.tsConfigFilePath"
		);
		assert!(
			config["zedKnip"]["diagnostics"].is_object(),
			"workspace configuration must include zedKnip.diagnostics object"
		);
		assert_eq!(
			init_options["config"]["zedKnip"]["tsConfigFilePath"].as_str(),
			Some("tsconfig.app.json"),
			"initialization options must include config.zedKnip.tsConfigFilePath"
		);
	}

	#[test]
	fn settings_reject_removed_noop_settings() {
		let err = settings_from_lsp_settings(zed::settings::LspSettings {
			binary: None,
			initialization_options: None,
			settings: Some(zed::serde_json::json!({"server_path": "x"})),
		})
		.unwrap_err();
		assert!(
			err.contains("server_path"),
			"error must name the removed key 'server_path', got: {err}"
		);

		let err = settings_from_lsp_settings(zed::settings::LspSettings {
			binary: None,
			initialization_options: None,
			settings: Some(zed::serde_json::json!({"log_level": "info"})),
		})
		.unwrap_err();
		assert!(
			err.contains("log_level"),
			"error must name the removed key 'log_level', got: {err}"
		);

		let err = settings_from_lsp_settings(zed::settings::LspSettings {
			binary: None,
			initialization_options: None,
			settings: Some(zed::serde_json::json!({"package_manager": "npm"})),
		})
		.unwrap_err();
		assert!(
			err.contains("package_manager"),
			"error must name the removed key 'package_manager', got: {err}"
		);
	}

	#[derive(Debug, Clone)]
	struct MockManagedInstall {
		path: PathBuf,
	}

	impl ManagedInstall for MockManagedInstall {
		fn install(&self, _root: &Path, _pm: PackageManager) -> Result<PathBuf, KnipError> {
			Ok(self.path.clone())
		}
	}

	#[test]
	fn require_config_blocks_workspace_without_knip_config() {
		let worktree = TestWorktree::new("require-config-missing");
		worktree.write("package.json", "{\"packageManager\":\"npm@10.0.0\"}\n");

		let settings = Some(KnipSettings {
			require_config: true,
			..KnipSettings::default()
		});

		let error = settings_for_worktree(&worktree, settings).unwrap_err();

		assert!(
			error.contains("require_config"),
			"error must mention require_config, got: {error}"
		);
	}

	#[test]
	fn settings_reject_missing_explicit_config_path() {
		let worktree = TestWorktree::new("config-path-missing");
		worktree.write("package.json", "{\"packageManager\":\"npm@10.0.0\"}\n");

		let settings = Some(KnipSettings {
			config_path: Some("missing/knip.json".to_string()),
			..KnipSettings::default()
		});

		let error = settings_for_worktree(&worktree, settings).unwrap_err();
		let expected = worktree.root.join("missing/knip.json").display().to_string();

		assert!(
			error.contains(&expected),
			"error must mention the resolved path '{expected}', got: {error}"
		);
		assert!(
			error.contains("config_path") || error.to_lowercase().contains("config"),
			"error must mention the config_path setting, got: {error}"
		);
	}

	#[test]
	fn advanced_settings_reject_custom_language_server_path() {
		let worktree = TestWorktree::new("advanced-custom-path");
		worktree.write("package.json", "{\"packageManager\":\"npm@10.0.0\"}\n");
		worktree.write("tsconfig.json", "{}");

		let settings = KnipSettings {
			binary_path: Some("tools/knip".to_string()),
			ts_config_path: Some("tsconfig.json".to_string()),
			..KnipSettings::default()
		};

		let cache = WorktreeCache::new(worktree.root.clone());
		let error = resolve_knip(&settings, &cache, &ManagedInstallDisabled).unwrap_err();
		let message = error.to_string();

		assert!(
			message.contains("managed"),
			"error must mention managed install, got: {message}"
		);
		assert!(
			message.contains("binary.path"),
			"error must mention binary.path, got: {message}"
		);
	}

	#[test]
	fn advanced_settings_force_managed_install() {
		let worktree = TestWorktree::new("advanced-force-managed");
		worktree.write("package.json", "{\"packageManager\":\"npm@10.0.0\"}\n");
		worktree.write("tsconfig.json", "{}");
		worktree.executable("node_modules/.bin/knip-language-server");
		let managed = worktree.executable("managed/knip-language-server");

		let settings = KnipSettings {
			ts_config_path: Some("tsconfig.json".to_string()),
			..KnipSettings::default()
		};

		let cache = WorktreeCache::new(worktree.root.clone());
		let installer = MockManagedInstall { path: managed.clone() };
		let resolved = resolve_knip(&settings, &cache, &installer).unwrap();

		assert_eq!(
			resolved.executable_path, managed,
			"resolver must use managed install, not workspace-local, when requires_managed_patch"
		);
		assert_eq!(resolved.install_source, InstallSource::ManagedCache);
	}

	#[test]
	fn ts_config_path_rejects_absolute_or_parent_paths() {
		let worktree = TestWorktree::new("ts-config-path-invalid");
		worktree.write("package.json", "{\"packageManager\":\"npm@10.0.0\"}\n");

		let settings_absolute = Some(KnipSettings {
			ts_config_path: Some("/etc/passwd".to_string()),
			..KnipSettings::default()
		});
		let error = settings_for_worktree(&worktree, settings_absolute).unwrap_err();
		assert!(
			error.contains("ts_config_path"),
			"error for absolute path must mention ts_config_path, got: {error}"
		);

		let settings_parent = Some(KnipSettings {
			ts_config_path: Some("../escape.json".to_string()),
			..KnipSettings::default()
		});
		let error = settings_for_worktree(&worktree, settings_parent).unwrap_err();
		assert!(
			error.contains("ts_config_path") || error.contains(".."),
			"error for parent traversal must mention ts_config_path or '..', got: {error}"
		);
	}

	#[test]
	fn ts_config_path_rejects_missing_file() {
		let worktree = TestWorktree::new("ts-config-path-missing-file");
		worktree.write("package.json", "{\"packageManager\":\"npm@10.0.0\"}\n");

		let settings = Some(KnipSettings {
			ts_config_path: Some("missing/tsconfig.app.json".to_string()),
			..KnipSettings::default()
		});

		let error = settings_for_worktree(&worktree, settings).unwrap_err();

		assert!(
			error.contains("ts_config_path"),
			"error must mention ts_config_path, got: {error}"
		);
		let expected = worktree.root.join("missing/tsconfig.app.json").display().to_string();
		assert!(
			error.contains(&expected),
			"error must mention the resolved path '{expected}', got: {error}"
		);
		assert!(
			error.contains("file not found") || error.contains("not found"),
			"error must explain the file is missing, got: {error}"
		);
	}

	#[test]
	fn custom_binary_path_supports_baseline_stdio_launch() {
		let worktree = TestWorktree::new("custom-baseline-stdio");
		worktree.write("package.json", "{\"packageManager\":\"npm@10.0.0\"}\n");
		worktree.write("knip.json", "{}\n");
		let custom_binary = worktree.executable("tools/custom-knip-language-server");

		let settings = KnipSettings {
			binary_path: Some("tools/custom-knip-language-server".to_string()),
			..KnipSettings::default()
		};
		let worktree = worktree.with_settings_override(settings);

		let command =
			language_server_command_for_worktree(KNIP_LANGUAGE_SERVER_ID, &worktree, &ProductionResolver).unwrap();

		assert_eq!(
			command.command,
			custom_binary.display().to_string(),
			"custom binary.path must be used when no advanced setting is enabled"
		);
		assert_eq!(
			command.args,
			vec!["--stdio".to_string()],
			"args must be exactly ['--stdio'] for a baseline custom binary"
		);
		assert!(command.env.is_empty(), "env must be empty for a baseline custom binary");
	}

	#[test]
	fn custom_binary_rejects_ts_config_path() {
		let worktree = TestWorktree::new("custom-rejects-ts-config");
		worktree.write("package.json", "{\"packageManager\":\"npm@10.0.0\"}\n");
		worktree.write("tsconfig.json", "{}");
		worktree.executable("tools/custom-knip-language-server");

		let settings = KnipSettings {
			binary_path: Some("tools/custom-knip-language-server".to_string()),
			ts_config_path: Some("tsconfig.json".to_string()),
			..KnipSettings::default()
		};
		let worktree = worktree.with_settings_override(settings);

		let error =
			language_server_command_for_worktree(KNIP_LANGUAGE_SERVER_ID, &worktree, &ProductionResolver).unwrap_err();

		assert!(
			error.contains("ts_config_path"),
			"error must mention ts_config_path, got: {error}"
		);
		assert!(
			error.contains("managed install"),
			"error must mention managed install, got: {error}"
		);
		assert!(
			error.contains("binary.path"),
			"error must mention binary.path, got: {error}"
		);
	}

	#[test]
	fn preprocessor_configuration_emits_array_in_order() {
		let settings = settings_from_lsp_settings(zed::settings::LspSettings {
			binary: None,
			initialization_options: None,
			settings: Some(zed::serde_json::json!({
				"preprocessor": ["./tools/prep1.mjs", "prep-package", "@scope/prep2"]
			})),
		})
		.unwrap();

		let config = knip_workspace_configuration(&settings);

		let prep_array = config["zedKnip"]["preprocessor"].as_array().unwrap();
		assert_eq!(prep_array.len(), 3, "preprocessor array must have 3 entries");
		assert_eq!(prep_array[0].as_str(), Some("./tools/prep1.mjs"));
		assert_eq!(prep_array[1].as_str(), Some("prep-package"));
		assert_eq!(prep_array[2].as_str(), Some("@scope/prep2"));
	}

	#[test]
	fn preprocessor_options_serialized_as_object() {
		let settings = settings_from_lsp_settings(zed::settings::LspSettings {
			binary: None,
			initialization_options: None,
			settings: Some(zed::serde_json::json!({
				"preprocessor": ["prep"],
				"preprocessor_options": {
					"key1": "value1",
					"key2": 42
				}
			})),
		})
		.unwrap();

		let config = knip_workspace_configuration(&settings);

		let opts_obj = config["zedKnip"]["preprocessorOptions"].as_object().unwrap();
		assert_eq!(opts_obj.get("key1").map(|v| v.as_str()), Some(Some("value1")));
		assert_eq!(opts_obj.get("key2").map(|v| v.as_i64()), Some(Some(42)));
	}

	#[test]
	fn preprocessor_options_default_to_empty_object() {
		let settings = settings_from_lsp_settings(zed::settings::LspSettings {
			binary: None,
			initialization_options: None,
			settings: Some(zed::serde_json::json!({
				"preprocessor": ["prep"]
			})),
		})
		.unwrap();

		let config = knip_workspace_configuration(&settings);

		assert!(
			config["zedKnip"]["preprocessorOptions"].is_object(),
			"preprocessorOptions must be an object even when not configured"
		);
		let opts_obj = config["zedKnip"]["preprocessorOptions"].as_object().unwrap();
		assert!(
			opts_obj.is_empty(),
			"preprocessorOptions must be empty object when not configured"
		);
	}

	#[test]
	fn preprocessor_fingerprint_changes_with_list_order_and_options() {
		let settings1 = settings_from_lsp_settings(zed::settings::LspSettings {
			binary: None,
			initialization_options: None,
			settings: Some(zed::serde_json::json!({
				"preprocessor": ["a", "b"]
			})),
		})
		.unwrap();
		let config1 = knip_workspace_configuration(&settings1);
		let fp1 = config1["zedKnip"]["preprocessorFingerprint"].as_str().unwrap();

		let settings2 = settings_from_lsp_settings(zed::settings::LspSettings {
			binary: None,
			initialization_options: None,
			settings: Some(zed::serde_json::json!({
				"preprocessor": ["b", "a"]
			})),
		})
		.unwrap();
		let config2 = knip_workspace_configuration(&settings2);
		let fp2 = config2["zedKnip"]["preprocessorFingerprint"].as_str().unwrap();

		assert_ne!(
			fp1, fp2,
			"fingerprint must change when preprocessor order changes: {} vs {}",
			fp1, fp2
		);

		let settings3 = settings_from_lsp_settings(zed::settings::LspSettings {
			binary: None,
			initialization_options: None,
			settings: Some(zed::serde_json::json!({
				"preprocessor": ["a", "b"],
				"preprocessor_options": { "opt": "val" }
			})),
		})
		.unwrap();
		let config3 = knip_workspace_configuration(&settings3);
		let fp3 = config3["zedKnip"]["preprocessorFingerprint"].as_str().unwrap();

		assert_ne!(
			fp1, fp3,
			"fingerprint must change when options are added: {} vs {}",
			fp1, fp3
		);
	}

	#[test]
	fn preprocessor_hard_launch_preserves_stdio_args_and_empty_env() {
		let worktree = TestWorktree::new("preprocessor-stdio-launch");
		worktree.write("package.json", "{\"packageManager\":\"npm@10.0.0\"}\n");
		worktree.write("knip.json", "{}\n");
		worktree.executable("node_modules/.bin/knip-language-server");

		let settings = KnipSettings {
			preprocessor: vec!["./tools/prep.mjs".to_string()],
			preprocessor_options: Some(std::collections::BTreeMap::from([(
				"key".to_string(),
				zed::serde_json::json!("value"),
			)])),
			..KnipSettings::default()
		};
		let worktree = worktree.with_settings_override(settings);

		let fake_command = zed::Command {
			command: worktree.root.join("managed/knip-language-server").display().to_string(),
			args: vec!["--stdio".to_string()],
			env: vec![],
		};
		let resolver = MockResolver {
			result: Ok(fake_command),
		};

		let command = language_server_command_for_worktree(KNIP_LANGUAGE_SERVER_ID, &worktree, &resolver).unwrap();

		assert_eq!(
			command.args,
			vec!["--stdio".to_string()],
			"args must be exactly ['--stdio']; preprocessors flow through LSP config, not CLI args"
		);
		assert!(
			command.env.is_empty(),
			"env must be empty; preprocessor config flows through LSP initialization options"
		);
	}

	#[test]
	fn custom_binary_rejects_preprocessor() {
		let worktree = TestWorktree::new("custom-rejects-preprocessor");
		worktree.write("package.json", "{\"packageManager\":\"npm@10.0.0\"}\n");
		worktree.executable("tools/custom-knip-language-server");

		let settings = KnipSettings {
			binary_path: Some("tools/custom-knip-language-server".to_string()),
			preprocessor: vec!["echo".to_string(), "setup".to_string()],
			..KnipSettings::default()
		};
		let worktree = worktree.with_settings_override(settings);

		let error =
			language_server_command_for_worktree(KNIP_LANGUAGE_SERVER_ID, &worktree, &ProductionResolver).unwrap_err();

		assert!(
			error.contains("preprocessor"),
			"error must mention preprocessor, got: {error}"
		);
		assert!(
			error.contains("managed install"),
			"error must mention managed install, got: {error}"
		);
		assert!(
			error.contains("binary.path"),
			"error must mention binary.path, got: {error}"
		);
	}

	#[test]
	fn custom_binary_rejects_preprocessor_options() {
		let worktree = TestWorktree::new("custom-rejects-preprocessor-options");
		worktree.write("package.json", "{\"packageManager\":\"npm@10.0.0\"}\n");
		worktree.executable("tools/custom-knip-language-server");

		use std::collections::BTreeMap;
		let mut opts = BTreeMap::new();
		opts.insert("verbose".to_string(), zed::serde_json::Value::Bool(true));

		let settings = KnipSettings {
			binary_path: Some("tools/custom-knip-language-server".to_string()),
			preprocessor: vec!["./local-preprocessor".to_string()],
			preprocessor_options: Some(opts),
			..KnipSettings::default()
		};
		let worktree = worktree.with_settings_override(settings);

		let error =
			language_server_command_for_worktree(KNIP_LANGUAGE_SERVER_ID, &worktree, &ProductionResolver).unwrap_err();

		assert!(
			error.contains("preprocessor_options"),
			"error must mention preprocessor_options, got: {error}"
		);
		assert!(
			error.contains("managed install"),
			"error must mention managed install, got: {error}"
		);
		assert!(
			error.contains("binary.path"),
			"error must mention binary.path, got: {error}"
		);
	}

	#[test]
	fn custom_binary_rejects_preprocessor_and_options() {
		let worktree = TestWorktree::new("custom-rejects-preprocessor-both");
		worktree.write("package.json", "{\"packageManager\":\"npm@10.0.0\"}\n");
		worktree.executable("tools/custom-knip-language-server");

		use std::collections::BTreeMap;
		let mut opts = BTreeMap::new();
		opts.insert("verbose".to_string(), zed::serde_json::Value::Bool(false));

		let settings = KnipSettings {
			binary_path: Some("tools/custom-knip-language-server".to_string()),
			preprocessor: vec!["echo".to_string()],
			preprocessor_options: Some(opts),
			..KnipSettings::default()
		};
		let worktree = worktree.with_settings_override(settings);

		let error =
			language_server_command_for_worktree(KNIP_LANGUAGE_SERVER_ID, &worktree, &ProductionResolver).unwrap_err();

		assert!(
			error.contains("preprocessor") && error.contains("preprocessor_options"),
			"error must mention both preprocessor settings, got: {error}"
		);
		assert!(
			error.contains("managed install"),
			"error must mention managed install, got: {error}"
		);
	}

	#[test]
	fn managed_preprocessor_auto_install_works() {
		let worktree = TestWorktree::new("managed-preprocessor-works");
		worktree.write("package.json", "{\"packageManager\":\"npm@10.0.0\"}\n");
		worktree.executable("node_modules/.bin/knip-language-server");
		let managed = worktree.executable("managed/knip-language-server");

		let settings = KnipSettings {
			preprocessor: vec!["echo".to_string()],
			..KnipSettings::default()
		};

		let cache = WorktreeCache::new(worktree.root.clone());
		let installer = MockManagedInstall { path: managed.clone() };
		let resolved = resolve_knip(&settings, &cache, &installer).unwrap();

		assert_eq!(
			resolved.executable_path, managed,
			"resolver must use managed install when preprocessor is configured without custom binary"
		);
		assert_eq!(resolved.install_source, InstallSource::ManagedCache);
	}
}
