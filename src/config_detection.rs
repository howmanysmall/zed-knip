use std::path::{Path, PathBuf};

const CONFIG_FILE_NAMES: &[&str] = &[
	"knip.json",
	"knip.jsonc",
	"knip.ts",
	"knip.mts",
	"knip.cts",
	"knip.js",
	"knip.mjs",
	"knip.cjs",
	"knip.config.json",
	"knip.config.jsonc",
	"knip.config.ts",
	"knip.config.mts",
	"knip.config.cts",
	"knip.config.js",
	"knip.config.mjs",
	"knip.config.cjs",
	".kniprc",
	".kniprc.json",
	".kniprc.jsonc",
	".kniprc.yaml",
	".kniprc.yml",
];

pub fn detect_config(workspace_root: &Path) -> Option<PathBuf> {
	CONFIG_FILE_NAMES
		.iter()
		.map(|file_name| workspace_root.join(file_name))
		.find(|candidate| candidate.is_file())
}

pub fn known_config_file_names() -> &'static [&'static str] {
	CONFIG_FILE_NAMES
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::{
		fs,
		time::{SystemTime, UNIX_EPOCH},
	};

	#[derive(Debug)]
	struct TestWorkspace {
		root: PathBuf,
	}

	impl TestWorkspace {
		fn new(name: &str) -> Self {
			let nanos = SystemTime::now()
				.duration_since(UNIX_EPOCH)
				.unwrap_or_default()
				.as_nanos();
			let root = std::env::temp_dir().join(format!("zed-knip-config-{name}-{nanos}"));
			fs::create_dir_all(&root).unwrap_or_else(|error| panic!("failed to create {}: {error}", root.display()));
			Self { root }
		}

		fn write(&self, relative_path: &str) -> PathBuf {
			let path = self.root.join(relative_path);
			if let Some(parent) = path.parent() {
				fs::create_dir_all(parent)
					.unwrap_or_else(|error| panic!("failed to create {}: {error}", parent.display()));
			}
			fs::write(&path, "{}\n").unwrap_or_else(|error| panic!("failed to write {}: {error}", path.display()));
			path
		}
	}

	impl Drop for TestWorkspace {
		fn drop(&mut self) {
			let _ = fs::remove_dir_all(&self.root);
		}
	}

	#[test]
	fn detect_config_finds_first_supported_root_file() {
		let workspace = TestWorkspace::new("first-supported");
		let expected = workspace.write("knip.ts");
		workspace.write("knip.config.js");

		assert_eq!(detect_config(&workspace.root), Some(expected));
	}

	#[test]
	fn detect_config_supports_dot_kniprc_variants() {
		let workspace = TestWorkspace::new("dot-kniprc");
		let expected = workspace.write(".kniprc");

		assert_eq!(detect_config(&workspace.root), Some(expected));
	}

	#[test]
	fn detect_config_ignores_nested_files_to_keep_checks_bounded() {
		let workspace = TestWorkspace::new("nested");
		workspace.write("packages/app/knip.json");

		assert_eq!(detect_config(&workspace.root), None);
	}

	#[test]
	fn known_config_file_names_are_bounded_named_root_checks() {
		assert!(known_config_file_names().contains(&"knip.json"));
		assert!(known_config_file_names().contains(&".kniprc"));
		assert!(known_config_file_names()
			.iter()
			.all(|file_name| !file_name.contains('/')));
	}
}
