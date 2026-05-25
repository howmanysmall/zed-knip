import { createCamelCaseDetector } from "./camel-case-detector";
import { createContainsDetector } from "./contains-detector";
import { createEndWithDetector } from "./end-with-detector";
import { createKeywordsDetector } from "./keywords-detector";

import type { Detector } from "./detector";

const JS_KEYWORDS = [
	"public",
	"abstract",
	"class",
	"implements",
	"extends",
	"return",
	"throw",
	"private",
	"protected",
	"enum",
	"continue",
	"assert",
	"boolean",
	"this",
	"instanceof",
	"interface",
	"static",
	"void",
	"super",
	"true",
	"case:",
	"let",
	"const",
	"var",
	"async",
	"await",
	"break",
	"yield",
	"typeof",
	"import",
	"export",
] as const;

const OPERATORS = ["++", "||", "&&", "===", "?.", "??"] as const;

const CODE_PATTERNS: ReadonlyArray<RegExp | string> = [
	"for(",
	"if(",
	"while(",
	"catch(",
	"switch(",
	"try{",
	"else{",
	"this.",
	"window.",
	/;\s+\/\//u,
	"import '",
	'import "',
	"require(",
];

const LINE_ENDINGS = ["}", ";", "{"] as const;

/**
 * Creates a set of detectors for identifying JavaScript/TypeScript code patterns.
 *
 * @returns Array of configured detectors.
 */
export function createJavaScriptDetectors(): ReadonlyArray<Detector> {
	return [
		createEndWithDetector(0.95, LINE_ENDINGS),
		createKeywordsDetector(0.7, OPERATORS),
		createKeywordsDetector(0.3, JS_KEYWORDS),
		createContainsDetector(0.95, CODE_PATTERNS),
		createCamelCaseDetector(0.5),
	];
}
