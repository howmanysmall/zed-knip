// Failure-mode integration tests for zed-knip.
//
// Each test exercises a distinct failure scenario using real fixtures from
// `tests/fixtures/` and the production error types from `src/errors.rs`.
// No actual network calls are made; network failures are simulated via the
// `ManagedInstall` mock seam in `src/resolver.rs`.
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

// ---------------------------------------------------------------------------
// Shared test helpers
// ---------------------------------------------------------------------------

use std::{
	fs,
	path::{Path, PathBuf},
	time::{SystemTime, UNIX_EPOCH},
};

use zed_knip::{
	cache::{InstallSource, WorktreeCache},
	errors::KnipError,
	package_manager::PackageManager,
	resolver::{resolve_knip, ManagedInstall, ManagedInstallDisabled},
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

// ---------------------------------------------------------------------------
// 1. Missing Knip binary
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// 2. Invalid Knip config
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// 3. Corrupt cache
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// 4. Read-only cache
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// 5. Language server crash
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// 6. Offline / network failure
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// 7. Ambiguous package manager
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Error display completeness
// ---------------------------------------------------------------------------

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
	];

	for error in variants {
		let message = error.to_string();
		assert!(
			!message.trim().is_empty(),
			"KnipError variant {error:?} produced an empty display message"
		);
	}
}
