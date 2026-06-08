use crate::{
	cache::{CacheState, InstallSource, InvalidationInputs, StaleReason, WorktreeCache},
	errors::KnipError,
	is_executable,
	logging::{InstallProgress, Logger},
	package_manager::PackageManager,
	settings::KnipSettings,
};
use std::{
	collections::hash_map::DefaultHasher,
	env, fs,
	hash::{Hash, Hasher},
	io,
	path::{Path, PathBuf},
};
use zed_extension_api as zed;

const LANGUAGE_SERVER_BIN: &str = "knip-language-server";
const LANGUAGE_SERVER_PACKAGE: &str = "@knip/language-server";
const MANAGED_LANGUAGE_SERVER_BIN: &str = "node_modules/.bin/knip-language-server";
pub(crate) const DID_SAVE_REFRESH_PATCH_MARKER: &str = "zed-knip: refresh Knip diagnostics on textDocument/didSave";
pub(crate) const EDITOR_WORKFLOW_PATCH_MARKER: &str = "zed-knip: editor workflow patch";
const DEFAULT_MANAGED_VERSION: &str = "latest";

/// Resolves or installs a managed Knip language-server binary for a workspace.
pub trait ManagedInstall {
	/// Installs the language server and returns the executable path.
	fn install(&self, workspace_root: &Path, package_manager: PackageManager) -> Result<PathBuf, KnipError>;
}

/// Managed-install implementation that always reports configuration as disabled.
#[derive(Debug, Default, Clone, Copy)]
pub struct ManagedInstallDisabled;

impl ManagedInstall for ManagedInstallDisabled {
	fn install(&self, _workspace_root: &Path, _package_manager: PackageManager) -> Result<PathBuf, KnipError> {
		Err(KnipError::FailedManagedInstall {
			reason: "managed install is not configured".to_string(),
		})
	}
}

/// Managed-install implementation backed by Zed's npm host APIs.
#[derive(Debug, Default, Clone, Copy)]
pub struct ZedNpmManagedInstall;

impl ManagedInstall for ZedNpmManagedInstall {
	fn install(&self, _workspace_root: &Path, _package_manager: PackageManager) -> Result<PathBuf, KnipError> {
		let executable_path = env::current_dir()
			.map_err(|error| KnipError::FailedManagedInstall {
				reason: format!("failed to resolve extension working directory: {error}"),
			})?
			.join(MANAGED_LANGUAGE_SERVER_BIN);

		let installed_version = zed::npm_package_installed_version(LANGUAGE_SERVER_PACKAGE).map_err(|error| {
			KnipError::FailedManagedInstall {
				reason: format!("failed to inspect installed {LANGUAGE_SERVER_PACKAGE}: {error}"),
			}
		})?;

		let latest_version = match zed::npm_package_latest_version(LANGUAGE_SERVER_PACKAGE) {
			Ok(version) => version,
			Err(_error) if installed_version.is_some() && executable_path.is_file() => {
				patch_managed_language_server(&executable_path)?;
				return Ok(executable_path);
			}
			Err(error) => {
				return Err(KnipError::NetworkUnavailable {
					detail: format!("failed to resolve latest {LANGUAGE_SERVER_PACKAGE}: {error}"),
				});
			}
		};

		if installed_version.as_ref() != Some(&latest_version) {
			zed::npm_install_package(LANGUAGE_SERVER_PACKAGE, &latest_version).map_err(|error| {
				KnipError::FailedManagedInstall {
					reason: format!("failed to install {LANGUAGE_SERVER_PACKAGE}@{latest_version}: {error}"),
				}
			})?;
		}

		patch_managed_language_server(&executable_path)?;
		Ok(executable_path)
	}
}

pub(crate) fn patch_managed_language_server(executable_path: &Path) -> Result<(), KnipError> {
	let server_path = managed_language_server_server_path(executable_path)?;
	let source = fs::read_to_string(&server_path).map_err(|error| KnipError::FailedManagedInstall {
		reason: format!("failed to read managed {LANGUAGE_SERVER_PACKAGE} server: {error}"),
	})?;

	let (did_save_applied, after_did_save) =
		match apply_did_save_refresh_patch(&source).map_err(|reason| KnipError::FailedManagedInstall {
			reason: format!("failed to patch managed {LANGUAGE_SERVER_PACKAGE} server: {reason}"),
		})? {
			Some(patched) => (true, patched),
			None => (false, source),
		};

	match apply_editor_workflow_patch(&after_did_save).map_err(|reason| KnipError::FailedManagedInstall {
		reason: format!("failed to patch managed {LANGUAGE_SERVER_PACKAGE} server: {reason}"),
	})? {
		Some(patched) => fs::write(&server_path, patched).map_err(|error| KnipError::FailedManagedInstall {
			reason: format!("failed to write managed {LANGUAGE_SERVER_PACKAGE} server: {error}"),
		}),
		None if did_save_applied => {
			fs::write(&server_path, after_did_save).map_err(|error| KnipError::FailedManagedInstall {
				reason: format!("failed to write managed {LANGUAGE_SERVER_PACKAGE} server: {error}"),
			})
		}
		None => Ok(()),
	}
}

fn managed_language_server_server_path(executable_path: &Path) -> Result<PathBuf, KnipError> {
	let cli_path = match fs::read_link(executable_path) {
		Ok(target) if target.is_absolute() => target,
		Ok(target) => executable_path.parent().unwrap_or_else(|| Path::new("")).join(target),
		Err(error) => {
			return Err(KnipError::FailedManagedInstall {
				reason: format!("failed to resolve managed {LANGUAGE_SERVER_BIN} symlink: {error}"),
			});
		}
	};

	Ok(cli_path.parent().unwrap_or_else(|| Path::new("")).join("server.js"))
}

pub(crate) fn apply_did_save_refresh_patch(source: &str) -> Result<Option<String>, String> {
	if source.contains(DID_SAVE_REFRESH_PATCH_MARKER) {
		return Ok(None);
	}

	let patched = replace_once(
		source,
		"import { FileChangeType, ProposedFeatures, TextDocuments } from 'vscode-languageserver';",
		"import { FileChangeType, ProposedFeatures, TextDocumentSyncKind, TextDocuments } from 'vscode-languageserver';",
		"vscode-languageserver import",
	)?;
	let patched = replace_once(
		&patched,
		"const capabilities = {\n        codeActionProvider:",
		"const capabilities = {\n        textDocumentSync: {\n          openClose: true,\n          change: TextDocumentSyncKind.None,\n          save: { includeText: false },\n        },\n        codeActionProvider:",
		"initialize capabilities",
	)?;
	let patched = replace_once(
		&patched,
		"this.documents.listen(this.connection);\n    this.connection.listen();",
		&format!(
			"this.documents.listen(this.connection);\n    // {DID_SAVE_REFRESH_PATCH_MARKER}\n    this.documents.onDidSave(event => {{\n      void this.handleFileChanges({{ changes: [{{ uri: event.document.uri, type: FileChangeType.Changed }}] }});\n    }});\n    this.connection.listen();"
		),
		"document save listener",
	)?;

	Ok(Some(patched))
}

pub(crate) fn apply_editor_workflow_patch(source: &str) -> Result<Option<String>, String> {
	if source.contains(EDITOR_WORKFLOW_PATCH_MARKER) {
		return Ok(None);
	}

	let patched = replace_once(
		source,
		"args: { config: configFilePath }",
		"args: { config: configFilePath, ...(config?.zedKnip?.tsConfigFilePath ? { tsConfig: path.resolve(this.cwd ?? process.cwd(), config.zedKnip.tsConfigFilePath) } : {}) }",
		"createOptions editor-workflow",
	)?;

	let diag_replacement = format!(
		"          const document = this.documents.get(uri);\n          // {EDITOR_WORKFLOW_PATCH_MARKER}\n          if (config?.zedKnip?.diagnostics) {{\n            const _d = config.zedKnip.diagnostics;\n            const _inc = _d.includeIssueTypes ?? [];\n            const _exc = _d.excludeIssueTypes ?? [];\n            const _pfx = _d.excludePathPrefixes ?? [];\n            const _sev = _d.severityByIssueType ?? {{}};\n            if (_inc.length > 0 && !_inc.includes(issue.type)) continue;\n            if (_exc.includes(issue.type)) continue;\n            if (_pfx.length > 0) {{\n              const _rel = path.relative(this.cwd ?? process.cwd(), issue.filePath).split(path.sep).join('/');\n              if (_pfx.some(p => _rel === p || _rel.startsWith(p + '/'))) continue;\n            }}\n            if (_sev[issue.type] === 'off') continue;\n          }}\n          const diagnostic = issueToDiagnostic(issue, rules, config, document);"
	);

	let patched = replace_once(
		&patched,
		"          const document = this.documents.get(uri);\n          const diagnostic = issueToDiagnostic(issue, rules, config, document);",
		&diag_replacement,
		"buildDiagnostics editor-workflow",
	)?;

	Ok(Some(patched))
}

fn replace_once(source: &str, needle: &str, replacement: &str, label: &str) -> Result<String, String> {
	if !source.contains(needle) {
		return Err(format!("missing {label} patch anchor"));
	}

	Ok(source.replacen(needle, replacement, 1))
}

/// Parameters for a managed-install backend invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedInstallRequest {
	pub package_manager: PackageManager,
	pub version: String,
	pub cache_dir: PathBuf,
	pub executable_path: PathBuf,
}

/// Backend-level failure modes surfaced during managed installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallFailure {
	NetworkUnavailable { detail: String },
	ReadOnlyCache { path: PathBuf },
	CorruptCache { detail: String },
	Failed { reason: String },
}

/// Low-level backend responsible for populating a managed cache directory.
pub trait InstallBackend {
	fn install(&self, request: &ManagedInstallRequest) -> Result<(), InstallFailure>;
}

/// Backend placeholder used when no managed-install backend is configured.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnconfiguredInstallBackend;

impl InstallBackend for UnconfiguredInstallBackend {
	fn install(&self, _request: &ManagedInstallRequest) -> Result<(), InstallFailure> {
		Err(InstallFailure::Failed {
			reason: "no managed install backend is configured".to_string(),
		})
	}
}

/// Outcome category for a managed-install resolution attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagedInstallStatus {
	CacheHit,
	Installed { previous_state: CacheState },
}

/// Result of resolving a managed Knip executable for a workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedInstallOutcome {
	pub executable_path: PathBuf,
	pub cache: WorktreeCache,
	pub status: ManagedInstallStatus,
}

/// High-level managed installer that handles caching, invalidation, and backend execution.
#[derive(Debug, Clone)]
pub struct ManagedInstaller<B = UnconfiguredInstallBackend> {
	cache_root: PathBuf,
	logger: Logger,
	backend: B,
	version: String,
}

impl<B> ManagedInstaller<B>
where
	B: InstallBackend,
{
	#[must_use]
	/// Creates a managed installer rooted at `cache_root`.
	pub fn new(cache_root: PathBuf, logger: Logger, backend: B) -> Self {
		Self {
			cache_root,
			logger,
			backend,
			version: DEFAULT_MANAGED_VERSION.to_string(),
		}
	}

	#[must_use]
	/// Overrides the managed language-server version requested from the backend.
	pub fn with_version(mut self, version: impl Into<String>) -> Self {
		self.version = version.into();
		self
	}

	/// Resolves a managed Knip executable, reusing or refreshing the cache as needed.
	pub fn resolve(
		&self,
		cache: &WorktreeCache,
		current_inputs: InvalidationInputs,
		settings: &KnipSettings,
		package_manager: PackageManager,
	) -> Result<ManagedInstallOutcome, KnipError> {
		self.logger.install_progress(&InstallProgress::CheckingCache);

		let state = cache.check_validity(&current_inputs);
		if state == CacheState::Hit {
			if let Some(executable_path) = valid_managed_cache_path(cache) {
				self.logger.install_progress(&InstallProgress::CacheHit {
					version: cache.version.clone().unwrap_or_else(|| self.version.clone()),
				});
				self.logger.install_progress(&InstallProgress::Ready {
					path: executable_path.display().to_string(),
				});
				return Ok(ManagedInstallOutcome {
					executable_path,
					cache: cache.clone(),
					status: ManagedInstallStatus::CacheHit,
				});
			}

			return self.recover_corrupt_cache(cache, current_inputs, settings, package_manager);
		}

		if !settings.auto_install {
			return Err(KnipError::MissingKnip {
				workspace_root: cache.worktree_root.clone(),
			});
		}

		self.install_for_state(cache, current_inputs, package_manager, state)
	}

	fn recover_corrupt_cache(
		&self,
		cache: &WorktreeCache,
		current_inputs: InvalidationInputs,
		settings: &KnipSettings,
		package_manager: PackageManager,
	) -> Result<ManagedInstallOutcome, KnipError> {
		if !settings.auto_install {
			let error = cache.mark_corrupt();
			self.logger.install_progress(&InstallProgress::Failed {
				reason: error.to_string(),
			});
			return Err(error);
		}

		self.clear_cache_dir(cache)?;
		self.install_for_state(
			cache,
			current_inputs,
			package_manager,
			CacheState::Stale(StaleReason::Corrupt),
		)
	}

	fn install_for_state(
		&self,
		cache: &WorktreeCache,
		current_inputs: InvalidationInputs,
		package_manager: PackageManager,
		previous_state: CacheState,
	) -> Result<ManagedInstallOutcome, KnipError> {
		let cache_dir = self.worktree_cache_dir(&cache.worktree_root);
		ensure_writable_cache_dir(&cache_dir)?;

		let executable_path = cache_dir.join(LANGUAGE_SERVER_BIN);
		let request = ManagedInstallRequest {
			package_manager,
			version: self.version.clone(),
			cache_dir: cache_dir.clone(),
			executable_path: executable_path.clone(),
		};

		self.logger.install_progress(&InstallProgress::Downloading {
			version: request.version.clone(),
		});

		self.backend.install(&request).map_err(|failure| {
			let error = install_failure_to_error(failure, &cache_dir);
			self.logger.install_progress(&InstallProgress::Failed {
				reason: error.to_string(),
			});
			error
		})?;

		self.logger.install_progress(&InstallProgress::Verifying);
		if !is_valid_executable(&executable_path) {
			let error = KnipError::CorruptCache {
				path: cache_dir.clone(),
				detail: format!("{} is missing or is not executable", executable_path.display()),
			};
			self.logger.install_progress(&InstallProgress::Failed {
				reason: error.to_string(),
			});
			let _ = fs::remove_dir_all(&cache_dir);
			return Err(error);
		}

		let updated_cache = WorktreeCache {
			worktree_root: cache.worktree_root.clone(),
			executable_path: Some(executable_path.clone()),
			package_manager: Some(package_manager.to_string()),
			config_path: cache.config_path.clone(),
			version: Some(request.version),
			install_source: InstallSource::ManagedCache,
			last_error: None,
			invalidation_inputs: current_inputs,
		};

		self.logger.install_progress(&InstallProgress::Ready {
			path: executable_path.display().to_string(),
		});

		Ok(ManagedInstallOutcome {
			executable_path,
			cache: updated_cache,
			status: ManagedInstallStatus::Installed { previous_state },
		})
	}

	fn clear_cache_dir(&self, cache: &WorktreeCache) -> Result<(), KnipError> {
		let cache_dir = cache
			.executable_path
			.as_deref()
			.and_then(Path::parent)
			.map(Path::to_path_buf)
			.unwrap_or_else(|| self.worktree_cache_dir(&cache.worktree_root));

		match fs::remove_dir_all(&cache_dir) {
			Ok(()) => Ok(()),
			Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
			Err(error) if is_read_only_error(&error) => Err(WorktreeCache::mark_read_only(cache_dir)),
			Err(error) => Err(KnipError::FailedManagedInstall {
				reason: format!("failed to clear corrupt cache: {error}"),
			}),
		}
	}

	#[must_use]
	/// Returns the cache directory used for the given worktree.
	pub fn worktree_cache_dir(&self, worktree_root: &Path) -> PathBuf {
		self.cache_root.join(worktree_cache_key(worktree_root))
	}
}

impl ManagedInstaller<UnconfiguredInstallBackend> {
	#[must_use]
	/// Creates a managed installer with the placeholder unconfigured backend.
	pub fn unconfigured(cache_root: PathBuf, logger: Logger) -> Self {
		Self::new(cache_root, logger, UnconfiguredInstallBackend)
	}
}

impl<B> ManagedInstall for ManagedInstaller<B>
where
	B: InstallBackend,
{
	fn install(&self, workspace_root: &Path, package_manager: PackageManager) -> Result<PathBuf, KnipError> {
		let cache = WorktreeCache::new(workspace_root.to_path_buf());
		let outcome = self.resolve(
			&cache,
			InvalidationInputs::default(),
			&KnipSettings::default(),
			package_manager,
		)?;
		Ok(outcome.executable_path)
	}
}

fn valid_managed_cache_path(cache: &WorktreeCache) -> Option<PathBuf> {
	if cache.install_source != InstallSource::ManagedCache {
		return None;
	}

	let executable_path = cache.executable_path.as_ref()?;
	if is_valid_executable(executable_path) {
		Some(executable_path.clone())
	} else {
		None
	}
}

fn ensure_writable_cache_dir(cache_dir: &Path) -> Result<(), KnipError> {
	fs::create_dir_all(cache_dir).map_err(|error| cache_dir_io_error(error, cache_dir))?;

	let probe = cache_dir.join(".write-test");
	fs::write(&probe, b"write-test").map_err(|error| cache_dir_io_error(error, cache_dir))?;
	fs::remove_file(&probe).map_err(|error| cache_dir_io_error(error, cache_dir))?;

	Ok(())
}

fn cache_dir_io_error(error: io::Error, cache_dir: &Path) -> KnipError {
	if is_read_only_error(&error) {
		KnipError::ReadOnlyCache {
			path: cache_dir.to_path_buf(),
		}
	} else {
		KnipError::FailedManagedInstall {
			reason: format!("failed to prepare cache directory {}: {error}", cache_dir.display()),
		}
	}
}

fn install_failure_to_error(failure: InstallFailure, cache_dir: &Path) -> KnipError {
	match failure {
		InstallFailure::NetworkUnavailable { detail } => KnipError::NetworkUnavailable { detail },
		InstallFailure::ReadOnlyCache { path } => KnipError::ReadOnlyCache { path },
		InstallFailure::CorruptCache { detail } => KnipError::CorruptCache {
			path: cache_dir.to_path_buf(),
			detail,
		},
		InstallFailure::Failed { reason } => KnipError::FailedManagedInstall { reason },
	}
}

fn is_read_only_error(error: &io::Error) -> bool {
	matches!(
		error.kind(),
		io::ErrorKind::PermissionDenied | io::ErrorKind::ReadOnlyFilesystem
	)
}

fn is_valid_executable(path: &Path) -> bool {
	path.is_file() && is_executable(path)
}

fn worktree_cache_key(worktree_root: &Path) -> String {
	let display = worktree_root.display().to_string();
	let mut hasher = DefaultHasher::new();
	display.hash(&mut hasher);
	let hash = hasher.finish();
	let sanitized = display
		.chars()
		.map(|character| {
			if character.is_ascii_alphanumeric() {
				character.to_ascii_lowercase()
			} else {
				'-'
			}
		})
		.collect::<String>()
		.trim_matches('-')
		.to_string();

	if sanitized.is_empty() {
		format!("worktree-{hash:016x}")
	} else {
		format!("{sanitized}-{hash:016x}")
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::settings::LogLevel;
	use std::{cell::RefCell, os::unix::fs::PermissionsExt, time::SystemTime};

	#[derive(Debug)]
	struct TestWorkspace {
		root: PathBuf,
	}

	impl TestWorkspace {
		fn new(name: &str) -> Self {
			let nanos = SystemTime::now()
				.duration_since(SystemTime::UNIX_EPOCH)
				.unwrap_or_default()
				.as_nanos();
			let root = std::env::temp_dir().join(format!("zed-knip-managed-install-{name}-{nanos}"));
			fs::create_dir_all(&root).unwrap_or_else(|error| panic!("failed to create {}: {error}", root.display()));
			Self { root }
		}

		fn cache_root(&self) -> PathBuf {
			self.root.join("extension-cache")
		}

		fn executable(&self, relative_path: &str) -> PathBuf {
			let path = self.root.join(relative_path);
			write_executable(&path);
			path
		}
	}

	impl Drop for TestWorkspace {
		fn drop(&mut self) {
			let _ = fs::remove_dir_all(&self.root);
		}
	}

	#[derive(Debug, Clone)]
	struct RecordingBackend {
		calls: RefCell<Vec<ManagedInstallRequest>>,
		result: Result<(), InstallFailure>,
	}

	impl RecordingBackend {
		fn success() -> Self {
			Self {
				calls: RefCell::new(Vec::new()),
				result: Ok(()),
			}
		}

		fn failure(failure: InstallFailure) -> Self {
			Self {
				calls: RefCell::new(Vec::new()),
				result: Err(failure),
			}
		}
	}

	impl InstallBackend for RecordingBackend {
		fn install(&self, request: &ManagedInstallRequest) -> Result<(), InstallFailure> {
			self.calls.borrow_mut().push(request.clone());
			if self.result.is_ok() {
				write_executable(&request.executable_path);
			}
			self.result.clone()
		}
	}

	fn installer(workspace: &TestWorkspace, backend: RecordingBackend) -> ManagedInstaller<RecordingBackend> {
		ManagedInstaller::new(workspace.cache_root(), Logger::new(LogLevel::Info), backend).with_version("5.33.0")
	}

	fn current_inputs() -> InvalidationInputs {
		InvalidationInputs {
			package_json_mtime: Some(SystemTime::UNIX_EPOCH),
			lockfile_mtime: Some(SystemTime::UNIX_EPOCH),
			knip_config_mtime: Some(SystemTime::UNIX_EPOCH),
			settings_hash: 7,
		}
	}

	fn managed_cache(workspace: &TestWorkspace, executable_path: PathBuf) -> WorktreeCache {
		WorktreeCache {
			worktree_root: workspace.root.clone(),
			executable_path: Some(executable_path),
			package_manager: Some("pnpm".to_string()),
			config_path: Some(workspace.root.join("knip.json")),
			version: Some("5.33.0".to_string()),
			install_source: InstallSource::ManagedCache,
			last_error: None,
			invalidation_inputs: current_inputs(),
		}
	}

	fn write_executable(path: &Path) {
		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent).unwrap_or_else(|error| panic!("failed to create {}: {error}", parent.display()));
		}
		fs::write(path, "#!/usr/bin/env node\n")
			.unwrap_or_else(|error| panic!("failed to write {}: {error}", path.display()));
		#[cfg(unix)]
		{
			let mut permissions = fs::metadata(path)
				.unwrap_or_else(|error| panic!("failed to stat {}: {error}", path.display()))
				.permissions();
			permissions.set_mode(0o755);
			fs::set_permissions(path, permissions)
				.unwrap_or_else(|error| panic!("failed to chmod {}: {error}", path.display()));
		}
	}

	#[test]
	fn cache_hit_returns_cached_path_without_installing() {
		let workspace = TestWorkspace::new("cache-hit");
		let executable = workspace.executable("extension-cache/worktree/knip-language-server");
		let cache = managed_cache(&workspace, executable.clone());
		let backend = RecordingBackend::success();
		let installer = installer(&workspace, backend);

		let outcome = installer
			.resolve(&cache, current_inputs(), &KnipSettings::default(), PackageManager::Pnpm)
			.unwrap();

		assert_eq!(outcome.executable_path, executable);
		assert_eq!(outcome.status, ManagedInstallStatus::CacheHit);
		assert!(installer.backend.calls.borrow().is_empty());
	}

	#[test]
	fn cache_miss_installs_lazily_when_auto_install_enabled() {
		let workspace = TestWorkspace::new("cache-miss");
		let cache = WorktreeCache::new(workspace.root.clone());
		let backend = RecordingBackend::success();
		let installer = installer(&workspace, backend);

		let outcome = installer
			.resolve(&cache, current_inputs(), &KnipSettings::default(), PackageManager::Npm)
			.unwrap();

		assert!(outcome.executable_path.starts_with(workspace.cache_root()));
		assert_eq!(outcome.cache.install_source, InstallSource::ManagedCache);
		assert_eq!(outcome.cache.package_manager, Some("npm".to_string()));
		assert_eq!(installer.backend.calls.borrow().len(), 1);
	}

	#[test]
	fn cache_miss_does_not_install_when_auto_install_disabled() {
		let workspace = TestWorkspace::new("disabled");
		let cache = WorktreeCache::new(workspace.root.clone());
		let backend = RecordingBackend::success();
		let installer = installer(&workspace, backend);
		let settings = KnipSettings {
			auto_install: false,
			..KnipSettings::default()
		};

		let error = installer
			.resolve(&cache, current_inputs(), &settings, PackageManager::Npm)
			.unwrap_err();

		assert_eq!(
			error,
			KnipError::MissingKnip {
				workspace_root: workspace.root.clone()
			}
		);
		assert!(installer.backend.calls.borrow().is_empty());
	}

	#[test]
	fn stale_cache_reinstalls_and_records_stale_reason() {
		let workspace = TestWorkspace::new("stale");
		let executable = workspace.executable("old-cache/knip-language-server");
		let cache = managed_cache(&workspace, executable);
		let mut stale_inputs = current_inputs();
		stale_inputs.settings_hash = 99;
		let backend = RecordingBackend::success();
		let installer = installer(&workspace, backend);

		let outcome = installer
			.resolve(&cache, stale_inputs, &KnipSettings::default(), PackageManager::Yarn)
			.unwrap();

		assert_eq!(
			outcome.status,
			ManagedInstallStatus::Installed {
				previous_state: CacheState::Stale(StaleReason::SettingsChanged)
			}
		);
		assert_eq!(outcome.cache.package_manager, Some("yarn".to_string()));
		assert_eq!(installer.backend.calls.borrow().len(), 1);
	}

	#[test]
	fn corrupt_cache_is_cleared_and_retried() {
		let workspace = TestWorkspace::new("corrupt");
		let corrupt_dir = workspace.root.join("corrupt-cache");
		let corrupt_path = corrupt_dir.join("knip-language-server");
		fs::create_dir_all(&corrupt_dir).unwrap();
		fs::write(&corrupt_path, "not executable\n").unwrap();
		let cache = managed_cache(&workspace, corrupt_path);
		let backend = RecordingBackend::success();
		let installer = installer(&workspace, backend);

		let outcome = installer
			.resolve(&cache, current_inputs(), &KnipSettings::default(), PackageManager::Pnpm)
			.unwrap();

		assert!(!corrupt_dir.exists());
		assert_eq!(
			outcome.status,
			ManagedInstallStatus::Installed {
				previous_state: CacheState::Stale(StaleReason::Corrupt)
			}
		);
		assert_eq!(installer.backend.calls.borrow().len(), 1);
	}

	#[test]
	fn corrupt_cache_errors_when_auto_install_disabled() {
		let workspace = TestWorkspace::new("corrupt-disabled");
		let corrupt_path = workspace.root.join("corrupt-cache/knip-language-server");
		fs::create_dir_all(corrupt_path.parent().unwrap()).unwrap();
		fs::write(&corrupt_path, "not executable\n").unwrap();
		let cache = managed_cache(&workspace, corrupt_path);
		let backend = RecordingBackend::success();
		let installer = installer(&workspace, backend);
		let settings = KnipSettings {
			auto_install: false,
			..KnipSettings::default()
		};

		let error = installer
			.resolve(&cache, current_inputs(), &settings, PackageManager::Pnpm)
			.unwrap_err();

		assert!(matches!(error, KnipError::CorruptCache { .. }));
		assert!(installer.backend.calls.borrow().is_empty());
	}

	#[test]
	fn backend_read_only_cache_error_surfaces_cache_path() {
		let workspace = TestWorkspace::new("readonly");
		let cache = WorktreeCache::new(workspace.root.clone());
		let readonly = workspace.cache_root().join("readonly");
		let backend = RecordingBackend::failure(InstallFailure::ReadOnlyCache { path: readonly.clone() });
		let installer = installer(&workspace, backend);

		let error = installer
			.resolve(&cache, current_inputs(), &KnipSettings::default(), PackageManager::Pnpm)
			.unwrap_err();

		assert_eq!(error, KnipError::ReadOnlyCache { path: readonly });
	}

	#[test]
	fn offline_network_failure_surfaces_detail() {
		let workspace = TestWorkspace::new("offline");
		let cache = WorktreeCache::new(workspace.root.clone());
		let backend = RecordingBackend::failure(InstallFailure::NetworkUnavailable {
			detail: "offline mode is enabled".to_string(),
		});
		let installer = installer(&workspace, backend);

		let error = installer
			.resolve(&cache, current_inputs(), &KnipSettings::default(), PackageManager::Pnpm)
			.unwrap_err();

		assert_eq!(
			error,
			KnipError::NetworkUnavailable {
				detail: "offline mode is enabled".to_string()
			}
		);
	}

	#[test]
	fn corrupt_download_is_reported_and_cache_directory_removed() {
		let workspace = TestWorkspace::new("corrupt-download");
		let cache = WorktreeCache::new(workspace.root.clone());
		let backend = RecordingBackend::failure(InstallFailure::CorruptCache {
			detail: "checksum mismatch".to_string(),
		});
		let installer = installer(&workspace, backend);

		let error = installer
			.resolve(&cache, current_inputs(), &KnipSettings::default(), PackageManager::Pnpm)
			.unwrap_err();

		assert!(matches!(error, KnipError::CorruptCache { .. }));
		assert_eq!(installer.backend.calls.borrow().len(), 1);
	}

	#[test]
	fn cache_location_is_per_worktree_under_extension_cache_root() {
		let workspace = TestWorkspace::new("cache-location");
		let other = TestWorkspace::new("cache-location-other");
		let backend = RecordingBackend::success();
		let installer = installer(&workspace, backend);

		let cache_dir = installer.worktree_cache_dir(&workspace.root);
		let other_cache_dir = installer.worktree_cache_dir(&other.root);

		assert!(cache_dir.starts_with(workspace.cache_root()));
		assert!(other_cache_dir.starts_with(workspace.cache_root()));
		assert_ne!(cache_dir, other_cache_dir);
	}

	const EDITOR_WORKFLOW_SOURCE: &str = concat!(
		"import path from 'node:path';\n",
		"buildDiagnostics(issues, config, rules) {\n",
		"  for (const issue of Object.values(issuesForFile)) {\n",
		"          const document = this.documents.get(uri);\n",
		"          const diagnostic = issueToDiagnostic(issue, rules, config, document);\n",
		"  }\n",
		"}\n",
		"const options = await knip.createOptions({ cwd: this.cwd, isSession: true, args: { config: configFilePath } });\n",
	);

	#[test]
	fn managed_patch_adds_editor_workflow_support() {
		let result = apply_editor_workflow_patch(EDITOR_WORKFLOW_SOURCE)
			.expect("apply_editor_workflow_patch should not return Err on well-formed source");
		let patched = result.expect("should return Some for unpatched source");

		assert!(
			patched.contains(EDITOR_WORKFLOW_PATCH_MARKER),
			"patched source must contain the editor-workflow marker"
		);
		assert!(patched.contains("tsConfig"), "patched source must reference tsConfig");
		assert!(patched.contains("zedKnip"), "patched source must reference zedKnip");
		assert!(
			patched.contains("includeIssueTypes"),
			"patched source must contain includeIssueTypes filter"
		);
		assert!(
			patched.contains("excludeIssueTypes"),
			"patched source must contain excludeIssueTypes filter"
		);
		assert!(
			patched.contains("excludePathPrefixes"),
			"patched source must contain excludePathPrefixes filter"
		);
		assert!(
			patched.contains("severityByIssueType"),
			"patched source must contain severityByIssueType filter"
		);
		assert!(patched.contains("'off'"), "patched source must handle 'off' severity");
	}

	#[test]
	fn managed_patch_is_idempotent() {
		let first = apply_editor_workflow_patch(EDITOR_WORKFLOW_SOURCE)
			.expect("first application should not error")
			.expect("first application should return Some");

		let second = apply_editor_workflow_patch(&first).expect("second application should not error");

		assert!(
			second.is_none(),
			"second application must return None (idempotent); source already contains editor-workflow marker"
		);
	}

	#[test]
	fn managed_patch_upgrades_old_marker_only() {
		let source = concat!(
			"import path from 'node:path';\n",
			"// zed-knip: refresh Knip diagnostics on textDocument/didSave\n",
			"// (did-save patch content)\n",
			"          const document = this.documents.get(uri);\n",
			"          const diagnostic = issueToDiagnostic(issue, rules, config, document);\n",
			"args: { config: configFilePath }\n",
		);

		assert!(
			source.contains(DID_SAVE_REFRESH_PATCH_MARKER),
			"test source must contain the did-save marker"
		);
		assert!(
			!source.contains(EDITOR_WORKFLOW_PATCH_MARKER),
			"test source must NOT contain the editor-workflow marker"
		);

		let result =
			apply_editor_workflow_patch(source).expect("should not error on source with only old did-save marker");
		let patched = result.expect("should apply editor-workflow patch to old-marker-only source");

		assert!(
			patched.contains(EDITOR_WORKFLOW_PATCH_MARKER),
			"upgraded source must contain the editor-workflow marker"
		);
		assert!(
			patched.contains(DID_SAVE_REFRESH_PATCH_MARKER),
			"upgraded source must still contain the original did-save marker"
		);
	}

	#[test]
	fn managed_patch_reports_missing_editor_anchor() {
		let source = "// source with no editor-workflow patch anchors\n";

		let result = apply_editor_workflow_patch(source);

		assert!(result.is_err(), "expected Err when a patch anchor is missing");
		let err = result.unwrap_err();
		assert!(
			err.contains("editor-workflow"),
			"error message must name the editor-workflow anchor; got: {err}"
		);
	}
}
