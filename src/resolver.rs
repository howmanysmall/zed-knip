use crate::{
	cache::{InstallSource, WorktreeCache},
	errors::KnipError,
	is_executable,
	package_manager::{self, PackageManager, PackageManagerError},
	settings::KnipSettings,
};
use std::path::{Path, PathBuf};
use zed_extension_api as zed;

const LANGUAGE_SERVER_BIN: &str = "knip-language-server";

pub use crate::managed_install::{ManagedInstall, ManagedInstallDisabled, ZedNpmManagedInstall};

/// Resolved Knip executable and package-manager state for a workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedKnip {
	pub executable_path: PathBuf,
	pub package_manager: PackageManager,
	pub install_source: InstallSource,
}

/// Prepared command payload used to launch the Knip language server.
#[derive(Debug, Clone)]
pub struct KnipLanguageServerCommand {
	pub command: zed::Command,
	pub working_dir: PathBuf,
}

/// Resolves the Knip language-server executable for a workspace.
pub fn resolve_knip(
	settings: &KnipSettings,
	cache: &WorktreeCache,
	managed_install: &impl ManagedInstall,
) -> Result<ResolvedKnip, KnipError> {
	let workspace_root = cache.worktree_root.as_path();

	if let Some(explicit_path) = settings.binary_path.as_deref() {
		let executable_path = resolve_configured_path(workspace_root, explicit_path);
		validate_explicit_path(&executable_path)?;
		return Ok(ResolvedKnip {
			executable_path,
			package_manager: resolve_package_manager(settings, cache)?,
			install_source: InstallSource::ExplicitPath,
		});
	}

	let package_manager = resolve_package_manager(settings, cache)?;

	if let Some(executable_path) = workspace_local_language_server(workspace_root) {
		validate_candidate_path(&executable_path)?;
		return Ok(ResolvedKnip {
			executable_path,
			package_manager,
			install_source: InstallSource::WorkspaceLocal,
		});
	}

	if let Some(executable_path) = managed_cache_path(cache) {
		validate_candidate_path(&executable_path)?;
		return Ok(ResolvedKnip {
			executable_path,
			package_manager,
			install_source: InstallSource::ManagedCache,
		});
	}

	if !settings.auto_install {
		return Err(KnipError::MissingKnip {
			workspace_root: workspace_root.to_path_buf(),
		});
	}

	let executable_path = managed_install.install(workspace_root, package_manager)?;
	validate_candidate_path(&executable_path)?;
	Ok(ResolvedKnip {
		executable_path,
		package_manager,
		install_source: InstallSource::ManagedCache,
	})
}

/// Builds the final Zed command used to launch the Knip language server.
pub fn build_language_server_command(
	resolved: &ResolvedKnip,
	settings: &KnipSettings,
	workspace_root: &Path,
) -> KnipLanguageServerCommand {
	let workspace = workspace_root.display().to_string();
	let mut args = vec!["--stdio".to_string(), "--cwd".to_string(), workspace.clone()];

	if let Some(config_path) = settings.config_path.as_deref() {
		args.push("--config".to_string());
		args.push(
			resolve_configured_path(workspace_root, config_path)
				.display()
				.to_string(),
		);
	}

	let command = zed::Command {
		command: resolved.executable_path.display().to_string(),
		args,
		env: vec![
			("PWD".to_string(), workspace.clone()),
			("KNIP_WORKSPACE_ROOT".to_string(), workspace),
			("KNIP_PACKAGE_MANAGER".to_string(), resolved.package_manager.to_string()),
			("KNIP_LOG_LEVEL".to_string(), settings.log_level.to_string()),
		],
	};

	KnipLanguageServerCommand {
		command,
		working_dir: workspace_root.to_path_buf(),
	}
}

fn resolve_package_manager(settings: &KnipSettings, cache: &WorktreeCache) -> Result<PackageManager, KnipError> {
	if let Some(package_manager) = settings.package_manager.as_deref() {
		return parse_package_manager_setting(package_manager);
	}

	if let Some(package_manager) = cache.package_manager.as_deref() {
		return parse_package_manager_setting(package_manager);
	}

	package_manager::detect(&cache.worktree_root).map_err(package_manager_error_to_knip_error)
}

fn parse_package_manager_setting(package_manager: &str) -> Result<PackageManager, KnipError> {
	package_manager::parse(package_manager).map_err(package_manager_error_to_knip_error)
}

fn package_manager_error_to_knip_error(error: PackageManagerError) -> KnipError {
	match error {
		PackageManagerError::NotFound => KnipError::UnsupportedWorkspace {
			reason: "No supported package manager lockfile or packageManager field was found.".to_string(),
		},
		PackageManagerError::Ambiguous { found } => KnipError::AmbiguousPackageManager { found },
		PackageManagerError::UnsupportedPackageManager { found } => KnipError::UnsupportedPackageManager { found },
	}
}

fn workspace_local_language_server(workspace_root: &Path) -> Option<PathBuf> {
	let bin_dir = workspace_root.join("node_modules").join(".bin");
	[bin_dir.join(LANGUAGE_SERVER_BIN)]
		.into_iter()
		.find(|candidate| candidate.is_file())
}

fn managed_cache_path(cache: &WorktreeCache) -> Option<PathBuf> {
	if cache.install_source == InstallSource::ManagedCache {
		cache.executable_path.clone()
	} else {
		None
	}
}

fn resolve_configured_path(workspace_root: &Path, configured_path: &str) -> PathBuf {
	let path = PathBuf::from(configured_path);
	if path.is_absolute() {
		path
	} else {
		workspace_root.join(path)
	}
}

fn validate_explicit_path(path: &Path) -> Result<(), KnipError> {
	validate_executable_path(path, |missing_path| KnipError::InvalidExplicitPath {
		path: missing_path,
	})
}

fn validate_candidate_path(path: &Path) -> Result<(), KnipError> {
	validate_executable_path(path, |missing_path| KnipError::MissingKnip {
		workspace_root: missing_path.parent().unwrap_or(missing_path.as_path()).to_path_buf(),
	})
}

fn validate_executable_path(path: &Path, missing_error: impl FnOnce(PathBuf) -> KnipError) -> Result<(), KnipError> {
	if !path.is_file() {
		return Err(missing_error(path.to_path_buf()));
	}

	if !is_executable(path) {
		return Err(KnipError::NonExecutablePath {
			path: path.to_path_buf(),
		});
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::managed_install::{
		apply_did_save_refresh_patch, patch_managed_language_server, DID_SAVE_REFRESH_PATCH_MARKER,
	};
	use crate::settings::LogLevel;
	use std::{
		fs::{self, File},
		io,
		path::Path,
		time::{SystemTime, UNIX_EPOCH},
	};

	#[derive(Debug, Clone)]
	struct TestWorkspace {
		root: PathBuf,
	}

	impl TestWorkspace {
		fn new(name: &str) -> Self {
			let nanos = SystemTime::now()
				.duration_since(UNIX_EPOCH)
				.unwrap_or_default()
				.as_nanos();
			let root = std::env::temp_dir().join(format!("zed-knip-resolver-{name}-{nanos}"));
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

		fn package_json(&self, package_manager: &str) {
			self.write(
				"package.json",
				&format!("{{\"packageManager\":\"{package_manager}@1.0.0\"}}"),
			);
		}
	}

	impl Drop for TestWorkspace {
		fn drop(&mut self) {
			let _ = fs::remove_dir_all(&self.root);
		}
	}

	#[derive(Debug, Clone)]
	struct MockManagedInstall {
		result: Result<PathBuf, KnipError>,
	}

	impl ManagedInstall for MockManagedInstall {
		fn install(&self, _workspace_root: &Path, _package_manager: PackageManager) -> Result<PathBuf, KnipError> {
			self.result.clone()
		}
	}

	fn cache(root: &Path) -> WorktreeCache {
		WorktreeCache::new(root.to_path_buf())
	}

	fn make_executable(path: &Path) -> io::Result<()> {
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
	fn resolver_explicit_path_wins_over_workspace_local_and_cache() {
		let workspace = TestWorkspace::new("explicit-wins");
		workspace.package_json("npm");
		let explicit = workspace.executable("tools/knip-language-server");
		workspace.executable("node_modules/.bin/knip-language-server");
		let cached = workspace.executable("cache/knip-language-server");
		let mut cache = cache(&workspace.root);
		cache.executable_path = Some(cached);
		cache.install_source = InstallSource::ManagedCache;
		let settings = KnipSettings {
			binary_path: Some("tools/knip-language-server".to_string()),
			..KnipSettings::default()
		};

		let resolved = resolve_knip(&settings, &cache, &ManagedInstallDisabled).unwrap();

		assert_eq!(resolved.executable_path, explicit);
		assert_eq!(resolved.install_source, InstallSource::ExplicitPath);
	}

	#[test]
	fn resolver_uses_workspace_local_install_before_managed_cache() {
		let workspace = TestWorkspace::new("workspace-local");
		workspace.package_json("pnpm");
		let local = workspace.executable("node_modules/.bin/knip-language-server");
		let cached = workspace.executable("cache/knip-language-server");
		let mut cache = cache(&workspace.root);
		cache.executable_path = Some(cached);
		cache.install_source = InstallSource::ManagedCache;

		let resolved = resolve_knip(&KnipSettings::default(), &cache, &ManagedInstallDisabled).unwrap();

		assert_eq!(resolved.executable_path, local);
		assert_eq!(resolved.package_manager, PackageManager::Pnpm);
		assert_eq!(resolved.install_source, InstallSource::WorkspaceLocal);
	}

	#[test]
	fn resolver_does_not_start_knip_cli_as_language_server() {
		let workspace = TestWorkspace::new("knip-cli-not-lsp");
		workspace.package_json("bun");
		workspace.executable("node_modules/.bin/knip");
		let managed = workspace.executable("managed/knip-language-server");
		let installer = MockManagedInstall {
			result: Ok(managed.clone()),
		};

		let resolved = resolve_knip(&KnipSettings::default(), &cache(&workspace.root), &installer).unwrap();

		assert_eq!(resolved.executable_path, managed);
		assert_eq!(resolved.install_source, InstallSource::ManagedCache);
	}

	#[test]
	fn resolver_uses_managed_cache_hit_when_workspace_local_missing() {
		let workspace = TestWorkspace::new("managed-cache");
		workspace.package_json("yarn");
		let cached = workspace.executable("cache/knip-language-server");
		let mut cache = cache(&workspace.root);
		cache.executable_path = Some(cached.clone());
		cache.install_source = InstallSource::ManagedCache;

		let resolved = resolve_knip(&KnipSettings::default(), &cache, &ManagedInstallDisabled).unwrap();

		assert_eq!(resolved.executable_path, cached);
		assert_eq!(resolved.package_manager, PackageManager::Yarn);
		assert_eq!(resolved.install_source, InstallSource::ManagedCache);
	}

	#[test]
	fn resolver_disabled_managed_install_returns_missing_knip() {
		let workspace = TestWorkspace::new("disabled-managed");
		workspace.package_json("bun");
		let settings = KnipSettings {
			auto_install: false,
			..KnipSettings::default()
		};

		let error = resolve_knip(&settings, &cache(&workspace.root), &ManagedInstallDisabled).unwrap_err();

		assert_eq!(
			error,
			KnipError::MissingKnip {
				workspace_root: workspace.root.clone()
			}
		);
	}

	#[test]
	fn resolver_missing_knip_uses_explicit_managed_install_path() {
		let workspace = TestWorkspace::new("managed-install");
		workspace.package_json("npm");
		let managed = workspace.executable("managed/knip-language-server");
		let installer = MockManagedInstall {
			result: Ok(managed.clone()),
		};

		let resolved = resolve_knip(&KnipSettings::default(), &cache(&workspace.root), &installer).unwrap();

		assert_eq!(resolved.executable_path, managed);
		assert_eq!(resolved.install_source, InstallSource::ManagedCache);
	}

	#[test]
	fn managed_language_server_patch_adds_save_based_refresh_once() {
		let source = r#"import { FileChangeType, ProposedFeatures, TextDocuments } from 'vscode-languageserver';

class LanguageServer {
  constructor() {
    this.documents.listen(this.connection);
    this.connection.listen();
  }

  onInitialize() {
    const capabilities = {
        codeActionProvider: {
          codeActionKinds: [CodeActionKind.QuickFix],
        },
    };
    return { capabilities };
  }
}
"#;

		let patched = apply_did_save_refresh_patch(source)
			.expect("patch must apply")
			.expect("first patch must update source");

		assert!(patched.contains("TextDocumentSyncKind"));
		assert!(patched.contains("textDocumentSync"));
		assert!(patched.contains("this.documents.onDidSave"));
		assert!(patched.contains("FileChangeType.Changed"));
		assert!(patched.contains(DID_SAVE_REFRESH_PATCH_MARKER));
		assert!(
			apply_did_save_refresh_patch(&patched)
				.expect("patched source must be accepted")
				.is_none(),
			"patch must be idempotent"
		);
	}

	#[cfg(unix)]
	#[test]
	fn managed_language_server_patch_resolves_bin_symlink_to_server_file() {
		use std::os::unix::fs::symlink;

		let workspace = TestWorkspace::new("managed-patch");
		let cli = workspace.write(
			"node_modules/@knip/language-server/src/cli.js",
			"#!/usr/bin/env node\nimport './index.js';\n",
		);
		workspace.write(
			"node_modules/@knip/language-server/src/server.js",
			"import { FileChangeType, ProposedFeatures, TextDocuments } from 'vscode-languageserver';\n\nclass LanguageServer {\n  constructor() {\n    this.documents.listen(this.connection);\n    this.connection.listen();\n  }\n\n  buildDiagnostics(issues, config, rules) {\n    for (const issue of Object.values(issues)) {\n      for (const uri of Object.keys(issue.files)) {\n          const document = this.documents.get(uri);\n          const diagnostic = issueToDiagnostic(issue, rules, config, document);\n          this.issuesByUri.set(uri, []);\n      }\n    }\n  }\n\n  async resolveConfig() {\n    const configFilePath = config?.configFilePath;\n    const options = await knip.createOptions({ cwd: this.cwd, isSession: true, args: { config: configFilePath } });\n    return options;\n  }\n\n  onInitialize() {\n    const capabilities = {\n        codeActionProvider: {},\n    };\n    return { capabilities };\n  }\n}\n",
		);
		let bin_dir = workspace.root.join("node_modules").join(".bin");
		fs::create_dir_all(&bin_dir).unwrap_or_else(|error| panic!("failed to create {}: {error}", bin_dir.display()));
		let executable = bin_dir.join("knip-language-server");
		symlink("../@knip/language-server/src/cli.js", &executable)
			.unwrap_or_else(|error| panic!("failed to link {} to {}: {error}", executable.display(), cli.display()));

		patch_managed_language_server(&executable).unwrap();

		let server = fs::read_to_string(workspace.root.join("node_modules/@knip/language-server/src/server.js"))
			.expect("patched server must be readable");
		assert!(server.contains(DID_SAVE_REFRESH_PATCH_MARKER));
		assert!(server.contains("this.documents.onDidSave"));
	}

	#[test]
	fn resolver_invalid_explicit_path_returns_invalid_path() {
		let workspace = TestWorkspace::new("invalid-explicit");
		workspace.package_json("npm");
		let invalid = workspace.root.join("missing/knip-language-server");
		let settings = KnipSettings {
			binary_path: Some("missing/knip-language-server".to_string()),
			..KnipSettings::default()
		};

		let error = resolve_knip(&settings, &cache(&workspace.root), &ManagedInstallDisabled).unwrap_err();

		assert_eq!(error, KnipError::InvalidExplicitPath { path: invalid });
	}

	#[test]
	fn resolver_non_executable_path_returns_non_executable() {
		let workspace = TestWorkspace::new("non-executable");
		workspace.package_json("npm");
		let path = workspace.write("tools/knip-language-server", "not executable\n");
		let settings = KnipSettings {
			binary_path: Some("tools/knip-language-server".to_string()),
			..KnipSettings::default()
		};

		let error = resolve_knip(&settings, &cache(&workspace.root), &ManagedInstallDisabled).unwrap_err();

		assert_eq!(error, KnipError::NonExecutablePath { path });
	}

	#[test]
	fn resolver_ambiguous_manager_returns_ambiguous_error() {
		let workspace = TestWorkspace::new("ambiguous-manager");
		workspace.write("package.json", "{}");
		workspace.write("package-lock.json", "{}");
		workspace.write("yarn.lock", "");

		let error = resolve_knip(
			&KnipSettings::default(),
			&cache(&workspace.root),
			&ManagedInstallDisabled,
		)
		.unwrap_err();

		assert_eq!(
			error,
			KnipError::AmbiguousPackageManager {
				found: vec!["package-lock.json".to_string(), "yarn.lock".to_string()]
			}
		);
	}

	#[test]
	fn resolver_offline_failed_install_surfaces_install_error() {
		let workspace = TestWorkspace::new("offline-install");
		workspace.package_json("npm");
		let installer = MockManagedInstall {
			result: Err(KnipError::NetworkUnavailable {
				detail: "offline".to_string(),
			}),
		};

		let error = resolve_knip(&KnipSettings::default(), &cache(&workspace.root), &installer).unwrap_err();

		assert_eq!(
			error,
			KnipError::NetworkUnavailable {
				detail: "offline".to_string()
			}
		);
	}

	#[test]
	fn command_builder_sets_exact_command_args_env_and_working_dir() {
		let workspace = TestWorkspace::new("command-builder");
		workspace.package_json("pnpm");
		let executable = workspace.executable("node_modules/.bin/knip-language-server");
		let config = workspace.write("knip.json", "{}\n");
		let settings = KnipSettings {
			log_level: LogLevel::Debug,
			config_path: Some("knip.json".to_string()),
			..KnipSettings::default()
		};
		let resolved = ResolvedKnip {
			executable_path: executable.clone(),
			package_manager: PackageManager::Pnpm,
			install_source: InstallSource::WorkspaceLocal,
		};

		let command = build_language_server_command(&resolved, &settings, &workspace.root);

		assert_eq!(command.working_dir, workspace.root);
		assert_eq!(command.command.command, executable.display().to_string());
		assert_eq!(
			command.command.args,
			vec![
				"--stdio".to_string(),
				"--cwd".to_string(),
				command.working_dir.display().to_string(),
				"--config".to_string(),
				config.display().to_string(),
			]
		);
		assert_eq!(
			command.command.env,
			vec![
				("PWD".to_string(), command.working_dir.display().to_string()),
				(
					"KNIP_WORKSPACE_ROOT".to_string(),
					command.working_dir.display().to_string()
				),
				("KNIP_PACKAGE_MANAGER".to_string(), "pnpm".to_string()),
				("KNIP_LOG_LEVEL".to_string(), "debug".to_string()),
			]
		);
	}

	#[test]
	fn command_builder_preserves_path_with_spaces() {
		let workspace = TestWorkspace::new("command builder spaces");
		workspace.package_json("npm");
		let executable = workspace.executable("node_modules/.bin/knip-language-server");
		let resolved = ResolvedKnip {
			executable_path: executable.clone(),
			package_manager: PackageManager::Npm,
			install_source: InstallSource::WorkspaceLocal,
		};

		let command = build_language_server_command(&resolved, &KnipSettings::default(), &workspace.root);

		assert_eq!(command.command.command, executable.display().to_string());
		assert!(command.command.args.contains(&workspace.root.display().to_string()));
		assert_eq!(command.working_dir, workspace.root);
	}

	#[test]
	fn perf_command_builder_creates_one_language_server_process_per_worktree() {
		let workspace = TestWorkspace::new("single-process");
		workspace.package_json("npm");
		let executable = workspace.executable("node_modules/.bin/knip-language-server");
		let resolved = ResolvedKnip {
			executable_path: executable.clone(),
			package_manager: PackageManager::Npm,
			install_source: InstallSource::WorkspaceLocal,
		};

		let command = build_language_server_command(&resolved, &KnipSettings::default(), &workspace.root);

		assert_eq!(command.command.command, executable.display().to_string());
		assert_eq!(
			command
				.command
				.args
				.iter()
				.filter(|arg| arg.as_str() == "--cwd")
				.count(),
			1
		);
		assert_eq!(
			command
				.command
				.args
				.iter()
				.filter(|arg| arg.as_str() == "--stdio")
				.count(),
			1
		);
		assert_eq!(command.working_dir, workspace.root);
	}

	#[test]
	fn resolver_rejects_managed_cache_non_executable() {
		let workspace = TestWorkspace::new("managed-non-executable");
		workspace.package_json("npm");
		let cached = workspace.write("cache/knip-language-server", "not executable\n");
		let mut cache = cache(&workspace.root);
		cache.executable_path = Some(cached.clone());
		cache.install_source = InstallSource::ManagedCache;

		let error = resolve_knip(&KnipSettings::default(), &cache, &ManagedInstallDisabled).unwrap_err();

		assert_eq!(error, KnipError::NonExecutablePath { path: cached });
	}

	#[test]
	fn resolver_does_not_treat_directory_as_explicit_executable() {
		let workspace = TestWorkspace::new("explicit-directory");
		workspace.package_json("npm");
		let directory = workspace.root.join("tools");
		fs::create_dir_all(&directory)
			.unwrap_or_else(|error| panic!("failed to create {}: {error}", directory.display()));
		let settings = KnipSettings {
			binary_path: Some("tools".to_string()),
			..KnipSettings::default()
		};

		let error = resolve_knip(&settings, &cache(&workspace.root), &ManagedInstallDisabled).unwrap_err();

		assert_eq!(error, KnipError::InvalidExplicitPath { path: directory });
	}

	#[test]
	fn resolver_accepts_absolute_explicit_path() {
		let workspace = TestWorkspace::new("absolute-explicit");
		workspace.package_json("npm");
		let explicit = workspace.executable("absolute/knip-language-server");
		let settings = KnipSettings {
			binary_path: Some(explicit.display().to_string()),
			..KnipSettings::default()
		};

		let resolved = resolve_knip(&settings, &cache(&workspace.root), &ManagedInstallDisabled).unwrap();

		assert_eq!(resolved.executable_path, explicit);
	}

	#[test]
	fn resolver_keeps_explicit_package_manager_override_for_command_env() {
		let workspace = TestWorkspace::new("manager-override");
		workspace.write("package.json", "{}");
		workspace.write("package-lock.json", "{}");
		workspace.write("yarn.lock", "");
		let executable = workspace.executable("node_modules/.bin/knip-language-server");
		let settings = KnipSettings {
			package_manager: Some("bun".to_string()),
			..KnipSettings::default()
		};

		let resolved = resolve_knip(&settings, &cache(&workspace.root), &ManagedInstallDisabled).unwrap();

		assert_eq!(resolved.executable_path, executable);
		assert_eq!(resolved.package_manager, PackageManager::Bun);
	}

	#[test]
	fn test_helper_can_create_file_for_non_spawned_executable_fixture() {
		let workspace = TestWorkspace::new("file-helper");
		let path = workspace.root.join("touch");
		File::create(&path).unwrap_or_else(|error| panic!("failed to create {}: {error}", path.display()));

		assert!(path.is_file());
	}

	#[test]
	fn resolver_workspace_root_with_spaces_resolves_workspace_local() {
		let workspace = TestWorkspace::new("path with spaces");
		workspace.package_json("npm");
		let local = workspace.executable("node_modules/.bin/knip-language-server");

		let resolved = resolve_knip(
			&KnipSettings::default(),
			&cache(&workspace.root),
			&ManagedInstallDisabled,
		)
		.unwrap();

		assert_eq!(resolved.executable_path, local);
		assert!(
			resolved.executable_path.display().to_string().contains(' '),
			"executable path must contain a space"
		);
	}

	#[test]
	fn resolver_absolute_explicit_path_with_spaces_is_accepted() {
		let workspace = TestWorkspace::new("explicit path with spaces");
		workspace.package_json("npm");
		let explicit = workspace.executable("tools/knip language server");
		let settings = KnipSettings {
			binary_path: Some(explicit.display().to_string()),
			..KnipSettings::default()
		};

		let resolved = resolve_knip(&settings, &cache(&workspace.root), &ManagedInstallDisabled).unwrap();

		assert_eq!(resolved.executable_path, explicit);
		assert_eq!(resolved.install_source, InstallSource::ExplicitPath);
	}

	#[test]
	fn resolver_relative_explicit_path_with_spaces_is_joined_to_workspace_root() {
		let workspace = TestWorkspace::new("relative-spaces");
		workspace.package_json("npm");
		let explicit = workspace.executable("my tools/knip-language-server");
		let settings = KnipSettings {
			binary_path: Some("my tools/knip-language-server".to_string()),
			..KnipSettings::default()
		};

		let resolved = resolve_knip(&settings, &cache(&workspace.root), &ManagedInstallDisabled).unwrap();

		assert_eq!(resolved.executable_path, explicit);
		assert_eq!(resolved.install_source, InstallSource::ExplicitPath);
	}

	#[test]
	fn command_builder_encodes_path_with_spaces_as_display_string() {
		let workspace = TestWorkspace::new("cmd spaces");
		workspace.package_json("npm");
		let executable = workspace.executable("node_modules/.bin/knip-language-server");
		let resolved = ResolvedKnip {
			executable_path: executable.clone(),
			package_manager: PackageManager::Npm,
			install_source: InstallSource::WorkspaceLocal,
		};

		let command = build_language_server_command(&resolved, &KnipSettings::default(), &workspace.root);

		let root_str = workspace.root.display().to_string();
		assert!(
			root_str.contains(' '),
			"workspace root must contain a space for this test"
		);
		assert_eq!(command.command.command, executable.display().to_string());
		assert!(command.command.args.contains(&root_str));
		assert_eq!(
			command
				.command
				.env
				.iter()
				.find(|(k, _)| k == "PWD")
				.map(|(_, v)| v.as_str()),
			Some(root_str.as_str())
		);
	}

	#[test]
	fn resolver_cache_with_spaces_in_executable_path_is_used() {
		let workspace = TestWorkspace::new("cache-spaces");
		workspace.package_json("bun");
		let cached = workspace.executable("my cache dir/knip-language-server");
		let mut cache = cache(&workspace.root);
		cache.executable_path = Some(cached.clone());
		cache.install_source = InstallSource::ManagedCache;

		let resolved = resolve_knip(&KnipSettings::default(), &cache, &ManagedInstallDisabled).unwrap();

		assert_eq!(resolved.executable_path, cached);
		assert_eq!(resolved.install_source, InstallSource::ManagedCache);
	}

	#[test]
	fn resolver_uses_nested_workspace_root_in_monorepo() {
		let workspace = TestWorkspace::new("monorepo-nested-root");
		workspace.package_json("pnpm");
		workspace.executable("node_modules/.bin/knip-language-server");
		workspace.write(
			"packages/app/package.json",
			"{\"name\":\"app\",\"packageManager\":\"npm@10.0.0\"}",
		);
		let nested_local = workspace.executable("packages/app/node_modules/.bin/knip-language-server");
		let nested_root = workspace.root.join("packages/app");

		let resolved = resolve_knip(&KnipSettings::default(), &cache(&nested_root), &ManagedInstallDisabled).unwrap();

		assert_eq!(resolved.executable_path, nested_local);
		assert_eq!(resolved.package_manager, PackageManager::Npm);
		assert_eq!(resolved.install_source, InstallSource::WorkspaceLocal);
	}

	#[test]
	fn resolver_nested_workspace_does_not_inherit_parent_package_manager() {
		let workspace = TestWorkspace::new("monorepo-no-parent-manager");
		workspace.package_json("pnpm");
		workspace.executable("node_modules/.bin/knip-language-server");
		let nested_root = workspace.root.join("packages/lib");
		fs::create_dir_all(&nested_root)
			.unwrap_or_else(|error| panic!("failed to create {}: {error}", nested_root.display()));

		let error = resolve_knip(&KnipSettings::default(), &cache(&nested_root), &ManagedInstallDisabled).unwrap_err();

		assert_eq!(
			error,
			KnipError::UnsupportedWorkspace {
				reason: "No supported package manager lockfile or packageManager field was found.".to_string()
			}
		);
	}

	#[cfg(unix)]
	#[test]
	fn resolver_follows_symlink_to_workspace_local_executable() {
		use std::os::unix::fs::symlink;

		let workspace = TestWorkspace::new("symlink-resolver");
		workspace.package_json("pnpm");
		let real = workspace.executable("real/knip-language-server");
		let link_dir = workspace.root.join("node_modules").join(".bin");
		fs::create_dir_all(&link_dir)
			.unwrap_or_else(|error| panic!("failed to create {}: {error}", link_dir.display()));
		let link = link_dir.join("knip-language-server");
		symlink(&real, &link).unwrap_or_else(|error| panic!("failed to create symlink {}: {error}", link.display()));

		let resolved = resolve_knip(
			&KnipSettings::default(),
			&cache(&workspace.root),
			&ManagedInstallDisabled,
		)
		.unwrap();

		assert_eq!(resolved.executable_path, link);
		assert_eq!(resolved.install_source, InstallSource::WorkspaceLocal);
	}

	#[test]
	fn resolve_configured_path_returns_absolute_path_unchanged() {
		let workspace = TestWorkspace::new("abs-path-unchanged");
		let absolute = workspace.root.join("tools").join("knip");

		let result = resolve_configured_path(&workspace.root, &absolute.display().to_string());

		assert_eq!(result, absolute);
		assert!(result.is_absolute());
	}

	#[test]
	fn resolve_configured_path_joins_relative_path_to_workspace_root() {
		let workspace = TestWorkspace::new("rel-path-join");

		let result = resolve_configured_path(&workspace.root, "tools/knip");

		assert_eq!(result, workspace.root.join("tools").join("knip"));
		assert!(result.is_absolute());
	}

	#[test]
	fn resolve_configured_path_handles_relative_path_with_spaces() {
		let workspace = TestWorkspace::new("rel-spaces-join");

		let result = resolve_configured_path(&workspace.root, "my tools/knip language server");

		assert_eq!(result, workspace.root.join("my tools").join("knip language server"));
	}
}
