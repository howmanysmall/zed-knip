use std::{collections::BTreeMap, fmt, path::Path, str::FromStr};
use zed_extension_api::serde_json::Value;

/// Valid Knip issue types for diagnostics filtering.
pub const VALID_ISSUE_TYPES: &[&str] = &[
	"files",
	"dependencies",
	"devDependencies",
	"optionalPeerDependencies",
	"unlisted",
	"binaries",
	"unresolved",
	"exports",
	"types",
	"nsExports",
	"nsTypes",
	"duplicates",
	"enumMembers",
	"namespaceMembers",
	"catalog",
];

/// Diagnostic severity level for a Knip issue type override.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
	Error,
	Warning,
	Information,
	Hint,
	Off,
}

impl DiagnosticSeverity {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Error => "error",
			Self::Warning => "warn",
			Self::Information => "info",
			Self::Hint => "hint",
			Self::Off => "off",
		}
	}

	/// Parse a severity string with issue-type context for error messages.
	pub(crate) fn parse_with_type(issue_type: &str, value: &str) -> Result<Self, KnipSettingsError> {
		match value.trim().to_ascii_lowercase().as_str() {
			"error" => Ok(Self::Error),
			"warn" | "warning" => Ok(Self::Warning),
			"info" | "information" => Ok(Self::Information),
			"hint" => Ok(Self::Hint),
			"off" => Ok(Self::Off),
			_ => Err(KnipSettingsError::InvalidSeverityValue {
				issue_type: issue_type.to_owned(),
				value: value.to_owned(),
			}),
		}
	}
}

impl fmt::Display for DiagnosticSeverity {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl FromStr for DiagnosticSeverity {
	type Err = KnipSettingsError;

	fn from_str(input: &str) -> Result<Self, Self::Err> {
		Self::parse_with_type("", input)
	}
}

/// Diagnostics filter and severity override configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KnipDiagnosticsSettings {
	/// Allowlist of issue types to show (empty = show all).
	pub include_issue_types: Vec<String>,
	/// Blocklist of issue types to hide.
	pub exclude_issue_types: Vec<String>,
	/// Hide diagnostics for files under these relative path prefixes.
	pub exclude_path_prefixes: Vec<String>,
	/// Override severity for specific issue types.
	pub severity_by_issue_type: BTreeMap<String, DiagnosticSeverity>,
}

/// User-facing Knip settings for the Zed extension.
///
/// Defaults match the extension's baseline behavior; callers can merge a user
/// override struct on top of [`KnipSettings::default()`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnipSettings {
	/// Explicit path to the Knip language server binary.
	///
	/// Default: `None`.
	pub binary_path: Option<String>,
	/// Enable managed installation of Knip.
	///
	/// Default: `true`.
	pub auto_install: bool,
	/// Explicit Knip config path.
	///
	/// Default: `None`.
	pub config_path: Option<String>,
	/// Require a Knip config file to exist.
	///
	/// Default: `false`.
	pub require_config: bool,
	/// Path to the TypeScript config file for Knip analysis.
	///
	/// Default: `None`.
	pub ts_config_path: Option<String>,
	/// Diagnostics filter and severity configuration.
	///
	/// Default: all defaults (show all, no overrides).
	pub diagnostics: KnipDiagnosticsSettings,
	/// Internal: detected or overridden package manager for the workspace.
	///
	/// Not user-configurable via settings JSON.
	/// Populated internally from workspace metadata before command resolution.
	pub package_manager: Option<String>,
	/// Internal: log level for the language server.
	///
	/// Not user-configurable via settings JSON.
	/// Used internally when constructing resolver inputs.
	pub log_level: LogLevel,
	/// Preprocessor tools to run before Knip analysis.
	///
	/// Entries starting with `./` are local paths; others are package specifiers.
	///
	/// Default: empty.
	pub preprocessor: Vec<String>,
	/// Options passed to preprocessor tools.
	///
	/// Must be a JSON object. Requires at least one preprocessor to be configured.
	///
	/// Default: None.
	pub preprocessor_options: Option<BTreeMap<String, Value>>,
}

impl Default for KnipSettings {
	fn default() -> Self {
		Self {
			binary_path: None,
			auto_install: true,
			config_path: None,
			require_config: false,
			ts_config_path: None,
			diagnostics: KnipDiagnosticsSettings::default(),
			package_manager: None,
			log_level: LogLevel::Info,
			preprocessor: Vec::new(),
			preprocessor_options: None,
		}
	}
}

impl KnipSettings {
	/// Merge `overrides` on top of `self`.
	///
	/// `Option` fields keep the base value when the override is `None`.
	/// `preprocessor` keeps base when override is empty.
	/// `preprocessor_options` keeps base when override is `None`.
	pub fn merge(self, overrides: Self) -> Self {
		Self {
			binary_path: overrides.binary_path.or(self.binary_path),
			auto_install: overrides.auto_install,
			config_path: overrides.config_path.or(self.config_path),
			require_config: overrides.require_config,
			ts_config_path: overrides.ts_config_path.or(self.ts_config_path),
			diagnostics: overrides.diagnostics,
			package_manager: overrides.package_manager.or(self.package_manager),
			log_level: overrides.log_level,
			preprocessor: if overrides.preprocessor.is_empty() {
				self.preprocessor
			} else {
				overrides.preprocessor
			},
			preprocessor_options: overrides.preprocessor_options.or(self.preprocessor_options),
		}
	}

	/// Returns `true` if these settings require the managed-install patch to take effect.
	///
	/// The patch is needed whenever advanced editor settings are present that the vanilla
	/// language server does not support natively.
	pub fn requires_managed_patch(&self) -> bool {
		self.ts_config_path.is_some() || !self.diagnostics_is_default() || !self.preprocessor.is_empty()
	}

	/// Returns `true` if all diagnostics sub-fields are at their defaults (all empty).
	pub fn diagnostics_is_default(&self) -> bool {
		self.diagnostics.include_issue_types.is_empty()
			&& self.diagnostics.exclude_issue_types.is_empty()
			&& self.diagnostics.exclude_path_prefixes.is_empty()
			&& self.diagnostics.severity_by_issue_type.is_empty()
	}

	/// Returns the names of currently active advanced settings that require the
	/// managed-install patch.
	///
	/// Used in error messages to tell the user exactly which settings to unset.
	/// Only user-facing advanced settings are listed; `binary.path` is not
	/// included here because the resolver adds it separately when relevant.
	pub fn advanced_settings_list(&self) -> Vec<&'static str> {
		let mut advanced: Vec<&'static str> = Vec::new();
		if self.ts_config_path.is_some() {
			advanced.push("lsp.knip.settings.ts_config_path");
		}
		if !self.diagnostics.include_issue_types.is_empty() {
			advanced.push("lsp.knip.settings.diagnostics.include_issue_types");
		}
		if !self.diagnostics.exclude_issue_types.is_empty() {
			advanced.push("lsp.knip.settings.diagnostics.exclude_issue_types");
		}
		if !self.diagnostics.exclude_path_prefixes.is_empty() {
			advanced.push("lsp.knip.settings.diagnostics.exclude_path_prefixes");
		}
		if !self.diagnostics.severity_by_issue_type.is_empty() {
			advanced.push("lsp.knip.settings.diagnostics.severity_by_issue_type");
		}
		if !self.preprocessor.is_empty() {
			advanced.push("lsp.knip.settings.preprocessor");
		}
		if self.preprocessor_options.is_some() {
			advanced.push("lsp.knip.settings.preprocessor_options");
		}
		advanced
	}

	/// Validate user-provided settings values.
	pub fn validate(&self) -> Result<(), KnipSettingsError> {
		if matches!(self.binary_path.as_deref(), Some("")) {
			return Err(KnipSettingsError::EmptyBinaryPath);
		}

		if matches!(self.config_path.as_deref(), Some("")) {
			return Err(KnipSettingsError::EmptyConfigPath);
		}

		// Validate issue types in include/exclude lists and severity map keys.
		for issue_type in self
			.diagnostics
			.include_issue_types
			.iter()
			.chain(self.diagnostics.exclude_issue_types.iter())
			.chain(self.diagnostics.severity_by_issue_type.keys())
		{
			if !VALID_ISSUE_TYPES.contains(&issue_type.as_str()) {
				return Err(KnipSettingsError::InvalidIssueType {
					issue_type: issue_type.clone(),
				});
			}
		}

		// Validate diagnostics path prefixes.
		for prefix in &self.diagnostics.exclude_path_prefixes {
			if prefix.is_empty() {
				return Err(KnipSettingsError::InvalidPathPrefix {
					reason: "path prefix cannot be empty".to_string(),
				});
			}
			if Path::new(prefix).is_absolute() {
				return Err(KnipSettingsError::InvalidPathPrefix {
					reason: format!("path prefix '{prefix}' must be relative, not absolute"),
				});
			}
			if prefix.contains("..") {
				return Err(KnipSettingsError::InvalidPathPrefix {
					reason: format!("path prefix '{prefix}' must not contain '..' traversal"),
				});
			}
		}

		// Validate preprocessor specifiers.
		for prep in &self.preprocessor {
			validate_preprocessor_specifier(prep)?;
		}

		// Validate preprocessor_options: if present, requires at least one preprocessor.
		if self.preprocessor_options.is_some() && self.preprocessor.is_empty() {
			return Err(KnipSettingsError::InvalidPreprocessorOptions {
				reason: "preprocessor_options requires at least one preprocessor to be configured".to_string(),
			});
		}

		Ok(())
	}
}

/// Knip language-server log levels.
///
/// Internal language-server log level representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
	Trace,
	Debug,
	Info,
	Warn,
	Error,
}

impl LogLevel {
	pub(crate) fn as_str(self) -> &'static str {
		match self {
			Self::Trace => "trace",
			Self::Debug => "debug",
			Self::Info => "info",
			Self::Warn => "warn",
			Self::Error => "error",
		}
	}
}

impl fmt::Display for LogLevel {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl FromStr for LogLevel {
	type Err = KnipSettingsError;

	fn from_str(input: &str) -> Result<Self, Self::Err> {
		match input.trim().to_ascii_lowercase().as_str() {
			"trace" => Ok(Self::Trace),
			"debug" => Ok(Self::Debug),
			"info" => Ok(Self::Info),
			"warn" => Ok(Self::Warn),
			"error" => Ok(Self::Error),
			invalid => Err(KnipSettingsError::InvalidLogLevel(invalid.to_owned())),
		}
	}
}

/// Validate a preprocessor specifier string.
///
/// Returns `Ok(())` for valid entries:
/// - Local paths starting with `./` (e.g., `./tools/preprocess.mjs`)
/// - Package names (e.g., `preprocessor-package`)
/// - Package subpaths (e.g., `pkg/subpath`, `@scope/pkg/subpath`)
///
/// Returns an error for invalid entries:
/// - Empty strings
/// - Paths starting with `../`, `/`, `~/`
/// - Protocol prefixes like `file:`, `node:`, `data:`, `http:`, `https:`
fn validate_preprocessor_specifier(specifier: &str) -> Result<(), KnipSettingsError> {
	if specifier.is_empty() {
		return Err(KnipSettingsError::InvalidPreprocessor {
			value: specifier.to_string(),
			reason: "preprocessor specifier cannot be empty".to_string(),
		});
	}

	if specifier.starts_with("../") {
		return Err(KnipSettingsError::InvalidPreprocessor {
			value: specifier.to_string(),
			reason: "preprocessor specifier cannot start with '../'".to_string(),
		});
	}

	if specifier.starts_with('/') {
		return Err(KnipSettingsError::InvalidPreprocessor {
			value: specifier.to_string(),
			reason: "preprocessor specifier cannot be an absolute path".to_string(),
		});
	}

	if specifier.starts_with("~/~") || specifier.starts_with('~') {
		return Err(KnipSettingsError::InvalidPreprocessor {
			value: specifier.to_string(),
			reason: "preprocessor specifier cannot start with '~'".to_string(),
		});
	}

	let invalid_prefixes = ["file:", "node:", "data:", "http://", "https://"];
	for prefix in invalid_prefixes {
		if specifier.starts_with(prefix) {
			return Err(KnipSettingsError::InvalidPreprocessor {
				value: specifier.to_string(),
				reason: format!("preprocessor specifier cannot start with '{}'", prefix),
			});
		}
	}

	Ok(())
}

/// Errors produced while validating Knip settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnipSettingsError {
	/// The explicit Knip binary path was set to an empty string.
	EmptyBinaryPath,
	/// The explicit Knip config path was set to an empty string.
	EmptyConfigPath,
	/// The requested log level could not be parsed.
	InvalidLogLevel(String),
	/// A removed setting was used; names the setting and its replacement.
	RemovedSetting {
		name: &'static str,
		replacement: &'static str,
	},
	/// An invalid Knip issue type was used in diagnostics configuration.
	InvalidIssueType { issue_type: String },
	/// An invalid severity value was used for a diagnostics override.
	InvalidSeverityValue { issue_type: String, value: String },
	/// An invalid diagnostics path prefix was provided.
	InvalidPathPrefix { reason: String },
	/// A removed env-based setting was used; names the env key and replacement.
	RemovedEnvSetting {
		name: &'static str,
		replacement: &'static str,
	},
	/// An invalid preprocessor specifier was provided.
	InvalidPreprocessor { value: String, reason: String },
	/// An invalid preprocessor_options value was provided.
	InvalidPreprocessorOptions { reason: String },
}

impl fmt::Display for KnipSettingsError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::EmptyBinaryPath => write!(f, "The Knip binary path cannot be empty."),
			Self::EmptyConfigPath => write!(f, "The Knip config path cannot be empty."),
			Self::InvalidLogLevel(level) => {
				write!(
					f,
					"Invalid Knip log level '{level}'. Use trace, debug, info, warn, or error."
				)
			}
			Self::RemovedSetting { name, replacement } => {
				write!(f, "Removed setting '{name}'. Use '{replacement}' instead.")
			}
			Self::InvalidIssueType { issue_type } => {
				write!(
					f,
					"Invalid Knip issue type '{issue_type}'. Use one of: {}.",
					VALID_ISSUE_TYPES.join(", ")
				)
			}
			Self::InvalidSeverityValue { issue_type, value } => {
				write!(
					f,
					"Invalid severity '{value}' for issue type '{issue_type}'. Use error, warn, info, hint, or off."
				)
			}
			Self::InvalidPathPrefix { reason } => {
				write!(f, "Invalid diagnostic path prefix: {reason}")
			}
			Self::RemovedEnvSetting { name, replacement } => {
				write!(
					f,
					"Removed env-based setting 'lsp.knip.binary.env.{name}'. The Knip language server does not consume this env var. Use '{replacement}' (and other LSP settings) instead."
				)
			}
			Self::InvalidPreprocessor { value, reason } => {
				write!(
					f,
					"Invalid lsp.knip.settings.preprocessor entry '{value}': {reason}. Valid entries are local paths starting with './', package names, or package subpaths."
				)
			}
			Self::InvalidPreprocessorOptions { reason } => {
				write!(f, "Invalid lsp.knip.settings.preprocessor_options: {reason}.")
			}
		}
	}
}

impl std::error::Error for KnipSettingsError {}

#[cfg(test)]
mod tests {
	use super::{DiagnosticSeverity, KnipDiagnosticsSettings, KnipSettings, KnipSettingsError, LogLevel, Value};
	use std::str::FromStr;

	#[test]
	fn settings_merge_default_values() {
		let settings = KnipSettings::default();

		assert_eq!(
			settings,
			KnipSettings {
				binary_path: None,
				auto_install: true,
				log_level: LogLevel::Info,
				config_path: None,
				require_config: false,
				ts_config_path: None,
				diagnostics: KnipDiagnosticsSettings::default(),
				package_manager: None,
				preprocessor: Vec::new(),
				preprocessor_options: None,
			}
		);
	}

	#[test]
	fn settings_merge_user_override_auto_install() {
		let settings = KnipSettings::default().merge(KnipSettings {
			auto_install: false,
			..KnipSettings::default()
		});

		assert!(!settings.auto_install);
	}

	#[test]
	fn settings_invalid_empty_binary_path() {
		let settings = KnipSettings {
			binary_path: Some(String::new()),
			..KnipSettings::default()
		};

		assert_eq!(settings.validate(), Err(KnipSettingsError::EmptyBinaryPath));
	}

	#[test]
	fn settings_invalid_empty_config_path() {
		let settings = KnipSettings {
			config_path: Some(String::new()),
			..KnipSettings::default()
		};

		assert_eq!(settings.validate(), Err(KnipSettingsError::EmptyConfigPath));
	}

	#[test]
	fn settings_invalid_log_level_parse() {
		assert_eq!(
			LogLevel::from_str("garbage"),
			Err(KnipSettingsError::InvalidLogLevel("garbage".to_string()))
		);
	}

	#[test]
	fn settings_accept_supported_editor_workflow_settings() {
		let settings = KnipSettings {
			binary_path: Some("/usr/local/bin/knip-language-server".to_string()),
			auto_install: false,
			config_path: Some("knip.config.ts".to_string()),
			require_config: true,
			ts_config_path: Some("tsconfig.app.json".to_string()),
			diagnostics: KnipDiagnosticsSettings {
				include_issue_types: vec!["exports".to_string(), "types".to_string()],
				exclude_issue_types: vec!["files".to_string()],
				exclude_path_prefixes: vec!["src/generated/".to_string()],
				severity_by_issue_type: [("dependencies".to_string(), DiagnosticSeverity::Warning)]
					.into_iter()
					.collect(),
			},
			..KnipSettings::default()
		};

		assert!(settings.validate().is_ok());
		assert!(settings.requires_managed_patch());
	}

	#[test]
	fn settings_requires_managed_patch_true_for_ts_config_path() {
		let settings = KnipSettings {
			ts_config_path: Some("tsconfig.app.json".to_string()),
			..KnipSettings::default()
		};

		assert!(settings.requires_managed_patch());
	}

	#[test]
	fn settings_requires_managed_patch_true_for_non_default_diagnostics() {
		let settings = KnipSettings {
			diagnostics: KnipDiagnosticsSettings {
				include_issue_types: vec!["exports".to_string()],
				..KnipDiagnosticsSettings::default()
			},
			..KnipSettings::default()
		};

		assert!(settings.requires_managed_patch());
	}

	#[test]
	fn settings_requires_managed_patch_false_for_baseline() {
		assert!(!KnipSettings::default().requires_managed_patch());
	}

	#[test]
	fn settings_reject_invalid_diagnostics_filters() {
		// Invalid issue type in include list.
		let bad_include = KnipSettings {
			diagnostics: KnipDiagnosticsSettings {
				include_issue_types: vec!["not_a_real_type".to_string()],
				..KnipDiagnosticsSettings::default()
			},
			..KnipSettings::default()
		};
		assert!(
			matches!(bad_include.validate(), Err(KnipSettingsError::InvalidIssueType { .. })),
			"expected InvalidIssueType for unknown include_issue_types entry"
		);

		// Invalid issue type in exclude list.
		let bad_exclude = KnipSettings {
			diagnostics: KnipDiagnosticsSettings {
				exclude_issue_types: vec!["badType".to_string()],
				..KnipDiagnosticsSettings::default()
			},
			..KnipSettings::default()
		};
		assert!(matches!(
			bad_exclude.validate(),
			Err(KnipSettingsError::InvalidIssueType { .. })
		));

		// Invalid severity string via FromStr.
		assert!(
			matches!(
				DiagnosticSeverity::from_str("offX"),
				Err(KnipSettingsError::InvalidSeverityValue { .. })
			),
			"expected InvalidSeverityValue for unrecognised severity string"
		);

		// Invalid severity with issue type context via parse_with_type.
		let err = DiagnosticSeverity::parse_with_type("exports", "offX").unwrap_err();
		assert!(
			matches!(&err, KnipSettingsError::InvalidSeverityValue { issue_type, value }
				if issue_type == "exports" && value == "offX"),
			"parse_with_type must populate issue_type, got: {err}"
		);

		// Empty path prefix.
		let empty_prefix = KnipSettings {
			diagnostics: KnipDiagnosticsSettings {
				exclude_path_prefixes: vec![String::new()],
				..KnipDiagnosticsSettings::default()
			},
			..KnipSettings::default()
		};
		assert!(matches!(
			empty_prefix.validate(),
			Err(KnipSettingsError::InvalidPathPrefix { .. })
		));

		// Absolute path prefix.
		let abs_prefix = KnipSettings {
			diagnostics: KnipDiagnosticsSettings {
				exclude_path_prefixes: vec!["/absolute/path".to_string()],
				..KnipDiagnosticsSettings::default()
			},
			..KnipSettings::default()
		};
		assert!(matches!(
			abs_prefix.validate(),
			Err(KnipSettingsError::InvalidPathPrefix { .. })
		));

		// Path prefix containing "..".
		let dotdot_prefix = KnipSettings {
			diagnostics: KnipDiagnosticsSettings {
				exclude_path_prefixes: vec!["../up".to_string()],
				..KnipDiagnosticsSettings::default()
			},
			..KnipSettings::default()
		};
		assert!(matches!(
			dotdot_prefix.validate(),
			Err(KnipSettingsError::InvalidPathPrefix { .. })
		));
	}

	#[test]
	fn preprocessor_settings_accept_local_path() {
		let settings = KnipSettings {
			preprocessor: vec!["./tools/preprocess.mjs".to_string()],
			..KnipSettings::default()
		};
		assert!(settings.validate().is_ok());
		assert!(settings.requires_managed_patch());
	}

	#[test]
	fn preprocessor_settings_accept_package_name() {
		let settings = KnipSettings {
			preprocessor: vec!["preprocessor-package".to_string()],
			..KnipSettings::default()
		};
		assert!(settings.validate().is_ok());
	}

	#[test]
	fn preprocessor_settings_accept_package_subpath() {
		let settings = KnipSettings {
			preprocessor: vec!["pkg/subpath".to_string()],
			..KnipSettings::default()
		};
		assert!(settings.validate().is_ok());
	}

	#[test]
	fn preprocessor_settings_accept_scoped_package_subpath() {
		let settings = KnipSettings {
			preprocessor: vec!["@scope/pkg/subpath".to_string()],
			..KnipSettings::default()
		};
		assert!(settings.validate().is_ok());
	}

	#[test]
	fn preprocessor_settings_reject_parent_dir_reference() {
		let settings = KnipSettings {
			preprocessor: vec!["../x.js".to_string()],
			..KnipSettings::default()
		};
		assert!(matches!(
			settings.validate(),
			Err(KnipSettingsError::InvalidPreprocessor { .. })
		));
	}

	#[test]
	fn preprocessor_settings_reject_absolute_path() {
		let settings = KnipSettings {
			preprocessor: vec!["/x.js".to_string()],
			..KnipSettings::default()
		};
		assert!(matches!(
			settings.validate(),
			Err(KnipSettingsError::InvalidPreprocessor { .. })
		));
	}

	#[test]
	fn preprocessor_settings_reject_home_path() {
		let settings = KnipSettings {
			preprocessor: vec!["~/x.js".to_string()],
			..KnipSettings::default()
		};
		assert!(matches!(
			settings.validate(),
			Err(KnipSettingsError::InvalidPreprocessor { .. })
		));
	}

	#[test]
	fn preprocessor_settings_reject_file_protocol() {
		let settings = KnipSettings {
			preprocessor: vec!["file:x.js".to_string()],
			..KnipSettings::default()
		};
		assert!(matches!(
			settings.validate(),
			Err(KnipSettingsError::InvalidPreprocessor { .. })
		));
	}

	#[test]
	fn preprocessor_settings_reject_http_protocol() {
		let settings = KnipSettings {
			preprocessor: vec!["http://x".to_string()],
			..KnipSettings::default()
		};
		assert!(matches!(
			settings.validate(),
			Err(KnipSettingsError::InvalidPreprocessor { .. })
		));
	}

	#[test]
	fn preprocessor_settings_reject_https_protocol() {
		let settings = KnipSettings {
			preprocessor: vec!["https://x".to_string()],
			..KnipSettings::default()
		};
		assert!(matches!(
			settings.validate(),
			Err(KnipSettingsError::InvalidPreprocessor { .. })
		));
	}

	#[test]
	fn preprocessor_settings_reject_empty_string() {
		let settings = KnipSettings {
			preprocessor: vec![String::new()],
			..KnipSettings::default()
		};
		assert!(matches!(
			settings.validate(),
			Err(KnipSettingsError::InvalidPreprocessor { .. })
		));
	}

	#[test]
	fn preprocessor_settings_reject_node_protocol() {
		let settings = KnipSettings {
			preprocessor: vec!["node:fs".to_string()],
			..KnipSettings::default()
		};
		assert!(matches!(
			settings.validate(),
			Err(KnipSettingsError::InvalidPreprocessor { .. })
		));
	}

	#[test]
	fn preprocessor_settings_reject_data_protocol() {
		let settings = KnipSettings {
			preprocessor: vec!["data:text/plain".to_string()],
			..KnipSettings::default()
		};
		assert!(matches!(
			settings.validate(),
			Err(KnipSettingsError::InvalidPreprocessor { .. })
		));
	}

	#[test]
	fn preprocessor_options_requires_preprocessor() {
		use std::collections::BTreeMap;
		let mut options = BTreeMap::new();
		options.insert("key".to_string(), Value::String("value".to_string()));

		let settings = KnipSettings {
			preprocessor_options: Some(options),
			..KnipSettings::default()
		};
		assert!(matches!(
			settings.validate(),
			Err(KnipSettingsError::InvalidPreprocessorOptions { .. })
		));
	}

	#[test]
	fn preprocessor_options_valid_with_preprocessor() {
		use std::collections::BTreeMap;
		let mut options = BTreeMap::new();
		options.insert("key".to_string(), Value::String("value".to_string()));

		let settings = KnipSettings {
			preprocessor: vec!["preprocessor-package".to_string()],
			preprocessor_options: Some(options),
			..KnipSettings::default()
		};
		assert!(settings.validate().is_ok());
		assert!(settings.requires_managed_patch());
	}

	#[test]
	fn preprocessor_merge_empty_override_keeps_base() {
		let base = KnipSettings {
			preprocessor: vec!["base-prep".to_string()],
			..KnipSettings::default()
		};
		let merged = base.merge(KnipSettings {
			preprocessor: Vec::new(),
			..KnipSettings::default()
		});
		assert_eq!(merged.preprocessor, vec!["base-prep".to_string()]);
	}

	#[test]
	fn preprocessor_merge_non_empty_override_replaces_base() {
		let base = KnipSettings {
			preprocessor: vec!["base-prep".to_string()],
			..KnipSettings::default()
		};
		let merged = base.merge(KnipSettings {
			preprocessor: vec!["override-prep".to_string()],
			..KnipSettings::default()
		});
		assert_eq!(merged.preprocessor, vec!["override-prep".to_string()]);
	}

	#[test]
	fn preprocessor_options_merge_none_keeps_base() {
		use std::collections::BTreeMap;
		let mut base_options = BTreeMap::new();
		base_options.insert("key".to_string(), Value::String("value".to_string()));

		let base = KnipSettings {
			preprocessor: vec!["prep".to_string()],
			preprocessor_options: Some(base_options),
			..KnipSettings::default()
		};
		let merged = base.merge(KnipSettings {
			preprocessor_options: None,
			..KnipSettings::default()
		});
		assert!(merged.preprocessor_options.is_some());
	}

	#[test]
	fn preprocessor_options_merge_some_replaces_base() {
		use std::collections::BTreeMap;
		let mut base_options = BTreeMap::new();
		base_options.insert("base-key".to_string(), Value::String("base-value".to_string()));

		let mut override_options = BTreeMap::new();
		override_options.insert("override-key".to_string(), Value::String("override-value".to_string()));

		let base = KnipSettings {
			preprocessor: vec!["prep".to_string()],
			preprocessor_options: Some(base_options),
			..KnipSettings::default()
		};
		let merged = base.merge(KnipSettings {
			preprocessor_options: Some(override_options),
			..KnipSettings::default()
		});
		assert!(merged.preprocessor_options.is_some());
		let opts = merged.preprocessor_options.unwrap();
		assert!(opts.contains_key("override-key"));
		assert!(!opts.contains_key("base-key"));
	}

	#[test]
	fn preprocessor_advanced_settings_list_includes_preprocessor() {
		let settings = KnipSettings {
			preprocessor: vec!["prep".to_string()],
			..KnipSettings::default()
		};
		let advanced = settings.advanced_settings_list();
		assert!(advanced.contains(&"lsp.knip.settings.preprocessor"));
	}

	#[test]
	fn preprocessor_advanced_settings_list_includes_preprocessor_options() {
		use std::collections::BTreeMap;
		let mut options = BTreeMap::new();
		options.insert("key".to_string(), Value::String("value".to_string()));

		let settings = KnipSettings {
			preprocessor: vec!["prep".to_string()],
			preprocessor_options: Some(options),
			..KnipSettings::default()
		};
		let advanced = settings.advanced_settings_list();
		assert!(advanced.contains(&"lsp.knip.settings.preprocessor_options"));
	}

	#[test]
	fn preprocessor_requires_managed_patch_when_non_empty() {
		let settings = KnipSettings {
			preprocessor: vec!["prep".to_string()],
			..KnipSettings::default()
		};
		assert!(settings.requires_managed_patch());
	}

	#[test]
	fn preprocessor_requires_managed_patch_false_when_empty() {
		let settings = KnipSettings {
			preprocessor: Vec::new(),
			..KnipSettings::default()
		};
		assert!(!settings.requires_managed_patch());
	}
}
