// Failure-mode integration tests for zed-knip.
//
// Each test exercises a distinct failure scenario using real fixtures from
// `tests/fixtures/` and the production error types from `src/errors.rs`.
// No actual network calls are made; network failures are simulated via the
// `ManagedInstall` mock seam in `src/managed_install.rs`.
//
// Run with:
//   mise x -- cargo nextest run -E 'test(failure)'

mod fixtures {
	use std::path::PathBuf;

	pub fn fixture_path(name: &str) -> PathBuf {
		PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.join("tests")
			.join("fixtures")
			.join(name)
	}
}

use std::{
	fs,
	path::{Path, PathBuf},
	time::{SystemTime, UNIX_EPOCH},
};

use zed_knip::{
	cache::{InstallSource, WorktreeCache},
	errors::KnipError,
	managed_install::{ManagedInstall, ManagedInstallDisabled},
	package_manager::PackageManager,
	resolver::resolve_knip,
	settings::KnipSettings,
};

/// Temporary workspace that cleans itself up on drop.
struct TempWorkspace {
	root: PathBuf,
}

impl TempWorkspace {
	fn new(label: &str) -> Self {
		let nanos = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.unwrap_or_default()
			.as_nanos();
		let root = std::env::temp_dir().join(format!("zed-knip-failure-{label}-{nanos}"));
		fs::create_dir_all(&root).unwrap_or_else(|e| panic!("failed to create temp dir {}: {e}", root.display()));
		Self { root }
	}

	fn write(&self, rel: &str, content: &str) -> PathBuf {
		let path = self.root.join(rel);
		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent).unwrap_or_else(|e| panic!("failed to create {}: {e}", parent.display()));
		}
		fs::write(&path, content).unwrap_or_else(|e| panic!("failed to write {}: {e}", path.display()));
		path
	}

	fn executable(&self, rel: &str) -> PathBuf {
		let path = self.write(rel, "#!/usr/bin/env node\n");
		make_executable(&path).unwrap_or_else(|e| panic!("failed to chmod {}: {e}", path.display()));
		path
	}
}

impl Drop for TempWorkspace {
	fn drop(&mut self) {
		let _ = fs::remove_dir_all(&self.root);
	}
}

fn make_executable(path: &Path) -> std::io::Result<()> {
	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		let mut perms = fs::metadata(path)?.permissions();
		perms.set_mode(0o755);
		fs::set_permissions(path, perms)
	}
	#[cfg(not(unix))]
	{
		let _ = path;
		Ok(())
	}
}

fn empty_cache(root: &Path) -> WorktreeCache {
	WorktreeCache::new(root.to_path_buf())
}

/// Mock installer that always returns the given result.
#[derive(Debug, Clone)]
struct MockInstall {
	result: Result<PathBuf, KnipError>,
}

impl ManagedInstall for MockInstall {
	fn install(&self, _root: &Path, _pm: PackageManager) -> Result<PathBuf, KnipError> {
		self.result.clone()
	}
}

fn failing_install(error: KnipError) -> MockInstall {
	MockInstall { result: Err(error) }
}

/// When no Knip binary exists in the workspace and auto-install is disabled,
/// the resolver must return `MissingKnip` with the workspace root path.
#[test]
fn failure_missing_knip_binary_returns_actionable_error() {
	let fixture = fixtures::fixture_path("missing-knip");

	let settings = KnipSettings {
		auto_install: false,
		..KnipSettings::default()
	};
	let cache = empty_cache(&fixture);

	let error = resolve_knip(&settings, &cache, &ManagedInstallDisabled).unwrap_err();

	assert_eq!(
		error,
		KnipError::MissingKnip {
			workspace_root: fixture.clone()
		},
		"expected MissingKnip for fixture {fixture:?}"
	);

	let message = error.to_string();
	assert!(
		message.contains(fixture.to_string_lossy().as_ref()),
		"error message must contain workspace path: {message}"
	);
	assert!(
		message.to_lowercase().contains("install"),
		"error message must mention install instructions: {message}"
	);
}

/// The `missing-knip` fixture has a `package-lock.json` (npm) but no binary.
/// The error message must reference the workspace root, not a generic path.
#[test]
fn failure_missing_knip_error_message_contains_workspace_root() {
	let fixture = fixtures::fixture_path("missing-knip");
	let settings = KnipSettings {
		auto_install: false,
		..KnipSettings::default()
	};

	let error = resolve_knip(&settings, &empty_cache(&fixture), &ManagedInstallDisabled).unwrap_err();

	let message = error.to_string();
	assert!(
		message.contains(fixture.to_string_lossy().as_ref()),
		"error must contain workspace root path; got: {message}"
	);
}

/// When the user points to an invalid config file via settings, the resolver
/// must surface `InvalidConfig` with the config path.
#[test]
fn failure_invalid_config_error_contains_config_path() {
	let fixture = fixtures::fixture_path("invalid-config");
	let config_path = fixture.join("knip.json");

	let error = KnipError::InvalidConfig {
		path: config_path.clone(),
		reason: "unexpected token at line 1".to_string(),
	};

	let message = error.to_string();
	assert!(
		message.contains(config_path.to_string_lossy().as_ref()),
		"error must contain config path; got: {message}"
	);
	assert!(
		message.to_lowercase().contains("fix"),
		"error must mention remediation; got: {message}"
	);
}

/// The `invalid-config` fixture contains a syntactically broken `knip.json`.
/// Constructing the error with that path must produce a user-facing message
/// that names the file and tells the user to fix it.
#[test]
fn failure_invalid_config_fixture_path_is_named_in_error() {
	let fixture = fixtures::fixture_path("invalid-config");
	let config_path = fixture.join("knip.json");

	assert!(config_path.is_file(), "fixture knip.json must exist at {config_path:?}");

	let error = KnipError::InvalidConfig {
		path: config_path.clone(),
		reason: "JSON parse error".to_string(),
	};

	let message = error.to_string();
	assert!(
		message.contains("knip.json"),
		"message must name the config file: {message}"
	);
	assert!(
		message.contains("JSON parse error"),
		"message must include reason: {message}"
	);
}

/// `WorktreeCache::mark_corrupt` must produce a `CorruptCache` error whose
/// message contains the cache path and instructs the user to delete it.
#[test]
fn failure_corrupt_cache_error_contains_path_and_remediation() {
	let workspace = TempWorkspace::new("corrupt-cache");
	let cache = WorktreeCache {
		worktree_root: workspace.root.clone(),
		executable_path: Some(workspace.root.join("cache/knip")),
		package_manager: Some("npm".to_string()),
		config_path: Some(workspace.root.join("knip.json")),
		version: Some("5.0.0".to_string()),
		install_source: InstallSource::ManagedCache,
		last_error: None,
		invalidation_inputs: Default::default(),
	};

	let error = cache.mark_corrupt();

	assert_eq!(
		error,
		KnipError::CorruptCache {
			path: workspace.root.clone(),
			detail: "Cache contents failed validation".to_string(),
		}
	);

	let message = error.to_string();
	assert!(
		message.contains(workspace.root.to_string_lossy().as_ref()),
		"error must contain cache path; got: {message}"
	);
	assert!(
		message.to_lowercase().contains("delete"),
		"error must instruct user to delete cache; got: {message}"
	);
}

/// `CorruptCache` constructed directly must include the detail string.
#[test]
fn failure_corrupt_cache_detail_appears_in_message() {
	let path = PathBuf::from("/tmp/zed-knip-cache");
	let detail = "checksum mismatch for knip-language-server";

	let error = KnipError::CorruptCache {
		path: path.clone(),
		detail: detail.to_string(),
	};

	let message = error.to_string();
	assert!(
		message.contains(detail),
		"error must contain detail string; got: {message}"
	);
	assert!(
		message.contains(path.to_string_lossy().as_ref()),
		"error must contain path; got: {message}"
	);
}

/// `WorktreeCache::mark_read_only` must produce a `ReadOnlyCache` error whose
/// message contains the path and tells the user to make it writable.
#[test]
fn failure_read_only_cache_error_contains_path_and_remediation() {
	let cache_path = PathBuf::from("/var/cache/zed-knip");

	let error = WorktreeCache::mark_read_only(cache_path.clone());

	assert_eq!(
		error,
		KnipError::ReadOnlyCache {
			path: cache_path.clone()
		}
	);

	let message = error.to_string();
	assert!(
		message.contains(cache_path.to_string_lossy().as_ref()),
		"error must contain cache path; got: {message}"
	);
	assert!(
		message.to_lowercase().contains("writable"),
		"error must mention making the path writable; got: {message}"
	);
}

/// A read-only cache path that contains spaces must still appear verbatim.
#[test]
fn failure_read_only_cache_path_with_spaces_preserved_in_message() {
	let cache_path = PathBuf::from("/home/user/my cache/zed-knip");

	let error = WorktreeCache::mark_read_only(cache_path.clone());
	let message = error.to_string();

	assert!(
		message.contains("my cache"),
		"path with spaces must appear verbatim in error; got: {message}"
	);
}

/// A crash with a known exit code must name the code in the error message.
#[test]
fn failure_language_server_crash_with_exit_code_names_code() {
	let error = KnipError::LanguageServerCrash { exit_code: Some(137) };
	let message = error.to_string();

	assert!(
		message.contains("137"),
		"error must contain exit code 137; got: {message}"
	);
	assert!(
		message.to_lowercase().contains("restart"),
		"error must mention restarting; got: {message}"
	);
}

/// A crash without an exit code (e.g. signal kill) must still be actionable.
#[test]
fn failure_language_server_crash_without_exit_code_is_actionable() {
	let error = KnipError::LanguageServerCrash { exit_code: None };
	let message = error.to_string();

	assert!(
		!message.is_empty(),
		"crash error without exit code must produce a non-empty message"
	);
	assert!(
		message.to_lowercase().contains("restart"),
		"error must mention restarting; got: {message}"
	);
}

/// Exit code 1 (generic failure) must be distinguishable from no exit code.
#[test]
fn failure_language_server_crash_exit_code_one_is_distinct_from_none() {
	let with_code = KnipError::LanguageServerCrash { exit_code: Some(1) }.to_string();
	let without_code = KnipError::LanguageServerCrash { exit_code: None }.to_string();

	assert_ne!(
		with_code, without_code,
		"crash with exit code 1 must differ from crash with no exit code"
	);
	assert!(
		with_code.contains("1"),
		"message with code must contain '1'; got: {with_code}"
	);
}

/// When the managed installer fails with `NetworkUnavailable`, the resolver
/// must surface that error unchanged so the user sees the network detail.
#[test]
fn failure_offline_network_error_surfaces_detail() {
	let workspace = TempWorkspace::new("offline");
	workspace.write("package.json", r#"{"packageManager":"npm@10.0.0"}"#);

	let installer = failing_install(KnipError::NetworkUnavailable {
		detail: "proxy returned 407 Proxy Authentication Required".to_string(),
	});

	let error = resolve_knip(&KnipSettings::default(), &empty_cache(&workspace.root), &installer).unwrap_err();

	assert_eq!(
		error,
		KnipError::NetworkUnavailable {
			detail: "proxy returned 407 Proxy Authentication Required".to_string()
		}
	);

	let message = error.to_string();
	assert!(
		message.contains("proxy returned 407"),
		"error must contain network detail; got: {message}"
	);
	assert!(
		message.to_lowercase().contains("connection"),
		"error must mention checking connection; got: {message}"
	);
}

/// A generic "offline" detail must also be surfaced correctly.
#[test]
fn failure_offline_generic_detail_is_preserved() {
	let workspace = TempWorkspace::new("offline-generic");
	workspace.write("package.json", r#"{"packageManager":"pnpm@8.0.0"}"#);

	let installer = failing_install(KnipError::NetworkUnavailable {
		detail: "offline".to_string(),
	});

	let error = resolve_knip(&KnipSettings::default(), &empty_cache(&workspace.root), &installer).unwrap_err();

	let message = error.to_string();
	assert!(
		message.contains("offline"),
		"error must contain 'offline'; got: {message}"
	);
}

/// The `multiple-lockfiles` fixture has both `package-lock.json` and
/// `yarn.lock`. Detection must return `AmbiguousPackageManager` listing both.
#[test]
fn failure_ambiguous_package_manager_lists_conflicting_lockfiles() {
	let fixture = fixtures::fixture_path("multiple-lockfiles");

	let settings = KnipSettings {
		auto_install: false,
		..KnipSettings::default()
	};

	let error = resolve_knip(&settings, &empty_cache(&fixture), &ManagedInstallDisabled).unwrap_err();

	assert_eq!(
		error,
		KnipError::AmbiguousPackageManager {
			found: vec!["package-lock.json".to_string(), "yarn.lock".to_string()]
		},
		"expected AmbiguousPackageManager for multiple-lockfiles fixture"
	);

	let message = error.to_string();
	assert!(
		message.contains("package-lock.json"),
		"error must list package-lock.json; got: {message}"
	);
	assert!(
		message.contains("yarn.lock"),
		"error must list yarn.lock; got: {message}"
	);
}

/// The ambiguous error message must tell the user how to resolve the conflict.
#[test]
fn failure_ambiguous_package_manager_error_is_actionable() {
	let fixture = fixtures::fixture_path("multiple-lockfiles");

	let settings = KnipSettings {
		auto_install: false,
		..KnipSettings::default()
	};

	let error = resolve_knip(&settings, &empty_cache(&fixture), &ManagedInstallDisabled).unwrap_err();
	let message = error.to_string();

	assert!(
		message.to_lowercase().contains("remove") || message.to_lowercase().contains("set"),
		"error must suggest removing extra lockfile or setting package manager explicitly; got: {message}"
	);
}

/// A settings override must bypass ambiguous detection entirely.
#[test]
fn failure_ambiguous_package_manager_resolved_by_settings_override() {
	let fixture = fixtures::fixture_path("multiple-lockfiles");
	let workspace = TempWorkspace::new("ambiguous-override");

	// Copy the fixture lockfiles into a temp workspace so we can add a binary.
	for name in ["package.json", "package-lock.json", "yarn.lock"] {
		let src = fixture.join(name);
		let dst = workspace.root.join(name);
		fs::copy(&src, &dst).unwrap_or_else(|e| panic!("failed to copy {name}: {e}"));
	}
	workspace.executable("node_modules/.bin/knip-language-server");

	let settings = KnipSettings {
		package_manager: Some("npm".to_string()),
		..KnipSettings::default()
	};

	// With an explicit override the resolver must succeed.
	let resolved = resolve_knip(&settings, &empty_cache(&workspace.root), &ManagedInstallDisabled).unwrap();

	assert_eq!(resolved.package_manager, PackageManager::Npm);
}

/// Every `KnipError` variant must produce a non-empty, non-whitespace message.
#[test]
fn failure_all_error_variants_produce_non_empty_messages() {
	let variants: &[KnipError] = &[
		KnipError::MissingKnip {
			workspace_root: PathBuf::from("/workspace"),
		},
		KnipError::InvalidExplicitPath {
			path: PathBuf::from("/workspace/bin/knip"),
		},
		KnipError::NonExecutablePath {
			path: PathBuf::from("/workspace/bin/knip"),
		},
		KnipError::UnsupportedPackageManager {
			found: "rush".to_string(),
		},
		KnipError::FailedManagedInstall {
			reason: "download timed out".to_string(),
		},
		KnipError::NetworkUnavailable {
			detail: "no route to host".to_string(),
		},
		KnipError::ReadOnlyCache {
			path: PathBuf::from("/cache/knip"),
		},
		KnipError::CorruptCache {
			path: PathBuf::from("/cache/knip"),
			detail: "bad checksum".to_string(),
		},
		KnipError::InvalidConfig {
			path: PathBuf::from("/workspace/knip.json"),
			reason: "unexpected token".to_string(),
		},
		KnipError::LanguageServerCrash { exit_code: Some(1) },
		KnipError::LanguageServerCrash { exit_code: None },
		KnipError::UnsupportedWorkspace {
			reason: "no lockfile found".to_string(),
		},
		KnipError::AmbiguousPackageManager {
			found: vec!["package-lock.json".to_string(), "yarn.lock".to_string()],
		},
		KnipError::AdvancedSettingsRequireManaged {
			advanced: vec!["ts_config_path"],
		},
		KnipError::InvalidTsConfigPath {
			path: PathBuf::from("tsconfig.json"),
			reason: "file not found".to_string(),
		},
		KnipError::RequireConfigMissing {
			workspace_root: PathBuf::from("/workspace"),
		},
	];

	for error in variants {
		let message = error.to_string();
		assert!(
			!message.trim().is_empty(),
			"KnipError variant {error:?} produced an empty display message"
		);
	}
}

// =====================================================================
// Preprocessor failure modes (Task 7)
// =====================================================================
// JS-runtime failures are verified at the source level against the pinned
// managed-server fixture; settings-level validation uses the public
// `KnipSettings::validate()` API.

/// An invalid preprocessor specifier (parent-dir traversal) must be rejected
/// at parse time and the error must name the specifier so the user can
/// locate the bad entry in their `settings.json`.
#[test]
fn failure_preprocessor_invalid_specifier_settings_validation_error_naming_specifier() {
	let settings = KnipSettings {
		preprocessor: vec!["../escape.js".to_string()],
		..KnipSettings::default()
	};

	let error = settings
		.validate()
		.expect_err("parent-dir specifier must be rejected at parse time");

	let message = error.to_string();
	assert!(
		message.contains("../escape.js"),
		"error must name the offending specifier, got: {message}"
	);
	assert!(
		message.contains("lsp.knip.settings.preprocessor"),
		"error must name the lsp.knip.settings.preprocessor key, got: {message}"
	);
	assert!(
		message.contains("'../'") || message.to_lowercase().contains("parent"),
		"error must explain why the specifier is invalid, got: {message}"
	);
}

/// Every specifier shape we reject at parse time must surface the bad value
/// in the user-facing message so it can be fixed without grep'ing the JSON.
#[test]
fn failure_preprocessor_invalid_specifier_variants_each_name_their_value() {
	let bad_specifiers = [
		("../x.js", "parent directory traversal"),
		("/abs/x.js", "absolute path"),
		("~/x.js", "home expansion"),
		("file:x.js", "file: protocol"),
		("node:fs", "node: protocol"),
		("data:text/plain", "data: protocol"),
		("http://x", "http: protocol"),
		("https://x", "https: protocol"),
		("", "empty string"),
	];

	for (bad, label) in bad_specifiers {
		let settings = KnipSettings {
			preprocessor: vec![bad.to_string()],
			..KnipSettings::default()
		};
		let error = settings
			.validate()
			.expect_err(&format!("settings.validate() must reject {label} specifier '{bad}'"));

		let message = error.to_string();
		assert!(
			message.contains(bad) || bad.is_empty(),
			"error for {label} specifier must name the bad value, got: {message}"
		);
		assert!(
			message.contains("lsp.knip.settings.preprocessor"),
			"error for {label} specifier must name the setting, got: {message}"
		);
	}
}

/// `preprocessor_options` requires at least one preprocessor entry to be
/// configured. The error string must name the option and explain the
/// dependency so the user can resolve it without reading source.
#[test]
fn failure_preprocessor_options_without_preprocessor_rejected_with_actionable_message() {
	use std::collections::BTreeMap;
	let mut options = BTreeMap::new();
	options.insert("key".to_string(), serde_json_lite_value("value"));

	let settings = KnipSettings {
		preprocessor_options: Some(options),
		..KnipSettings::default()
	};

	let error = settings
		.validate()
		.expect_err("preprocessor_options without preprocessor must be rejected at parse time");

	let message = error.to_string();
	assert!(
		message.contains("lsp.knip.settings.preprocessor_options"),
		"error must name the lsp.knip.settings.preprocessor_options key, got: {message}"
	);
	assert!(
		message.contains("preprocessor") && message.to_lowercase().contains("requires"),
		"error must explain the dependency on lsp.knip.settings.preprocessor, got: {message}"
	);
}

/// The preprocessor-options error must surface the underlying `KnipSettingsError`
/// variant exactly so settings_for_worktree() can serialize it to the user.
#[test]
fn failure_preprocessor_options_without_preprocessor_surfaces_typed_error() {
	use std::collections::BTreeMap;
	let mut options = BTreeMap::new();
	options.insert("flag".to_string(), zed_extension_api::serde_json::Value::Bool(true));

	let settings = KnipSettings {
		preprocessor_options: Some(options),
		..KnipSettings::default()
	};

	let error = settings
		.validate()
		.expect_err("preprocessor_options without preprocessor must produce InvalidPreprocessorOptions");

	assert!(
		matches!(
			error,
			zed_knip::settings::KnipSettingsError::InvalidPreprocessorOptions { .. }
		),
		"error must be the InvalidPreprocessorOptions variant, got: {error:?}"
	);
}

/// The preprocessor specifier error must be `InvalidPreprocessor` (not a
/// generic rejection) so callers can distinguish it from other validation
/// failures programmatically.
#[test]
fn failure_preprocessor_invalid_specifier_surfaces_typed_error() {
	let settings = KnipSettings {
		preprocessor: vec!["../escape.js".to_string()],
		..KnipSettings::default()
	};

	let error = settings
		.validate()
		.expect_err("parent-dir specifier must produce InvalidPreprocessor");

	assert!(
		matches!(error, zed_knip::settings::KnipSettingsError::InvalidPreprocessor { ref value, .. } if value == "../escape.js"),
		"error must be the InvalidPreprocessor variant with value=../escape.js, got: {error:?}"
	);
}

/// When the dynamic `import(specifier)` call rejects, the patched JS
/// must wrap the cause in an `Error` whose message names the specifier.
/// This source-level assertion guards the import-failure path so the
/// `start()` / `handleFileChanges()` `try/catch` has something specific
/// to log to `connection.console.error`.
#[test]
fn failure_preprocessor_import_failure_patched_source_names_specifier() {
	let source = include_str!("../tests/fixtures/managed-server/knip-language-server-server.js");
	let patched = zed_knip::managed_install::apply_preprocessor_patch(source)
		.expect("patch must apply")
		.expect("fixture must be unpatched");

	assert!(
		patched.contains("Failed to import preprocessor '${resolved.spec}' from ${resolved.url}"),
		"import-failure path must embed the specifier in the thrown error"
	);
	assert!(
		patched.contains("await import(resolved.url)"),
		"import-failure path must wrap the dynamic import() call"
	);
}

/// When a preprocessor module's default export is not a function, the
/// patched JS must throw an `Error` that names the specifier so the
/// `start()` / `handleFileChanges()` fail-closed branch can surface it.
#[test]
fn failure_preprocessor_non_function_export_patched_source_names_specifier() {
	let source = include_str!("../tests/fixtures/managed-server/knip-language-server-server.js");
	let patched = zed_knip::managed_install::apply_preprocessor_patch(source)
		.expect("patch must apply")
		.expect("fixture must be unpatched");

	assert!(
		patched.contains("Preprocessor '${input.spec}' must export a function"),
		"non-function-export error must embed the specifier"
	);
	assert!(
		patched.contains("if (typeof preprocessor !== 'function')"),
		"non-function-export check must use typeof guard"
	);
	assert!(
		patched.contains("const preprocessor = module.default ?? module"),
		"non-function-export check must read module.default ?? module"
	);
}

/// A preprocessor that throws (sync) or rejects (async) must be
/// caught at the orchestration site without bringing down the LSP.
/// The `start()` and `handleFileChanges()` sites must each have a
/// `try { ... } catch (error) { ... }` around the `runZedKnipPreprocessors`
/// call so the patched language server keeps running.
#[test]
fn failure_preprocessor_thrown_error_patched_source_surfaces_via_console_error() {
	let source = include_str!("../tests/fixtures/managed-server/knip-language-server-server.js");
	let patched = zed_knip::managed_install::apply_preprocessor_patch(source)
		.expect("patch must apply")
		.expect("fixture must be unpatched");

	let start_try_catch = "try {\n          const reporterOptions = buildZedKnipReporterOptions.call(this, session.getResults(), session, config, configFilePath);\n          const output = await runZedKnipPreprocessors(\n            zedKnipPreprocessorConfig.preprocessors,\n            reporterOptions,\n            this.cwd ?? process.cwd()\n          );\n          this.zedKnipTransformedIssues = output.issues;\n          this.zedKnipTransformedResults = output;\n          this.zedKnipPreprocessorFingerprint = zedKnipPreprocessorConfig.fingerprint;\n        } catch (error) {\n          this.zedKnipPreprocessorFingerprint = PREPROCESSOR_PATCH_FAILED_FINGERPRINT;\n          this.zedKnipTransformedIssues = null;\n          this.zedKnipTransformedResults = null;\n          const message = `Knip preprocessor failed: ${error?.message ?? error}`;\n          this.connection.console.error(message);\n          if (this.connection.window?.showMessage) {\n            this.connection.window.showMessage(1, message);\n          }\n        }";
	assert!(
		patched.contains(start_try_catch),
		"start() must wrap runZedKnipPreprocessors in try/catch and surface the error via console.error"
	);

	let file_change_try_catch = "} catch (error) {\n          this.zedKnipPreprocessorFingerprint = PREPROCESSOR_PATCH_FAILED_FINGERPRINT;\n          this.zedKnipTransformedIssues = null;\n          this.zedKnipTransformedResults = null;\n          const message = `Knip preprocessor failed after file changes: ${error?.message ?? error}`;\n          this.connection.console.error(message);\n          if (this.connection.window?.showMessage) {\n            this.connection.window.showMessage(1, message);\n          }\n          this.publishDiagnostics(new Map());\n          return null;\n        }";
	assert!(
		patched.contains(file_change_try_catch),
		"handleFileChanges() must wrap runZedKnipPreprocessors in try/catch and surface the error via console.error"
	);
}

// Helper: build a JSON-ish value for preprocessor_options using the
// `zed_extension_api` re-export so the test never needs a real JSON dep.
fn serde_json_lite_value(text: &str) -> zed_extension_api::serde_json::Value {
	zed_extension_api::serde_json::Value::String(text.to_string())
}
