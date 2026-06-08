//! Extension logging utilities and non-blocking install UX model.
//!
//! All output goes through `eprintln!` which is WASM-compatible and does not
//! block the Zed UI thread. Callers hold a [`Logger`] configured with the
//! active [`LogLevel`] and call the level-specific helpers; messages below the
//! configured level are silently dropped.
//!
//! Install progress is modelled as a pure-data [`InstallProgress`] enum whose
//! [`fmt::Display`] impl produces the user-visible string. Callers emit the string
//! through the logger at [`LogLevel::Info`] so the UX path is also filtered by
//! the user's log-level preference.

use std::fmt;

use crate::settings::LogLevel;

// ── Logger ────────────────────────────────────────────────────────────────────

/// A lightweight, non-blocking logger for the Knip Zed extension.
///
/// Output is written to stderr via `eprintln!`, which is safe from WASM and
/// never blocks the Zed UI thread. Messages whose level is below the
/// configured [`LogLevel`] are dropped without allocation.
///
/// # Example
///
/// ```
/// use zed_knip::logging::Logger;
/// use zed_knip::settings::LogLevel;
///
/// let logger = Logger::new(LogLevel::Info);
/// logger.info("Knip extension activated");
/// logger.debug("this is suppressed at Info level");
/// ```
#[derive(Debug, Clone)]
pub struct Logger {
	level: LogLevel,
}

impl Logger {
	/// Create a new logger that emits messages at or above `level`.
	#[must_use]
	pub fn new(level: LogLevel) -> Self {
		Self { level }
	}

	/// Return the currently configured minimum log level.
	#[must_use]
	pub fn level(&self) -> LogLevel {
		self.level
	}

	/// Update the minimum log level.
	pub fn set_level(&mut self, level: LogLevel) {
		self.level = level;
	}

	/// Emit a trace-level message (most verbose).
	pub fn trace(&self, msg: &str) {
		self.emit(LogLevel::Trace, msg);
	}

	/// Emit a debug-level message.
	pub fn debug(&self, msg: &str) {
		self.emit(LogLevel::Debug, msg);
	}

	/// Emit an info-level message.
	pub fn info(&self, msg: &str) {
		self.emit(LogLevel::Info, msg);
	}

	/// Emit a warn-level message.
	pub fn warn(&self, msg: &str) {
		self.emit(LogLevel::Warn, msg);
	}

	/// Emit an error-level message.
	pub fn error(&self, msg: &str) {
		self.emit(LogLevel::Error, msg);
	}

	/// Format and emit `msg` at `level`, dropping it if below the threshold.
	fn emit(&self, level: LogLevel, msg: &str) {
		if level >= self.level {
			eprintln!("[knip] {level}: {msg}");
		}
	}

	/// Emit an [`InstallProgress`] event at info level.
	///
	/// This is the primary hook for the non-blocking install UX: callers
	/// construct an [`InstallProgress`] value and pass it here; the logger
	/// formats it and writes to stderr without touching the Zed UI thread.
	pub fn install_progress(&self, progress: &InstallProgress) {
		self.info(&progress.to_string());
	}
}

// ── LogLevel ordering ─────────────────────────────────────────────────────────

impl PartialOrd for LogLevel {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for LogLevel {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.severity().cmp(&other.severity())
	}
}

impl LogLevel {
	/// Numeric severity: higher = more severe, less verbose.
	fn severity(self) -> u8 {
		match self {
			Self::Trace => 0,
			Self::Debug => 1,
			Self::Info => 2,
			Self::Warn => 3,
			Self::Error => 4,
		}
	}
}

// ── InstallProgress ───────────────────────────────────────────────────────────

/// Non-blocking install UX model.
///
/// Each variant represents a discrete, user-visible stage of the managed Knip
/// install flow. Values are pure data — no I/O, no blocking — so callers can
/// construct them on any code path and hand them to [`Logger::install_progress`]
/// when they want to surface the message.
///
/// The [`fmt::Display`] impl produces the string shown to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallProgress {
	/// Checking whether a cached install is still valid.
	CheckingCache,
	/// A valid cached install was found; no download needed.
	CacheHit { version: String },
	/// No valid cache; starting a fresh download.
	Downloading { version: String },
	/// Download finished; verifying the binary.
	Verifying,
	/// Binary verified and ready to use.
	Ready { path: String },
	/// Install failed with a user-facing reason.
	Failed { reason: String },
}

impl fmt::Display for InstallProgress {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::CheckingCache => write!(f, "Knip: checking install cache…"),
			Self::CacheHit { version } => {
				write!(f, "Knip {version}: using cached install.")
			}
			Self::Downloading { version } => {
				write!(f, "Knip {version}: downloading managed install…")
			}
			Self::Verifying => write!(f, "Knip: verifying downloaded binary…"),
			Self::Ready { path } => write!(f, "Knip: ready at {path}."),
			Self::Failed { reason } => write!(f, "Knip install failed: {reason}."),
		}
	}
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
	use super::{InstallProgress, LogLevel, Logger};

	// ── Logger level filtering ────────────────────────────────────────────

	#[test]
	fn logger_emits_message_at_configured_level() {
		// Smoke-test: constructing a logger at Info and calling info() should
		// not panic. We cannot capture eprintln! output in unit tests, so we
		// verify the predicate that controls emission instead.
		let logger = Logger::new(LogLevel::Info);
		assert!(LogLevel::Info >= logger.level());
	}

	#[test]
	fn logger_suppresses_message_below_configured_level() {
		let logger = Logger::new(LogLevel::Warn);
		// Debug < Warn → should be suppressed.
		assert!(LogLevel::Debug < logger.level());
	}

	#[test]
	fn logger_emits_message_above_configured_level() {
		let logger = Logger::new(LogLevel::Debug);
		// Error > Debug → should be emitted.
		assert!(LogLevel::Error >= logger.level());
	}

	#[test]
	fn logger_set_level_updates_threshold() {
		let mut logger = Logger::new(LogLevel::Info);
		logger.set_level(LogLevel::Error);
		assert_eq!(logger.level(), LogLevel::Error);
		// Warn < Error → suppressed after update.
		assert!(LogLevel::Warn < logger.level());
	}

	#[test]
	fn logger_trace_level_emits_all() {
		let logger = Logger::new(LogLevel::Trace);
		// Every level is >= Trace.
		for level in [
			LogLevel::Trace,
			LogLevel::Debug,
			LogLevel::Info,
			LogLevel::Warn,
			LogLevel::Error,
		] {
			assert!(level >= logger.level(), "{level} should be >= Trace");
		}
	}

	#[test]
	fn logger_error_level_suppresses_all_but_error() {
		let logger = Logger::new(LogLevel::Error);
		for level in [LogLevel::Trace, LogLevel::Debug, LogLevel::Info, LogLevel::Warn] {
			assert!(level < logger.level(), "{level} should be < Error");
		}
		assert!(LogLevel::Error >= logger.level());
	}

	// ── LogLevel ordering ─────────────────────────────────────────────────

	#[test]
	fn log_level_ordering_trace_is_lowest() {
		assert!(LogLevel::Trace < LogLevel::Debug);
		assert!(LogLevel::Trace < LogLevel::Info);
		assert!(LogLevel::Trace < LogLevel::Warn);
		assert!(LogLevel::Trace < LogLevel::Error);
	}

	#[test]
	fn log_level_ordering_error_is_highest() {
		assert!(LogLevel::Error > LogLevel::Warn);
		assert!(LogLevel::Error > LogLevel::Info);
		assert!(LogLevel::Error > LogLevel::Debug);
		assert!(LogLevel::Error > LogLevel::Trace);
	}

	#[test]
	fn log_level_ordering_same_level_is_equal() {
		assert_eq!(LogLevel::Info, LogLevel::Info);
		assert!(LogLevel::Info >= LogLevel::Info);
		assert!(LogLevel::Info <= LogLevel::Info);
	}

	// ── InstallProgress display ───────────────────────────────────────────

	#[test]
	fn install_progress_checking_cache_display() {
		assert_eq!(
			InstallProgress::CheckingCache.to_string(),
			"Knip: checking install cache…"
		);
	}

	#[test]
	fn install_progress_cache_hit_display_includes_version() {
		let msg = InstallProgress::CacheHit {
			version: "5.33.0".to_string(),
		}
		.to_string();
		assert_eq!(msg, "Knip 5.33.0: using cached install.");
	}

	#[test]
	fn install_progress_downloading_display_includes_version() {
		let msg = InstallProgress::Downloading {
			version: "5.33.0".to_string(),
		}
		.to_string();
		assert_eq!(msg, "Knip 5.33.0: downloading managed install…");
	}

	#[test]
	fn install_progress_verifying_display() {
		assert_eq!(
			InstallProgress::Verifying.to_string(),
			"Knip: verifying downloaded binary…"
		);
	}

	#[test]
	fn install_progress_ready_display_includes_path() {
		let msg = InstallProgress::Ready {
			path: "/home/user/.cache/knip/bin".to_string(),
		}
		.to_string();
		assert_eq!(msg, "Knip: ready at /home/user/.cache/knip/bin.");
	}

	#[test]
	fn install_progress_failed_display_includes_reason() {
		let msg = InstallProgress::Failed {
			reason: "network timeout".to_string(),
		}
		.to_string();
		assert_eq!(msg, "Knip install failed: network timeout.");
	}

	// ── Logger::install_progress integration ─────────────────────────────

	#[test]
	fn logger_install_progress_does_not_panic() {
		let logger = Logger::new(LogLevel::Info);
		// Should not panic; output goes to stderr.
		logger.install_progress(&InstallProgress::CheckingCache);
		logger.install_progress(&InstallProgress::CacheHit {
			version: "5.0.0".to_string(),
		});
		logger.install_progress(&InstallProgress::Downloading {
			version: "5.0.0".to_string(),
		});
		logger.install_progress(&InstallProgress::Verifying);
		logger.install_progress(&InstallProgress::Ready {
			path: "/tmp/knip".to_string(),
		});
		logger.install_progress(&InstallProgress::Failed {
			reason: "test".to_string(),
		});
	}

	#[test]
	fn logger_install_progress_suppressed_when_level_above_info() {
		// At Error level, info messages (including install progress) are suppressed.
		// We verify the predicate: Info < Error.
		let logger = Logger::new(LogLevel::Error);
		assert!(LogLevel::Info < logger.level());
	}
}
