use crate::errors::KnipError;
use std::{path::PathBuf, time::SystemTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeCache {
	pub worktree_root: PathBuf,
	pub executable_path: Option<PathBuf>,
	pub package_manager: Option<String>,
	pub config_path: Option<PathBuf>,
	pub version: Option<String>,
	pub install_source: InstallSource,
	pub last_error: Option<String>,
	pub invalidation_inputs: InvalidationInputs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InvalidationInputs {
	pub package_json_mtime: Option<SystemTime>,
	pub lockfile_mtime: Option<SystemTime>,
	pub knip_config_mtime: Option<SystemTime>,
	pub settings_hash: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallSource {
	WorkspaceLocal,
	ManagedCache,
	ExplicitPath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheState {
	Hit,
	Miss,
	Stale(StaleReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaleReason {
	PackageChanged,
	ConfigChanged,
	SettingsChanged,
	Corrupt,
	ReadOnly,
}

impl WorktreeCache {
	pub fn new(worktree_root: PathBuf) -> Self {
		Self {
			worktree_root,
			executable_path: None,
			package_manager: None,
			config_path: None,
			version: None,
			install_source: InstallSource::WorkspaceLocal,
			last_error: None,
			invalidation_inputs: InvalidationInputs::default(),
		}
	}

	pub fn check_validity(&self, current: &InvalidationInputs) -> CacheState {
		if self.executable_path.is_none() || self.package_manager.is_none() || self.config_path.is_none() {
			return CacheState::Miss;
		}

		if self.invalidation_inputs.package_json_mtime != current.package_json_mtime
			|| self.invalidation_inputs.lockfile_mtime != current.lockfile_mtime
		{
			return CacheState::Stale(StaleReason::PackageChanged);
		}

		if self.invalidation_inputs.knip_config_mtime != current.knip_config_mtime {
			return CacheState::Stale(StaleReason::ConfigChanged);
		}

		if self.invalidation_inputs.settings_hash != current.settings_hash {
			return CacheState::Stale(StaleReason::SettingsChanged);
		}

		CacheState::Hit
	}

	pub fn mark_corrupt(&self) -> KnipError {
		KnipError::CorruptCache {
			path: self.worktree_root.clone(),
			detail: "Cache contents failed validation".to_string(),
		}
	}

	pub fn mark_read_only(path: PathBuf) -> KnipError {
		KnipError::ReadOnlyCache { path }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn time(seconds: u64) -> SystemTime {
		SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(seconds)
	}

	fn valid_cache() -> WorktreeCache {
		WorktreeCache {
			worktree_root: PathBuf::from("/workspace"),
			executable_path: Some(PathBuf::from("/workspace/node_modules/.bin/knip")),
			package_manager: Some("pnpm".to_string()),
			config_path: Some(PathBuf::from("/workspace/knip.json")),
			version: Some("5.0.0".to_string()),
			install_source: InstallSource::WorkspaceLocal,
			last_error: None,
			invalidation_inputs: InvalidationInputs {
				package_json_mtime: Some(time(10)),
				lockfile_mtime: Some(time(20)),
				knip_config_mtime: Some(time(30)),
				settings_hash: 42,
			},
		}
	}

	mod cache_invalidation {
		use super::*;

		#[test]
		fn cache_hit_when_inputs_unchanged() {
			let cache = valid_cache();
			let current = cache.invalidation_inputs;

			assert_eq!(cache.check_validity(&current), CacheState::Hit);
		}

		#[test]
		fn cache_miss_when_entry_incomplete() {
			let mut cache = valid_cache();
			cache.executable_path = None;

			assert_eq!(cache.check_validity(&cache.invalidation_inputs), CacheState::Miss);
		}

		#[test]
		fn cache_stale_on_package_json_change() {
			let cache = valid_cache();
			let mut current = cache.invalidation_inputs;
			current.package_json_mtime = Some(time(11));

			assert_eq!(
				cache.check_validity(&current),
				CacheState::Stale(StaleReason::PackageChanged)
			);
		}

		#[test]
		fn cache_stale_on_config_change() {
			let cache = valid_cache();
			let mut current = cache.invalidation_inputs;
			current.knip_config_mtime = Some(time(31));

			assert_eq!(
				cache.check_validity(&current),
				CacheState::Stale(StaleReason::ConfigChanged)
			);
		}

		#[test]
		fn cache_stale_on_settings_change() {
			let cache = valid_cache();
			let mut current = cache.invalidation_inputs;
			current.settings_hash = 99;

			assert_eq!(
				cache.check_validity(&current),
				CacheState::Stale(StaleReason::SettingsChanged)
			);
		}

		#[test]
		fn perf_cache_invalidation_is_limited_to_known_inputs() {
			let cache = valid_cache();
			let current = InvalidationInputs {
				package_json_mtime: Some(time(10)),
				lockfile_mtime: Some(time(20)),
				knip_config_mtime: Some(time(30)),
				settings_hash: 42,
			};

			assert_eq!(cache.check_validity(&current), CacheState::Hit);
		}

		#[test]
		fn cache_is_isolated_by_monorepo_worktree_root() {
			let root_cache = WorktreeCache::new(PathBuf::from("/repo"));
			let nested_cache = WorktreeCache::new(PathBuf::from("/repo/packages/app"));

			assert_ne!(root_cache.worktree_root, nested_cache.worktree_root);
		}

		#[test]
		fn cache_validity_does_not_reuse_inputs_from_another_worktree() {
			let mut root_cache = valid_cache();
			root_cache.worktree_root = PathBuf::from("/repo");
			root_cache.executable_path = Some(PathBuf::from("/repo/node_modules/.bin/knip"));
			root_cache.config_path = Some(PathBuf::from("/repo/knip.json"));

			let nested_inputs = InvalidationInputs {
				package_json_mtime: Some(time(11)),
				lockfile_mtime: Some(time(21)),
				knip_config_mtime: Some(time(31)),
				settings_hash: 42,
			};

			assert_eq!(
				root_cache.check_validity(&nested_inputs),
				CacheState::Stale(StaleReason::PackageChanged)
			);
		}
	}

	mod cache_corrupt {
		use super::*;

		#[test]
		fn cache_corrupt_returns_actionable_error() {
			let error = valid_cache().mark_corrupt();
			let message = error.to_string();

			assert!(message.contains("/workspace"));
			assert!(message.contains("Delete"));
		}

		#[test]
		fn cache_read_only_returns_actionable_error() {
			let error = WorktreeCache::mark_read_only(PathBuf::from("/workspace/.cache/knip"));
			let message = error.to_string();

			assert!(message.contains("/workspace/.cache/knip"));
			assert!(message.contains("writable"));
		}
	}
}
