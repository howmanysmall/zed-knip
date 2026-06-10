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
	let settings_section = content
		.split("## Settings")
		.nth(1)
		.expect("README must have a 'Settings' section");
	// `package_manager` is excluded: collides with module name `package_manager.rs`.
	for removed in ["server_path", "log_level"] {
		assert!(
			!settings_section.contains(removed),
			"README Settings section must not list removed setting '{removed}'"
		);
	}
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

#[test]
fn readme_documents_supported_knip_settings_only() {
	let content = include_str!("../README.md");
	let settings_section = content
		.split("## Settings")
		.nth(1)
		.expect("README must have a 'Settings' section");

	for setting in [
		"auto_install",
		"config_path",
		"require_config",
		"ts_config_path",
		"binary.path",
		"preprocessor",
		"preprocessor_options",
	] {
		assert!(
			settings_section.contains(setting),
			"README Settings must list '{}'",
			setting
		);
	}

	let has_diagnostics_filter = settings_section.contains("diagnostics filters")
		|| settings_section.contains("filter diagnostics")
		|| settings_section.contains("include/exclude diagnostics")
		|| settings_section.contains("diagnostics.include_issue_types")
		|| settings_section.contains("diagnostics.exclude_issue_types")
		|| settings_section.contains("diagnostics.severity_by_issue_type")
		|| settings_section.contains("diagnostics.exclude_path_prefixes");

	assert!(
		has_diagnostics_filter,
		"README must document diagnostics filter settings (include/exclude/severity)"
	);

	assert!(
		content.contains("ts_config_path"),
		"README must mention 'ts_config_path'"
	);
	assert!(
		content.to_lowercase().contains("managed install") || content.to_lowercase().contains("managed"),
		"README must mention 'managed install' or 'managed' in context of advanced settings"
	);
}

#[test]
fn readme_documents_diagnostic_filter_semantics() {
	let content = include_str!("../README.md");

	for key in [
		"include_issue_types",
		"exclude_issue_types",
		"exclude_path_prefixes",
		"severity_by_issue_type",
	] {
		assert!(
			content.contains(key),
			"README must mention diagnostic filter key '{}'",
			key
		);
	}

	let issue_types = [
		"files",
		"dependencies",
		"devDependencies",
		"unlisted",
		"binaries",
		"unresolved",
		"exports",
		"types",
		"duplicates",
		"nsExports",
		"nsTypes",
		"enumMembers",
		"namespaceMembers",
		"catalog",
	];
	let found_issue_types = issue_types.iter().filter(|&&it| content.contains(it)).count();
	assert!(
		found_issue_types >= 5,
		"README must list at least 5 valid issue types, found only {}",
		found_issue_types
	);

	let severities = ["error", "warn", "info", "hint", "off"];
	for sev in severities {
		let mut found = false;
		let mut start = 0;
		while let Some(pos) = content[start..].find("severity") {
			let abs_pos = start + pos;
			let window_start = abs_pos.saturating_sub(200);
			let window_end = (abs_pos + 200).min(content.len());
			if content[window_start..window_end].contains(sev) {
				found = true;
				break;
			}
			start = abs_pos + 1;
		}
		assert!(
			found,
			"README must list severity value '{}' within 200 chars of 'severity'",
			sev
		);
	}
}

#[test]
fn readme_rejects_unsupported_knip_claim_about_cli_args() {
	let content = include_str!("../README.md");
	assert!(
		!content.contains("--tsConfig"),
		"README must not claim --tsConfig is usable (should be rejected/omitted)"
	);
	assert!(
		!content.contains("--reporter"),
		"README must not claim --reporter is usable"
	);
}

/// Preprocessor settings may be documented, but README must not instruct users
/// to set preprocessors via `lsp.knip.binary.arguments` or launch args.
#[test]
fn readme_preprocessor_must_not_guide_launch_args() {
	let content = include_str!("../README.md");
	assert!(
		!content.contains("--preprocessor"),
		"README must not mention --preprocessor flag usage"
	);
	// Proximity check: binary.arguments and preprocessor within 200 chars indicates misleading guidance
	let content_lower = content.to_lowercase();
	let mut search_pos = 0_usize;
	while let Some(pos) = content_lower[search_pos..].find("preprocessor") {
		let abs_pos = search_pos + pos;
		let window_start = abs_pos.saturating_sub(200);
		let window_end = (abs_pos + "preprocessor".len() + 200).min(content.len());
		let window = &content[window_start..window_end];
		if window.contains("binary.arguments") {
			panic!("README must not guide users to set preprocessors via lsp.knip.binary.arguments");
		}
		search_pos = abs_pos + 1;
	}
}

#[test]
fn extension_manifest_process_args_uses_stdio_only() {
	let content = include_str!("../extension.toml");
	assert!(
		content.contains("args = [\"--stdio\"]"),
		"extension.toml process exec args must be exactly [\"--stdio\"]"
	);
	assert!(
		!content.contains("**"),
		"extension.toml process exec line must not contain wildcard '**'"
	);
}

#[test]
fn extension_manifest_capability_section_documented() {
	let content = include_str!("../extension.toml");
	assert!(
		content.contains("[language_servers]") || content.contains("language_servers"),
		"extension.toml must contain language_servers section"
	);
}

#[test]
fn readme_supported_settings_match_settings_rs() {
	let content = include_str!("../README.md");
	for setting in [
		"auto_install",
		"config_path",
		"require_config",
		"ts_config_path",
		"binary.path",
	] {
		assert!(
			content.contains(setting),
			"README must mention supported setting '{}'",
			setting
		);
	}
}
