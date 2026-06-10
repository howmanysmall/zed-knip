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
pub const PREPROCESSOR_PATCH_MARKER: &str = "// ZED-KNIP: preprocessor patch v1";
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
	let mut source = fs::read_to_string(&server_path).map_err(|error| KnipError::FailedManagedInstall {
		reason: format!("failed to read managed {LANGUAGE_SERVER_PACKAGE} server: {error}"),
	})?;
	let mut changed = false;

	if let Some(patched) = apply_did_save_refresh_patch(&source).map_err(|reason| KnipError::FailedManagedInstall {
		reason: format!("failed to patch managed {LANGUAGE_SERVER_PACKAGE} server: {reason}"),
	})? {
		source = patched;
		changed = true;
	}

	if let Some(patched) = apply_editor_workflow_patch(&source).map_err(|reason| KnipError::FailedManagedInstall {
		reason: format!("failed to patch managed {LANGUAGE_SERVER_PACKAGE} server: {reason}"),
	})? {
		source = patched;
		changed = true;
	}

	if source.contains("KNIP_CONFIG_LOCATIONS") || source.contains(PREPROCESSOR_PATCH_MARKER) {
		if let Some(patched) = apply_preprocessor_patch(&source).map_err(|reason| KnipError::FailedManagedInstall {
			reason: format!("failed to patch managed {LANGUAGE_SERVER_PACKAGE} server: {reason}"),
		})? {
			source = patched;
			changed = true;
		}
	}

	if changed {
		fs::write(&server_path, source).map_err(|error| KnipError::FailedManagedInstall {
			reason: format!("failed to write managed {LANGUAGE_SERVER_PACKAGE} server: {error}"),
		})?;
	}

	Ok(())
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
		"args: { config: configFilePath, ...(config?.zedKnip?.tsConfigFilePath ? { tsConfig: config.zedKnip.tsConfigFilePath } : {}) }",
		"createOptions editor-workflow",
	)?;

	let diag_replacement = format!(
		"          const document = this.documents.get(uri);\n          // {EDITOR_WORKFLOW_PATCH_MARKER}\n          if (config?.zedKnip?.diagnostics) {{\n            const _d = config.zedKnip.diagnostics;\n            const _inc = _d.includeIssueTypes ?? [];\n            const _exc = _d.excludeIssueTypes ?? [];\n            const _pfx = _d.excludePathPrefixes ?? [];\n            const _sev = _d.severityByIssueType ?? {{}};\n            if (_inc.length > 0 && !_inc.includes(issue.type)) continue;\n            if (_exc.includes(issue.type)) continue;\n            if (_pfx.length > 0) {{\n              const _rel = path.relative(this.cwd ?? process.cwd(), issue.filePath).split(path.sep).join('/');\n              if (_pfx.some(p => _rel === p || _rel.startsWith(p + '/'))) continue;\n            }}\n            if (_sev[issue.type] === 'off') continue;\n          }}\n          const diagnostic = issueToDiagnostic(issue, rules, config, document);\n          if (config?.zedKnip?.diagnostics) {{\n            const _s = (config.zedKnip.diagnostics.severityByIssueType ?? {{}})[issue.type];\n            if (_s && _s !== 'off') {{\n              const _map = {{ error: 1, warn: 2, info: 3, hint: 4 }};\n              if (_map[_s] !== undefined) diagnostic.severity = _map[_s];\n            }}\n          }}"
	);

	let patched = replace_once(
		&patched,
		"          const document = this.documents.get(uri);\n          const diagnostic = issueToDiagnostic(issue, rules, config, document);",
		&diag_replacement,
		"buildDiagnostics editor-workflow",
	)?;

	Ok(Some(patched))
}

pub fn apply_preprocessor_patch(source: &str) -> Result<Option<String>, String> {
	if source.contains(PREPROCESSOR_PATCH_MARKER) {
		return Ok(None);
	}

	let helpers = format!(
		"const RESTART_FOR = new Set(['package.json', ...KNIP_CONFIG_LOCATIONS]);\n\n{PREPROCESSOR_PATCH_MARKER}\nconst PREPROCESSOR_PATCH_FAILED_FINGERPRINT = '__zed_knip_preprocessor_failed__';\n\n/** @param {{ zedKnip?: {{ preprocessor?: unknown, preprocessorOptions?: unknown, preprocessorFingerprint?: unknown }} }} config */\nfunction normalizeZedKnipPreprocessorConfig(config) {{\n  const zedKnip = config?.zedKnip ?? {{}};\n  const preprocessors = Array.isArray(zedKnip.preprocessor)\n    ? zedKnip.preprocessor.filter(spec => typeof spec === 'string' && spec.length > 0)\n    : [];\n  const preprocessorOptions =\n    zedKnip.preprocessorOptions && typeof zedKnip.preprocessorOptions === 'object' && !Array.isArray(zedKnip.preprocessorOptions)\n      ? zedKnip.preprocessorOptions\n      : {{}};\n  const fingerprint = typeof zedKnip.preprocessorFingerprint === 'string'\n    ? zedKnip.preprocessorFingerprint\n    : `${{preprocessors.join('|')}}:${{JSON.stringify(Object.keys(preprocessorOptions).sort())}}`;\n  return {{ preprocessors, preprocessorOptions, fingerprint }};\n}}\n\n/** @param {{ kind: 'local' | 'package', url: string, spec: string }} resolved */\nasync function importZedKnipPreprocessor(resolved) {{\n  try {{\n    return await import(resolved.url);\n  }} catch (error) {{\n    throw new Error(`Failed to import preprocessor '${{resolved.spec}}' from ${{resolved.url}}: ${{error?.message ?? error}}`);\n  }}\n}}\n\n/** @param {{ cwd: string, spec: string }} input */\nfunction resolveZedKnipPreprocessor(input) {{\n  const {{ spec, cwd }} = input;\n  if (spec.startsWith('./')) {{\n    return {{ kind: 'local', url: pathToFileURL(path.resolve(cwd, spec)).href, spec }};\n  }}\n  const resolved = createRequire(path.join(cwd, 'package.json')).resolve(spec);\n  return {{ kind: 'package', url: pathToFileURL(resolved).href, spec }};\n}}\n\n/** @param {{ cwd: string, spec: string }} input */\nasync function loadZedKnipPreprocessor(input) {{\n  const resolved = resolveZedKnipPreprocessor(input);\n  const module = await importZedKnipPreprocessor(resolved);\n  const preprocessor = module.default ?? module;\n  if (typeof preprocessor !== 'function') {{\n    throw new Error(`Preprocessor '${{input.spec}}' must export a function`);\n  }}\n  return preprocessor;\n}}\n\nfunction buildZedKnipReporterOptions(rawResults, session, config, configFilePath) {{\n  return {{\n    report: {{}},\n    counters: {{ processed: 0, total: 0 }},\n    tagHints: new Set(),\n    configurationHints: [],\n    enabledPlugins: {{}},\n    isDisableConfigHints: false,\n    isDisableTagHints: false,\n    isTreatConfigHintsAsErrors: false,\n    isTreatTagHintsAsErrors: false,\n    isProduction: false,\n    isShowProgress: false,\n    options: '',\n    includedWorkspaceDirs: [],\n    selectedWorkspaces: undefined,\n    maxShowIssues: undefined,\n    ...(rawResults ?? {{}}),\n    issues: session.getIssues().issues,\n    cwd: this.cwd ?? process.cwd(),\n    preprocessorOptions: config?.zedKnip?.preprocessorOptions ?? {{}},\n    configFilePath,\n  }};\n}}\n\nasync function runZedKnipPreprocessors(preprocessors, reporterOptions, cwd) {{\n  let currentReporterOptions = reporterOptions;\n  for (const spec of preprocessors) {{\n    const preprocessor = await loadZedKnipPreprocessor({{ spec, cwd }});\n    const nextReporterOptions = await Promise.resolve(preprocessor(currentReporterOptions));\n    if (nextReporterOptions === null || nextReporterOptions === undefined || typeof nextReporterOptions !== 'object') {{\n      throw new Error(`Preprocessor '${{spec}}' must return a reporter options object`);\n    }}\n    if (!('issues' in nextReporterOptions)) {{\n      throw new Error(`Preprocessor '${{spec}}' must return reporter options with an issues field`);\n    }}\n    currentReporterOptions = nextReporterOptions;\n  }}\n  return currentReporterOptions;\n}}"
	);

	let mut patched = replace_preprocessor_anchor(
		source,
		"const RESTART_FOR = new Set(['package.json', ...KNIP_CONFIG_LOCATIONS]);",
		&helpers,
		"helpers",
	)?;
	patched = replace_preprocessor_anchor(
		&patched,
		"  /** @type {Map<string, import('vscode-languageserver').Diagnostic[]>} */\n  cycleDiagnostics = new Map();",
		"  /** @type {Map<string, import('vscode-languageserver').Diagnostic[]>} */\n  cycleDiagnostics = new Map();\n\n  zedKnipPreprocessorFingerprint = null;\n\n  zedKnipTransformedIssues = null;\n\n  zedKnipTransformedResults = null;",
		"class state",
	)?;
	patched = replace_preprocessor_anchor(
		&patched,
		"      const session = await knip.createSession(options);\n      this.connection.console.log(`Finished building module graph (${Date.now() - start}ms)`);\n\n      this.session = session;",
		"      const session = await knip.createSession(options);\n      this.connection.console.log(`Finished building module graph (${Date.now() - start}ms)`);\n\n      this.initConfig = config;\n      this.zedKnipPreprocessorFingerprint = null;\n      this.zedKnipTransformedIssues = null;\n      this.zedKnipTransformedResults = null;\n      const zedKnipPreprocessorConfig = normalizeZedKnipPreprocessorConfig(config);\n      if (zedKnipPreprocessorConfig.preprocessors.length > 0) {\n        try {\n          const reporterOptions = buildZedKnipReporterOptions.call(this, session.getResults(), session, config, configFilePath);\n          const output = await runZedKnipPreprocessors(\n            zedKnipPreprocessorConfig.preprocessors,\n            reporterOptions,\n            this.cwd ?? process.cwd()\n          );\n          this.zedKnipTransformedIssues = output.issues;\n          this.zedKnipTransformedResults = output;\n          this.zedKnipPreprocessorFingerprint = zedKnipPreprocessorConfig.fingerprint;\n        } catch (error) {\n          this.zedKnipPreprocessorFingerprint = PREPROCESSOR_PATCH_FAILED_FINGERPRINT;\n          this.zedKnipTransformedIssues = null;\n          this.zedKnipTransformedResults = null;\n          const message = `Knip preprocessor failed: ${error?.message ?? error}`;\n          this.connection.console.error(message);\n          if (this.connection.window?.showMessage) {\n            this.connection.window.showMessage(1, message);\n          }\n        }\n      }\n\n      this.session = session;",
		"start path",
	)?;
	patched = replace_preprocessor_anchor(
		&patched,
		"      this.publishDiagnostics(this.buildDiagnostics(session.getIssues().issues, config, this.rules));",
		"      const zedKnipDiagnosticsIssues = this.zedKnipTransformedIssues !== null\n        ? this.zedKnipTransformedIssues\n        : zedKnipPreprocessorConfig.preprocessors.length > 0\n          ? {}\n          : session.getIssues().issues;\n      this.publishDiagnostics(this.buildDiagnostics(zedKnipDiagnosticsIssues, config, this.rules));",
		"diagnostic publish path",
	)?;
	patched = replace_preprocessor_anchor(
		&patched,
		"  getResults() {\n    if (!this.session) return null;\n    return this.session.getResults();\n  }",
		"  getResults() {\n    if (!this.session) return null;\n    if (this.zedKnipTransformedResults !== null) return this.zedKnipTransformedResults;\n    const { preprocessors } = normalizeZedKnipPreprocessorConfig(this.initConfig ?? {});\n    if (preprocessors.length > 0 || this.zedKnipPreprocessorFingerprint === PREPROCESSOR_PATCH_FAILED_FINGERPRINT) return null;\n    return this.session.getResults();\n  }",
		"results path",
	)?;
	patched = replace_preprocessor_anchor(
		&patched,
		"      const result = await this.session.handleFileChanges(changes);\n\n      if (!result) return;\n\n      this.connection.console.log(\n        `Module graph updated (${Math.floor(result.duration)}ms • ${(result.mem / 1024 / 1024).toFixed(2)}M)`\n      );\n\n      const config = await this.getConfig();\n      this.publishDiagnostics(this.buildDiagnostics(this.session.getIssues().issues, config, this.rules));",
		"      const result = await this.session.handleFileChanges(changes);\n\n      const config = await this.getConfig();\n      this.initConfig = config;\n      const zedKnipPreprocessorConfig = normalizeZedKnipPreprocessorConfig(config);\n      if (this.zedKnipPreprocessorFingerprint !== zedKnipPreprocessorConfig.fingerprint) {\n        this.zedKnipPreprocessorFingerprint = null;\n        this.zedKnipTransformedIssues = null;\n        this.zedKnipTransformedResults = null;\n      }\n      if (zedKnipPreprocessorConfig.preprocessors.length > 0 && result) {\n        try {\n          const reporterOptions = buildZedKnipReporterOptions.call(this, this.session.getResults(), this.session, config, config.configFilePath);\n          const output = await runZedKnipPreprocessors(\n            zedKnipPreprocessorConfig.preprocessors,\n            reporterOptions,\n            this.cwd ?? process.cwd()\n          );\n          this.zedKnipTransformedIssues = output.issues;\n          this.zedKnipTransformedResults = output;\n          this.zedKnipPreprocessorFingerprint = zedKnipPreprocessorConfig.fingerprint;\n        } catch (error) {\n          this.zedKnipPreprocessorFingerprint = PREPROCESSOR_PATCH_FAILED_FINGERPRINT;\n          this.zedKnipTransformedIssues = null;\n          this.zedKnipTransformedResults = null;\n          const message = `Knip preprocessor failed after file changes: ${error?.message ?? error}`;\n          this.connection.console.error(message);\n          if (this.connection.window?.showMessage) {\n            this.connection.window.showMessage(1, message);\n          }\n          this.publishDiagnostics(new Map());\n          return null;\n        }\n      }\n\n      if (!result) return;\n\n      this.connection.console.log(\n        `Module graph updated (${Math.floor(result.duration)}ms • ${(result.mem / 1024 / 1024).toFixed(2)}M)`\n      );\n\n      const zedKnipDiagnosticsIssues = this.zedKnipTransformedIssues !== null\n        ? this.zedKnipTransformedIssues\n        : zedKnipPreprocessorConfig.preprocessors.length > 0\n          ? {}\n          : this.session.getIssues().issues;\n      this.publishDiagnostics(this.buildDiagnostics(zedKnipDiagnosticsIssues, config, this.rules));",
		"file-change path",
	)?;

	Ok(Some(patched))
}

fn replace_preprocessor_anchor(source: &str, needle: &str, replacement: &str, label: &str) -> Result<String, String> {
	if !source.contains(needle) {
		return Err(format!("missing preprocessor patch anchor: {label}"));
	}

	Ok(source.replacen(needle, replacement, 1))
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
	use std::{cell::RefCell, os::unix::fs::symlink, os::unix::fs::PermissionsExt, time::SystemTime};

	const MANAGED_SERVER_FIXTURE: &str =
		include_str!("../tests/fixtures/managed-server/knip-language-server-server.js");

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
		assert!(
			patched.contains("diagnostic.severity"),
			"patched source must override diagnostic.severity based on severityByIssueType"
		);
	}

	#[test]
	#[allow(non_snake_case)]
	fn managed_patch_passes_tsConfig_relative_unchanged() {
		let result = apply_editor_workflow_patch(EDITOR_WORKFLOW_SOURCE)
			.expect("apply_editor_workflow_patch should not return Err on well-formed source");
		let patched = result.expect("should return Some for unpatched source");

		assert!(
			patched.contains("tsConfig: config.zedKnip.tsConfigFilePath"),
			"patched source must pass the relative configured tsConfig value unchanged so upstream join(dir, options.tsConfigFile) resolves it"
		);
		assert!(
			!patched.contains("path.resolve(this.cwd"),
			"patched source must NOT resolve tsConfig to an absolute path; upstream CLI --tsConfig semantics expect a relative value"
		);
	}

	#[test]
	fn managed_patch_severity_overrides_lsp_diagnostic_severity() {
		let result = apply_editor_workflow_patch(EDITOR_WORKFLOW_SOURCE)
			.expect("apply_editor_workflow_patch should not return Err on well-formed source");
		let patched = result.expect("should return Some for unpatched source");

		assert!(
			patched.contains("diagnostic.severity = _map[_s]"),
			"patched source must assign diagnostic.severity from the severity map"
		);
		assert!(
			patched.contains("error: 1, warn: 2, info: 3, hint: 4"),
			"patched source must map error=1, warn=2, info=3, hint=4 to LSP DiagnosticSeverity"
		);
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

	fn remove_fixture_anchor(anchor: &str) -> String {
		MANAGED_SERVER_FIXTURE.replacen(anchor, "// removed preprocessor test anchor", 1)
	}

	#[test]
	fn managed_preprocessor_fixture_header_records_source_date_and_version() {
		assert!(
			MANAGED_SERVER_FIXTURE.starts_with(
				"/*\n * Source URL: https://raw.githubusercontent.com/webpro-nl/knip/main/packages/language-server/src/server.js\n * Fetch date: 2026-06-10\n * Observed @knip/language-server version: 3.0.3"
			),
			"managed server fixture must start with source/date/version header"
		);
	}

	#[test]
	fn managed_preprocessor_patch_is_idempotent() {
		let first = apply_preprocessor_patch(MANAGED_SERVER_FIXTURE)
			.expect("first preprocessor patch should not error")
			.expect("first preprocessor patch should return Some");

		let second = apply_preprocessor_patch(&first).expect("second preprocessor patch should not error");
		let marker_count = first.matches(PREPROCESSOR_PATCH_MARKER).count();

		assert!(second.is_none(), "second patch application must be idempotent");
		assert_eq!(
			marker_count, 1,
			"patched source must contain exactly one preprocessor marker"
		);
	}

	#[test]
	fn managed_preprocessor_patch_reports_missing_anchors() {
		let anchors = [
			(
				"helpers",
				"const RESTART_FOR = new Set(['package.json', ...KNIP_CONFIG_LOCATIONS]);",
			),
			(
				"class state",
				"  /** @type {Map<string, import('vscode-languageserver').Diagnostic[]>} */\n  cycleDiagnostics = new Map();",
			),
			(
				"start path",
				"      const session = await knip.createSession(options);\n      this.connection.console.log(`Finished building module graph (${Date.now() - start}ms)`);\n\n      this.session = session;",
			),
			(
				"diagnostic publish path",
				"      this.publishDiagnostics(this.buildDiagnostics(session.getIssues().issues, config, this.rules));",
			),
			(
				"results path",
				"  getResults() {\n    if (!this.session) return null;\n    return this.session.getResults();\n  }",
			),
			(
				"file-change path",
				"      const result = await this.session.handleFileChanges(changes);\n\n      if (!result) return;\n\n      this.connection.console.log(\n        `Module graph updated (${Math.floor(result.duration)}ms • ${(result.mem / 1024 / 1024).toFixed(2)}M)`\n      );\n\n      const config = await this.getConfig();\n      this.publishDiagnostics(this.buildDiagnostics(this.session.getIssues().issues, config, this.rules));",
			),
		];

		for (label, anchor) in anchors {
			let source = remove_fixture_anchor(anchor);
			let error = apply_preprocessor_patch(&source).unwrap_err();
			assert_eq!(
				error,
				format!("missing preprocessor patch anchor: {label}"),
				"missing anchor error must name {label}"
			);
		}
	}

	#[test]
	fn managed_preprocessor_patch_represents_sequential_transforms() {
		let patched = apply_preprocessor_patch(MANAGED_SERVER_FIXTURE)
			.expect("preprocessor patch should not error")
			.expect("preprocessor patch should apply");

		assert!(
			patched.contains("for (const spec of preprocessors)")
				&& patched.contains("await Promise.resolve(preprocessor(currentReporterOptions))")
				&& patched.contains("currentReporterOptions = nextReporterOptions"),
			"patched source must run preprocessors sequentially and await sync/async outputs"
		);
	}

	#[test]
	fn managed_preprocessor_patch_fail_closed_logic_is_present() {
		let patched = apply_preprocessor_patch(MANAGED_SERVER_FIXTURE)
			.expect("preprocessor patch should not error")
			.expect("preprocessor patch should apply");

		assert!(patched.contains("this.zedKnipPreprocessorFingerprint = PREPROCESSOR_PATCH_FAILED_FINGERPRINT"));
		assert!(patched.contains("this.zedKnipTransformedIssues = null"));
		assert!(patched.contains("this.zedKnipTransformedResults = null"));
		assert!(patched.contains("this.publishDiagnostics(new Map())"));
		assert!(patched.contains("return null"));
		assert!(
			patched.contains("if (preprocessors.length > 0 || this.zedKnipPreprocessorFingerprint === PREPROCESSOR_PATCH_FAILED_FINGERPRINT) return null"),
			"REQUEST_RESULTS must return null after configured/failing preprocessors instead of raw results"
		);
	}

	#[test]
	fn managed_preprocessor_patch_defaults_reporter_options() {
		let patched = apply_preprocessor_patch(MANAGED_SERVER_FIXTURE)
			.expect("preprocessor patch should not error")
			.expect("preprocessor patch should apply");

		for required_default in [
			"report: {}",
			"counters: { processed: 0, total: 0 }",
			"tagHints: new Set()",
			"configurationHints: []",
			"enabledPlugins: {}",
			"isDisableConfigHints: false",
			"isDisableTagHints: false",
			"isTreatConfigHintsAsErrors: false",
			"isTreatTagHintsAsErrors: false",
			"isProduction: false",
			"isShowProgress: false",
			"options: ''",
			"includedWorkspaceDirs: []",
			"selectedWorkspaces: undefined",
			"maxShowIssues: undefined",
			"preprocessorOptions: config?.zedKnip?.preprocessorOptions ?? {}",
		] {
			assert!(
				patched.contains(required_default),
				"missing reporter default {required_default}"
			);
		}
	}

	#[test]
	fn managed_preprocessor_patch_orchestration_is_all_or_nothing() {
		let workspace = TestWorkspace::new("preprocessor-all-or-nothing");
		let package_dir = workspace.root.join("package");
		let bin_dir = workspace.root.join("bin");
		fs::create_dir_all(&package_dir).unwrap();
		fs::create_dir_all(&bin_dir).unwrap();
		let cli_path = package_dir.join("cli.js");
		let server_path = package_dir.join("server.js");
		let executable_path = bin_dir.join("knip-language-server");
		fs::write(&cli_path, "#!/usr/bin/env node\n").unwrap();
		symlink(&cli_path, &executable_path).unwrap();
		let unsupported_source = remove_fixture_anchor(
			"      this.publishDiagnostics(this.buildDiagnostics(session.getIssues().issues, config, this.rules));",
		);
		fs::write(&server_path, &unsupported_source).unwrap();

		let error = patch_managed_language_server(&executable_path).unwrap_err();
		let after = fs::read_to_string(&server_path).unwrap();

		assert!(
			matches!(error, KnipError::FailedManagedInstall { reason } if reason.contains("missing preprocessor patch anchor: diagnostic publish path")),
			"orchestration error must name the missing preprocessor anchor"
		);
		assert_eq!(
			after, unsupported_source,
			"server.js must remain unchanged after patch-stage failure"
		);
	}

	// ============================================================
	// Task 7: result consistency + settings-change invalidation
	// ============================================================
	// These source-level assertions guard the invariants that:
	//   1. diagnostics and `REQUEST_RESULTS` read from the SAME
	//      transformed state slot (`zedKnipTransformedIssues` /
	//      `zedKnipTransformedResults`) so a transformed result can
	//      never be served from one path while stale on the other,
	//   2. `start()`, `handleFileChanges()`, and `getResults()` each
	//      compare the active fingerprint before serving transformed
	//      state, and on mismatch clear and recompute the state.
	//
	// The patched JS is the only place these invariants live, so
	// pinning the exact text is the only way to keep them honest
	// against accidental refactors that would split the storage.

	fn patched_preprocessor_source() -> String {
		apply_preprocessor_patch(MANAGED_SERVER_FIXTURE)
			.expect("preprocessor patch must not error on pinned fixture")
			.expect("pinned fixture must be unpatched")
	}

	#[test]
	fn managed_preprocessor_results_consistency_diagnostics_use_same_state_as_get_results() {
		let patched = patched_preprocessor_source();

		assert!(
			patched.contains("zedKnipTransformedIssues"),
			"diagnostic publish path must read from zedKnipTransformedIssues"
		);
		assert!(
			patched.contains("zedKnipTransformedResults"),
			"getResults() must return zedKnipTransformedResults"
		);
		assert!(
			patched.contains(
				"this.publishDiagnostics(this.buildDiagnostics(zedKnipDiagnosticsIssues, config, this.rules))"
			),
			"diagnostic publish path must route through zedKnipDiagnosticsIssues"
		);
		assert!(
			patched.contains("const zedKnipDiagnosticsIssues = this.zedKnipTransformedIssues !== null\n        ? this.zedKnipTransformedIssues\n        : zedKnipPreprocessorConfig.preprocessors.length > 0\n          ? {}\n          : session.getIssues().issues;"),
			"diagnostic publish path must read zedKnipTransformedIssues with the empty-object fallback for configured-but-untransformed preprocessors"
		);
		assert!(
			patched.contains("if (this.zedKnipTransformedResults !== null) return this.zedKnipTransformedResults"),
			"getResults() must return the transformed slot when populated"
		);
	}

	#[test]
	fn managed_preprocessor_results_consistency_handle_file_changes_uses_same_state() {
		let patched = patched_preprocessor_source();

		let file_change_block = patched
			.split("handleFileChanges")
			.nth(1)
			.expect("patched source must contain handleFileChanges");

		assert!(
			file_change_block.contains("this.zedKnipTransformedIssues = output.issues"),
			"handleFileChanges() must assign output.issues to this.zedKnipTransformedIssues"
		);
		assert!(
			file_change_block.contains("this.zedKnipTransformedResults = output"),
			"handleFileChanges() must assign output to this.zedKnipTransformedResults"
		);
		assert!(
			file_change_block.contains("this.zedKnipPreprocessorFingerprint = zedKnipPreprocessorConfig.fingerprint"),
			"handleFileChanges() must record the post-transform fingerprint"
		);
		assert!(
			file_change_block.contains("zedKnipDiagnosticsIssues = this.zedKnipTransformedIssues !== null"),
			"handleFileChanges() must read transformed issues for its diagnostic publish"
		);
	}

	#[test]
	fn managed_preprocessor_results_consistency_fail_closed_clears_all_state() {
		let patched = patched_preprocessor_source();

		assert!(
			patched.contains("this.zedKnipPreprocessorFingerprint = PREPROCESSOR_PATCH_FAILED_FINGERPRINT")
				&& patched.contains("this.zedKnipTransformedIssues = null")
				&& patched.contains("this.zedKnipTransformedResults = null"),
			"fail-closed branch must clear fingerprint + transformed issues + transformed results"
		);
	}

	#[test]
	fn managed_preprocessor_fingerprint_invalidation_start_path_clears_state_on_each_invocation() {
		let patched = patched_preprocessor_source();

		let start_block = patched
			.split("async start()")
			.nth(1)
			.expect("patched source must contain start()");
		assert!(
			start_block.contains("this.zedKnipPreprocessorFingerprint = null")
				&& start_block.contains("this.zedKnipTransformedIssues = null")
				&& start_block.contains("this.zedKnipTransformedResults = null"),
			"start() must clear fingerprint + transformed state on each invocation"
		);
	}

	#[test]
	fn managed_preprocessor_fingerprint_invalidation_handle_file_changes_compares_and_clears() {
		let patched = patched_preprocessor_source();

		assert!(
			patched.contains("const zedKnipPreprocessorConfig = normalizeZedKnipPreprocessorConfig(config);"),
			"handleFileChanges() must normalize the active config to derive the fingerprint"
		);
		assert!(
			patched.contains(
				"if (this.zedKnipPreprocessorFingerprint !== zedKnipPreprocessorConfig.fingerprint) {\n        this.zedKnipPreprocessorFingerprint = null;\n        this.zedKnipTransformedIssues = null;\n        this.zedKnipTransformedResults = null;\n      }"
			),
			"handleFileChanges() must clear transformed state when the active fingerprint diverges from the recorded one"
		);
	}

	#[test]
	fn managed_preprocessor_fingerprint_invalidation_get_results_returns_null_after_mismatch() {
		let patched = patched_preprocessor_source();

		assert!(
			patched.contains(
				"if (preprocessors.length > 0 || this.zedKnipPreprocessorFingerprint === PREPROCESSOR_PATCH_FAILED_FINGERPRINT) return null"
			),
			"getResults() must return null after a failed-fingerprint sentinel OR when preprocessors are still configured"
		);
	}

	#[test]
	fn managed_preprocessor_fingerprint_deterministic_string_is_documented_in_patch() {
		let patched = patched_preprocessor_source();

		assert!(
			patched.contains(
				"const fingerprint = typeof zedKnip.preprocessorFingerprint === 'string'\n    ? zedKnip.preprocessorFingerprint\n    : `${preprocessors.join('|')}:${JSON.stringify(Object.keys(preprocessorOptions).sort())}`"
			),
			"normalizeZedKnipPreprocessorConfig() must build the fingerprint from ordered join + sorted option keys"
		);
	}
}
