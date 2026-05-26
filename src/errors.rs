use std::{fmt, path::PathBuf};

#[cfg(test)]
mod tests {
	use super::KnipError;
	use std::path::PathBuf;

	#[test]
	fn error_missing_knip_display_mentions_workspace_install_path_and_managed_install() {
		let error = KnipError::MissingKnip {
			workspace_root: PathBuf::from("/workspace"),
		};

		assert_eq!(
			error.to_string(),
			"Knip is not installed for workspace /workspace. Install Knip in the workspace, set the explicit Knip path in settings, or enable the managed install option."
		);
	}

	#[test]
	fn error_missing_knip_invalid_explicit_path_display() {
		let error = KnipError::InvalidExplicitPath {
			path: PathBuf::from("/workspace/bin/knip"),
		};

		assert_eq!(
			error.to_string(),
			"The configured Knip path /workspace/bin/knip is invalid. Update the explicit path setting to a valid Knip executable."
		);
	}

	#[test]
	fn error_missing_knip_failed_managed_install_display() {
		let error = KnipError::FailedManagedInstall {
			reason: "download timed out".to_string(),
		};

		assert_eq!(
			error.to_string(),
			"Managed Knip install failed: download timed out. Fix the error above, then retry the managed install."
		);
	}

	#[test]
	fn error_missing_knip_network_unavailable_display() {
		let error = KnipError::NetworkUnavailable {
			detail: "proxy denied the request".to_string(),
		};

		assert_eq!(
			error.to_string(),
			"Knip could not be downloaded because the network is unavailable: proxy denied the request. Check your connection, proxy, or offline mode, then retry."
		);
	}

	#[test]
	fn error_missing_knip_read_only_cache_display() {
		let error = KnipError::ReadOnlyCache {
			path: PathBuf::from("/workspace/.cache/knip"),
		};

		assert_eq!(
			error.to_string(),
			"Cannot write to the Knip cache at /workspace/.cache/knip. Make the cache directory writable or move the cache to a writable location."
		);
	}

	#[test]
	fn error_missing_knip_language_server_crash_display() {
		let error = KnipError::LanguageServerCrash { exit_code: Some(137) };

		assert_eq!(
			error.to_string(),
			"Knip language server crashed with exit code 137. Restart the language server or reload the workspace after fixing the crash."
		);
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnipError {
	MissingKnip { workspace_root: PathBuf },
	InvalidExplicitPath { path: PathBuf },
	NonExecutablePath { path: PathBuf },
	UnsupportedPackageManager { found: String },
	FailedManagedInstall { reason: String },
	NetworkUnavailable { detail: String },
	ReadOnlyCache { path: PathBuf },
	CorruptCache { path: PathBuf, detail: String },
	InvalidConfig { path: PathBuf, reason: String },
	LanguageServerCrash { exit_code: Option<i32> },
	UnsupportedWorkspace { reason: String },
	AmbiguousPackageManager { found: Vec<String> },
}

impl fmt::Display for KnipError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::MissingKnip { workspace_root } => write!(
				f,
				"Knip is not installed for workspace {workspace_root}. Install Knip in the workspace, set the explicit Knip path in settings, or enable the managed install option.",
				workspace_root = workspace_root.display()
			),
			Self::InvalidExplicitPath { path } => write!(
				f,
				"The configured Knip path {path} is invalid. Update the explicit path setting to a valid Knip executable.",
				path = path.display()
			),
			Self::NonExecutablePath { path } => write!(
				f,
				"The configured Knip path {path} is not executable. Make it executable (for example, chmod +x) or update the explicit path setting.",
				path = path.display()
			),
			Self::UnsupportedPackageManager { found } => write!(
				f,
				"Unsupported package manager {found}. Use a supported package manager or set the package manager explicitly in settings."
			),
			Self::FailedManagedInstall { reason } => write!(
				f,
				"Managed Knip install failed: {reason}. Fix the error above, then retry the managed install."
			),
			Self::NetworkUnavailable { detail } => write!(
				f,
				"Knip could not be downloaded because the network is unavailable: {detail}. Check your connection, proxy, or offline mode, then retry."
			),
			Self::ReadOnlyCache { path } => write!(
				f,
				"Cannot write to the Knip cache at {path}. Make the cache directory writable or move the cache to a writable location.",
				path = path.display()
			),
			Self::CorruptCache { path, detail } => write!(
				f,
				"The Knip cache at {path} is corrupt: {detail}. Delete the cache directory so it can be rebuilt.",
				path = path.display()
			),
			Self::InvalidConfig { path, reason } => write!(
				f,
				"Invalid Knip config at {path}: {reason}. Fix the config file and try again.",
				path = path.display()
			),
			Self::LanguageServerCrash { exit_code } => match exit_code {
				Some(exit_code) => write!(
					f,
					"Knip language server crashed with exit code {exit_code}. Restart the language server or reload the workspace after fixing the crash."
				),
				None => write!(
					f,
					"Knip language server crashed. Restart the language server or reload the workspace after fixing the crash."
				),
			},
			Self::UnsupportedWorkspace { reason } => write!(
				f,
				"This workspace is not supported: {reason}. Open the project root or adjust the workspace so Knip can detect it."
			),
			Self::AmbiguousPackageManager { found } => write!(
				f,
				"Multiple package managers were detected ({}). Remove the extra lockfile(s) or set the package manager explicitly in settings.",
				found.join(", ")
			),
		}
	}
}

impl std::error::Error for KnipError {}
