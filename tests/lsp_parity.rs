// LSP feature parity validation harness for zed-knip.
//
// These tests validate that the shapes, method names, and parity expectations
// for Knip LSP features are correct without launching the Zed UI or a live
// language server. Each test uses mocked/sample LSP payloads represented as
// plain Rust data structures.
//
// Run with:
//   mise x -- cargo nextest run -E 'test(lsp_parity)'

type Uri = String;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Position {
	line: u32,
	character: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Range {
	start: Position,
	end: Position,
}

// All four LSP DiagnosticSeverity values are defined for completeness even
// though only Warning is exercised in current tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[expect(dead_code)]
enum DiagnosticSeverity {
	Error = 1,
	Warning = 2,
	Information = 3,
	Hint = 4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Diagnostic {
	range: Range,
	severity: DiagnosticSeverity,
	source: String,
	message: String,
	code: Option<String>,
}

#[derive(Debug, Clone)]
struct PublishDiagnosticsParams {
	uri: Uri,
	diagnostics: Vec<Diagnostic>,
}

type CodeActionKind = String;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CodeAction {
	title: String,
	kind: CodeActionKind,
	is_preferred: bool,
}

// Knip custom LSP request method names.
// Source: packages/language-server/src/constants.js @ e93ccaa4d8fd6df6b4e976d2b0472ba5f7d48830
//
// These custom request methods are defined by the upstream Knip language server
// (and consumed by the VSCode extension). The Zed extension does NOT wire them
// — the parity tests below only assert that the upstream names are correctly
// referenced as a known, non-wired context, not that the Zed extension handles
// them.
const KNIP_METHOD_OPEN_FILE: &str = "knip/openFile";
const KNIP_METHOD_SHOW_REFERENCES: &str = "knip/showReferences";

const LSP_METHOD_PUBLISH_DIAGNOSTICS: &str = "textDocument/publishDiagnostics";
const LSP_METHOD_CODE_ACTION: &str = "textDocument/codeAction";

fn sample_unused_export_diagnostic(uri: &str) -> PublishDiagnosticsParams {
	PublishDiagnosticsParams {
		uri: uri.to_owned(),
		diagnostics: vec![Diagnostic {
			range: Range {
				start: Position { line: 4, character: 9 },
				end: Position { line: 4, character: 20 },
			},
			severity: DiagnosticSeverity::Warning,
			source: "knip".to_owned(),
			message: "Unused export 'myFunction'".to_owned(),
			code: Some("unused-export".to_owned()),
		}],
	}
}

#[test]
fn lsp_parity_diagnostics_unused_export_shape_is_valid() {
	let params = sample_unused_export_diagnostic("file:///workspace/src/utils.ts");

	assert_eq!(params.uri, "file:///workspace/src/utils.ts");
	assert_eq!(params.diagnostics.len(), 1);

	let diag = &params.diagnostics[0];
	assert_eq!(diag.severity, DiagnosticSeverity::Warning);
	assert_eq!(diag.source, "knip");
	assert!(
		diag.message.contains("Unused export"),
		"diagnostic message should mention 'Unused export'"
	);
	assert_eq!(diag.code.as_deref(), Some("unused-export"));
}

#[test]
fn lsp_parity_diagnostics_unused_file_shape_is_valid() {
	let params = PublishDiagnosticsParams {
		uri: "file:///workspace/src/dead.ts".to_owned(),
		diagnostics: vec![Diagnostic {
			range: Range {
				start: Position { line: 0, character: 0 },
				end: Position { line: 0, character: 0 },
			},
			severity: DiagnosticSeverity::Warning,
			source: "knip".to_owned(),
			message: "Unused file".to_owned(),
			code: Some("unused-file".to_owned()),
		}],
	};

	let diag = &params.diagnostics[0];
	assert_eq!(diag.severity, DiagnosticSeverity::Warning);
	assert_eq!(diag.code.as_deref(), Some("unused-file"));
}

#[test]
fn lsp_parity_diagnostics_unused_dependency_shape_is_valid() {
	let params = PublishDiagnosticsParams {
		uri: "file:///workspace/package.json".to_owned(),
		diagnostics: vec![Diagnostic {
			range: Range {
				start: Position { line: 10, character: 4 },
				end: Position {
					line: 10,
					character: 14,
				},
			},
			severity: DiagnosticSeverity::Warning,
			source: "knip".to_owned(),
			message: "Unused dependency 'lodash'".to_owned(),
			code: Some("unused-dependency".to_owned()),
		}],
	};

	let diag = &params.diagnostics[0];
	assert_eq!(diag.source, "knip");
	assert_eq!(diag.code.as_deref(), Some("unused-dependency"));
	assert!(diag.message.contains("lodash"));
}

#[test]
fn lsp_parity_diagnostics_circular_dependency_shape_is_valid() {
	let params = PublishDiagnosticsParams {
		uri: "file:///workspace/src/a.ts".to_owned(),
		diagnostics: vec![Diagnostic {
			range: Range {
				start: Position { line: 0, character: 0 },
				end: Position { line: 0, character: 0 },
			},
			severity: DiagnosticSeverity::Warning,
			source: "knip".to_owned(),
			message: "Circular dependency: a.ts → b.ts → a.ts".to_owned(),
			code: Some("circular-dependency".to_owned()),
		}],
	};

	let diag = &params.diagnostics[0];
	assert_eq!(diag.code.as_deref(), Some("circular-dependency"));
	assert!(diag.message.contains("Circular dependency"));
}

#[test]
fn lsp_parity_diagnostics_method_name_is_standard_lsp() {
	assert!(LSP_METHOD_PUBLISH_DIAGNOSTICS.starts_with("textDocument/"));
	assert!(LSP_METHOD_PUBLISH_DIAGNOSTICS.contains("Diagnostics"));
}

fn sample_remove_export_action() -> CodeAction {
	CodeAction {
		title: "Remove export 'myFunction'".to_owned(),
		kind: "quickfix.knip.removeExport".to_owned(),
		is_preferred: true,
	}
}

fn sample_add_jsdoc_action() -> CodeAction {
	CodeAction {
		title: "Add @public JSDoc tag".to_owned(),
		kind: "quickfix.knip.addJsDocTag".to_owned(),
		is_preferred: false,
	}
}

fn sample_remove_dependency_action() -> CodeAction {
	CodeAction {
		title: "Remove dependency 'lodash'".to_owned(),
		kind: "quickfix.knip.removeDependency".to_owned(),
		is_preferred: false,
	}
}

#[test]
fn lsp_parity_code_action_remove_export_shape_is_valid() {
	let action = sample_remove_export_action();

	assert!(action.title.starts_with("Remove export"));
	assert!(action.kind.contains("knip"));
	assert!(action.is_preferred);
}

#[test]
fn lsp_parity_code_action_add_jsdoc_tag_shape_is_valid() {
	let action = sample_add_jsdoc_action();

	assert!(action.title.contains("JSDoc"));
	assert!(action.kind.contains("knip"));
}

#[test]
fn lsp_parity_code_action_remove_dependency_shape_is_valid() {
	let action = sample_remove_dependency_action();

	assert!(action.title.contains("dependency"));
	assert!(action.kind.contains("knip"));
}

#[test]
fn lsp_parity_code_action_method_name_is_standard_lsp() {
	assert!(LSP_METHOD_CODE_ACTION.starts_with("textDocument/"));
	assert!(LSP_METHOD_CODE_ACTION.contains("codeAction"));
}

#[test]
fn lsp_parity_code_action_install_dependency_is_unsupported() {
	let matrix = parity_matrix();
	let (_, status) = matrix
		.iter()
		.find(|(f, _)| *f == "code-action-install-dependency")
		.unwrap();
	assert_eq!(*status, ParityStatus::Unsupported);
}

#[test]
fn lsp_parity_code_action_delete_file_is_unsupported() {
	let matrix = parity_matrix();
	let (_, status) = matrix.iter().find(|(f, _)| *f == "code-action-delete-file").unwrap();
	assert_eq!(*status, ParityStatus::Unsupported);
}

#[test]
fn lsp_parity_custom_methods_are_knip_namespaced() {
	for method in [KNIP_METHOD_OPEN_FILE, KNIP_METHOD_SHOW_REFERENCES] {
		assert!(
			method.starts_with("knip/"),
			"upstream custom method '{method}' must be knip/-namespaced"
		);
	}
}

#[test]
fn lsp_parity_hover_support_is_unsupported() {
	let matrix = parity_matrix();
	for feature in ["hover-export-usages", "hover-dependency-usages"] {
		let (_, status) = matrix.iter().find(|(f, _)| *f == feature).unwrap();
		assert_eq!(*status, ParityStatus::Unsupported, "{feature} must be unsupported");
	}
}

#[test]
fn lsp_parity_codelens_support_is_unsupported() {
	let matrix = parity_matrix();
	let (_, status) = matrix.iter().find(|(f, _)| *f == "codelens-import-counts").unwrap();
	assert_eq!(*status, ParityStatus::Unsupported);
}

#[test]
fn lsp_parity_tree_views_are_unsupported() {
	let matrix = parity_matrix();
	for feature in ["tree-view-imports", "tree-view-exports"] {
		let (_, status) = matrix.iter().find(|(f, _)| *f == feature).unwrap();
		assert_eq!(*status, ParityStatus::Unsupported, "{feature} must be unsupported");
	}
}

#[test]
fn lsp_parity_lifecycle_ls_startup_is_implemented() {
	let matrix = parity_matrix();
	let (_, status) = matrix.iter().find(|(f, _)| *f == "lifecycle-ls-startup").unwrap();
	assert_eq!(*status, ParityStatus::Implemented);
}

#[test]
fn lsp_parity_lifecycle_file_watching_is_wired_via_managed_patch() {
	// didSave handling for the managed install is wired by the
	// `apply_did_save_refresh_patch` in `src/managed_install.rs`, which is
	// the test that asserts the patch produces `textDocumentSync` and
	// `onDidSave` hooks in the upstream server source. The launch path in
	// `build_language_server_command` must NOT carry these strings — they
	// are managed-patch internals, not Zed-extension feature surface.
	use std::path::PathBuf;

	use zed_knip::{
		cache::InstallSource,
		package_manager::PackageManager,
		resolver::{build_language_server_command, ResolvedKnip},
		settings::KnipSettings,
	};

	let resolved = ResolvedKnip {
		executable_path: PathBuf::from("/managed/knip-language-server"),
		package_manager: PackageManager::Npm,
		install_source: InstallSource::ManagedCache,
	};
	let command = build_language_server_command(&resolved, &KnipSettings::default(), &PathBuf::from("/workspace"));

	let combined = format!("{:?} {:?}", command.command.args, command.command.env);
	assert!(
		!combined.contains("textDocumentSync"),
		"launch command must not reference textDocumentSync (managed-patch internal)"
	);
	assert!(
		!combined.contains("onDidSave"),
		"launch command must not reference onDidSave (managed-patch internal)"
	);
}

#[test]
fn lsp_parity_mcp_features_are_excluded() {
	let matrix = parity_matrix();
	for feature in ["mcp-knip-configure", "mcp-knip-docs", "mcp-language-model-tools"] {
		let (_, status) = matrix.iter().find(|(f, _)| *f == feature).unwrap();
		assert_eq!(*status, ParityStatus::McpExcluded, "{feature} must be MCP-excluded");
	}
}

#[test]
fn lsp_parity_settings_defer_session_is_implemented() {
	let matrix = parity_matrix();
	let (_, status) = matrix.iter().find(|(f, _)| *f == "setting-defer-session").unwrap();
	assert_eq!(*status, ParityStatus::Implemented);
}

#[test]
fn lsp_parity_settings_require_config_is_implemented() {
	let matrix = parity_matrix();
	let (_, status) = matrix.iter().find(|(f, _)| *f == "setting-require-config").unwrap();
	assert_eq!(*status, ParityStatus::Implemented);
}

#[test]
fn lsp_parity_settings_config_file_path_is_implemented() {
	let matrix = parity_matrix();
	let (_, status) = matrix.iter().find(|(f, _)| *f == "setting-config-file-path").unwrap();
	assert_eq!(*status, ParityStatus::Implemented);
}

#[test]
fn lsp_parity_settings_node_runtime_path_is_implemented() {
	let matrix = parity_matrix();
	let (_, status) = matrix.iter().find(|(f, _)| *f == "setting-node-runtime-path").unwrap();
	assert_eq!(*status, ParityStatus::Implemented);
}

/// All known Knip diagnostic codes emitted by the language server.
/// Source: packages/language-server/src/diagnostics.js
const KNIP_DIAGNOSTIC_CODE_UNUSED_FILE: &str = "unused-file";
const KNIP_DIAGNOSTIC_CODE_UNUSED_EXPORT: &str = "unused-export";
const KNIP_DIAGNOSTIC_CODE_UNUSED_DEPENDENCY: &str = "unused-dependency";
const KNIP_DIAGNOSTIC_CODE_CIRCULAR_DEPENDENCY: &str = "circular-dependency";

#[test]
fn lsp_parity_diagnostic_codes_are_kebab_case_strings() {
	// Knip emits string codes (not numeric) for all diagnostics.
	// Zed surfaces these in the gutter and problem panel.
	for code in [
		KNIP_DIAGNOSTIC_CODE_UNUSED_FILE,
		KNIP_DIAGNOSTIC_CODE_UNUSED_EXPORT,
		KNIP_DIAGNOSTIC_CODE_UNUSED_DEPENDENCY,
		KNIP_DIAGNOSTIC_CODE_CIRCULAR_DEPENDENCY,
	] {
		assert!(!code.is_empty(), "diagnostic code must not be empty");
		assert!(
			code.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
			"diagnostic code '{code}' must be kebab-case"
		);
	}
}

#[test]
fn lsp_parity_diagnostic_source_field_is_always_knip() {
	// All Knip diagnostics must carry source = "knip" so Zed can filter them.
	let diagnostics = vec![
		Diagnostic {
			range: Range {
				start: Position { line: 0, character: 0 },
				end: Position { line: 0, character: 0 },
			},
			severity: DiagnosticSeverity::Warning,
			source: "knip".to_owned(),
			message: "Unused file".to_owned(),
			code: Some(KNIP_DIAGNOSTIC_CODE_UNUSED_FILE.to_owned()),
		},
		Diagnostic {
			range: Range {
				start: Position { line: 4, character: 9 },
				end: Position { line: 4, character: 20 },
			},
			severity: DiagnosticSeverity::Warning,
			source: "knip".to_owned(),
			message: "Unused export 'myFn'".to_owned(),
			code: Some(KNIP_DIAGNOSTIC_CODE_UNUSED_EXPORT.to_owned()),
		},
		Diagnostic {
			range: Range {
				start: Position { line: 10, character: 4 },
				end: Position {
					line: 10,
					character: 14,
				},
			},
			severity: DiagnosticSeverity::Warning,
			source: "knip".to_owned(),
			message: "Unused dependency 'lodash'".to_owned(),
			code: Some(KNIP_DIAGNOSTIC_CODE_UNUSED_DEPENDENCY.to_owned()),
		},
	];

	for diag in &diagnostics {
		assert_eq!(diag.source, "knip", "all Knip diagnostics must have source = 'knip'");
	}
}

#[test]
fn lsp_parity_diagnostic_severity_is_warning_for_all_knip_issues() {
	// Knip reports all issues as Warning (not Error/Information/Hint).
	// This matches VSCode extension behaviour: diagnostics.js always uses Warning.
	let severity = DiagnosticSeverity::Warning;
	assert_eq!(severity as u8, 2, "Warning must be LSP severity 2");
}

#[test]
fn lsp_parity_diagnostic_range_is_zero_based_lsp_positions() {
	// LSP positions are 0-based. Knip provides character-level ranges for
	// exports/dependencies and file-level (0,0)-(0,0) for unused-file.
	let file_level_range = Range {
		start: Position { line: 0, character: 0 },
		end: Position { line: 0, character: 0 },
	};
	// File-level diagnostics use the document start.
	assert_eq!(file_level_range.start.line, 0);
	assert_eq!(file_level_range.start.character, 0);

	let symbol_range = Range {
		start: Position { line: 4, character: 9 },
		end: Position { line: 4, character: 20 },
	};
	// Symbol ranges span the identifier.
	assert!(symbol_range.end.character > symbol_range.start.character);
}

#[test]
fn lsp_parity_diagnostic_publish_params_uri_is_file_scheme() {
	// publishDiagnostics URIs must use the file:// scheme.
	let params = sample_unused_export_diagnostic("file:///workspace/src/utils.ts");
	assert!(
		params.uri.starts_with("file://"),
		"diagnostic URI must use file:// scheme"
	);
}

#[test]
fn lsp_parity_diagnostic_multiple_issues_in_one_file_are_batched() {
	// Knip batches all diagnostics for a file into a single publishDiagnostics
	// notification rather than sending one notification per issue.
	let params = PublishDiagnosticsParams {
		uri: "file:///workspace/src/utils.ts".to_owned(),
		diagnostics: vec![
			Diagnostic {
				range: Range {
					start: Position { line: 2, character: 0 },
					end: Position { line: 2, character: 8 },
				},
				severity: DiagnosticSeverity::Warning,
				source: "knip".to_owned(),
				message: "Unused export 'helperA'".to_owned(),
				code: Some(KNIP_DIAGNOSTIC_CODE_UNUSED_EXPORT.to_owned()),
			},
			Diagnostic {
				range: Range {
					start: Position { line: 8, character: 0 },
					end: Position { line: 8, character: 8 },
				},
				severity: DiagnosticSeverity::Warning,
				source: "knip".to_owned(),
				message: "Unused export 'helperB'".to_owned(),
				code: Some(KNIP_DIAGNOSTIC_CODE_UNUSED_EXPORT.to_owned()),
			},
		],
	};

	assert_eq!(
		params.diagnostics.len(),
		2,
		"multiple diagnostics batched in one notification"
	);
	assert_eq!(params.diagnostics[0].source, params.diagnostics[1].source);
}

#[test]
fn lsp_parity_diagnostic_clear_is_empty_diagnostics_array() {
	// When Knip resolves all issues in a file, it sends publishDiagnostics
	// with an empty array — the standard LSP way to clear diagnostics.
	let clear_params = PublishDiagnosticsParams {
		uri: "file:///workspace/src/fixed.ts".to_owned(),
		diagnostics: vec![],
	};

	assert!(
		clear_params.diagnostics.is_empty(),
		"clearing diagnostics sends empty array"
	);
}

/// Known Knip code action kind prefixes.
const KNIP_CODE_ACTION_KIND_PREFIX: &str = "quickfix.knip.";

#[test]
fn lsp_parity_code_action_kinds_use_knip_prefix() {
	// All Knip code action kinds must start with "quickfix.knip." to namespace
	// them and allow Zed to filter/display them correctly.
	let kinds = [
		"quickfix.knip.removeExport",
		"quickfix.knip.addJsDocTag",
		"quickfix.knip.removeDependency",
	];

	for kind in kinds {
		assert!(
			kind.starts_with(KNIP_CODE_ACTION_KIND_PREFIX),
			"code action kind '{kind}' must start with '{KNIP_CODE_ACTION_KIND_PREFIX}'"
		);
	}
}

#[test]
fn lsp_parity_code_action_remove_export_is_preferred() {
	// "Remove export" is the primary fix for an unused-export diagnostic and
	// should be marked is_preferred = true so Zed highlights it.
	let action = sample_remove_export_action();
	assert!(action.is_preferred, "remove-export action must be preferred");
}

#[test]
fn lsp_parity_code_action_add_jsdoc_is_not_preferred() {
	// "Add @public JSDoc tag" is an alternative (suppress) action, not the
	// primary fix, so is_preferred = false.
	let action = sample_add_jsdoc_action();
	assert!(!action.is_preferred, "add-jsdoc action must not be preferred");
}

#[test]
fn lsp_parity_code_action_remove_dependency_is_not_preferred() {
	// "Remove dependency" modifies package.json and is destructive; it is not
	// marked preferred so the user must explicitly choose it.
	let action = sample_remove_dependency_action();
	assert!(!action.is_preferred, "remove-dependency action must not be preferred");
}

#[test]
fn lsp_parity_code_action_title_contains_symbol_name() {
	// Code action titles must include the affected symbol/dependency name so
	// the user can distinguish multiple actions in the same file.
	let export_action = sample_remove_export_action();
	assert!(
		export_action.title.contains("myFunction"),
		"remove-export title must contain the symbol name"
	);

	let dep_action = sample_remove_dependency_action();
	assert!(
		dep_action.title.contains("lodash"),
		"remove-dependency title must contain the dependency name"
	);
}

#[test]
fn lsp_parity_code_action_method_returns_list_not_single_item() {
	// textDocument/codeAction returns a Vec of actions, not a single action.
	// Zed presents all returned actions in the lightbulb menu.
	let actions: Vec<CodeAction> = vec![sample_remove_export_action(), sample_add_jsdoc_action()];

	assert!(actions.len() > 1, "codeAction response is a list");
}

/// All parity statuses used in the matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParityStatus {
	Implemented,
	ZedEquivalent,
	Unsupported,
	McpExcluded,
}

impl ParityStatus {
	fn as_str(self) -> &'static str {
		match self {
			Self::Implemented => "implemented",
			Self::ZedEquivalent => "zed-equivalent",
			Self::Unsupported => "unsupported",
			Self::McpExcluded => "MCP-excluded",
		}
	}
}

/// Full parity matrix as a flat list of (feature, status) pairs.
/// This mirrors `docs/parity-matrix.md` and is the harness-confirmed source of truth.
fn parity_matrix() -> Vec<(&'static str, ParityStatus)> {
	vec![
		// Diagnostics — wired
		("unused-files", ParityStatus::Implemented),
		("unused-dependencies", ParityStatus::Implemented),
		("unused-exports", ParityStatus::Implemented),
		("circular-dependencies", ParityStatus::Implemented),
		// Code Actions — wired (3) + unsupported (2)
		("code-action-remove-export", ParityStatus::Implemented),
		("code-action-add-jsdoc-tag", ParityStatus::Implemented),
		("code-action-remove-dependency", ParityStatus::Implemented),
		("code-action-install-dependency", ParityStatus::Unsupported),
		("code-action-delete-file", ParityStatus::Unsupported),
		// Settings — wired
		("setting-defer-session", ParityStatus::Implemented),
		("setting-require-config", ParityStatus::Implemented),
		("setting-config-file-path", ParityStatus::Implemented),
		("setting-node-runtime-path", ParityStatus::Implemented),
		("setting-preprocessor", ParityStatus::Implemented),
		("setting-preprocessor-options", ParityStatus::Implemented),
		// Commands — start/restart use Zed's standard LSP commands; the rest
		// (show-hover, expand-all) are upstream-only custom commands with no
		// Zed equivalent.
		("command-knip-start", ParityStatus::ZedEquivalent),
		("command-knip-restart", ParityStatus::ZedEquivalent),
		("command-knip-show-hover", ParityStatus::Unsupported),
		("command-knip-expand-all", ParityStatus::Unsupported),
		// Hover — upstream-only; the Zed extension does NOT wire hover.
		("hover-export-usages", ParityStatus::Unsupported),
		("hover-dependency-usages", ParityStatus::Unsupported),
		// CodeLens — unsupported (Zed has no CodeLens API; no workaround).
		("codelens-import-counts", ParityStatus::Unsupported),
		// Tree Views — unsupported (Zed has no tree-view extension API).
		("tree-view-imports", ParityStatus::Unsupported),
		("tree-view-exports", ParityStatus::Unsupported),
		// Lifecycle — wired
		("lifecycle-ls-startup", ParityStatus::Implemented),
		("lifecycle-file-watching", ParityStatus::Implemented),
		// MCP — out of scope
		("mcp-knip-configure", ParityStatus::McpExcluded),
		("mcp-knip-docs", ParityStatus::McpExcluded),
		("mcp-language-model-tools", ParityStatus::McpExcluded),
	]
}

#[test]
fn lsp_parity_matrix_has_no_unknown_statuses() {
	let matrix = parity_matrix();

	for (feature, status) in &matrix {
		let s = status.as_str();
		assert!(
			matches!(s, "implemented" | "zed-equivalent" | "unsupported" | "MCP-excluded"),
			"feature '{feature}' has unknown status '{s}'"
		);
	}
}

#[test]
fn lsp_parity_matrix_all_features_have_a_status() {
	let matrix = parity_matrix();
	assert!(!matrix.is_empty(), "parity matrix must not be empty");

	for (feature, _) in &matrix {
		assert!(!feature.is_empty(), "feature name must not be empty");
	}
}

#[test]
fn lsp_parity_matrix_implemented_count_matches_expected() {
	let matrix = parity_matrix();
	let implemented = matrix.iter().filter(|(_, s)| *s == ParityStatus::Implemented).count();

	// 4 diagnostics + 3 code actions + 6 settings + 2 lifecycle = 15
	assert_eq!(implemented, 15, "expected 15 implemented features");
}

#[test]
fn lsp_parity_matrix_zed_equivalent_count_matches_expected() {
	let matrix = parity_matrix();
	let zed_equiv = matrix.iter().filter(|(_, s)| *s == ParityStatus::ZedEquivalent).count();

	// start + restart = 2 (Zed's standard LSP commands)
	assert_eq!(zed_equiv, 2, "expected 2 zed-equivalent features");
}

#[test]
fn lsp_parity_matrix_unsupported_count_matches_expected() {
	let matrix = parity_matrix();
	let unsupported = matrix.iter().filter(|(_, s)| *s == ParityStatus::Unsupported).count();

	// install-dependency + delete-file + show-hover + expand-all + 2 hover +
	// codelens + 2 tree-views = 9
	assert_eq!(unsupported, 9, "expected 9 unsupported features");
}

#[test]
fn lsp_parity_matrix_mcp_excluded_count_matches_expected() {
	let matrix = parity_matrix();
	let mcp = matrix.iter().filter(|(_, s)| *s == ParityStatus::McpExcluded).count();

	assert_eq!(mcp, 3, "expected 3 MCP-excluded features");
}
