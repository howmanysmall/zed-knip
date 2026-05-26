pub mod cache;
pub mod config_detection;
pub mod errors;
pub mod logging;
pub mod managed_install;
pub mod package_manager;
pub mod reports;
pub mod resolver;
pub mod settings;

use zed_extension_api as zed;

pub struct ZedKnipExtension;

impl zed::Extension for ZedKnipExtension {
	fn new() -> Self {
		Self
	}
}

zed::register_extension!(ZedKnipExtension);

#[cfg(test)]
const MODULE_COUNT: usize = 9;

#[cfg(test)]
mod tests {
	#[test]
	fn scaffold_smoke_test() {
		assert_eq!(super::MODULE_COUNT, 9);
	}
}
