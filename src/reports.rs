//! Zed-equivalent report framework for Knip diagnostics.
//!
//! Maps VSCode-only UI surfaces (tree views, CodeLens, `knip.showReferences`) to
//! markdown-formatted slash-command output suitable for Zed's Assistant panel.
//!
//! # Design
//!
//! All report types are plain data structs. Callers populate them from Knip
//! language-server responses and call [`FormatMarkdown::format_markdown`] to
//! produce the final string. No Knip analysis logic lives here.

use std::fmt;

// ---------------------------------------------------------------------------
// Shared primitives
// ---------------------------------------------------------------------------

/// A source location: file path plus optional line/column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
	/// Workspace-relative file path.
	pub file: String,
	/// 1-based line number, if known.
	pub line: Option<u32>,
	/// 1-based column number, if known.
	pub column: Option<u32>,
}

impl Location {
	/// Create a location with only a file path.
	pub fn file(file: impl Into<String>) -> Self {
		Self {
			file: file.into(),
			line: None,
			column: None,
		}
	}

	/// Create a location with file, line, and column.
	pub fn at(file: impl Into<String>, line: u32, column: u32) -> Self {
		Self {
			file: file.into(),
			line: Some(line),
			column: Some(column),
		}
	}
}

impl fmt::Display for Location {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.file)?;
		if let Some(line) = self.line {
			write!(f, ":{line}")?;
			if let Some(col) = self.column {
				write!(f, ":{col}")?;
			}
		}
		Ok(())
	}
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Render a report as a markdown string for Zed slash-command output.
pub trait FormatMarkdown {
	fn format_markdown(&self) -> String;
}

// ---------------------------------------------------------------------------
// Unused exports report  (VSCode: Exports tree view)
// ---------------------------------------------------------------------------

/// A single unused export symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnusedExport {
	/// Symbol name (e.g. `MyClass`, `helperFn`).
	pub name: String,
	/// Where the export is declared.
	pub location: Location,
	/// Export kind (e.g. `function`, `class`, `type`, `variable`).
	pub kind: String,
}

/// Report of all unused exports in the workspace.
///
/// Zed equivalent of the VSCode Exports tree view and CodeLens import counts.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UnusedExportsReport {
	pub entries: Vec<UnusedExport>,
}

impl UnusedExportsReport {
	pub fn new(entries: Vec<UnusedExport>) -> Self {
		Self { entries }
	}

	pub fn is_empty(&self) -> bool {
		self.entries.is_empty()
	}
}

impl FormatMarkdown for UnusedExportsReport {
	fn format_markdown(&self) -> String {
		if self.entries.is_empty() {
			return "## Unused Exports\n\nNo unused exports found. ✓\n".to_owned();
		}

		let mut out = format!("## Unused Exports ({} found)\n\n", self.entries.len());
		out.push_str("| Symbol | Kind | Location |\n");
		out.push_str("|--------|------|----------|\n");

		for entry in &self.entries {
			out.push_str(&format!(
				"| `{}` | {} | `{}` |\n",
				entry.name, entry.kind, entry.location
			));
		}

		out
	}
}

// ---------------------------------------------------------------------------
// Unused imports report  (VSCode: Imports tree view)
// ---------------------------------------------------------------------------

/// A single unused import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnusedImport {
	/// Imported symbol name.
	pub name: String,
	/// Source module being imported from.
	pub source: String,
	/// Where the import statement appears.
	pub location: Location,
}

/// Report of all unused imports in the workspace.
///
/// Zed equivalent of the VSCode Imports tree view.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UnusedImportsReport {
	pub entries: Vec<UnusedImport>,
}

impl UnusedImportsReport {
	pub fn new(entries: Vec<UnusedImport>) -> Self {
		Self { entries }
	}

	pub fn is_empty(&self) -> bool {
		self.entries.is_empty()
	}
}

impl FormatMarkdown for UnusedImportsReport {
	fn format_markdown(&self) -> String {
		if self.entries.is_empty() {
			return "## Unused Imports\n\nNo unused imports found. ✓\n".to_owned();
		}

		let mut out = format!("## Unused Imports ({} found)\n\n", self.entries.len());
		out.push_str("| Symbol | Source | Location |\n");
		out.push_str("|--------|--------|----------|\n");

		for entry in &self.entries {
			out.push_str(&format!(
				"| `{}` | `{}` | `{}` |\n",
				entry.name, entry.source, entry.location
			));
		}

		out
	}
}

// ---------------------------------------------------------------------------
// Cycle report  (VSCode: contention/cycle display)
// ---------------------------------------------------------------------------

/// One circular dependency cycle: an ordered list of file paths forming the loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cycle {
	/// Files in cycle order. The last file imports the first, closing the loop.
	pub files: Vec<String>,
}

impl Cycle {
	pub fn new(files: Vec<impl Into<String>>) -> Self {
		Self {
			files: files.into_iter().map(Into::into).collect(),
		}
	}

	fn format_chain(&self) -> String {
		self.files.join(" → ")
	}
}

/// Report of all detected circular dependency cycles.
///
/// Zed equivalent of the VSCode contention/cycle display.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CyclesReport {
	pub cycles: Vec<Cycle>,
}

impl CyclesReport {
	pub fn new(cycles: Vec<Cycle>) -> Self {
		Self { cycles }
	}

	pub fn is_empty(&self) -> bool {
		self.cycles.is_empty()
	}
}

impl FormatMarkdown for CyclesReport {
	fn format_markdown(&self) -> String {
		if self.cycles.is_empty() {
			return "## Circular Dependencies\n\nNo circular dependencies found. ✓\n".to_owned();
		}

		let mut out = format!("## Circular Dependencies ({} found)\n\n", self.cycles.len());

		for (i, cycle) in self.cycles.iter().enumerate() {
			out.push_str(&format!("{}. `{}`\n", i + 1, cycle.format_chain()));
		}

		out
	}
}

// ---------------------------------------------------------------------------
// References report  (VSCode: knip.showReferences command)
// ---------------------------------------------------------------------------

/// One reference to a symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
	/// Where the reference appears.
	pub location: Location,
	/// Optional surrounding code snippet for context.
	pub snippet: Option<String>,
}

impl Reference {
	pub fn new(location: Location) -> Self {
		Self {
			location,
			snippet: None,
		}
	}

	pub fn with_snippet(location: Location, snippet: impl Into<String>) -> Self {
		Self {
			location,
			snippet: Some(snippet.into()),
		}
	}
}

/// Report of all references to a named symbol.
///
/// Zed equivalent of the VSCode `knip.showReferences` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferencesReport {
	/// The symbol whose references are listed.
	pub symbol: String,
	pub references: Vec<Reference>,
}

impl ReferencesReport {
	pub fn new(symbol: impl Into<String>, references: Vec<Reference>) -> Self {
		Self {
			symbol: symbol.into(),
			references,
		}
	}

	pub fn is_empty(&self) -> bool {
		self.references.is_empty()
	}
}

impl FormatMarkdown for ReferencesReport {
	fn format_markdown(&self) -> String {
		if self.references.is_empty() {
			return format!("## References: `{}`\n\nNo references found.\n", self.symbol);
		}

		let mut out = format!("## References: `{}` ({} found)\n\n", self.symbol, self.references.len());

		for r in &self.references {
			out.push_str(&format!("- `{}`", r.location));
			if let Some(snippet) = &r.snippet {
				out.push_str(&format!(" — `{snippet}`"));
			}
			out.push('\n');
		}

		out
	}
}

// ---------------------------------------------------------------------------
// Combined workspace summary
// ---------------------------------------------------------------------------

/// Aggregated workspace-level Knip report.
///
/// Combines all sub-reports into a single markdown document, suitable for a
/// `/knip-report` slash command response.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceSummaryReport {
	pub unused_exports: UnusedExportsReport,
	pub unused_imports: UnusedImportsReport,
	pub cycles: CyclesReport,
}

impl WorkspaceSummaryReport {
	pub fn new(unused_exports: UnusedExportsReport, unused_imports: UnusedImportsReport, cycles: CyclesReport) -> Self {
		Self {
			unused_exports,
			unused_imports,
			cycles,
		}
	}

	/// Returns `true` when all sub-reports are empty (workspace is clean).
	pub fn is_clean(&self) -> bool {
		self.unused_exports.is_empty() && self.unused_imports.is_empty() && self.cycles.is_empty()
	}
}

impl FormatMarkdown for WorkspaceSummaryReport {
	fn format_markdown(&self) -> String {
		if self.is_clean() {
			return "# Knip Report\n\nWorkspace is clean — no issues found. ✓\n".to_owned();
		}

		let mut out = "# Knip Report\n\n".to_owned();
		out.push_str(&self.unused_exports.format_markdown());
		out.push('\n');
		out.push_str(&self.unused_imports.format_markdown());
		out.push('\n');
		out.push_str(&self.cycles.format_markdown());
		out
	}
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
	use super::*;

	// --- Location ---

	#[test]
	fn location_file_only_displays_path() {
		let loc = Location::file("src/index.ts");
		assert_eq!(loc.to_string(), "src/index.ts");
	}

	#[test]
	fn location_with_line_and_column_displays_all_parts() {
		let loc = Location::at("src/index.ts", 10, 5);
		assert_eq!(loc.to_string(), "src/index.ts:10:5");
	}

	#[test]
	fn location_with_line_only_omits_column() {
		let loc = Location {
			file: "src/a.ts".to_owned(),
			line: Some(3),
			column: None,
		};
		assert_eq!(loc.to_string(), "src/a.ts:3");
	}

	// --- UnusedExportsReport ---

	#[test]
	fn unused_exports_empty_report_shows_clean_message() {
		let report = UnusedExportsReport::default();
		let md = report.format_markdown();
		assert!(md.contains("No unused exports found"));
		assert!(md.contains('✓'));
	}

	#[test]
	fn unused_exports_report_contains_symbol_name_and_location() {
		let report = UnusedExportsReport::new(vec![UnusedExport {
			name: "helperFn".to_owned(),
			location: Location::at("src/utils.ts", 12, 1),
			kind: "function".to_owned(),
		}]);
		let md = report.format_markdown();
		assert!(md.contains("helperFn"));
		assert!(md.contains("src/utils.ts:12:1"));
		assert!(md.contains("function"));
		assert!(md.contains("1 found"));
	}

	#[test]
	fn unused_exports_report_count_reflects_entry_count() {
		let entries = vec![
			UnusedExport {
				name: "A".to_owned(),
				location: Location::file("a.ts"),
				kind: "class".to_owned(),
			},
			UnusedExport {
				name: "B".to_owned(),
				location: Location::file("b.ts"),
				kind: "type".to_owned(),
			},
		];
		let report = UnusedExportsReport::new(entries);
		assert!(report.format_markdown().contains("2 found"));
	}

	// --- UnusedImportsReport ---

	#[test]
	fn unused_imports_empty_report_shows_clean_message() {
		let report = UnusedImportsReport::default();
		let md = report.format_markdown();
		assert!(md.contains("No unused imports found"));
	}

	#[test]
	fn unused_imports_report_contains_symbol_source_and_location() {
		let report = UnusedImportsReport::new(vec![UnusedImport {
			name: "useState".to_owned(),
			source: "react".to_owned(),
			location: Location::at("src/App.tsx", 1, 10),
		}]);
		let md = report.format_markdown();
		assert!(md.contains("useState"));
		assert!(md.contains("react"));
		assert!(md.contains("src/App.tsx:1:10"));
	}

	// --- CyclesReport ---

	#[test]
	fn cycles_empty_report_shows_clean_message() {
		let report = CyclesReport::default();
		let md = report.format_markdown();
		assert!(md.contains("No circular dependencies found"));
	}

	#[test]
	fn cycles_report_formats_chain_with_arrows() {
		let report = CyclesReport::new(vec![Cycle::new(vec!["a.ts", "b.ts", "a.ts"])]);
		let md = report.format_markdown();
		assert!(md.contains("a.ts → b.ts → a.ts"));
		assert!(md.contains("1 found"));
	}

	#[test]
	fn cycles_report_numbers_multiple_cycles() {
		let report = CyclesReport::new(vec![
			Cycle::new(vec!["x.ts", "y.ts"]),
			Cycle::new(vec!["p.ts", "q.ts", "r.ts"]),
		]);
		let md = report.format_markdown();
		assert!(md.contains("1."));
		assert!(md.contains("2."));
		assert!(md.contains("2 found"));
	}

	// --- ReferencesReport ---

	#[test]
	fn references_empty_report_shows_no_references_message() {
		let report = ReferencesReport::new("myFn", vec![]);
		let md = report.format_markdown();
		assert!(md.contains("myFn"));
		assert!(md.contains("No references found"));
	}

	#[test]
	fn references_report_lists_locations() {
		let report = ReferencesReport::new(
			"doThing",
			vec![
				Reference::new(Location::at("src/a.ts", 5, 3)),
				Reference::with_snippet(Location::at("src/b.ts", 10, 1), "doThing(arg)"),
			],
		);
		let md = report.format_markdown();
		assert!(md.contains("doThing"));
		assert!(md.contains("src/a.ts:5:3"));
		assert!(md.contains("src/b.ts:10:1"));
		assert!(md.contains("doThing(arg)"));
		assert!(md.contains("2 found"));
	}

	// --- WorkspaceSummaryReport ---

	#[test]
	fn workspace_summary_clean_workspace_shows_clean_message() {
		let report = WorkspaceSummaryReport::default();
		assert!(report.is_clean());
		let md = report.format_markdown();
		assert!(md.contains("Workspace is clean"));
	}

	#[test]
	fn workspace_summary_with_issues_includes_all_sections() {
		let report = WorkspaceSummaryReport::new(
			UnusedExportsReport::new(vec![UnusedExport {
				name: "Foo".to_owned(),
				location: Location::file("foo.ts"),
				kind: "class".to_owned(),
			}]),
			UnusedImportsReport::default(),
			CyclesReport::new(vec![Cycle::new(vec!["a.ts", "b.ts"])]),
		);
		assert!(!report.is_clean());
		let md = report.format_markdown();
		assert!(md.contains("# Knip Report"));
		assert!(md.contains("Unused Exports"));
		assert!(md.contains("Unused Imports"));
		assert!(md.contains("Circular Dependencies"));
		assert!(md.contains("Foo"));
		assert!(md.contains("a.ts → b.ts"));
	}

	#[test]
	fn workspace_summary_is_clean_only_when_all_sub_reports_empty() {
		let with_export = WorkspaceSummaryReport {
			unused_exports: UnusedExportsReport::new(vec![UnusedExport {
				name: "X".to_owned(),
				location: Location::file("x.ts"),
				kind: "variable".to_owned(),
			}]),
			..WorkspaceSummaryReport::default()
		};
		assert!(!with_export.is_clean());
	}
}
