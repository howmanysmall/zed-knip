import type { Detector } from "./detector";

const WORD_SPLIT_REGEX = /[ \t(),{}]/u;

/**
 * Creates a detector that counts keyword occurrences. Splits line by word boundaries and counts matches.
 *
 * @param probability - Base probability (0-1).
 * @param keywords - Keywords to detect.
 * @returns Detector instance.
 */
export function createKeywordsDetector(probability: number, keywords: ReadonlyArray<string>): Detector {
	const keywordsSet = new Set(keywords);

	return {
		probability,
		scan(line: string): number {
			const words = line.split(WORD_SPLIT_REGEX);
			let count = 0;

			for (const word of words) if (keywordsSet.has(word)) count += 1;

			return count;
		},
	};
}
