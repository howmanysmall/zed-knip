use std::{fmt, str::FromStr};

/// User-facing Knip settings for the Zed extension.
///
/// Defaults match the extension's baseline behavior; callers can merge a user
/// override struct on top of [`KnipSettings::default()`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnipSettings {
	/// Explicit path to the Knip language server binary.
	///
	/// Default: `None`.
	pub server_path: Option<String>,
	/// Override for the detected package manager.
	///
	/// Default: `None`.
	pub package_manager: Option<String>,
	/// Enable managed installation of Knip.
	///
	/// Default: `true`.
	pub auto_install: bool,
	/// Log verbosity for the Knip language server.
	///
	/// Default: [`LogLevel::Info`].
	pub log_level: LogLevel,
	/// Explicit Knip config path.
	///
	/// Default: `None`.
	pub config_path: Option<String>,
	/// Require a Knip config file to exist.
	///
	/// Default: `false`.
	pub require_config: bool,
}

impl Default for KnipSettings {
	fn default() -> Self {
		Self {
			server_path: None,
			package_manager: None,
			auto_install: true,
			log_level: LogLevel::Info,
			config_path: None,
			require_config: false,
		}
	}
}

impl KnipSettings {
	/// Merge `overrides` on top of `self`.
	///
	/// `Option` fields keep the base value when the override is `None`.
	pub fn merge(self, overrides: Self) -> Self {
		Self {
			server_path: overrides.server_path.or(self.server_path),
			package_manager: overrides.package_manager.or(self.package_manager),
			auto_install: overrides.auto_install,
			log_level: overrides.log_level,
			config_path: overrides.config_path.or(self.config_path),
			require_config: overrides.require_config,
		}
	}

	/// Validate user-provided settings values.
	pub fn validate(&self) -> Result<(), KnipSettingsError> {
		if matches!(self.server_path.as_deref(), Some("")) {
			return Err(KnipSettingsError::EmptyServerPath);
		}

		if matches!(self.config_path.as_deref(), Some("")) {
			return Err(KnipSettingsError::EmptyConfigPath);
		}

		Ok(())
	}
}

/// Knip language-server log levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
	Trace,
	Debug,
	Info,
	Warn,
	Error,
}

impl LogLevel {
	fn as_str(self) -> &'static str {
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

/// Errors produced while validating Knip settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnipSettingsError {
	/// The explicit Knip binary path was set to an empty string.
	EmptyServerPath,
	/// The explicit Knip config path was set to an empty string.
	EmptyConfigPath,
	/// The requested log level could not be parsed.
	InvalidLogLevel(String),
}

impl fmt::Display for KnipSettingsError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::EmptyServerPath => write!(f, "The Knip server path cannot be empty."),
			Self::EmptyConfigPath => write!(f, "The Knip config path cannot be empty."),
			Self::InvalidLogLevel(level) => {
				write!(
					f,
					"Invalid Knip log level '{level}'. Use trace, debug, info, warn, or error."
				)
			}
		}
	}
}

impl std::error::Error for KnipSettingsError {}

#[cfg(test)]
mod tests {
	use super::{KnipSettings, KnipSettingsError, LogLevel};
	use std::str::FromStr;

	#[test]
	fn settings_merge_default_values() {
		let settings = KnipSettings::default();

		assert_eq!(
			settings,
			KnipSettings {
				server_path: None,
				package_manager: None,
				auto_install: true,
				log_level: LogLevel::Info,
				config_path: None,
				require_config: false,
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
	fn settings_invalid_empty_server_path() {
		let settings = KnipSettings {
			server_path: Some(String::new()),
			..KnipSettings::default()
		};

		assert_eq!(settings.validate(), Err(KnipSettingsError::EmptyServerPath));
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
}
