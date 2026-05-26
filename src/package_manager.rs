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
	if workspace_root.join("bun.lockb").is_file() {
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
		"bun.lockb" => Some(PackageManager::Bun),
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::{detect, PackageManager, PackageManagerError};
	use std::path::{Path, PathBuf};

	fn fixture(name: &str) -> PathBuf {
		Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
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
}
