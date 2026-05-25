import type { Detector } from "./detector";

const ESCAPE = /[-/^$*+?.()|[\]{}]/gu;
const REGEXP_ESCAPED = String.raw`\$&`;
const WHITESPACE_GLOBAL_REGEX = /\s+/gu;

function escapeForRegex(value: string): string {
	return value.replaceAll(ESCAPE, REGEXP_ESCAPED);
}

/**
 * Creates a detector that finds patterns in compressed text (whitespace removed). Supports both string literals and
 * RegExp patterns.
 *
 * @param probability - Base probability (0-1).
 * @param patterns - Patterns to detect (strings are escaped, RegExp used as-is).
 * @returns Detector instance.
 */
export function createContainsDetector(probability: number, patterns: ReadonlyArray<RegExp | string>): Detector {
	const compiledPatterns = patterns.map((pattern) =>
		typeof pattern === "string" ? new RegExp(escapeForRegex(pattern), "gu") : new RegExp(pattern.source, "gu"),
	);

	return {
		probability,
		scan(line: string): number {
			const compressed = line.replace(WHITESPACE_GLOBAL_REGEX, "");
			let total = 0;

			for (const pattern of compiledPatterns) {
				pattern.lastIndex = 0;
				const matches = compressed.match(pattern);
				if (matches) total += matches.length;
			}

			return total;
		},
	};
}
