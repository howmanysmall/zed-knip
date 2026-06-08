#[test]
fn extension_manifest_rejects_stale_knip_args() {
	let content = include_str!("../extension.toml");
	assert!(
		!content.contains("--cwd"),
		"extension.toml must not declare --cwd in process:exec capability args"
	);
	assert!(
		!content.contains("--config"),
		"extension.toml must not declare --config in process:exec capability args"
	);
	assert!(
		!content.contains("--tsConfig"),
		"extension.toml must not declare --tsConfig in process:exec capability args"
	);
}

#[test]
fn extension_manifest_version_is_0_4_0() {
	let content = include_str!("../extension.toml");
	let version_line = content
		.lines()
		.find(|line| line.trim_start().starts_with("version"))
		.expect("extension.toml must have a version line");
	let version = version_line
		.split('=')
		.nth(1)
		.expect("version line must contain '='")
		.trim()
		.trim_matches('"');
	assert_eq!(version, "0.4.0", "extension.toml version must be 0.4.0, got: {version}");
}
