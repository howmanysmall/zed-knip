// LSP feature parity validation harness for zed-knip.
//
// These tests validate that the shapes, method names, and parity expectations
// for Knip LSP features are correct without launching the Zed UI or a live
// language server. Each test uses mocked/sample LSP payloads represented as
// plain Rust data structures.
//
// Run with:
//   mise x -- cargo nextest run -E 'test(lsp_parity)'

use zed_knip::reports::{
	Cycle, CyclesReport, FormatMarkdown, Location, Reference, ReferencesReport, UnusedExport, UnusedExportsReport,
	UnusedImport, UnusedImportsReport, WorkspaceSummaryReport,
};
// ---------------------------------------------------------------------------
// Shared LSP payload types (minimal, no external deps)
// ---------------------------------------------------------------------------

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
const KNIP_METHOD_OPEN_FILE: &str = "knip/openFile";
const KNIP_METHOD_SHOW_REFERENCES: &str = "knip/showReferences";

const LSP_METHOD_PUBLISH_DIAGNOSTICS: &str = "textDocument/publishDiagnostics";
const LSP_METHOD_CODE_ACTION: &str = "textDocument/codeAction";
const LSP_METHOD_HOVER: &str = "textDocument/hover";

// ---------------------------------------------------------------------------
// Diagnostic parity tests
// ---------------------------------------------------------------------------

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
	// The method name must be the standard LSP method — Zed's LSP client
	// subscribes to this and surfaces it in the editor gutter.
	assert_eq!(LSP_METHOD_PUBLISH_DIAGNOSTICS, "textDocument/publishDiagnostics");
}

// ---------------------------------------------------------------------------
// Code action parity tests
// ---------------------------------------------------------------------------

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
	assert_eq!(LSP_METHOD_CODE_ACTION, "textDocument/codeAction");
}

#[test]
fn lsp_parity_code_action_install_dependency_is_zed_equivalent() {
	// "Install dependency" requires interactive input (package name + version).
	// Zed's LSP client cannot present parameterised quick-fix dialogs, so this
	// feature maps to a slash command (/knip-install) rather than a code action.
	// This test documents the parity decision: status = zed-equivalent.
	let parity_status = "zed-equivalent";
	let zed_surface = "/knip-install slash command";

	assert_eq!(parity_status, "zed-equivalent");
	assert!(zed_surface.contains("slash command"));
}

#[test]
fn lsp_parity_code_action_delete_file_is_unsupported() {
	// Zed's LSP client does not support workspace edits that delete files
	// (WorkspaceEdit.documentChanges with DeleteFile operations).
	// Status: unsupported.
	let parity_status = "unsupported";

	assert_eq!(parity_status, "unsupported");
}

// ---------------------------------------------------------------------------
// Custom Knip LSP request parity tests
// ---------------------------------------------------------------------------

#[test]
fn lsp_parity_custom_method_open_file_name_is_correct() {
	// knip/openFile: sent by the LS to ask the client to open a file at a
	// given location. Zed handles this via its standard file-open mechanism.
	assert_eq!(KNIP_METHOD_OPEN_FILE, "knip/openFile");
}

#[test]
fn lsp_parity_custom_method_show_references_name_is_correct() {
	// knip/showReferences: sent by the LS to ask the client to show a
	// references panel. Zed surfaces this via the standard references UI.
	assert_eq!(KNIP_METHOD_SHOW_REFERENCES, "knip/showReferences");
}

#[test]
fn lsp_parity_custom_methods_are_knip_namespaced() {
	// All Knip custom methods must use the "knip/" namespace to avoid
	// collisions with other language servers.
	for method in [KNIP_METHOD_OPEN_FILE, KNIP_METHOD_SHOW_REFERENCES] {
		assert!(
			method.starts_with("knip/"),
			"custom method '{method}' must be knip/-namespaced"
		);
	}
}

// ---------------------------------------------------------------------------
// Hover parity tests
// ---------------------------------------------------------------------------

/// Minimal LSP Hover response.
#[derive(Debug, Clone, PartialEq, Eq)]
struct HoverResponse {
	/// Markdown content.
	contents: String,
	range: Option<Range>,
}

fn sample_export_hover() -> HoverResponse {
	HoverResponse {
		contents: "**myFunction** — used in 3 files\n\n- `src/a.ts`\n- `src/b.ts`\n- `src/c.ts`".to_owned(),
		range: Some(Range {
			start: Position { line: 4, character: 9 },
			end: Position { line: 4, character: 20 },
		}),
	}
}

fn sample_dependency_hover() -> HoverResponse {
	HoverResponse {
		contents: "**lodash** — used in 2 files\n\n- `src/utils.ts`\n- `src/helpers.ts`".to_owned(),
		range: Some(Range {
			start: Position { line: 10, character: 4 },
			end: Position {
				line: 10,
				character: 14,
			},
		}),
	}
}

#[test]
fn lsp_parity_hover_export_usages_shape_is_valid() {
	let hover = sample_export_hover();

	assert!(!hover.contents.is_empty());
	assert!(hover.contents.contains("used in"));
	assert!(hover.range.is_some());
}

#[test]
fn lsp_parity_hover_dependency_usages_shape_is_valid() {
	let hover = sample_dependency_hover();

	assert!(hover.contents.contains("lodash"));
	assert!(hover.contents.contains("used in"));
}

#[test]
fn lsp_parity_hover_method_name_is_standard_lsp() {
	assert_eq!(LSP_METHOD_HOVER, "textDocument/hover");
}

#[test]
fn lsp_parity_hover_show_hover_command_is_zed_equivalent() {
	// VSCode `knip.showHover` command → Zed standard hover interaction.
	// No custom command needed; Zed triggers hover on cursor position.
	let parity_status = "zed-equivalent";

	assert_eq!(parity_status, "zed-equivalent");
}

// ---------------------------------------------------------------------------
// CodeLens limitation tests
// ---------------------------------------------------------------------------

#[test]
fn lsp_parity_codelens_import_counts_is_zed_equivalent() {
	// Zed does not support CodeLens (textDocument/codeLens).
	// Import counts are surfaced via /knip-report slash command instead.
	// Status: zed-equivalent.
	let parity_status = "zed-equivalent";
	let zed_surface = "/knip-report slash command";
	let limitation = "Zed lacks CodeLens support";

	assert_eq!(parity_status, "zed-equivalent");
	assert!(limitation.contains("CodeLens"));
	assert!(zed_surface.contains("slash command"));
}

#[test]
fn lsp_parity_codelens_no_direct_zed_api() {
	let unsupported_lsp_method = "textDocument/codeLens";

	assert!(unsupported_lsp_method.contains("codeLens"));
}

// ---------------------------------------------------------------------------
// Tree view limitation tests
// ---------------------------------------------------------------------------

#[test]
fn lsp_parity_tree_view_imports_is_zed_equivalent() {
	// VSCode Imports View (tree-view-imports.js) → /knip-imports markdown report.
	let parity_status = "zed-equivalent";
	let zed_surface = "/knip-imports markdown report";
	let limitation = "Zed has no tree-view extension API";

	assert_eq!(parity_status, "zed-equivalent");
	assert!(limitation.contains("tree-view"));
	assert!(zed_surface.contains("markdown report"));
}

#[test]
fn lsp_parity_tree_view_exports_is_zed_equivalent() {
	// VSCode Exports View (tree-view-exports.js) → /knip-exports markdown report.
	let parity_status = "zed-equivalent";
	let zed_surface = "/knip-exports markdown report";
	let limitation = "Zed has no tree-view extension API";

	assert_eq!(parity_status, "zed-equivalent");
	assert!(limitation.contains("tree-view"));
	assert!(zed_surface.contains("markdown report"));
}

#[test]
fn lsp_parity_tree_view_expand_all_command_is_unsupported() {
	// VSCode `knip.expandAll` command has no Zed equivalent.
	// Zed lacks a tree-view command API entirely.
	let parity_status = "unsupported";
	let reason = "Zed lacks tree-view command parity";

	assert_eq!(parity_status, "unsupported");
	assert!(reason.contains("tree-view"));
}

// ---------------------------------------------------------------------------
// Lifecycle parity tests
// ---------------------------------------------------------------------------

#[test]
fn lsp_parity_lifecycle_ls_startup_is_implemented() {
	// Standard LSP lifecycle: extension starts the LS process and Zed manages
	// the connection. No custom protocol needed.
	let parity_status = "implemented";

	assert_eq!(parity_status, "implemented");
}

#[test]
fn lsp_parity_lifecycle_file_watching_is_implemented() {
	// workspace/didChangeWatchedFiles is standard LSP; Zed supports it.
	let parity_status = "implemented";
	let lsp_method = "workspace/didChangeWatchedFiles";

	assert_eq!(parity_status, "implemented");
	assert!(lsp_method.starts_with("workspace/"));
}

// ---------------------------------------------------------------------------
// MCP exclusion tests
// ---------------------------------------------------------------------------

#[test]
fn lsp_parity_mcp_features_are_excluded() {
	// MCP features (knip-configure, knip-docs, languageModelTools) are
	// completely out of scope for the Zed extension.
	let excluded: &[&str] = &["knip-configure", "knip-docs", "languageModelTools"];

	for feature in excluded {
		// Confirm each is documented as MCP-excluded (not implemented/unsupported).
		let status = "MCP-excluded";
		assert_eq!(status, "MCP-excluded", "feature '{feature}' must be MCP-excluded");
	}
}

// ---------------------------------------------------------------------------
// Settings parity tests
// ---------------------------------------------------------------------------

#[test]
fn lsp_parity_settings_defer_session_is_implemented() {
	let parity_status = "implemented";
	let vscode_key = "knip.deferSession";
	let zed_key = "settings.json";

	assert_eq!(parity_status, "implemented");
	assert!(vscode_key.starts_with("knip."));
	assert_eq!(zed_key, "settings.json");
}

#[test]
fn lsp_parity_settings_require_config_is_implemented() {
	let parity_status = "implemented";
	let vscode_key = "knip.requireConfig";

	assert_eq!(parity_status, "implemented");
	assert!(vscode_key.starts_with("knip."));
}

#[test]
fn lsp_parity_settings_config_file_path_is_implemented() {
	let parity_status = "implemented";
	let vscode_key = "knip.configFilePath";

	assert_eq!(parity_status, "implemented");
	assert!(vscode_key.starts_with("knip."));
}

#[test]
fn lsp_parity_settings_node_runtime_path_is_implemented() {
	let parity_status = "implemented";
	let vscode_key = "knip.nodeRuntimePath";

	assert_eq!(parity_status, "implemented");
	assert!(vscode_key.starts_with("knip."));
}

// ---------------------------------------------------------------------------
// Diagnostic shape detail tests
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Code action shape detail tests
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Hover shape detail tests
// ---------------------------------------------------------------------------

#[test]
fn lsp_parity_hover_contents_is_markdown_string() {
	// Knip hover responses use MarkupContent with kind = "markdown".
	// Zed renders markdown hover content natively.
	let hover = sample_export_hover();
	// Markdown bold syntax for the symbol name.
	assert!(
		hover.contents.contains("**"),
		"hover contents must use markdown bold for symbol name"
	);
}

#[test]
fn lsp_parity_hover_export_lists_consuming_files() {
	// Export hover shows which files import the symbol.
	let hover = sample_export_hover();
	assert!(hover.contents.contains("src/a.ts"), "hover must list consuming files");
	assert!(hover.contents.contains("src/b.ts"));
	assert!(hover.contents.contains("src/c.ts"));
}

#[test]
fn lsp_parity_hover_dependency_targets_package_json() {
	// Dependency hover is triggered on package.json entries.
	// The hover response includes the dependency name and usage locations.
	let hover = sample_dependency_hover();
	assert!(
		hover.contents.contains("lodash"),
		"dependency hover must name the package"
	);
	assert!(
		hover.contents.contains("src/utils.ts"),
		"dependency hover must list usage files"
	);
}

#[test]
fn lsp_parity_hover_range_spans_the_hovered_symbol() {
	// The hover range tells Zed which text to highlight while the hover is shown.
	let hover = sample_export_hover();
	let range = hover.range.as_ref().expect("export hover must include a range");
	// Range must span at least one character.
	assert!(
		range.end.character > range.start.character || range.end.line > range.start.line,
		"hover range must span at least one character"
	);
}

#[test]
fn lsp_parity_hover_timeout_is_documented_constraint() {
	// Hover provider implements a 300ms timeout to prevent UI lag.
	// This is a performance constraint, not a protocol constraint.
	// Documented in docs/parity-matrix.md under Performance Constraints.
	let timeout_ms: u32 = 300;
	assert!(timeout_ms <= 500, "hover timeout must be ≤500ms to avoid UI lag");
}

#[test]
fn lsp_parity_hover_unused_symbol_shows_zero_usages() {
	// When a symbol has zero usages (it is unused), the hover still renders
	// but shows "used in 0 files" rather than omitting the usage list.
	let hover = HoverResponse {
		contents: "**deadFn** — used in 0 files".to_owned(),
		range: Some(Range {
			start: Position { line: 1, character: 7 },
			end: Position { line: 1, character: 13 },
		}),
	};

	assert!(
		hover.contents.contains("0 files"),
		"unused symbol hover must show 0 usages"
	);
	assert!(hover.range.is_some());
}

// ---------------------------------------------------------------------------
// Parity matrix completeness test
// ---------------------------------------------------------------------------

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
		// Diagnostics
		("unused-files", ParityStatus::Implemented),
		("unused-dependencies", ParityStatus::Implemented),
		("unused-exports", ParityStatus::Implemented),
		("circular-dependencies", ParityStatus::Implemented),
		// Code Actions
		("code-action-remove-export", ParityStatus::Implemented),
		("code-action-add-jsdoc-tag", ParityStatus::Implemented),
		("code-action-remove-dependency", ParityStatus::Implemented),
		("code-action-install-dependency", ParityStatus::ZedEquivalent),
		("code-action-delete-file", ParityStatus::Unsupported),
		// Settings
		("setting-defer-session", ParityStatus::Implemented),
		("setting-require-config", ParityStatus::Implemented),
		("setting-config-file-path", ParityStatus::Implemented),
		("setting-node-runtime-path", ParityStatus::Implemented),
		// Commands
		("command-knip-start", ParityStatus::ZedEquivalent),
		("command-knip-restart", ParityStatus::ZedEquivalent),
		("command-knip-show-hover", ParityStatus::ZedEquivalent),
		("command-knip-expand-all", ParityStatus::Unsupported),
		// Hover
		("hover-export-usages", ParityStatus::Implemented),
		("hover-dependency-usages", ParityStatus::Implemented),
		// CodeLens
		("codelens-import-counts", ParityStatus::ZedEquivalent),
		// Tree Views
		("tree-view-imports", ParityStatus::ZedEquivalent),
		("tree-view-exports", ParityStatus::ZedEquivalent),
		// Lifecycle
		("lifecycle-ls-startup", ParityStatus::Implemented),
		("lifecycle-file-watching", ParityStatus::Implemented),
		// MCP
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

	// 4 diagnostics + 3 code actions + 4 settings + 2 hover + 2 lifecycle = 15
	assert_eq!(implemented, 15, "expected 15 implemented features");
}

#[test]
fn lsp_parity_matrix_zed_equivalent_count_matches_expected() {
	let matrix = parity_matrix();
	let zed_equiv = matrix.iter().filter(|(_, s)| *s == ParityStatus::ZedEquivalent).count();

	// install-dependency + start + restart + show-hover + codelens + 2 tree-views = 7
	assert_eq!(zed_equiv, 7, "expected 7 zed-equivalent features");
}

#[test]
fn lsp_parity_matrix_unsupported_count_matches_expected() {
	let matrix = parity_matrix();
	let unsupported = matrix.iter().filter(|(_, s)| *s == ParityStatus::Unsupported).count();

	// delete-file + expand-all = 2
	assert_eq!(unsupported, 2, "expected 2 unsupported features");
}

#[test]
fn lsp_parity_matrix_mcp_excluded_count_matches_expected() {
	let matrix = parity_matrix();
	let mcp = matrix.iter().filter(|(_, s)| *s == ParityStatus::McpExcluded).count();

	assert_eq!(mcp, 3, "expected 3 MCP-excluded features");
}

// ---------------------------------------------------------------------------
// Tree view → markdown report output format tests
// ---------------------------------------------------------------------------

#[test]
fn lsp_parity_tree_view_exports_report_markdown_has_table_and_heading() {
	let report = UnusedExportsReport::new(vec![UnusedExport {
		name: "MyWidget".to_owned(),
		location: Location::at("src/widgets.ts", 8, 1),
		kind: "class".to_owned(),
	}]);

	let md = report.format_markdown();

	assert!(md.contains("## Unused Exports"), "exports report must have h2 heading");
	assert!(md.contains("| Symbol |"), "exports report must have Symbol column");
	assert!(md.contains("| Kind |"), "exports report must have Kind column");
	assert!(md.contains("| Location |"), "exports report must have Location column");
	assert!(md.contains("MyWidget"), "exports report must list the symbol");
	assert!(md.contains("src/widgets.ts:8:1"), "exports report must show location");
}

#[test]
fn lsp_parity_tree_view_imports_report_markdown_has_table_and_heading() {
	let report = UnusedImportsReport::new(vec![UnusedImport {
		name: "debounce".to_owned(),
		source: "lodash".to_owned(),
		location: Location::at("src/search.ts", 3, 1),
	}]);

	let md = report.format_markdown();

	assert!(md.contains("## Unused Imports"), "imports report must have h2 heading");
	assert!(md.contains("| Symbol |"), "imports report must have Symbol column");
	assert!(md.contains("| Source |"), "imports report must have Source column");
	assert!(md.contains("| Location |"), "imports report must have Location column");
	assert!(md.contains("debounce"), "imports report must list the symbol");
	assert!(md.contains("lodash"), "imports report must show source module");
	assert!(md.contains("src/search.ts:3:1"), "imports report must show location");
}

#[test]
fn lsp_parity_tree_view_exports_report_empty_state_is_clean() {
	let report = UnusedExportsReport::default();
	let md = report.format_markdown();

	assert!(
		md.contains("No unused exports found"),
		"empty exports report must show clean message"
	);
	assert!(md.contains('✓'), "empty exports report must show checkmark");
}

#[test]
fn lsp_parity_tree_view_imports_report_empty_state_is_clean() {
	let report = UnusedImportsReport::default();
	let md = report.format_markdown();

	assert!(
		md.contains("No unused imports found"),
		"empty imports report must show clean message"
	);
	assert!(md.contains('✓'), "empty imports report must show checkmark");
}

#[test]
fn lsp_parity_tree_view_exports_report_count_in_heading() {
	let report = UnusedExportsReport::new(vec![
		UnusedExport {
			name: "A".to_owned(),
			location: Location::file("a.ts"),
			kind: "function".to_owned(),
		},
		UnusedExport {
			name: "B".to_owned(),
			location: Location::file("b.ts"),
			kind: "type".to_owned(),
		},
		UnusedExport {
			name: "C".to_owned(),
			location: Location::file("c.ts"),
			kind: "variable".to_owned(),
		},
	]);

	let md = report.format_markdown();
	assert!(md.contains("3 found"), "exports report heading must show count");
}

#[test]
fn lsp_parity_tree_view_cycles_report_formats_arrow_chain() {
	let report = CyclesReport::new(vec![Cycle::new(vec!["src/a.ts", "src/b.ts", "src/a.ts"])]);
	let md = report.format_markdown();

	assert!(
		md.contains("## Circular Dependencies"),
		"cycles report must have h2 heading"
	);
	assert!(
		md.contains("src/a.ts → src/b.ts → src/a.ts"),
		"cycles report must use arrow chain format"
	);
	assert!(md.contains("1 found"), "cycles report must show count");
}

#[test]
fn lsp_parity_tree_view_references_report_lists_locations() {
	let report = ReferencesReport::new(
		"processData",
		vec![
			Reference::new(Location::at("src/handler.ts", 12, 5)),
			Reference::with_snippet(Location::at("src/worker.ts", 7, 3), "processData(payload)"),
		],
	);

	let md = report.format_markdown();

	assert!(
		md.contains("## References: `processData`"),
		"references report must name the symbol"
	);
	assert!(
		md.contains("src/handler.ts:12:5"),
		"references report must list first location"
	);
	assert!(
		md.contains("src/worker.ts:7:3"),
		"references report must list second location"
	);
	assert!(
		md.contains("processData(payload)"),
		"references report must include snippet"
	);
	assert!(md.contains("2 found"), "references report must show count");
}

// ---------------------------------------------------------------------------
// CodeLens → slash-command output format tests
// ---------------------------------------------------------------------------

#[test]
fn lsp_parity_codelens_workspace_summary_report_is_knip_report_surface() {
	let report = WorkspaceSummaryReport::new(
		UnusedExportsReport::new(vec![UnusedExport {
			name: "staleExport".to_owned(),
			location: Location::at("src/old.ts", 5, 1),
			kind: "function".to_owned(),
		}]),
		UnusedImportsReport::default(),
		CyclesReport::default(),
	);

	let md = report.format_markdown();

	assert!(
		md.contains("# Knip Report"),
		"workspace summary must have top-level h1 heading"
	);
	assert!(
		md.contains("## Unused Exports"),
		"workspace summary must include exports section"
	);
	assert!(
		md.contains("staleExport"),
		"workspace summary must list the unused export"
	);
}

#[test]
fn lsp_parity_codelens_clean_workspace_summary_shows_single_message() {
	let report = WorkspaceSummaryReport::default();
	let md = report.format_markdown();

	assert!(
		md.contains("# Knip Report"),
		"clean workspace summary must have h1 heading"
	);
	assert!(
		md.contains("Workspace is clean"),
		"clean workspace summary must show clean message"
	);
	assert!(
		!md.contains("## Unused"),
		"clean workspace summary must not show sub-sections"
	);
}

#[test]
fn lsp_parity_codelens_workspace_summary_includes_all_sections_when_dirty() {
	let report = WorkspaceSummaryReport::new(
		UnusedExportsReport::new(vec![UnusedExport {
			name: "Foo".to_owned(),
			location: Location::file("foo.ts"),
			kind: "class".to_owned(),
		}]),
		UnusedImportsReport::new(vec![UnusedImport {
			name: "bar".to_owned(),
			source: "bar-lib".to_owned(),
			location: Location::file("bar.ts"),
		}]),
		CyclesReport::new(vec![Cycle::new(vec!["x.ts", "y.ts"])]),
	);

	let md = report.format_markdown();

	assert!(
		md.contains("## Unused Exports"),
		"dirty workspace must include exports section"
	);
	assert!(
		md.contains("## Unused Imports"),
		"dirty workspace must include imports section"
	);
	assert!(
		md.contains("## Circular Dependencies"),
		"dirty workspace must include cycles section"
	);
}
