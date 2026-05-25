import { RuleConfigSeverity } from "@commitlint/types";

import type { UserConfig } from "@commitlint/types";

const configuration: UserConfig = {
	defaultIgnores: true,
	extends: ["@commitlint/config-conventional"],
	helpUrl: "https://github.com/howmanysmall/zed-knip/blob/main/CONTRIBUTING.md",
	rules: {
		"body-leading-blank": [RuleConfigSeverity.Error, "always"],
		"body-max-line-length": [RuleConfigSeverity.Error, "always", 72],
		"body-min-length": [RuleConfigSeverity.Warning, "always", 20],
		"footer-leading-blank": [RuleConfigSeverity.Error, "always"],
		"footer-max-line-length": [RuleConfigSeverity.Error, "always", 72],
		"header-max-length": [RuleConfigSeverity.Error, "always", 72],
		"references-empty": [RuleConfigSeverity.Disabled, "never"],
		"scope-case": [RuleConfigSeverity.Error, "always", ["lower-case"]],
		"scope-empty": [RuleConfigSeverity.Disabled, "always"],
		"scope-max-length": [RuleConfigSeverity.Error, "always", 30],
		"scope-min-length": [RuleConfigSeverity.Warning, "always", 2],
		"subject-case": [RuleConfigSeverity.Error, "never", ["start-case", "pascal-case", "upper-case"]],
		"subject-empty": [RuleConfigSeverity.Error, "never"],
		"subject-full-stop": [RuleConfigSeverity.Error, "never", "."],
		"subject-max-length": [RuleConfigSeverity.Error, "always", 72],
		"subject-min-length": [RuleConfigSeverity.Error, "always", 10],
		"type-case": [RuleConfigSeverity.Error, "always", ["lower-case"]],
		"type-empty": [RuleConfigSeverity.Error, "never"],
		"type-enum": [
			RuleConfigSeverity.Error,
			"always",
			["build", "chore", "ci", "docs", "feat", "fix", "perf", "refactor", "revert", "style", "test"],
		],
	},
};

export default configuration;
