use std::{fmt, fs, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
	Npm,
	Pnpm,
	Yarn,
	Bun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageManagerError {
	NotFound,
	Ambiguous { found: Vec<String> },
	UnsupportedPackageManager { found: String },
}

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

pub fn detect(workspace_root: &Path) -> Result<PackageManager, PackageManagerError> {
	if let Some(manager) = detect_from_package_json(workspace_root) {
		return manager;
	}

	let mut found = Vec::new();

	if workspace_root.join("package-lock.json").is_file() {
		found.push("package-lock.json".to_string());
	}
	if workspace_root.join("pnpm-lock.yaml").is_file() {
		found.push("pnpm-lock.yaml".to_string());
	}
	if workspace_root.join("yarn.lock").is_file() {
		found.push("yarn.lock".to_string());
	}
	if workspace_root.join("bun.lock").is_file() {
		found.push("bun.lock".to_string());
	} else if workspace_root.join("bun.lockb").is_file() {
		found.push("bun.lockb".to_string());
	}

	match found.as_slice() {
		[] => Err(PackageManagerError::NotFound),
		[lockfile] => match lockfile_to_manager(lockfile) {
			Some(manager) => Ok(manager),
			None => Err(PackageManagerError::NotFound),
		},
		_ => Err(PackageManagerError::Ambiguous { found }),
	}
}

fn detect_from_package_json(workspace_root: &Path) -> Option<Result<PackageManager, PackageManagerError>> {
	let package_json = workspace_root.join("package.json");
	let contents = fs::read_to_string(package_json).ok()?;
	let manager = extract_package_manager_field(&contents)?;
	Some(parse_package_manager(&manager))
}

fn extract_package_manager_field(contents: &str) -> Option<String> {
	let key = "\"packageManager\"";
	let start = contents.find(key)? + key.len();
	let after_key = contents[start..].find(':')? + start + 1;
	let value = contents[after_key..].trim_start();
	let value = value.strip_prefix('"')?;
	let end = value.find('"')?;
	Some(value[..end].to_string())
}

fn parse_package_manager(value: &str) -> Result<PackageManager, PackageManagerError> {
	let manager = value.split_once('@').map(|(name, _)| name).unwrap_or(value);
	match manager {
		"npm" => Ok(PackageManager::Npm),
		"pnpm" => Ok(PackageManager::Pnpm),
		"yarn" => Ok(PackageManager::Yarn),
		"bun" => Ok(PackageManager::Bun),
		_ => Err(PackageManagerError::UnsupportedPackageManager {
			found: value.to_string(),
		}),
	}
}

fn lockfile_to_manager(lockfile: &str) -> Option<PackageManager> {
	match lockfile {
		"package-lock.json" => Some(PackageManager::Npm),
		"pnpm-lock.yaml" => Some(PackageManager::Pnpm),
		"yarn.lock" => Some(PackageManager::Yarn),
		"bun.lock" => Some(PackageManager::Bun),
		"bun.lockb" => Some(PackageManager::Bun),
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::{PackageManager, PackageManagerError, detect};
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
	fn package_manager_detects_package_manager_field() {
		let manager = detect(&fixture("package-manager-field")).unwrap();

		assert_eq!(manager, PackageManager::Pnpm);
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
