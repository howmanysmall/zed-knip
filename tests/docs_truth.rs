#[test]
fn readme_does_not_claim_hover_support() {
	let content = include_str!("../README.md");
	assert!(
		!content.contains("Hover Support"),
		"README must not claim 'Hover Support' as a feature"
	);
	assert!(
		!content.to_lowercase().contains("hover"),
		"README must not mention hover capability"
	);
}

#[test]
fn readme_does_not_list_removed_settings() {
	let content = include_str!("../README.md");
	assert!(
		!content.contains("server_path"),
		"README must not list removed setting 'server_path'"
	);
	assert!(
		!content.contains("log_level"),
		"README must not list removed setting 'log_level'"
	);
	assert!(
		!content.contains("package_manager"),
		"README must not list removed setting 'package_manager'"
	);
}

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
