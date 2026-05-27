use std::{fmt, fs, path::Path, str::FromStr};
use zed_extension_api::serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
	Npm,
	Pnpm,
	Yarn,
	Bun,
	Deno,
	Vlt,
	Aube,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageManagerError {
	NotFound,
	Ambiguous { found: Vec<String> },
	UnsupportedPackageManager { found: String },
}

pub const LOCKFILES: &[(&str, PackageManager)] = &[
	("package-lock.json", PackageManager::Npm),
	("pnpm-lock.yaml", PackageManager::Pnpm),
	("yarn.lock", PackageManager::Yarn),
	("bun.lock", PackageManager::Bun),
	("deno.lock", PackageManager::Deno),
	("vlt-lock.json", PackageManager::Vlt),
	("aube-lock.yaml", PackageManager::Aube),
];

impl fmt::Display for PackageManagerError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::NotFound => write!(f, "No supported package manager lockfile was found."),
			Self::Ambiguous { found } => write!(
				f,
				"Multiple package managers were detected ({}). Remove the extra lockfile(s) or set the package manager explicitly in settings.",
				found.join(", ")
			),
			Self::UnsupportedPackageManager { found } => write!(
				f,
				"Unsupported package manager {found}. Use a supported package manager or set the package manager explicitly in settings."
			),
		}
	}
}

impl std::error::Error for PackageManagerError {}

impl fmt::Display for PackageManager {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Npm => f.write_str("npm"),
			Self::Pnpm => f.write_str("pnpm"),
			Self::Yarn => f.write_str("yarn"),
			Self::Bun => f.write_str("bun"),
			Self::Deno => f.write_str("deno"),
			Self::Vlt => f.write_str("vlt"),
			Self::Aube => f.write_str("aube"),
		}
	}
}

pub fn parse(name: &str) -> Result<PackageManager, PackageManagerError> {
	let name = name.trim();
	let manager = name.split_once('@').map(|(value, _)| value).unwrap_or(name);

	match manager {
		"npm" => Ok(PackageManager::Npm),
		"pnpm" => Ok(PackageManager::Pnpm),
		"yarn" => Ok(PackageManager::Yarn),
		"bun" => Ok(PackageManager::Bun),
		"deno" => Ok(PackageManager::Deno),
		"vlt" => Ok(PackageManager::Vlt),
		"aube" => Ok(PackageManager::Aube),
		_ => Err(PackageManagerError::UnsupportedPackageManager {
			found: name.to_string(),
		}),
	}
}

pub fn detect(workspace_root: &Path) -> Result<PackageManager, PackageManagerError> {
	let package_json = fs::read_to_string(workspace_root.join("package.json")).ok();
	detect_from_workspace_files(package_json.as_deref(), |lockfile| {
		workspace_root.join(lockfile).is_file()
	})
}

pub fn detect_from_workspace_files(
	package_json: Option<&str>,
	mut lockfile_exists: impl FnMut(&str) -> bool,
) -> Result<PackageManager, PackageManagerError> {
	if let Some(package_json) = package_json {
		if let Some(manager) = detect_from_package_json_contents(package_json) {
			return manager;
		}
	}

	let found = LOCKFILES
		.iter()
		.filter(|(lockfile, _)| lockfile_exists(lockfile))
		.map(|(lockfile, _)| (*lockfile).to_string())
		.collect::<Vec<_>>();

	match found.as_slice() {
		[] => Err(PackageManagerError::NotFound),
		[lockfile] => match lockfile_to_manager(lockfile) {
			Some(manager) => Ok(manager),
			None => Err(PackageManagerError::NotFound),
		},
		_ => Err(PackageManagerError::Ambiguous { found }),
	}
}

fn detect_from_package_json_contents(contents: &str) -> Option<Result<PackageManager, PackageManagerError>> {
	let manager = extract_package_manager_field(contents)?;
	Some(parse(&manager))
}

fn extract_package_manager_field(contents: &str) -> Option<String> {
	let value = Value::from_str(contents).ok()?;
	value.get("packageManager").and_then(Value::as_str).map(str::to_string)
}

fn lockfile_to_manager(lockfile: &str) -> Option<PackageManager> {
	LOCKFILES
		.iter()
		.find_map(|(candidate, manager)| (*candidate == lockfile).then_some(*manager))
}

#[cfg(test)]
mod tests {
	use super::{detect, PackageManager, PackageManagerError};
	use std::fs;
	use std::path::{Path, PathBuf};
	use std::time::{SystemTime, UNIX_EPOCH};

	fn fixture(name: &str) -> PathBuf {
		Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
	}

	#[derive(Debug)]
	struct TempWorkspace {
		root: PathBuf,
	}

	impl TempWorkspace {
		fn new(name: &str) -> Self {
			let nanos = SystemTime::now()
				.duration_since(UNIX_EPOCH)
				.unwrap_or_default()
				.as_nanos();
			let root = std::env::temp_dir().join(format!("zed-knip-package-manager-{name}-{nanos}"));
			fs::create_dir_all(&root).unwrap_or_else(|error| panic!("failed to create {}: {error}", root.display()));
			Self { root }
		}

		fn write(&self, relative_path: &str, contents: &str) {
			let path = self.root.join(relative_path);
			if let Some(parent) = path.parent() {
				fs::create_dir_all(parent)
					.unwrap_or_else(|error| panic!("failed to create {}: {error}", parent.display()));
			}
			fs::write(&path, contents).unwrap_or_else(|error| panic!("failed to write {}: {error}", path.display()));
		}
	}

	impl Drop for TempWorkspace {
		fn drop(&mut self) {
			let _ = fs::remove_dir_all(&self.root);
		}
	}

	#[test]
	fn package_manager_detects_npm_fixture() {
		let manager = detect(&fixture("npm")).unwrap();

		assert_eq!(manager, PackageManager::Npm);
	}

	#[test]
	fn package_manager_detects_pnpm_fixture() {
		let manager = detect(&fixture("pnpm")).unwrap();

		assert_eq!(manager, PackageManager::Pnpm);
	}

	#[test]
	fn package_manager_detects_yarn_fixture() {
		let manager = detect(&fixture("yarn")).unwrap();

		assert_eq!(manager, PackageManager::Yarn);
	}

	#[test]
	fn package_manager_detects_bun_fixture() {
		let manager = detect(&fixture("bun")).unwrap();

		assert_eq!(manager, PackageManager::Bun);
	}

	#[test]
	fn package_manager_detects_deno_fixture() {
		let manager = detect(&fixture("deno")).unwrap();

		assert_eq!(manager, PackageManager::Deno);
	}

	#[test]
	fn package_manager_detects_vlt_fixture() {
		let manager = detect(&fixture("vlt")).unwrap();

		assert_eq!(manager, PackageManager::Vlt);
	}

	#[test]
	fn package_manager_detects_aube_fixture() {
		let manager = detect(&fixture("aube")).unwrap();

		assert_eq!(manager, PackageManager::Aube);
	}

	#[test]
	fn package_manager_detects_package_manager_field() {
		let manager = detect(&fixture("package-manager-field")).unwrap();

		assert_eq!(manager, PackageManager::Pnpm);
	}

	#[test]
	fn package_manager_field_wins_over_lockfile() {
		let workspace = TempWorkspace::new("package-manager-precedence");
		workspace.write("package.json", r#"{"packageManager":"aube@1.15.0"}"#);
		workspace.write("bun.lock", "");

		let manager = detect(&workspace.root).unwrap();

		assert_eq!(manager, PackageManager::Aube);
	}

	#[test]
	fn package_manager_invalid_package_json_falls_back_to_lockfile() {
		let workspace = TempWorkspace::new("invalid-package-json");
		workspace.write("package.json", r#"{"packageManager":"pnpm@9.0.0""#);
		workspace.write("bun.lock", "");

		let manager = detect(&workspace.root).unwrap();

		assert_eq!(manager, PackageManager::Bun);
	}

	#[test]
	fn package_manager_non_string_package_manager_field_falls_back_to_lockfile() {
		let workspace = TempWorkspace::new("non-string-package-manager");
		workspace.write("package.json", r#"{"packageManager":{"name":"pnpm"}}"#);
		workspace.write("yarn.lock", "");

		let manager = detect(&workspace.root).unwrap();

		assert_eq!(manager, PackageManager::Yarn);
	}

	#[test]
	fn package_manager_detects_no_manager_error() {
		let error = detect(&fixture("no-manager")).unwrap_err();

		assert!(matches!(error, PackageManagerError::NotFound));
	}

	#[test]
	fn package_manager_ambiguous_multiple_lockfiles_error() {
		let error = detect(&fixture("multiple-lockfiles")).unwrap_err();

		match error {
			PackageManagerError::Ambiguous { found } => {
				assert_eq!(found, vec!["package-lock.json", "yarn.lock"]);
			}
			other => panic!("expected ambiguous error, got {other:?}"),
		}
	}

	#[test]
	fn perf_package_manager_detection_ignores_nested_lockfiles() {
		let workspace = TempWorkspace::new("nested-lockfiles");
		workspace.write("packages/app/package-lock.json", "{}");
		workspace.write("packages/app/pnpm-lock.yaml", "lockfileVersion: '9.0'\n");

		let error = detect(&workspace.root).unwrap_err();

		assert_eq!(error, PackageManagerError::NotFound);
	}

	#[test]
	fn package_manager_detects_pnpm_at_monorepo_root() {
		let manager = detect(&fixture("monorepo")).unwrap();

		assert_eq!(manager, PackageManager::Pnpm);
	}

	#[test]
	fn package_manager_detects_nested_package_root_without_parent_scan() {
		let manager = detect(&fixture("monorepo").join("packages/app")).unwrap();

		assert_eq!(manager, PackageManager::Npm);
	}

	#[test]
	fn package_manager_nested_package_root_does_not_inherit_monorepo_root_lockfile() {
		let error = detect(&fixture("monorepo").join("packages/lib")).unwrap_err();

		assert_eq!(error, PackageManagerError::NotFound);
	}

	#[test]
	fn package_manager_detects_npm_in_path_with_spaces_fixture() {
		let manager = detect(&fixture("path with spaces")).unwrap();

		assert_eq!(manager, PackageManager::Npm);
	}

	#[test]
	fn package_manager_detects_npm_in_temp_dir_with_spaces_in_path() {
		let workspace = TempWorkspace::new("dir with spaces");
		workspace.write("package-lock.json", "{}");

		let manager = detect(&workspace.root).unwrap();

		assert_eq!(manager, PackageManager::Npm);
	}

	#[test]
	fn package_manager_path_is_always_pathbuf_not_string_concatenation() {
		let workspace = TempWorkspace::new("spaces in name");
		workspace.write("yarn.lock", "");

		let root: &Path = &workspace.root;
		let manager = detect(root).unwrap();

		assert_eq!(manager, PackageManager::Yarn);
	}

	#[test]
	fn package_manager_detects_bun_in_deeply_nested_path_with_spaces() {
		let workspace = TempWorkspace::new("outer dir/inner dir");
		workspace.write("bun.lock", "");

		let manager = detect(&workspace.root).unwrap();

		assert_eq!(manager, PackageManager::Bun);
	}

	#[test]
	fn package_manager_detects_deno_in_deeply_nested_path_with_spaces() {
		let workspace = TempWorkspace::new("outer dir/inner dir");
		workspace.write("deno.lock", "");

		let manager = detect(&workspace.root).unwrap();

		assert_eq!(manager, PackageManager::Deno);
	}

	#[test]
	fn package_manager_relative_path_resolves_correctly() {
		let abs_path = fixture("npm");

		assert!(abs_path.is_absolute(), "fixture path must be absolute");
		let manager = detect(&abs_path).unwrap();
		assert_eq!(manager, PackageManager::Npm);
	}

	#[cfg(unix)]
	#[test]
	fn package_manager_detects_through_symlinked_workspace_root() {
		use std::os::unix::fs::symlink;

		let workspace = TempWorkspace::new("symlink-target");
		workspace.write("pnpm-lock.yaml", "lockfileVersion: '9.0'\n");

		let nanos = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap_or_default()
			.as_nanos();
		let link_path = std::env::temp_dir().join(format!("zed-knip-symlink-{nanos}"));
		symlink(&workspace.root, &link_path)
			.unwrap_or_else(|error| panic!("failed to create symlink {}: {error}", link_path.display()));

		let manager = detect(&link_path).unwrap();
		let _ = fs::remove_file(&link_path);

		assert_eq!(manager, PackageManager::Pnpm);
	}
}
