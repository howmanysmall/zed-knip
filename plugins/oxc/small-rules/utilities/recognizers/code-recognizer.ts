import { recognize } from "./detector";

import type { Detector } from "./detector";

const PROBABILITY_THRESHOLD = 0.9;

/**
 * Calculate combined probability that a line contains code. Uses: `p = 1 - (1-p1)(1-p2)...(1-pn)` for calculations.
 *
 * @param detectors - Array of detectors to use.
 * @param line - The line to analyze.
 * @returns Combined probability between 0 and 1.
 */
function computeProbability(detectors: ReadonlyArray<Detector>, line: string): number {
	let probability = 0;

	for (const detector of detectors) {
		const detected = recognize(detector, line);
		probability = 1 - (1 - probability) * (1 - detected);
	}

	return probability;
}

/**
 * Check if a line is likely code based on threshold.
 *
 * @param detectors - Array of detectors to use.
 * @param line - The line to check.
 * @returns True if probability >= threshold.
 */
function isLikelyCode(detectors: ReadonlyArray<Detector>, line: string): boolean {
	return computeProbability(detectors, line) >= PROBABILITY_THRESHOLD;
}

/**
 * Check if any line in the array is likely code.
 *
 * @param detectors - Array of detectors to use.
 * @param lines - Array of lines to check.
 * @returns True if at least one line meets the threshold.
 */
export function hasCodeLines(detectors: ReadonlyArray<Detector>, lines: ReadonlyArray<string>): boolean {
	return lines.some((line) => isLikelyCode(detectors, line));
}
