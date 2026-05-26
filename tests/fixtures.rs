use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

pub fn fixture_path(name: &str) -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("tests")
		.join("fixtures")
		.join(name)
}

#[test]
fn fixture_inventory() {
	let expected = [
		"npm",
		"pnpm",
		"yarn",
		"bun",
		"monorepo",
		"missing-knip",
		"invalid-config",
		"missing-config",
		"multiple-lockfiles",
		"path with spaces",
	];

	let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
	let actual = fs::read_dir(&fixture_root)
		.unwrap_or_else(|error| panic!("failed to read {}: {error}", fixture_root.display()))
		.filter_map(|entry| entry.ok())
		.filter_map(|entry| {
			entry
				.file_type()
				.ok()
				.filter(|file_type| file_type.is_dir())
				.map(|_| entry.file_name().to_string_lossy().into_owned())
		})
		.collect::<BTreeSet<_>>();

	for name in expected {
		assert!(fixture_path(name).is_dir(), "missing fixture directory: {name}");
		assert!(actual.contains(name), "fixture not discoverable: {name}");
	}

	assert_eq!(actual.len(), 12, "unexpected fixture inventory: {actual:?}");
}

#[test]
fn fixture_path_with_spaces_is_accessible_as_pathbuf() {
	let path = fixture_path("path with spaces");

	assert!(path.is_dir(), "path-with-spaces fixture must be a directory");
	assert!(path.is_absolute(), "fixture path must be absolute");
	assert!(
		path.display().to_string().contains(' '),
		"fixture path must contain a space character"
	);
}

#[test]
fn fixture_path_with_spaces_contains_expected_files() {
	let path = fixture_path("path with spaces");
	let package_json = path.join("package.json");
	let lockfile = path.join("package-lock.json");

	assert!(
		package_json.is_file(),
		"path-with-spaces fixture must have package.json"
	);
	assert!(
		lockfile.is_file(),
		"path-with-spaces fixture must have package-lock.json"
	);
}

#[test]
fn fixture_path_join_uses_pathbuf_not_string_concat() {
	let root = fixture_path("path with spaces");
	let nested = root.join("node_modules").join(".bin").join("knip language server");

	assert!(nested.display().to_string().contains(' '));
	assert!(nested.is_absolute());
}
