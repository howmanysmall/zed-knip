import type { Detector } from "./detector";

/**
 * Creates a detector for camelCase patterns. Returns 1 if lowercase char followed by uppercase char is found.
 *
 * @param probability - Base probability (0-1).
 * @returns Detector instance.
 */
export function createCamelCaseDetector(probability: number): Detector {
	return {
		probability,
		scan(line: string): number {
			for (let index = 0; index < line.length - 1; index += 1) {
				const current = line.charAt(index);
				const next = line.charAt(index + 1);

				if (current === current.toLowerCase() && next === next.toUpperCase() && next !== next.toLowerCase()) {
					return 1;
				}
			}

			return 0;
		},
	};
}
