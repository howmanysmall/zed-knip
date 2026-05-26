const PACKAGE_MANAGER_SOURCE: &str = include_str!("../src/package_manager.rs");
const CONFIG_DETECTION_SOURCE: &str = include_str!("../src/config_detection.rs");
const CACHE_SOURCE: &str = include_str!("../src/cache.rs");
const RESOLVER_SOURCE: &str = include_str!("../src/resolver.rs");
const SETTINGS_SOURCE: &str = include_str!("../src/settings.rs");

const FILE_SCAN_APIS: [&str; 4] = ["read_dir", "WalkDir", "glob(", "**/*"];
const BACKGROUND_POLLING_APIS: [&str; 7] = [
	"spawn(",
	"thread::spawn",
	"tokio::spawn",
	"set_interval",
	"sleep(",
	"watch(",
	"notify::",
];

#[test]
fn perf_package_manager_detection_uses_bounded_named_file_checks() {
	for file_name in [
		"package.json",
		"package-lock.json",
		"pnpm-lock.yaml",
		"yarn.lock",
		"bun.lock",
	] {
		assert!(
			PACKAGE_MANAGER_SOURCE.contains(file_name),
			"package-manager detection must keep checking the named root file {file_name}"
		);
	}

	assert_no_recursive_scan_apis("src/package_manager.rs", PACKAGE_MANAGER_SOURCE);
}

#[test]
fn perf_config_detection_has_no_recursive_file_scan_path() {
	assert_no_recursive_scan_apis("src/config_detection.rs", CONFIG_DETECTION_SOURCE);
}

#[test]
fn perf_cache_invalidation_tracks_only_known_named_inputs() {
	for input in [
		"package_json_mtime",
		"lockfile_mtime",
		"knip_config_mtime",
		"settings_hash",
	] {
		assert!(
			CACHE_SOURCE.contains(input),
			"cache invalidation must stay bounded to known input {input}"
		);
	}

	assert_no_recursive_scan_apis("src/cache.rs", CACHE_SOURCE);
}

#[test]
fn perf_resolver_uses_named_workspace_executable_candidates() {
	assert!(
		RESOLVER_SOURCE.contains("knip-language-server"),
		"resolver must keep checking the named executable candidate knip-language-server"
	);

	assert!(
		!RESOLVER_SOURCE.contains("\"language-server\""),
		"resolver must not fall back to legacy language-server executable names"
	);
	assert!(
		!RESOLVER_SOURCE.contains("KNIP_BIN"),
		"resolver must not start the Knip CLI as an LSP server"
	);

	assert_no_recursive_scan_apis("src/resolver.rs", RESOLVER_SOURCE);
}

#[test]
fn perf_no_background_polling_or_watcher_code_paths_exist() {
	for (path, source) in [
		("src/package_manager.rs", PACKAGE_MANAGER_SOURCE),
		("src/config_detection.rs", CONFIG_DETECTION_SOURCE),
		("src/cache.rs", CACHE_SOURCE),
		("src/resolver.rs", RESOLVER_SOURCE),
		("src/settings.rs", SETTINGS_SOURCE),
	] {
		for api in BACKGROUND_POLLING_APIS {
			assert!(
				!source.contains(api),
				"{path} must not introduce background polling/watcher API `{api}`"
			);
		}
	}
}

#[test]
fn perf_command_builder_documents_single_language_server_process_per_worktree() {
	assert!(
		RESOLVER_SOURCE.contains("build_language_server_command"),
		"resolver must keep command construction centralized"
	);
	assert!(
		RESOLVER_SOURCE.contains("--cwd"),
		"command builder must pass one worktree cwd instead of per-file launches"
	);
	assert!(
		RESOLVER_SOURCE.contains("KNIP_WORKSPACE_ROOT"),
		"command builder must carry one worktree root in process env"
	);
}

fn assert_no_recursive_scan_apis(path: &str, source: &str) {
	for api in FILE_SCAN_APIS {
		assert!(
			!source.contains(api),
			"{path} must not use recursive/all-file scan API `{api}`"
		);
	}
}
